<!-- Copyright 2023 OpenObserve Inc.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
-->
<template>
  <div
    v-if="variablesData.values?.length > 0"
    :key="variablesData.isVariablesLoading"
    class="flex q-mt-xs q-ml-xs"
  >
    <div
      v-for="(item, index) in variablesData.values"
      :key="
        item.name +
        item.value +
        item.type +
        item.options?.length +
        item.isLoading +
        item.isVariableLoadingPending +
        index
      "
      :data-test="`dashboard-variable-${item}-selector`"
    >
      <div v-if="item.type == 'query_values'">
        <VariableQueryValueSelector
          class="q-mr-lg q-mt-xs"
          v-show="!item.hideOnDashboard"
          v-model="item.value"
          :variableItem="item"
          @update:model-value="onVariablesValueUpdated(index)"
          :loadOptions="loadVariableOptions"
        />
      </div>
      <div v-else-if="item.type == 'constant'">
        <q-input
          v-show="!item.hideOnDashboard"
          class="q-mr-lg q-mt-xs"
          style="max-width: 150px !important"
          v-model="item.value"
          :label="item.label || item.name"
          dense
          outlined
          readonly
          data-test="dashboard-variable-constant-selector"
          @update:model-value="onVariablesValueUpdated(index)"
        ></q-input>
      </div>
      <div v-else-if="item.type == 'textbox'">
        <q-input
          v-show="!item.hideOnDashboard"
          class="q-mr-lg q-mt-xs"
          style="max-width: 150px !important"
          debounce="1000"
          v-model="item.value"
          :label="item.label || item.name"
          dense
          outlined
          data-test="dashboard-variable-textbox-selector"
          @update:model-value="onVariablesValueUpdated(index)"
        ></q-input>
      </div>
      <div v-else-if="item.type == 'custom'">
        <VariableCustomValueSelector
          v-show="!item.hideOnDashboard"
          class="q-mr-lg q-mt-xs"
          v-model="item.value"
          :variableItem="item"
          @update:model-value="onVariablesValueUpdated(index)"
        />
      </div>
      <div v-else-if="item.type == 'dynamic_filters'">
        <VariableAdHocValueSelector
          class="q-mr-lg q-mt-xs"
          v-model="item.value"
          :variableItem="item"
        />
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { getCurrentInstance, onMounted, watch } from "vue";
import { defineComponent, reactive } from "vue";
import streamService from "../../services/stream";
import { useStore } from "vuex";
import VariableQueryValueSelector from "./settings/VariableQueryValueSelector.vue";
import VariableCustomValueSelector from "./settings/VariableCustomValueSelector.vue";
import VariableAdHocValueSelector from "./settings/VariableAdHocValueSelector.vue";
import { isInvalidDate } from "@/utils/date";
import { addLabelsToSQlQuery } from "@/utils/query/sqlUtils";
import { b64EncodeUnicode, escapeSingleQuotes } from "@/utils/zincutils";
import { buildVariablesDependencyGraph } from "@/utils/dashboard/variables/variablesDependencyUtils";

export default defineComponent({
  name: "VariablesValueSelector",
  props: [
    "selectedTimeDate",
    "variablesConfig",
    "initialVariableValues",
    "showDynamicFilters",
  ],
  emits: ["variablesData"],
  components: {
    VariableQueryValueSelector,
    VariableAdHocValueSelector,
    VariableCustomValueSelector,
  },
  setup(props: any, { emit }) {
    const instance = getCurrentInstance();
    const store = useStore();
    // variables data derived from the variables config list
    const variablesData: any = reactive({
      isVariablesLoading: false,
      values: [],
    });
    // variables dependency graph
    let variablesDependencyGraph: any = {};

    // track old variables data
    const oldVariablesData: any = {};

    // currently executing promise
    // obj will have variable name as key
    // and reject object of promise as value
    const currentlyExecutingPromises: any = {};

    // reset variables data
    // it will executed once on mount
    const resetVariablesData = () => {
      variablesData.isVariablesLoading = false;
      variablesData.values = [];
    };

    const initializeVariablesData = () => {
      // reset the values
      resetVariablesData();

      // check if variables config list is not empty
      if (!props?.variablesConfig) return;

      // make list of variables using variables config list
      // set initial variables values from props
      props?.variablesConfig?.list?.forEach((item: any) => {
        let initialValue =
          item.type == "dynamic_filters"
            ? JSON.parse(
                decodeURIComponent(
                  // if initial value is not exist, use the default value : %5B%5D(which is [] in base64)
                  props.initialVariableValues?.value[item.name] ?? "%5B%5D"
                )
              ) ?? []
            : props.initialVariableValues?.value[item.name] ?? null;

        if (item.multiSelect) {
          initialValue = Array.isArray(initialValue)
            ? initialValue
            : [initialValue];
        }
        const variableData = {
          ...item,
          // isLoading is used to check that currently, if the variable is loading(it is used to show the loading icon)
          isLoading: false,
          // isVariableLoadingPending is used to check that variable loading is pending
          // if parent variable is not loaded or it's value is changed, isVariableLoadingPending will be true
          isVariableLoadingPending: true,
        };
        // need to use initial value
        // also, constant type variable should not be updated
        if (item.type != "constant") {
          // for textbox type variable, if initial value is not exist, use the default value
          if (item.type == "textbox") {
            variableData.value = initialValue ?? variableData.value;
          } else {
            // use the initial value
            variableData.value = initialValue;
          }
        }

        // push the variable to the list
        variablesData.values.push(variableData);

        // set old variables data
        oldVariablesData[item.name] = initialValue;
      });

      // if showDynamicFilters is true, add the Dynamic filters variable
      if (props.showDynamicFilters) {
        // get the initial value
        // need to decode the initial value from base64
        const initialValue =
          JSON.parse(
            decodeURIComponent(
              // if initial value is not exist, use the default value : %5B%5D(which is [] in base64)
              props.initialVariableValues?.value["Dynamic filters"] ?? "%5B%5D"
            )
          ) ?? [];

        // push the variable to the list
        variablesData.values.push({
          name: "Dynamic filters",
          type: "dynamic_filters",
          label: "Dynamic filters",
          value: initialValue,
          isLoading: false,
          isVariableLoadingPending: true,
          options: [],
        });

        // set old variables data
        oldVariablesData["Dynamic filters"] = initialValue;
      }

      // need to build variables dependency graph on variables config list change
      variablesDependencyGraph = buildVariablesDependencyGraph(
        variablesData.values
      );
    };

    const rejectAllPromises = () => {
      Object.keys(currentlyExecutingPromises).forEach((key) => {
        if (currentlyExecutingPromises[key])
          currentlyExecutingPromises[key](false);
      });
    };

    onMounted(() => {
      // make list of variables using variables config list
      initializeVariablesData();

      // reject all promises
      rejectAllPromises();

      // load all variables
      loadAllVariablesData();
    });

    watch(
      () => props.variablesConfig,
      async () => {
        // make list of variables using variables config list
        initializeVariablesData();

        // reject all promises
        rejectAllPromises();

        // load all variables
        loadAllVariablesData();
      }
    );

    // you may need to query the data if the variable configs or the data/time changes
    // watch(
    //   () => props.selectedTimeDate,
    //   () => {
    //     // reject all promises
    //     rejectAllPromises();

    //     loadAllVariablesData();
    //   },
    // );
    watch(
      () => variablesData,
      () => {
        emitVariablesData();
      },
      { deep: true }
    );

    const emitVariablesData = () => {
      instance?.proxy?.$forceUpdate();
      emit("variablesData", JSON.parse(JSON.stringify(variablesData)));
    };

    // it is used to change/update initial variables values from outside the component
    // NOTE: right now, it is not used after variables in variables feature
    const changeInitialVariableValues = async (
      newInitialVariableValues: any
    ) => {
      // reject all promises
      rejectAllPromises();

      // NOTE: need to re-initialize variables data
      resetVariablesData();

      // set initial variables values
      props.initialVariableValues.value = newInitialVariableValues;

      // make list of variables using variables config list
      initializeVariablesData();

      // load all variables
      loadAllVariablesData();
    };
    const handleQueryValuesLogic = (
      currentVariable: any,
      oldVariableSelectedValues: any[]
    ) => {
      // Pre-calculate the options values array
      const optionsValues =
        currentVariable.options.map((option: any) => option.value) ?? [];

      // if multi select
      if (currentVariable.multiSelect) {
        // old selected values
        const selectedValues = currentVariable.options
          .filter((option: any) =>
            oldVariableSelectedValues.includes(option.value)
          )
          .map((option: any) => option.value);

        // if selected values exist, select the values
        if (selectedValues.length > 0) {
          currentVariable.value = selectedValues;
        } else {
          //here, multiselect and old values will be not exist

          switch (currentVariable?.selectAllValueForMultiSelect) {
            case "custom":
              currentVariable.value = optionsValues.filter((value: any) =>
                currentVariable?.customMultiSelectValue.includes(value)
              );
              break;
            case "all":
              currentVariable.value = optionsValues;
              break;
            default:
              currentVariable.value = [currentVariable.options[0].value];
          }
        }
      } else {
        // here, multi select is false

        // old selected value
        const oldValue = currentVariable.options.find(
          (option: any) => option.value === oldVariableSelectedValues[0]
        )?.value;

        // if old value exist, select the old value
        if (oldValue) {
          currentVariable.value = oldValue;
        } else if (currentVariable.options.length > 0) {
          // here, multi select is false and old value not exist

          if (currentVariable.selectAllValueForMultiSelect === "custom") {
            const customValue = currentVariable.options.find(
              (variableOption: any) =>
                variableOption.value ===
                currentVariable.customMultiSelectValue[0]
            );

            // customValue can be undefined or default value
            currentVariable.value =
              customValue?.value ?? currentVariable.options[0].value;
          } else {
            currentVariable.value = currentVariable.options[0].value;
          }
        } else {
          currentVariable.value = null;
        }
      }
    };

    const handleCustomVariablesLogic = (
      currentVariable: any,
      oldVariableSelectedValues: any[]
    ) => {
      // Pre-calculate the selected options values array
      const selectedOptionsValues =
        currentVariable.options
          .filter((option: any) => option.selected)
          .map((option: any) => option.value) ?? [];

      // if multi select
      if (currentVariable.multiSelect) {
        // old selected values
        const selectedValues = currentVariable.options
          .filter((option: any) =>
            oldVariableSelectedValues.includes(option.value)
          )
          .map((option: any) => option.value);

        // if old selected values exist, select the values
        if (selectedValues.length > 0) {
          currentVariable.value = selectedValues;
        } else {
          // here, multiselect is true and old values will be not exist

          // if custom value is not defined, select the first option
          currentVariable.value =
            selectedOptionsValues.length > 0
              ? selectedOptionsValues
              : [currentVariable?.options?.[0]?.value];
        }
      } else {
        // here, multi select is false

        // old selected value
        const oldValue = currentVariable.options.find(
          (option: any) => option.value === oldVariableSelectedValues[0]
        )?.value;

        // if old value exist, select the old value
        if (oldValue) {
          currentVariable.value = oldValue;
        } else if (currentVariable.options.length > 0) {
          // here, multi select is false and old value not exist

          // if custom value is not defined, select the first option
          currentVariable.value =
            selectedOptionsValues.length > 0
              ? selectedOptionsValues[0]
              : currentVariable.options[0].value;
        } else {
          currentVariable.value = null;
        }
      }
    };

    const loadVariableOptions = async (currentVariable) => {
      console.log(
        "[VariablesValueSelector] loadVariableOptions called",
        currentVariable,
      );

      console.log(
        `[VariablesValueSelector] loadVariableOptions is called for currentVariable.name ${currentVariable.name}`,
      );
      console.log(
        "[VariablesValueSelector] currentVariable whole object",
        currentVariable,
      );

      await loadApiCallForQueryValues(currentVariable.name);

      console.log(
        "[VariablesValueSelector] variablesDependencyGraph",
        variablesDependencyGraph,
      );

      console.log(
        `[VariablesValueSelector] Loading dependent variables for JSON object of currentVariable ${JSON.stringify(
          currentVariable,
          null,
          2,
        )}`,
      );
      console.log(
        `[VariablesValueSelector] Loading dependent variables for variablesDependencyGraph ${JSON.stringify(variablesDependencyGraph)}`,
      );

      // Check for dependent variables
      const dependentVariables: any =
        JSON.stringify(
          variablesDependencyGraph[currentVariable.name]?.childVariables,
        ) || [];
      console.log(
        `[VariablesValueSelector] Loading dependent variables for dependentGraphName ${dependentVariables}`,
      );

      console.log(
        `[VariablesValueSelector] Dependent variables of ${currentVariable.name} are ${dependentVariables}`,
      );
    };

    const loadApiCallForQueryValues = async (variableName) => {
      return new Promise(async (resolve, reject) => {
        console.log(
          `[VariablesValueSelector] API call for ${variableName} is starting.`,
        );

        // Check if the variable exists in the list
        const currentVariable = variablesData.values.find(
          (variable) => variable.name === variableName,
        );
        console.log(
          "[VariablesValueSelector] currentVariable",
          currentVariable,
        );

        if (!currentVariable) {
          console.log(
            `[VariablesValueSelector] Variable ${variableName} not found.`,
          );
          return resolve(false);
        }

        // Set up conditions for variable-specific API calls
        switch (currentVariable.type) {
          case "query_values":
            await handleQueryValuesApiCall(currentVariable, resolve, reject);
            break;
          default:
            console.log(
              `[VariablesValueSelector] No specific API call configured for ${currentVariable.name}`,
            );
            resolve(true);
            break;
        }
      });
    };

    const handleQueryValuesApiCall = async (
      currentVariable,
      resolve,
      reject,
    ) => {
      try {
        console.log(
          `[VariablesValueSelector] Handling query_values API call for ${currentVariable.name}`,
        );

        // Prepare the query parameters based on the variable
        const filterConditions = currentVariable?.query_data?.filter ?? [];
        let queryContext = `SELECT ${store.state.zoConfig.timestamp_column || "_timestamp"} FROM '${currentVariable?.query_data?.stream}'`;
        const constructedFilter = filterConditions.map((condition) => ({
          name: condition.name,
          operator: condition.operator,
          value: condition.value,
        }));

        queryContext = await addLabelsToSQlQuery(
          queryContext,
          constructedFilter,
        );

        // Perform API call for query values
        const res = await streamService.fieldValues({
          org_identifier: store.state.selectedOrganization.identifier,
          stream_name: currentVariable.query_data.stream,
          start_time: new Date(
            props.selectedTimeDate?.start_time?.toISOString(),
          ).getTime(),
          end_time: new Date(
            props.selectedTimeDate?.end_time?.toISOString(),
          ).getTime(),
          fields: [currentVariable.query_data.field],
          size: currentVariable?.query_data?.max_record_size ?? 10,
          query_context: queryContext,
        });

        // Process the response and handle options
        if (res.data.hits.length) {
          currentVariable.options = res.data.hits[0].values.map((value) => ({
            label: value.zo_sql_key ? value.zo_sql_key.toString() : "<blank>",
            value: value.zo_sql_key.toString(),
          }));
          resolve(true);
        } else {
          currentVariable.options = [];
          resolve(true);
        }
      } catch (err) {
        console.error(
          `[VariablesValueSelector] Error during query_values API call for ${currentVariable.name}:`,
          err,
        );
        currentVariable.options = [];
        resolve(true);
      }
    };

    // get single variable data based on index
    const loadSingleVariableDataByIndex = async (variableIndex: number) => {
      return new Promise(async (resolve, reject) => {
        console.log(
          `[VariablesValueSelector] loadSingleVariableDataByIndex is called for index ${variableIndex}`,
        );
        // if variableIndex is not valid, return
        if (variableIndex < 0) resolve(false);

        // variables data
        const currentVariable = variablesData.values[variableIndex];

        // if currentVariable is undefined, return
        if (!currentVariable) {
          console.log(
            `[VariablesValueSelector] loadSingleVariableDataByIndex is called for index ${variableIndex} but currentVariable is undefined`,
          );
          return resolve(false);
        }

        // assign current promise reject object to currentlyExecutingPromises object
        if (currentlyExecutingPromises[currentVariable.name]) {
          console.log(
            `[VariablesValueSelector] loadSingleVariableDataByIndex is called for index ${variableIndex} and currentVariable.name is ${currentVariable.name} and currentlyExecutingPromises[currentVariable.name] is true, so rejecting that promise`,
          );
          // if the variable is already loading, reject that promise
          currentlyExecutingPromises[currentVariable.name](false);
        }
        // assign current promise reject object to currentlyExecutingPromises object
        currentlyExecutingPromises[currentVariable.name] = reject;

        // need to load the current variable
        if (currentVariable.isVariableLoadingPending == false) {
          console.log(
            `[VariablesValueSelector] loadSingleVariableDataByIndex is called for index ${variableIndex} and currentVariable.isVariableLoadingPending is false, so resolving that promise`,
          );
          return resolve(false);
        }

        if (
          isInvalidDate(props.selectedTimeDate?.start_time) ||
          isInvalidDate(props.selectedTimeDate?.end_time)
        ) {
          console.log(
            `[VariablesValueSelector] loadSingleVariableDataByIndex is called for index ${variableIndex} and props.selectedTimeDate?.start_time or props.selectedTimeDate?.end_time is invalid, so resolving that promise`,
          );
          return resolve(false);
        }

        // check if all dependencies are loaded
        const isAnyDepndentVariableLoadingPending = variablesDependencyGraph[
          currentVariable.name
        ].parentVariables.find((parentVariable: any) => {
          console.log(
            `[VariablesValueSelector] loadSingleVariableDataByIndex is called for index ${variableIndex} and currentVariable.name is ${currentVariable.name} and parentVariable is ${parentVariable}`,
          );
          // get whole parent variable object from parent variable name
          const variableData = variablesData?.values?.find(
            (variable: any) => variable?.name == parentVariable,
          );

          // if parentVariable is not loaded, return
          return (
            variableData?.isLoading || variableData?.isVariableLoadingPending
          );
        });

        // if any dependent variable is loading, return
        if (isAnyDepndentVariableLoadingPending) {
          console.log(
            `[VariablesValueSelector] loadSingleVariableDataByIndex is called for index ${variableIndex} and isAnyDepndentVariableLoadingPending is true, so resolving that promise`,
          );
          return resolve(false);
        }

        switch (currentVariable.type) {
          case "query_values": {
            try {
              console.log(
                `[VariablesValueSelector] loadSingleVariableDataByIndex is called for index ${variableIndex} and currentVariable.type is query_values`,
              );
              // set loading as true
              currentVariable.isLoading = true;

              const filterConditions =
                currentVariable?.query_data?.filter ?? [];
              let dummyQuery = `SELECT ${
                store.state.zoConfig.timestamp_column || "_timestamp"
              } FROM '${currentVariable?.query_data?.stream}'`;
              const constructedFilter = filterConditions.map(
                (condition: any) => ({
                  name: condition.name,
                  operator: condition.operator,
                  value: condition.value,
                })
              );
              let queryContext = await addLabelsToSQlQuery(
                dummyQuery,
                constructedFilter
              );
              // replace variables placeholders
              // NOTE: must use for of loop because we have return statement in the loop
              for (let variable of variablesData?.values) {
                // if variable is loaded
                if (
                  variable.isLoading === false &&
                  variable.isVariableLoadingPending === false
                ) {
                  // replace it's value in the query if it is dependent on query context

                  if (Array.isArray(variable.value)) {
                    const arrayValues = variable.value
                      .map((value: any) => {
                        return `'${escapeSingleQuotes(value)}'`;
                      })
                      .join(", ");

                    queryContext = queryContext.replace(
                      `'$${variable.name}'`,
                      `${arrayValues}`
                    );
                  } else {
                    queryContext = queryContext.replace(
                      `$${variable.name}`,
                      variable.value
                    );
                  }
                }
                // above condition not matched, means variable is not loaded
                // so, check if it is dependent on query context
                else if (queryContext.includes(`$${variable.name}`)) {
                  // mark isLoading as false
                  currentVariable.isLoading = false;
                  currentVariable.isVariableLoadingPending = true;
                  resolve(false);
                }
              }

              // base64 encode the query
              queryContext = b64EncodeUnicode(queryContext) || "";
              const res = await streamService.fieldValues({
                org_identifier: store.state.selectedOrganization.identifier,
                stream_name: currentVariable.query_data.stream,
                start_time: new Date(
                  props.selectedTimeDate?.start_time?.toISOString()
                ).getTime(),
                end_time: new Date(
                  props.selectedTimeDate?.end_time?.toISOString()
                ).getTime(),
                fields: [currentVariable.query_data.field],
                size: currentVariable?.query_data?.max_record_size
                  ? currentVariable?.query_data?.max_record_size
                  : 10,
                type: currentVariable.query_data.stream_type,
                query_context: queryContext,
                no_count: true,
              });

              if (res.data.hits.length) {
                //set options value from the api response
                currentVariable.options = res.data.hits
                  .find(
                    (field: any) =>
                      field.field === currentVariable.query_data.field
                  )
                  .values.filter(
                    (value: any) => value.zo_sql_key || value.zo_sql_key === ""
                  )
                  .map((value: any) => ({
                    label:
                      value.zo_sql_key !== ""
                        ? value.zo_sql_key.toString()
                        : "&lt;blank&gt;",
                    value: value.zo_sql_key.toString(),
                  }));

                // Define oldVariableSelectedValues array
                let oldVariableSelectedValues: any = [];
                if (oldVariablesData[currentVariable.name]) {
                  oldVariableSelectedValues = Array.isArray(
                    oldVariablesData[currentVariable.name]
                  )
                    ? oldVariablesData[currentVariable.name]
                    : [oldVariablesData[currentVariable.name]];
                }

                // if the old value exists in the dropdown, set the old value; otherwise, set the first value of the dropdown; otherwise, set a blank string value
                if (
                  oldVariablesData[currentVariable.name] !== undefined ||
                  oldVariablesData[currentVariable.name] !== null
                ) {
                  if (currentVariable.type === "custom") {
                    handleCustomVariablesLogic(
                      currentVariable,
                      oldVariableSelectedValues
                    );
                  } else {
                    handleQueryValuesLogic(
                      currentVariable,
                      oldVariableSelectedValues
                    );
                  }
                } else {
                  currentVariable.value = currentVariable.options.length
                    ? currentVariable.options[0].value
                    : null;
                }

                resolve(true);
                break;
              } else {
                // no response hits found
                // set value as empty string
                currentVariable.value = currentVariable.multiSelect ? [] : null;
                // set options as empty array
                currentVariable.options = [];

                resolve(true);
                break;
              }
            } catch (err: any) {
              // some error occurred
              // set value as empty string
              currentVariable.value = null;
              // set options as empty array
              currentVariable.options = [];

              resolve(true);
              break;
            }
          }
          case "constant": {
            resolve(true);
            break;
          }
          case "textbox": {
            resolve(true);
            break;
          }
          case "custom": {
            currentVariable.options = currentVariable?.options;

            // Check if the old value exists and set it
            let oldVariableSelectedValues: any = [];
            if (oldVariablesData[currentVariable.name]) {
              oldVariableSelectedValues = Array.isArray(
                oldVariablesData[currentVariable.name]
              )
                ? oldVariablesData[currentVariable.name]
                : [oldVariablesData[currentVariable.name]];
            }

            // If multiSelect is true, set the value as an array containing old value and selected value
            if (currentVariable.type === "custom") {
              handleCustomVariablesLogic(
                currentVariable,
                oldVariableSelectedValues
              );
            } else {
              handleQueryValuesLogic(
                currentVariable,
                oldVariableSelectedValues
              );
            }
            resolve(true);
            break;
          }
          case "dynamic_filters": {
            resolve(true);
            break;
          }
          default:
            resolve(true);
            break;
        }

        resolve(true);
      })
        .then((res) => {
          if (res) {
            // if (!res) return;
            // if variableIndex is not valid, return
            if (variableIndex < 0) return;
            // variables data
            const currentVariable = variablesData.values[variableIndex];
            // if currentVariable is undefined, return
            if (!currentVariable) {
              return;
            }

            // remove the current promise from currentlyExecutingPromises
            currentlyExecutingPromises[currentVariable.name] = null;

            // set old variables data
            oldVariablesData[currentVariable.name] = currentVariable.value;

            // mark current variable as loaded
            currentVariable.isLoading = false;
            currentVariable.isVariableLoadingPending = false;

            // check all variables are loaded?
            // if all variables are loaded, set isVariablesLoading to false
            variablesData.isVariablesLoading = variablesData.values.some(
              (val: { isLoading: any; isVariableLoadingPending: any }) =>
                val.isLoading || val.isVariableLoadingPending
            );

            // now, load all it's child variables
            const childVariableIndices = variablesData.values.reduce(
              (indices: number[], variable: any, index: number) => {
                if (
                  variablesDependencyGraph[
                    currentVariable.name
                  ].childVariables.includes(variable.name)
                ) {
                  indices.push(index);
                }
                return indices;
              },
              []
            );

            // will force update the variables data
            emitVariablesData();

            Promise.all(
              childVariableIndices.map((childIndex: number) =>
                loadSingleVariableDataByIndex(childIndex)
              )
            );
          }
        })
        .catch((res) => {
          // if (!res) return;
          // if variableIndex is not valid, return
          if (variableIndex < 0) return;
          // variables data
          const currentVariable = variablesData.values[variableIndex];
          // if currentVariable is undefined, return
          if (!currentVariable) {
            return;
          }

          // remove the current promise from currentlyExecutingPromises
          currentlyExecutingPromises[currentVariable.name] = null;

          // set isLoading as false
          currentVariable.isLoading = false;
        });
    };

    const loadAllVariablesData = async () => {
      if (
        isInvalidDate(props.selectedTimeDate?.start_time) ||
        isInvalidDate(props.selectedTimeDate?.end_time)
      ) {
        return;
      }

      // set isVariableLoadingPending as true for all variables
      variablesData.values.forEach((variable: any) => {
        variable.isVariableLoadingPending = true;
      });
      console.log("loadAllVariablesData", variablesData.values);

      Promise.all(
        variablesData.values.map((it: any, index: number) =>
          loadSingleVariableDataByIndex(index)
        )
      );
    };

    const setLoadingStateToAllChildNode = (currentVariable: string) => {
      for (const variableName of variablesDependencyGraph[currentVariable]
        .childVariables) {
        const variableObj = variablesData.values.find(
          (it: any) => it.name === variableName
        );
        variableObj.isVariableLoadingPending = true;
        setLoadingStateToAllChildNode(variableObj.name);
      }
    };

    const onVariablesValueUpdated = (variableIndex: number) => {
      // if variableIndex is not valid, return
      if (variableIndex < 0) return;
      // variables data
      const currentVariable = variablesData.values[variableIndex];
      // if currentVariable is undefined, return
      if (!currentVariable) {
        return;
      }

      // currentVariable value changed.
      // so, set it to oldVariablesData
      oldVariablesData[currentVariable.name] = currentVariable.value;

      // set all child variables to loading state
      setLoadingStateToAllChildNode(currentVariable.name);

      Promise.all(
        variablesData.values.map((it: any, index: number) =>
          loadSingleVariableDataByIndex(index)
        )
      );
    };

    return {
      props,
      variablesData,
      changeInitialVariableValues,
      onVariablesValueUpdated,
      loadVariableOptions,
    };
  },
});
</script>
