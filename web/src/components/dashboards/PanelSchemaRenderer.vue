<!-- Copyright 2023 Zinc Labs Inc.

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
  <div style="width: 100%; height: 100%" @mouseleave="hideDrilldownPopUp">
    <div ref="chartPanelRef" style="height: 100%; position: relative">
      <div v-if="!errorDetail" style="height: 100%; width: 100%">
        <GeoMapRenderer
          v-if="panelSchema.type == 'geomap'"
          :data="
            panelData.chartType == 'geomap'
              ? panelData
              : { options: { backgroundColor: 'transparent' } }
          "
        />
        <TableRenderer
          v-else-if="panelSchema.type == 'table'"
          :data="
            panelData.chartType == 'table'
              ? panelData
              : { options: { backgroundColor: 'transparent' } }
          "
          @row-click="onChartClick"
          ref="tableRendererRef"
          :wrap-cells="panelSchema.config?.wrap_table_cells"
        />
        <div
          v-else-if="panelSchema.type == 'html'"
          class="col column"
          style="width: 100%; height: 100%; flex: 1"
        >
          <HTMLRenderer
            :htmlContent="panelSchema.htmlContent"
            style="width: 100%; height: 100%"
            class="col"
          />
        </div>
        <div
          v-else-if="panelSchema.type == 'markdown'"
          class="col column"
          style="width: 100%; height: 100%; flex: 1"
        >
          <MarkdownRenderer
            :markdownContent="panelSchema.markdownContent"
            style="width: 100%; height: 100%"
            class="col"
          />
        </div>
        <ChartRenderer
          v-else
          :data="
            panelSchema.queryType === 'promql' ||
            (data.length &&
              data[0]?.length &&
              panelData.chartType != 'geomap' &&
              panelData.chartType != 'table')
              ? panelData
              : { options: { backgroundColor: 'transparent' } }
          "
          @updated:data-zoom="$emit('updated:data-zoom', $event)"
          @error="errorDetail = $event"
          @click="onChartClick"
        />
      </div>
      <div v-if="!errorDetail" class="noData" data-test="no-data">
        {{ noData }}
      </div>
      <div
        v-if="errorDetail && !panelSchema?.error_config?.custom_error_handeling"
        class="errorMessage"
      >
        <q-icon size="md" name="warning" />
        <div style="height: 80%; width: 100%">{{ errorDetail }}</div>
      </div>
      <div
        v-if="
          errorDetail &&
          panelSchema?.error_config?.custom_error_handeling &&
          !panelSchema?.error_config?.default_data_on_error &&
          panelSchema?.error_config?.custom_error_message
        "
        class="customErrorMessage"
      >
        {{ panelSchema?.error_config?.custom_error_message }}
      </div>
      <div
        v-if="loading"
        class="row"
        style="position: absolute; top: 0px; width: 100%; z-index: 999"
      >
        <q-spinner-dots
          color="primary"
          size="40px"
          style="margin: 0 auto; z-index: 999"
        />
      </div>
      <div
        style="
          border: 1px solid gray;
          border-radius: 4px;
          padding: 3px;
          position: absolute;
          top: 0px;
          left: 0px;
          display: none;
          text-wrap: nowrap;
          z-index: 9999999;
        "
        :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
        ref="drilldownPopUpRef"
        @mouseleave="hideDrilldownPopUp"
      >
        <div
          v-for="(drilldown, index) in drilldownArray"
          :key="JSON.stringify(drilldown)"
          class="drilldown-item q-px-sm q-py-xs"
          style="
            display: flex;
            flex-direction: row;
            align-items: center;
            position: relative;
          "
        >
          <div
            @click="openDrilldown(index)"
            style="cursor: pointer; display: flex; align-items: center"
          >
            <q-icon class="q-mr-xs q-mt-xs" size="16px" name="link" />
            <span>{{ drilldown.name }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  watch,
  ref,
  toRefs,
  computed,
  inject,
  nextTick,
  defineAsyncComponent,
} from "vue";
import { useStore } from "vuex";
import { usePanelDataLoader } from "@/composables/dashboard/usePanelDataLoader";
import { convertPanelData } from "@/utils/dashboard/convertPanelData";
import { getAllDashboardsByFolderId, getFoldersList } from "@/utils/commons";
import { useRoute, useRouter } from "vue-router";

const ChartRenderer = defineAsyncComponent(() => {
  return import("@/components/dashboards/panels/ChartRenderer.vue");
});

const TableRenderer = defineAsyncComponent(() => {
  return import("@/components/dashboards/panels/TableRenderer.vue");
});

const GeoMapRenderer = defineAsyncComponent(() => {
  return import("@/components/dashboards/panels/GeoMapRenderer.vue");
});

const HTMLRenderer = defineAsyncComponent(() => {
  return import("./panels/HTMLRenderer.vue");
});

const MarkdownRenderer = defineAsyncComponent(() => {
  return import("./panels/MarkdownRenderer.vue");
});

export default defineComponent({
  name: "PanelSchemaRenderer",
  components: {
    ChartRenderer,
    TableRenderer,
    GeoMapRenderer,
    HTMLRenderer,
    MarkdownRenderer,
  },
  props: {
    selectedTimeObj: {
      required: true,
      type: Object,
    },
    panelSchema: {
      required: true,
      type: Object,
    },
    variablesData: {
      required: true,
      type: Object,
    },
    forceLoad: {
      type: Boolean,
      default: false,
      required: false,
    },
  },
  emits: [
    "updated:data-zoom",
    "error",
    "metadata-update",
    "update:initialVariableValues",
  ],
  setup(props, { emit }) {
    const store = useStore();
    const route = useRoute();
    const router = useRouter();

    // stores the converted data which can be directly used for rendering different types of panels
    const panelData: any = ref({}); // holds the data to render the panel after getting data from the api based on panel config
    const chartPanelRef: any = ref(null); // holds the ref to the whole div
    const drilldownArray: any = ref([]);
    const drilldownPopUpRef: any = ref(null);

    // get refs from props
    const { panelSchema, selectedTimeObj, variablesData, forceLoad } =
      toRefs(props);
    // calls the apis to get the data based on the panel config
    let { data, loading, errorDetail, metadata } = usePanelDataLoader(
      panelSchema,
      selectedTimeObj,
      variablesData,
      chartPanelRef,
      forceLoad
    );

    // need tableRendererRef to access downloadTableAsCSV method
    const tableRendererRef = ref(null);

    // hovered series state
    // used to show tooltip axis for all charts
    const hoveredSeriesState: any = inject("hoveredSeriesState", null);

    // ======= [START] dashboard PrintMode =======

    //inject variablesAndPanelsDataLoadingState from parent
    // default values will be empty object of panels and variablesData
    const variablesAndPanelsDataLoadingState: any = inject(
      "variablesAndPanelsDataLoadingState",
      { panels: {}, variablesData: {} }
    );

    // on loading state change, update the loading state of the panels in variablesAndPanelsDataLoadingState
    watch(loading, (updatedLoadingValue) => {
      if (variablesAndPanelsDataLoadingState) {
        // update the loading state of the current panel
        variablesAndPanelsDataLoadingState.panels = {
          ...variablesAndPanelsDataLoadingState?.panels,
          [panelSchema?.value?.id]: updatedLoadingValue,
        };
      }
    });

    // ======= [END] dashboard PrintMode =======

    watch(
      [data, store?.state],
      async () => {
        // panelData.value = convertPanelData(panelSchema.value, data.value, store);
        if (!errorDetail.value) {
          try {
            // passing chartpanelref to get width and height of DOM element
            panelData.value = await convertPanelData(
              panelSchema.value,
              data.value,
              store,
              chartPanelRef,
              hoveredSeriesState
            );

            errorDetail.value = "";
          } catch (error: any) {
            errorDetail.value = error.message;
          }
        } else {
          // if no data is available, then show the default data
          // if there is an error config in the panel schema, then show the default data on error
          // if no default data on error is set, then show the custom error message
          if (
            panelSchema.value?.error_config?.custom_error_handeling &&
            panelSchema.value?.error_config?.default_data_on_error
          ) {
            data.value = JSON.parse(
              panelSchema.value?.error_config?.default_data_on_error
            );
            errorDetail.value = "";
          }
        }
      },
      { deep: true }
    );

    // when we get the new metadata from the apis, emit the metadata update
    watch(metadata, () => {
      emit("metadata-update", metadata.value);
    });

    const handleNoData = (panelType: any) => {
      const xAlias = panelSchema.value.queries[0].fields.x.map(
        (it: any) => it.alias
      );
      const yAlias = panelSchema.value.queries[0].fields.y.map(
        (it: any) => it.alias
      );
      const zAlias = panelSchema.value.queries[0].fields.z.map(
        (it: any) => it.alias
      );

      switch (panelType) {
        case "area":
        case "area-stacked":
        case "bar":
        case "h-bar":
        case "stacked":
        case "h-stacked":
        case "line":
        case "scatter":
        case "table": {
          // return data.value[0].some((it: any) => {return (xAlias.every((x: any) => it[x]) && yAlias.every((y: any) => it[y]))});
          return (
            data.value[0]?.length > 1 ||
            (xAlias.every((x: any) => data.value[0][0][x] != null) &&
              yAlias.every((y: any) => data.value[0][0][y]) != null)
          );
        }
        case "gauge":
        case "metric": {
          return (
            data.value[0]?.length > 1 ||
            yAlias.every(
              (y: any) =>
                data.value[0][0][y] != null || data.value[0][0][y] === 0
            )
          );
        }
        case "heatmap": {
          return (
            data.value[0]?.length > 1 ||
            (xAlias.every((x: any) => data.value[0][0][x] != null) &&
              yAlias.every((y: any) => data.value[0][0][y] != null) &&
              zAlias.every((z: any) => data.value[0][0][z]) != null)
          );
        }
        case "pie":
        case "donut": {
          return (
            data.value[0]?.length > 1 ||
            yAlias.every((y: any) => data.value[0][0][y] != null)
          );
        }
        case "sankey": {
          const source = panelSchema.value.queries[0].fields.source.alias;
          const target = panelSchema.value.queries[0].fields.target.alias;
          const value = panelSchema.value.queries[0].fields.value.alias;
          return (
            data.value[0]?.length > 1 ||
            source.every((s: any) => data.value[0][0][s] != null) ||
            target.every((t: any) => data.value[0][0][t] != null) ||
            value.every((v: any) => data.value[0][0][v] != null)
          );
        }
        default:
          break;
      }
    };

    // Compute the value of the 'noData' variable
    const noData = computed(() => {
      // if panel type is 'html' or 'markdown', return an empty string
      if (
        panelSchema.value.type == "html" ||
        panelSchema.value.type == "markdown"
      ) {
        return "";
      }
      // Check if the queryType is 'promql'
      else if (panelSchema.value?.queryType == "promql") {
        // Check if the 'data' array has elements and every item has a non-empty 'result' array
        return data.value.length &&
          data.value.some((item: any) => item?.result?.length)
          ? "" // Return an empty string if there is data
          : "No Data"; // Return "No Data" if there is no data
      } else {
        // The queryType is not 'promql'
        return data.value.length &&
          data.value[0]?.length &&
          handleNoData(panelSchema.value.type)
          ? ""
          : "No Data"; // Return "No Data" if the 'data' array is empty, otherwise return an empty string
      }
    });

    // when the error changes, emit the error
    watch(errorDetail, () => {
      //check if there is an error message or not
      if (!errorDetail.value) return;
      emit("error", errorDetail);
    });

    const hideDrilldownPopUp = () => {
      if (drilldownPopUpRef.value) {
        drilldownPopUpRef.value.style.display = "none";
      }
    };

    // drilldown
    const replacePlaceholders = (str: any, obj: any) => {
      // if the str is same as the key, return it's value(it can be an string or array).
      for (const key in obj) {
        // ${varName} == str
        if (`\$\{${key}\}` == str) {
          return obj[key];
        }
      }

      return str.replace(/\$\{([^}]+)\}/g, function (_: any, key: any) {
        // Split the key into parts by either a dot or a ["xyz"] pattern and filter out empty strings
        let parts = key.split(/\.|\["(.*?)"\]/).filter(Boolean);

        let value = obj;
        for (let part of parts) {
          if (value && part in value) {
            value = value[part];
          } else {
            return "${" + key + "}";
          }
        }
        return value;
      });
    };

    // get offset from parent
    function getOffsetFromParent(parent: any, child: any) {
      const parentRect = parent.getBoundingClientRect();
      const childRect = child.getBoundingClientRect();

      return {
        left: childRect.left - parentRect.left,
        top: childRect.top - parentRect.top,
      };
    }

    // need to save click event params, to open drilldown
    let drilldownParams: any = [];

    const onChartClick = async (params: any, ...args: any) => {
      // check if drilldown data exists
      if (
        !panelSchema.value.config.drilldown ||
        panelSchema.value.config.drilldown.length == 0
      ) {
        return;
      }

      drilldownParams = [params, args];

      // drilldownarrayref offset values
      let offSetValues = { left: 0, top: 0 };

      // if type is table, calculate offset
      if (panelSchema.value.type == "table") {
        offSetValues = getOffsetFromParent(chartPanelRef.value, params?.target);

        // also, add offset of clicked position
        offSetValues.left += params?.offsetX;
        offSetValues.top += params?.offsetY;
      } else {
        // for all other charts
        offSetValues.left = params?.event?.offsetX;
        offSetValues.top = params?.event?.offsetY;
      }

      // set drilldown array, to show list of drilldowns
      drilldownArray.value = panelSchema.value.config.drilldown ?? [];

      // temporarily show the popup to calculate its dimensions
      drilldownPopUpRef.value.style.display = "block";

      // wait for the next DOM update cycle before calculating the dimensions
      await nextTick();

      // if not enough space to show drilldown at the bottom, show it above the cursor
      if (
        offSetValues.top + drilldownPopUpRef.value.offsetHeight >
        chartPanelRef.value.offsetHeight
      ) {
        offSetValues.top =
          offSetValues.top - drilldownPopUpRef.value.offsetHeight;
      }
      if (
        offSetValues.left + drilldownPopUpRef.value.offsetWidth >
        chartPanelRef.value.offsetWidth
      ) {
        // if not enough space on the right, show the popup to the left of the cursor
        offSetValues.left =
          offSetValues.left - drilldownPopUpRef.value.offsetWidth;
      }

      // 24 px takes panel header height
      drilldownPopUpRef.value.style.top = offSetValues?.top + 5 + "px";
      drilldownPopUpRef.value.style.left = offSetValues?.left + 5 + "px";

      // if drilldownArray has at least one element then only show the drilldown pop up
      if (drilldownArray.value.length > 0) {
        drilldownPopUpRef.value.style.display = "block";
      } else {
        // hide the popup if there's no drilldown
        hideDrilldownPopUp();
      }
    };

    const openDrilldown = async (index: any) => {
      // hide the drilldown pop up
      hideDrilldownPopUp();
      // if panelSchema exists
      if (panelSchema.value) {
        // check if drilldown data exists
        if (
          !panelSchema.value.config.drilldown ||
          panelSchema.value.config.drilldown.length == 0
        ) {
          return;
        }

        // find drilldown data
        const drilldownData = panelSchema.value.config.drilldown[index];

        // need to change dynamic variables to it's value using current variables, current chart data(params)
        // if pie, donut or heatmap then series name will come in name field
        // also, if value is an array, then last value will be taken
        const drilldownVariables: any = {};

        // if chart type is 'table' then we need to pass the table name
        if (panelSchema.value.type == "table") {
          const fields: any = {};
          panelSchema.value.queries.forEach((query: any) => {
            // take all field from x, y and z
            const panelFields: any = [
              ...query.fields.x,
              ...query.fields.y,
              ...query.fields.z,
            ];
            panelFields.forEach((field: any) => {
              // we have label and alias, use both in dynamic values
              fields[field.label] = drilldownParams[1][0][field.alias];
              fields[field.alias] = drilldownParams[1][0][field.alias];
            });
          });
          drilldownVariables.row = {
            field: fields,
            index: drilldownParams[1][1],
          };
        } else if (panelSchema.value.type == "sankey") {
          // if dataType is node then set node data
          // else set edge data
          if (drilldownParams[0].dataType == "node") {
            // set node data
            drilldownVariables.node = {
              __name: drilldownParams[0]?.name ?? "",
              __value: drilldownParams[0]?.value ?? "",
            };
          } else {
            // set edge data
            drilldownVariables.edge = {
              __source: drilldownParams[0]?.data?.source ?? "",
              __target: drilldownParams[0]?.data?.target ?? "",
              __value: drilldownParams[0]?.data?.value ?? "",
            };
          }
        } else {
          // we have an series object
          drilldownVariables.series = {
            __name: ["pie", "donut", "heatmap"].includes(panelSchema.value.type)
              ? drilldownParams[0].name
              : drilldownParams[0].seriesName,
            __value: Array.isArray(drilldownParams[0].value)
              ? drilldownParams[0].value[drilldownParams[0].value.length - 1]
              : drilldownParams[0].value,
          };
        }

        variablesData.value.values.forEach((variable: any) => {
          if (variable.type != "dynamic_filters") {
            drilldownVariables[variable.name] = variable.value;
          }
        });

        // if drilldown by url
        if (drilldownData.type == "byUrl") {
          try {
            // open url
            return window.open(
              replacePlaceholders(drilldownData.data.url, drilldownVariables),
              drilldownData.targetBlank ? "_blank" : "_self"
            );
          } catch (error) {}
        } else if (drilldownData.type == "byDashboard") {
          // we have folder, dashboard and tabs name
          // so we have to get id of folder, dashboard and tab

          // get folder id
          if (
            !store.state.organizationData.folders ||
            (Array.isArray(store.state.organizationData.folders) &&
              store.state.organizationData.folders.length === 0)
          ) {
            await getFoldersList(store);
          }
          const folderId = store.state.organizationData.folders.find(
            (folder: any) => folder.name == drilldownData.data.folder
          )?.folderId;

          if (!folderId) {
            return;
          }

          // get dashboard id
          const allDashboardData = await getAllDashboardsByFolderId(
            store,
            folderId
          );
          const dashboardData = allDashboardData.find(
            (dashboard: any) => dashboard.title == drilldownData.data.dashboard
          );

          if (!dashboardData) {
            return;
          }

          // get tab id
          const tabId =
            dashboardData.tabs.find(
              (tab: any) => tab.name == drilldownData.data.tab
            )?.tabId ?? dashboardData.tabs[0].tabId;

          // if targetBlank is true then create new url
          // else made changes in current router only
          if (drilldownData.targetBlank) {
            // get current origin
            const pos = window.location.pathname.indexOf("/web/");
            // if there is /web/ in path
            // url will be: origin from window.location.origin + pathname up to /web/ + /web/
            let currentUrl: any =
              pos > -1
                ? window.location.origin +
                  window.location.pathname.slice(0, pos) +
                  "/web"
                : window.location.origin;

            // always, go to view dashboard page
            currentUrl += "/dashboards/view?";

            // if pass all variables in url
            currentUrl += drilldownData.data.passAllVariables
              ? new URLSearchParams(route.query as any).toString()
              : "";

            const url = new URL(currentUrl);

            // set variables provided by user
            drilldownData.data.variables.forEach((variable: any) => {
              if (variable?.name?.trim() && variable?.value?.trim()) {
                url.searchParams.set(
                  "var-" +
                    replacePlaceholders(variable.name, drilldownVariables),
                  replacePlaceholders(variable.value, drilldownVariables)
                );
              }
            });

            url.searchParams.set("dashboard", dashboardData.dashboardId);
            url.searchParams.set("folder", folderId);
            url.searchParams.set("tab", tabId);
            currentUrl = url.toString();

            window.open(currentUrl, "_blank");
          } else {
            let oldParams: any = [];
            // if pass all variables is true
            if (drilldownData.data.passAllVariables) {
              // get current query params
              oldParams = route.query;
            }

            drilldownData.data.variables.forEach((variable: any) => {
              if (variable?.name?.trim() && variable?.value?.trim()) {
                oldParams[
                  "var-" +
                    replacePlaceholders(variable.name, drilldownVariables)
                ] = replacePlaceholders(variable.value, drilldownVariables);
              }
            });

            // make changes in router
            await router.push({
              path: "/dashboards/view",
              query: {
                ...oldParams,
                org_identifier: store.state.selectedOrganization.identifier,
                dashboard: dashboardData.dashboardId,
                folder: folderId,
                tab: tabId,
              },
            });

            // ======= [START] default variable values

            const initialVariableValues: any = {};
            Object.keys(route.query).forEach((key) => {
              if (key.startsWith("var-")) {
                const newKey = key.slice(4);
                initialVariableValues[newKey] = route.query[key];
              }
            });
            // ======= [END] default variable values

            emit("update:initialVariableValues", initialVariableValues);
          }
        }
      }
    };

    return {
      store,
      chartPanelRef,
      data,
      loading,
      errorDetail,
      panelData,
      noData,
      metadata,
      tableRendererRef,
      onChartClick,
      drilldownArray,
      openDrilldown,
      drilldownPopUpRef,
      hideDrilldownPopUp,
    };
  },
});
</script>

<style lang="scss" scoped>
.drilldown-item:hover {
  background-color: rgba(202, 201, 201, 0.908);
}
.errorMessage {
  position: absolute;
  top: 20%;
  width: 100%;
  height: 80%;
  overflow: hidden;
  text-align: center;
  color: rgba(255, 0, 0, 0.8);
  text-overflow: ellipsis;
}

.customErrorMessage {
  position: absolute;
  top: 20%;
  width: 100%;
  height: 80%;
  overflow: hidden;
  text-align: center;
  text-overflow: ellipsis;
}

.noData {
  position: absolute;
  top: 20%;
  width: 100%;
  text-align: center;
}
</style>
