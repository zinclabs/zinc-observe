// Copyright 2023 Zinc Labs Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

import {
  ref,
  watch,
  reactive,
  toRefs,
  onMounted,
  onUnmounted,
  inject,
} from "vue";
import queryService from "../../services/search";
import { useStore } from "vuex";
import { addLabelToPromQlQuery } from "@/utils/query/promQLUtils";
import { addLabelsToSQlQuery } from "@/utils/query/sqlUtils";
import { getStreamFromQuery } from "@/utils/query/sqlUtils";
import {
  formatInterval,
  formateRateInterval,
  getTimeInSecondsBasedOnUnit,
} from "@/utils/dashboard/variables/variablesUtils";

export const usePanelDataLoader = (
  panelSchema: any,
  selectedTimeObj: any,
  variablesData: any,
  chartPanelRef: any,
  forceLoad: any
) => {
  const log = (...args: any[]) => {
    // if (true) {
    //   console.log(panelSchema?.value?.title + ": ", ...args);
    // }
  };

  const state = reactive({
    data: [] as any,
    loading: false,
    errorDetail: "",
    metadata: {},
  });

  // observer for checking if panel is visible on the screen
  let observer: any = null;

  // is panel currently visible or not
  const isVisible: any = ref(false);

  // currently dependent variables data
  let currentDependentVariablesData = variablesData.value?.values
    ? JSON.parse(
        JSON.stringify(
          variablesData.value?.values
            ?.filter((it: any) => it.type != "dynamic_filters") // ad hoc filters are not considered as dependent filters as they are globally applied
            ?.filter((it: any) => {
              const regexForVariable = new RegExp(
                `.*\\$\\{?${it.name}(?::(csv|pipe|doublequote|singlequote))?}?.*`
              );

              return panelSchema.value.queries
                ?.map((q: any) => regexForVariable.test(q?.query))
                ?.includes(true);
            })
        )
      )
    : [];

  // console.log(
  //   "variablesData.value currentAdHocVariablesData",
  //   JSON.parse(JSON.stringify(variablesData.value))
  // );

  // console.log(
  //   "variablesData.value.values currentAdHocVariablesData",
  //   JSON.parse(JSON.stringify(variablesData.value.values))
  // );

  let currentDynamicVariablesData = variablesData.value?.values
    ? JSON.parse(
        JSON.stringify(
          variablesData.value?.values
            ?.filter((it: any) => it.type === "dynamic_filters")
            ?.map((it: any) => it?.value)
            ?.flat()
            ?.filter((it: any) => it?.operator && it?.name && it?.value)
        )
      )
    : [];
  // let currentAdHocVariablesData: any = null;
  // console.log("currentAdHocVariablesData", currentDynamicVariablesData);

  const store = useStore();

  let abortController = new AbortController();

  // [START] --------- New Functions ------------------------------------------
  // an async function that waits for the panel to become visible
  const waitForThePanelToBecomeVisible = (signal: any) => {
    return new Promise<void>((resolve, reject) => {
      // Immediately resolve if forceLoad is true
      if (forceLoad.value == true) {
        resolve();
        return;
      }
      // Immediately resolve if isVisible is already true
      if (isVisible.value) {
        resolve();
        return;
      }

      // Watch for changes in isVisible
      const stopWatching = watch(isVisible, (newValue) => {
        if (newValue) {
          resolve();
          stopWatching(); // Stop watching once isVisible is true
        }
      });

      // Listen to the abort signal
      signal.addEventListener("abort", () => {
        stopWatching(); // Stop watching on abort
        reject(new Error("Aborted waiting for loading"));
      });
    });
  };

  // an async function that waits for the variables to load
  const waitForTheVariablesToLoad = (signal: any) => {
    return new Promise<void>((resolve, reject) => {
      // Immediately resolve if variables are already loaded
      if (ifPanelVariablesCompletedLoading()) {
        resolve();
        return;
      }

      // Watch for changes in isVisible
      const stopWatching = watch(
        () => variablesData.value?.values,
        () => {
          if (ifPanelVariablesCompletedLoading()) {
            resolve();
            stopWatching(); // Stop watching once isVisible is true
          }
        }
      );

      // Listen to the abort signal
      signal.addEventListener("abort", () => {
        stopWatching(); // Stop watching on abort
        reject(new Error("Aborted waiting for loading"));
      });
    });
  };

  const loadData = async () => {
    try {
      log("loadData: entering...");

      if (abortController) {
        log("logData: aborting previous function call (if any)");
        abortController.abort();
      }

      // Create a new AbortController for the new operation
      abortController = new AbortController();

      // Checking if there are queries to execute
      if (!panelSchema.value.queries?.length || !hasAtLeastOneQuery()) {
        log("loadData: there are no queries to execute");
        state.loading = false;
        state.data = [];
        state.metadata = {};
        return;
      }

      log("loadData: now waiting for the panel to become visible");

      // Wait for isVisible to become true
      await waitForThePanelToBecomeVisible(abortController.signal);

      log("loadData: now waiting for the variables to load");

      // Wait for variables to load
      await waitForTheVariablesToLoad(abortController.signal);

      log("loadData: good to go... starting query executions...");

      const timestamps = selectedTimeObj.value;
      let startISOTimestamp: any;
      let endISOTimestamp: any;
      if (
        timestamps?.start_time &&
        timestamps?.end_time &&
        timestamps.start_time != "Invalid Date" &&
        timestamps.end_time != "Invalid Date"
      ) {
        startISOTimestamp = new Date(
          timestamps.start_time.toISOString()
        ).getTime();
        endISOTimestamp = new Date(timestamps.end_time.toISOString()).getTime();
      } else {
        return;
      }

      state.loading = true;

      // Check if the query type is "promql"
      if (panelSchema.value.queryType == "promql") {
        // Iterate through each query in the panel schema
        const queryPromises = panelSchema.value.queries?.map(
          async (it: any) => {
            const { query: query1, metadata: metadata1 } = replaceQueryValue(
              it.query,
              startISOTimestamp,
              endISOTimestamp,
              panelSchema.value.queryType
            );
            // console.log("Calling queryPromises", query1);

            const { query: query2, metadata: metadata2 } =
              await applyDynamicVariables(query1, panelSchema.value.queryType);

            const query = query2;
            const metadata = {
              originalQuery: it.query,
              query: query,
              startTime: startISOTimestamp,
              endTime: endISOTimestamp,
              queryType: panelSchema.value.queryType,
              variables: [...(metadata1 || []), ...(metadata2 || [])],
            };
            // console.log("Calling metrics_query_range API");
            return queryService
              .metrics_query_range({
                org_identifier: store.state.selectedOrganization.identifier,
                query: query,
                start_time: startISOTimestamp,
                end_time: endISOTimestamp,
              })
              .then((res) => {
                state.errorDetail = "";
                // console.log("API response received");
                return { result: res.data.data, metadata: metadata };
              })
              .catch((error) => {
                // Process API error for "promql"
                processApiError(error, "promql");
                return { result: null, metadata: metadata };
              });
          }
        );

        // Wait for all query promises to resolve
        const queryResults = await Promise.all(queryPromises);
        state.loading = false;
        state.data = queryResults.map((it: any) => it?.result);
        state.metadata = {
          queries: queryResults.map((it) => it?.metadata),
        };
      } else {
        // Call search API

        // Get the page type from the first query in the panel schema
        const pageType = panelSchema.value.queries[0]?.fields?.stream_type;

        const sqlqueryPromise = panelSchema.value.queries?.map(
          async (it: any) => {
            const { query: query1, metadata: metadata1 } = replaceQueryValue(
              it.query,
              startISOTimestamp,
              endISOTimestamp,
              panelSchema.value.queryType
            );

            const { query: query2, metadata: metadata2 } =
              await applyDynamicVariables(query1, panelSchema.value.queryType);

            const query = query2;

            const metadata = {
              originalQuery: it.query,
              query: query,
              startTime: startISOTimestamp,
              endTime: endISOTimestamp,
              queryType: panelSchema.value.queryType,
              variables: [...(metadata1 || []), ...(metadata2 || [])],
            };
            // console.log("Calling search API", query, metadata);
            return await queryService
              .search({
                org_identifier: store.state.selectedOrganization.identifier,
                query: {
                  query: {
                    sql: query,
                    sql_mode: "full",
                    start_time: startISOTimestamp,
                    end_time: endISOTimestamp,
                    size: -1,
                  },
                },
                page_type: pageType,
              })
              .then((res) => {
                // Set searchQueryData.data to the API response hits
                // state.data = res.data.hits;
                state.errorDetail = "";
                // console.log("API response received");

                return { result: res.data.hits, metadata: metadata };
              })
              .catch((error) => {
                // console.log("API error received", error);

                // Process API error for "sql"
                processApiError(error, "sql");
                return { result: null, metadata: metadata };
              });
          }
        );
        // Wait for all query promises to resolve
        const sqlqueryResults = await Promise.all(sqlqueryPromise);
        state.loading = false;
        state.data = sqlqueryResults.map((it) => it?.result);
        state.metadata = {
          queries: sqlqueryResults.map((it) => it?.metadata),
        };
        log("logaData: state.data", state.data);
        log("logaData: state.metadata", state.metadata);
      }
    } catch (error: any) {
      if (
        error.name === "AbortError" ||
        error.message === "Aborted waiting for loading"
      ) {
        log("logaData: Operation aborted");
      } else {
        log("logaData: An error occurred:", error);
      }
    }
  };

  watch(
    // Watching for changes in panelSchema, selectedTimeObj and forceLoad
    () => [panelSchema?.value, selectedTimeObj?.value, forceLoad.value],
    async () => {
      log("PanelSchema/Time Wather: called");
      loadData(); // Loading the data
    }
  );

  /**
   * Replaces the query with the corresponding variable values.
   *
   * @param {any} query - The query to be modified.
   * @return {any} The modified query with replaced values.
   */
  const replaceQueryValue = (
    query: any,
    startISOTimestamp: any,
    endISOTimestamp: any,
    queryType: any
  ) => {
    const metadata: any[] = [];

    //fixed variables value calculations
    //scrape interval by default 15 seconds
    const scrapeInterval =
      store.state.organizationData.organizationSettings.scrape_interval ?? 15;

    // timestamp in seconds / chart panel width
    const __interval =
      (endISOTimestamp - startISOTimestamp) /
      (chartPanelRef.value?.offsetWidth ?? 1000) /
      1000;

    // if less than 1, set it to 1
    // minimum will be 15000 millisecond
    // __interval = Math.max(15000, __interval);

    // round interval
    const formattedInterval = formatInterval(__interval);

    // calculate rate interval in seconds
    // we need formatted interval value in seconds
    const __rate_interval: any = Math.max(
      getTimeInSecondsBasedOnUnit(
        formattedInterval.value,
        formattedInterval.unit
      ) + scrapeInterval,
      4 * scrapeInterval
    );

    //get interval in ms
    const __interval_ms =
      getTimeInSecondsBasedOnUnit(
        formattedInterval.value,
        formattedInterval.unit
      ) * 1000;

    const fixedVariables = [
      {
        name: "__interval_ms",
        value: `${__interval_ms}ms`,
      },
      {
        name: "__interval",
        value: `${formattedInterval.value}${formattedInterval.unit}`,
      },
      {
        name: "__rate_interval",
        value: `${formateRateInterval(__rate_interval)}`,
      },
    ];

    // replace fixed variables with its values
    fixedVariables?.forEach((variable: any) => {
      const variableName = `$${variable.name}`;
      const variableValue = variable.value;
      if (query.includes(variableName)) {
        metadata.push({
          type: "fixed",
          name: variable.name,
          value: variable.value,
        });
      }
      query = query.replaceAll(variableName, variableValue);
    });

    if (currentDependentVariablesData?.length) {
      currentDependentVariablesData?.forEach((variable: any) => {
        const variableName = `$${variable.name}`;

        let variableValue = "";
        if (Array.isArray(variable.value)) {
          const value = variable.value
            .map((value: any) => `'${value}'`)
            .join(",");
          const possibleVariablesPlaceHolderTypes = [
            {
              placeHolder: `\${${variable.name}:csv}`,
              value: variable.value.join(","),
            },
            {
              placeHolder: `\${${variable.name}:pipe}`,
              value: variable.value.join("|"),
            },
            {
              placeHolder: `\${${variable.name}:doublequote}`,
              value: variable.value.map((value: any) => `"${value}"`).join(","),
            },
            {
              placeHolder: `\${${variable.name}:singlequote}`,
              value: value,
            },
            {
              placeHolder: `\${${variable.name}}`,
              value: queryType === "sql" ? value : variable.value.join("|"),
            },
            {
              placeHolder: `\$${variable.name}`,
              value: queryType === "sql" ? value : variable.value.join("|"),
            },
          ];

          possibleVariablesPlaceHolderTypes.forEach((placeHolderObj) => {
            if (query.includes(placeHolderObj.placeHolder)) {
              metadata.push({
                type: "variable",
                name: variable.name,
                value: placeHolderObj.value,
              });
            }
            query = query.replaceAll(
              placeHolderObj.placeHolder,
              placeHolderObj.value
            );
          });
        } else {
          variableValue = variable.value === null ? "" : variable.value;
          if (query.includes(variableName)) {
            metadata.push({
              type: "variable",
              name: variable.name,
              value: variable.value,
            });
          }
          query = query.replaceAll(variableName, variableValue);
        }
      });

      return { query, metadata };
    } else {
      return { query, metadata };
    }
  };

  const applyDynamicVariables = async (query: any, queryType: any) => {
    const metadata: any[] = [];
    // console.log(
    //   "variablesDataaaa currentAdHocVariablesData",
    //   JSON.stringify(variablesData.value, null, 2)
    // );

    const adHocVariables = variablesData.value?.values
      ?.filter((it: any) => it.type === "dynamic_filters")
      ?.map((it: any) => it?.value)
      .flat()
      ?.filter((it: any) => it?.operator && it?.name && it?.value);
    // console.log("adHocVariables", adHocVariables);

    if (!adHocVariables?.length) {
      // console.log("No adhoc variables found");
      return { query, metadata };
    }

    // continue if there are any adhoc queries
    if (queryType === "promql") {
      adHocVariables.forEach((variable: any) => {
        metadata.push({
          type: "dynamicVariable",
          name: variable.name,
          value: variable.value,
          operator: variable.operator,
        });
        // console.log(`Adding label to PromQL query: ${variable.name}`);
        query = addLabelToPromQlQuery(
          query,
          variable.name,
          variable.value,
          variable.operator
        );
      });
    }

    if (queryType === "sql") {
      const queryStream = await getStreamFromQuery(query);

      const applicableAdHocVariables = adHocVariables;
      // .filter((it: any) => {
      //   return it?.streams?.find((it: any) => it.name == queryStream);
      // });

      applicableAdHocVariables.forEach((variable: any) => {
        metadata.push({
          type: "dynamicVariable",
          name: variable.name,
          value: variable.value,
          operator: variable.operator,
        });
      });
      // console.log("Adding labels to SQL query");
      query = await addLabelsToSQlQuery(query, applicableAdHocVariables);
    }

    return { query, metadata };
  };

  /**
   * Processes an API error based on the given error and type.
   *
   * @param {any} error - The error object to be processed.
   * @param {any} type - The type of error being processed.
   */
  const processApiError = async (error: any, type: any) => {
    switch (type) {
      case "promql": {
        const errorDetailValue = error.response?.data?.error || error.message;
        const trimmedErrorMessage =
          errorDetailValue?.length > 300
            ? errorDetailValue.slice(0, 300) + " ..."
            : errorDetailValue;
        state.errorDetail = trimmedErrorMessage;
        break;
      }
      case "sql": {
        const errorDetailValue =
          error.response?.data.error_detail ||
          error.response?.data.message ||
          error.message;
        const trimmedErrorMessage =
          errorDetailValue?.length > 300
            ? errorDetailValue.slice(0, 300) + " ..."
            : errorDetailValue;
        state.errorDetail = trimmedErrorMessage;
        break;
      }
      default:
        break;
    }
  };

  const hasAtLeastOneQuery = () =>
    panelSchema.value.queries?.some((q: any) => q?.query);

  // [START] variables management

  // check when the variables data changes
  // 1. get the dependent variables
  // 2. compare the dependent variables data with the old dependent variables Data
  // 3. if the value of any current variable is changed, call the api
  watch(
    () => variablesData.value?.values,
    () => {
      // console.log("inside watch variablesData");
      // ensure the query is there
      // if (!panelSchema.value.queries?.length) {
      //   return;
      // }

      log("Variables Watcher: starting...");

      const newDependentVariablesData = getDependentVariablesData();
      const newDynamicVariablesData = getDynamicVariablesData();

      if (
        !newDependentVariablesData?.length &&
        !newDynamicVariablesData?.length &&
        !currentDependentVariablesData?.length &&
        !currentDynamicVariablesData?.length
      ) {
        // go ahead and bravly load the data
        log("Variables Watcher: no variables needed, returning false...");
        return;
      }

      if (variablesDataUpdated()) {
        loadData();
      }
    },
    { deep: true }
  );

  // [START] Variables functions
  const areDynamicVariablesStillLoading = () =>
    variablesData.value?.values?.some(
      (it: any) =>
        it.type === "dynamic_filters" &&
        (it.isLoading || it.isVariableLoadingPending)
    );

  const areDependentVariablesStillLoadingWith = (
    newDependentVariablesData: any
  ) =>
    newDependentVariablesData?.some(
      (it: any) => it.isLoading || it.isVariableLoadingPending
    );

  const getDependentVariablesData = () =>
    variablesData.value?.values
      ?.filter((it: any) => it.type != "dynamic_filters") // ad hoc filters are not considered as dependent filters as they are globally applied
      ?.filter((it: any) => {
        const regexForVariable = new RegExp(
          `.*\\$\\{?${it.name}(?::(csv|pipe|doublequote|singlequote))?}?.*`
        );

        return panelSchema.value.queries
          ?.map((q: any) => regexForVariable.test(q?.query))
          ?.includes(true);
      });

  const getDynamicVariablesData = () => {
    const sqlQueryStreams =
      panelSchema.value.queryType == "sql"
        ? panelSchema.value.queries.map((q: any) => getStreamFromQuery(q.query))
        : [];
    const adHocVariables = variablesData.value?.values
      ?.filter((it: any) => it.type === "dynamic_filters")
      ?.map((it: any) => it?.value)
      ?.flat()
      ?.filter((it: any) => it?.operator && it?.name && it?.value);
    // ?.filter((it: any) =>
    //   panelSchema.value.queryType == "sql"
    //     ? it.streams.find((it: any) => sqlQueryStreams.includes(it?.name))
    //     : true
    // );
    log("getDynamicVariablesData: adHocVariables", adHocVariables);
    return adHocVariables;
  };

  const updateCurrentDependentVariablesData = (
    newDependentVariablesData: any
  ) => {
    currentDependentVariablesData = JSON.parse(
      JSON.stringify(newDependentVariablesData)
    );
  };

  const updateCurrentDynamicVariablesData = (newDynamicVariablesData: any) => {
    currentDynamicVariablesData = JSON.parse(
      JSON.stringify(newDynamicVariablesData)
    );
  };

  const areArraysEqual = (array1: any, array2: any) => {
    // Check if both arrays have the same length
    if (array1?.length !== array2?.length) {
      return false;
    }

    // Sort both arrays
    const sortedArray1 = array1?.slice().sort();
    const sortedArray2 = array2?.slice().sort();

    // Compare sorted arrays element by element
    for (let i = 0; i < sortedArray1?.length; i++) {
      if (sortedArray1[i] !== sortedArray2[i]) {
        return false;
      }
    }

    // If all elements are equal, return true
    return true;
  };

  const isAllRegularVariablesValuesSameWith = (
    newDependentVariablesData: any
  ) =>
    newDependentVariablesData.every((it: any) => {
      const oldValue = currentDependentVariablesData.find(
        (it2: any) => it2.name == it.name
      );
      // return it.value == oldValue?.value && oldValue?.value != "";
      return it.multiSelect
        ? areArraysEqual(it.value, oldValue?.value)
        : it.value == oldValue?.value && oldValue?.value != "";
    });

  const isAllDynamicVariablesValuesSameWith = (newDynamicVariablesData: any) =>
    newDynamicVariablesData.every((it: any) => {
      const oldValue = currentDynamicVariablesData?.find(
        (it2: any) => it2.name == it.name
      );
      return (
        oldValue?.value != "" &&
        it.value == oldValue?.value &&
        it.operator == oldValue?.operator
      );
    });

  const ifPanelVariablesCompletedLoading = () => {
    // STEP 1: Check if there are any dynamic variables that are still loading
    log("Step1: checking if dynamic variables are loading, starting...");
    const newDynamicVariablesData = getDynamicVariablesData();

    if (areDynamicVariablesStillLoading()) {
      log("Step1: dynamic variables still loading..., returning false");
      return false;
    }

    // STEP 2: Check if any regular dependent variables are still loading

    log("Step2: checking if dependent variables are loading, starting...");

    const newDependentVariablesData = getDependentVariablesData();

    if (areDependentVariablesStillLoadingWith(newDependentVariablesData)) {
      log("Step2: regular variables still loading..., returning false");
      return false;
    }

    return true;
  };

  const variablesDataUpdated = () => {
    // STEP 1: Check if there are any dynamic variables that are still loading
    log("Step1: checking if dynamic variables are loading, starting...");
    const newDynamicVariablesData = getDynamicVariablesData();

    if (areDynamicVariablesStillLoading()) {
      log("Step1: dynamic variables still loading..., returning false");
      return false;
    }

    // STEP 2: Check if any regular dependent variables are still loading

    log("Step2: checking if dependent variables are loading, starting...");

    const newDependentVariablesData = getDependentVariablesData();

    if (areDependentVariablesStillLoadingWith(newDependentVariablesData)) {
      log("Step2: regular variables still loading..., returning false");
      return false;
    }

    // STEP 3: Check if any of the regular and dynamic variables count have changed
    // if count have changed, that means the variables are added or removed
    // so we need to fire the query
    log("Step3: checking if no of variables have changed, starting...");

    log(
      "Step3: newDependentVariablesData,",
      JSON.stringify(newDependentVariablesData, null, 2)
    );
    log(
      "Step3: newDynamicVariablesData...",
      JSON.stringify(newDynamicVariablesData, null, 2)
    );

    // if the length of the any of the regular and old dynamic data has changed,
    // we need to fire the query
    log(
      "Step3: newDependentVariablesData?.length",
      newDependentVariablesData?.length
    );
    log(
      "Step3: newDynamicVariablesData?.length",
      newDynamicVariablesData?.length
    );
    log(
      "Step3: currentDependentVariablesData?.length",
      currentDependentVariablesData?.length
    );
    log(
      "Step3: currentAdHocVariablesData?.length",
      currentDynamicVariablesData?.length
    );

    if (
      newDependentVariablesData?.length !=
        currentDependentVariablesData?.length ||
      newDynamicVariablesData?.length != currentDynamicVariablesData?.length
    ) {
      updateCurrentDependentVariablesData(newDependentVariablesData);
      updateCurrentDynamicVariablesData(newDynamicVariablesData);

      log(
        "Step3: length of the any of the regular and old dynamic data has changed, we need to fire the query"
      );
      return true;
    }

    log("Step3: finished...");
    // STEP 4: Now we know same number of variables are there and have updated,
    // we have to perform different action based on different combinations of variables types
    // 1. regular variables
    // 2. dynamic variables
    log("Step4: starting...");

    // now we have to check for different combinations for the count of regular and dynamic variables
    // 1. Regular variables  = 0 and Dynamic variables  = 0
    // 2. Regular variables >= 1 and Dynamic variables  = 0
    // 3. Regular variables  = 0 and Dynamic variables >= 1
    // 4. Regular variables >= 1 and Dynamic variables >= 1

    log(
      "Step4: newDependentVariablesData.length",
      newDependentVariablesData?.length
    );
    log(
      "Step4: newDynamicVariablesData.length",
      newDynamicVariablesData?.length
    );

    // execute different scenarios based on the count of variables
    if (
      !newDependentVariablesData?.length &&
      !newDynamicVariablesData?.length
    ) {
      // 1. Regular variables  = 0 and Dynamic variables  = 0
      // go ahead and bravly load the data
      !newDependentVariablesData?.length && !newDynamicVariablesData?.length;

      log(
        "Step4: 1: no variables are there, no waiting, can call the api, returning true..."
      );

      return true;
    } else if (
      newDependentVariablesData?.length &&
      !newDynamicVariablesData?.length
    ) {
      log("Step4: 2: Regular variables >= 1 and Dynamic variables  = 0");
      // 2. Regular variables >= 1 and Dynamic variables  = 0

      // log(
      //   "Step4: 2: checking agains old values, currentDependentVariablesData",
      //   JSON.stringify(currentDependentVariablesData, null, 2)
      // );

      // check if the values have changed or not
      const isAllRegularVariablesValuesSame =
        isAllRegularVariablesValuesSameWith(newDependentVariablesData);

      if (isAllRegularVariablesValuesSame) {
        log("Step4: 2: regular variables has same old value, returning false");
        return false;
      }

      updateCurrentDependentVariablesData(newDependentVariablesData);

      log("Step4: 2: regular variables values has changed, returning true");
      return true;
    } else if (
      !newDependentVariablesData?.length &&
      newDynamicVariablesData?.length
    ) {
      // 3. Regular variables  = 0 and Dynamic variables >= 1
      log("Step4: 3: Regular variables  = 0 and Dynamic variables >= 1");

      // check if dynamic variables are same or changed
      const isAllDynamicVariablesValuesSame =
        isAllDynamicVariablesValuesSameWith(newDynamicVariablesData);

      // check if values are changed or not
      if (isAllDynamicVariablesValuesSame) {
        log("Step4: 3: dynamic variables has same old value, returning false");
        return false;
      }

      updateCurrentDynamicVariablesData(newDynamicVariablesData);

      log("Step4: 3: dynamic variables values has changed, returning true");
      return true;
    } else if (
      newDependentVariablesData?.length &&
      newDynamicVariablesData?.length
    ) {
      // 4. Regular variables >= 1 and Dynamic variables >= 1
      log("Step4: 4: Regular variables >= 1 and Dynamic variables >= 1");

      // if any of the value has changed, we need to trigger the query
      // check if the values have changed or not
      const isAllRegularVariablesValuesSame =
        isAllRegularVariablesValuesSameWith(newDependentVariablesData);

      const isAllDynamicVariablesValuesSame =
        isAllDynamicVariablesValuesSameWith(newDynamicVariablesData);

      log(
        "Step4: 4: isAllRegularVariablesValuesSame",
        isAllRegularVariablesValuesSame
      );
      log(
        "Step4: 4: isAllDynamicVariablesValuesSame",
        isAllDynamicVariablesValuesSame
      );

      // if any has changed
      if (isAllRegularVariablesValuesSame && isAllDynamicVariablesValuesSame) {
        log(
          "Step4: 4: regular and dynamic variables has same old value, returning false"
        );
        return false;
      }

      // values have changed
      // let's update and fire the query
      updateCurrentDynamicVariablesData(newDynamicVariablesData);
      updateCurrentDependentVariablesData(newDependentVariablesData);

      log("Step4: 4: variables values has changed, returning true");
      return true;
    }
  };

  const handleIntersection = async (entries: any) => {
    isVisible.value = entries[0].isIntersecting;
  };

  onMounted(async () => {
    observer = new IntersectionObserver(handleIntersection, {
      root: null,
      rootMargin: "0px",
      threshold: 0.1, // Adjust as needed
    });

    observer.observe(chartPanelRef.value);
  });

  // remove intersection observer
  onUnmounted(() => {
    if (observer) {
      observer.disconnect();
    }
  });

  log("PanelSchema/Time Initial: should load the data");
  loadData(); // Loading the data

  return {
    ...toRefs(state),
    loadData,
  };
};
