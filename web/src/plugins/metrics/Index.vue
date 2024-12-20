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
  <div style="height: calc(100vh - 42px); overflow-y: auto" class="scroll">
    <div class="row" style="height: 40px; overflow-y: auto">
      <div class="flex items-center col">
        <div
          class="flex items-center q-table__title q-mx-md tw-font-semibold tw-text-xl"
        >
          <span>
            {{ t("search.metrics") }}
          </span>
        </div>
        <syntax-guide-metrics class="q-mr-sm" />
      </div>
      <div class="text-right col flex justify-end">
        <DateTimePickerDashboard
          v-if="
            !['html', 'markdown'].includes(dashboardPanelData.data.type) &&
            selectedDate
          "
          v-model="selectedDate"
          ref="dateTimePickerRef"
          :disable="disable"
          class="dashboard-icons tw-mt-1"
          data-test="metrics-date-picker"
        />
        <AutoRefreshInterval
          v-if="!['html', 'markdown'].includes(dashboardPanelData.data.type)"
          v-model="refreshInterval"
          trigger
          @trigger="runQuery"
          class="dashboard-icons tw-mt-1"
          data-test="metrics-auto-refresh"
        />
        <div
          v-if="!['html', 'markdown'].includes(dashboardPanelData.data.type)"
          class="dashboard-icons tw-my-1 tw-mx-2"
        >
          <q-btn
            v-if="config.isEnterprise == 'true' && searchRequestTraceIds.length"
            class="tw-text-xs tw-font-bold no-border"
            data-test="metrics-cancel"
            padding="sm md"
            color="negative"
            no-caps
            :label="t('panel.cancel')"
            @click="cancelAddPanelQuery"
          />
          <q-btn
            v-else
            class="tw-text-xs tw-font-bold no-border"
            data-test="metrics-apply"
            padding="sm"
            color="secondary"
            no-caps
            :label="t('metrics.runQuery')"
            @click="runQuery"
          />
        </div>
      </div>
    </div>
    <div class="row" style="height: calc(100vh - 82px); overflow-y: auto">
      <div
        class="col scroll"
        style="
          overflow-y: auto;
          height: 100%;
          min-width: 100px;
          max-width: 100px;
        "
      >
        <ChartSelection
          v-model:selectedChartType="dashboardPanelData.data.type"
          @update:selected-chart-type="resetAggregationFunction"
        />
      </div>
      <q-separator vertical />
      <!-- for query related chart only -->
      <div
        v-if="!['html', 'markdown'].includes(dashboardPanelData.data.type)"
        class="col"
        style="width: 100%; height: 100%"
      >
        <q-splitter
          v-model="dashboardPanelData.layout.splitter"
          @update:model-value="layoutSplitterUpdated"
          style="width: 100%; height: 100%"
        >
          <template #before>
            <div
              class="col scroll tw-border-t-2"
              style="height: calc(100vh - 82px); overflow-y: auto"
            >
              <div class="column" style="height: 100%">
                <div class="col-auto q-pa-sm">
                  <span class="text-weight-bold">{{ t("panel.fields") }}</span>
                </div>
                <div class="col" style="width: 100%">
                  <!-- <GetFields :editMode="editMode" /> -->
                  <FieldList :editMode="editMode" />
                </div>
              </div>
            </div>
          </template>
          <template #separator>
            <div class="splitter-vertical splitter-enabled"></div>
            <q-avatar
              color="primary"
              text-color="white"
              size="20px"
              icon="drag_indicator"
              style="top: 10px; left: 3.5px"
            />
          </template>
          <template #after>
            <div
              class="row"
              style="height: calc(100vh - 82px); overflow-y: auto"
            >
              <div class="col" style="height: 100%">
                <q-splitter
                  class="query-editor-splitter"
                  v-model="dashboardPanelData.layout.querySplitter"
                  horizontal
                  @update:model-value="querySplitterUpdated"
                  reverse
                  unit="px"
                  :limits="
                    !dashboardPanelData.layout.showQueryBar
                      ? [43, 400]
                      : [140, 400]
                  "
                  :disable="!dashboardPanelData.layout.showQueryBar"
                  style="height: 100%"
                >
                  <template #before>
                    <div
                      class="layout-panel-container col"
                      style="height: 100%"
                    >
                      <DashboardQueryBuilder
                        :dashboardData="currentDashboardData.data"
                      />
                      <q-separator />

                      <div v-if="isOutDated" class="tw-p-2">
                        <div
                          :style="{
                            borderColor: '#c3920d',
                            borderWidth: '1px',
                            borderStyle: 'solid',
                            backgroundColor:
                              store.state.theme == 'dark'
                                ? '#2a1f03'
                                : '#faf2da',
                            padding: '1%',
                            borderRadius: '5px',
                          }"
                        >
                          <div style="font-weight: 700">
                            Your chart is not up to date
                          </div>
                          <div>
                            Chart Configuration / Variables has been updated,
                            but the chart was not updated automatically. Click
                            on the "Apply" button to run the query again
                          </div>
                        </div>
                      </div>
                      <div class="tw-flex tw-justify-end tw-mr-2">
                        <span v-if="lastTriggeredAt" class="lastRefreshedAt">
                          <span class="lastRefreshedAtIcon">🕑</span
                          ><RelativeTime
                            :timestamp="lastTriggeredAt"
                            fullTimePrefix="Last Refreshed At: "
                          />
                        </span>
                      </div>
                      <div
                        class="col"
                        style="position: relative; height: 100%; width: 100%"
                      >
                        <div
                          style="
                            flex: 1;
                            min-height: calc(100% - 24px);
                            height: calc(100% - 24px);
                            width: 100%;
                            margin-top: 24px;
                          "
                        >
                          <PanelSchemaRenderer
                            v-if="chartData"
                            @metadata-update="metaDataValue"
                            :key="dashboardPanelData.data.type"
                            :panelSchema="chartData"
                            :selectedTimeObj="dashboardPanelData.meta.dateTime"
                            :variablesData="{}"
                            :width="6"
                            @error="handleChartApiError"
                            @updated:data-zoom="onDataZoom"
                            @updated:vrl-function-field-list="
                              updateVrlFunctionFieldList
                            "
                            @last-triggered-at-update="
                              handleLastTriggeredAtUpdate
                            "
                            searchType="ui"
                          />
                        </div>
                        <div
                          class="flex justify-end q-pr-lg q-mb-md q-pt-xs"
                          style="position: absolute; top: 0px; right: -13px"
                        >
                          <q-btn
                            size="md"
                            class="no-border"
                            no-caps
                            dense
                            style="padding: 2px 4px; z-index: 1"
                            color="primary"
                            @click="addToDashboard"
                            title="Add To Dashboard"
                            >Add To Dashboard</q-btn
                          >
                        </div>
                      </div>
                      <DashboardErrorsComponent
                        :errors="errorData"
                        class="col-auto"
                        style="flex-shrink: 0"
                      />
                    </div>
                  </template>
                  <template #separator>
                    <div
                      class="splitter"
                      :class="
                        dashboardPanelData.layout.showQueryBar
                          ? 'splitter-enabled'
                          : ''
                      "
                    ></div>
                  </template>
                  <template #after>
                    <div style="height: 100%; width: 100%" class="row column">
                      <DashboardQueryEditor />
                    </div>
                  </template>
                </q-splitter>
              </div>
              <q-separator vertical />
              <div class="col-auto">
                <PanelSidebar
                  :title="t('dashboard.configLabel')"
                  v-model="dashboardPanelData.layout.isConfigPanelOpen"
                >
                  <ConfigPanel
                    :dashboardPanelData="dashboardPanelData"
                    :variablesData="{}"
                  />
                </PanelSidebar>
              </div>
            </div>
          </template>
        </q-splitter>
      </div>
      <div
        v-if="dashboardPanelData.data.type == 'html'"
        class="col column"
        style="width: 100%; height: 100%; flex: 1"
      >
        <CustomHTMLEditor
          v-model="dashboardPanelData.data.htmlContent"
          style="width: 100%; height: 100%"
          class="col"
        />
        <DashboardErrorsComponent :errors="errorData" class="col-auto" />
      </div>
      <div
        v-if="dashboardPanelData.data.type == 'markdown'"
        class="col column"
        style="width: 100%; height: 100%; flex: 1"
      >
        <CustomMarkdownEditor
          v-model="dashboardPanelData.data.markdownContent"
          style="width: 100%; height: 100%"
          class="col"
        />
        <DashboardErrorsComponent :errors="errorData" class="col-auto" />
      </div>
    </div>
    <q-dialog
      v-model="showAddToDashboardDialog"
      position="right"
      full-height
      maximized
    >
      <add-to-dashboard
        @save="addPanelToDashboard"
        :dashboardPanelData="dashboardPanelData"
      />
    </q-dialog>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  computed,
  toRaw,
  nextTick,
  watch,
  reactive,
  onUnmounted,
  onMounted,
  defineAsyncComponent,
} from "vue";
import PanelSidebar from "@/components/dashboards/addPanel/PanelSidebar.vue";
import ChartSelection from "@/components/dashboards/addPanel/ChartSelection.vue";
import FieldList from "@/components/dashboards/addPanel/FieldList.vue";
import SyntaxGuideMetrics from "./SyntaxGuideMetrics.vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import DashboardQueryBuilder from "@/components/dashboards/addPanel/DashboardQueryBuilder.vue";
import useDashboardPanelData from "../../composables/useDashboardPanel";
import DateTimePickerDashboard from "@/components/DateTimePickerDashboard.vue";
import DashboardErrorsComponent from "@/components/dashboards/addPanel/DashboardErrors.vue";
import VariablesValueSelector from "@/components/dashboards/VariablesValueSelector.vue";
import PanelSchemaRenderer from "@/components/dashboards/PanelSchemaRenderer.vue";
import RelativeTime from "@/components/common/RelativeTime.vue";
import { isEqual } from "lodash-es";
import { provide } from "vue";
import useNotifications from "@/composables/useNotifications";
import config from "@/aws-exports";
import useCancelQuery from "@/composables/dashboard/useCancelQuery";
import AutoRefreshInterval from "@/components/AutoRefreshInterval.vue";
const AddToDashboard = defineAsyncComponent(() => {
  return import("./../metrics/AddToDashboard.vue");
});

const ConfigPanel = defineAsyncComponent(() => {
  return import("@/components/dashboards/addPanel/ConfigPanel.vue");
});

const QueryInspector = defineAsyncComponent(() => {
  return import("@/components/dashboards/QueryInspector.vue");
});

const CustomHTMLEditor = defineAsyncComponent(() => {
  return import("@/components/dashboards/addPanel/CustomHTMLEditor.vue");
});

const CustomMarkdownEditor = defineAsyncComponent(() => {
  return import("@/components/dashboards/addPanel/CustomMarkdownEditor.vue");
});

export default defineComponent({
  name: "Metrics",
  props: ["metaData"],

  components: {
    ChartSelection,
    FieldList,
    DashboardQueryBuilder,
    DateTimePickerDashboard,
    DashboardErrorsComponent,
    PanelSidebar,
    ConfigPanel,
    VariablesValueSelector,
    PanelSchemaRenderer,
    RelativeTime,
    DashboardQueryEditor: defineAsyncComponent(
      () => import("@/components/dashboards/addPanel/DashboardQueryEditor.vue"),
    ),
    QueryInspector,
    CustomHTMLEditor,
    CustomMarkdownEditor,
    SyntaxGuideMetrics,
    AddToDashboard,
    AutoRefreshInterval,
  },
  setup(props) {
    provide("dashboardPanelDataPageKey", "metrics");

    // This will be used to copy the chart data to the chart renderer component
    // This will deep copy the data object without reactivity and pass it on to the chart renderer
    const chartData = ref();
    const { t } = useI18n();
    const store = useStore();
    const { showErrorNotification } = useNotifications();
    const {
      dashboardPanelData,
      resetDashboardPanelData,
      resetDashboardPanelDataAndAddTimeField,
      resetAggregationFunction,
      validatePanel,
      removeXYFilters,
    } = useDashboardPanelData("metrics");
    const editMode = ref(false);
    const selectedDate: any = ref({
      valueType: "relative",
      startTime: null,
      endTime: null,
      relativeTimePeriod: "15m",
    });
    const dateTimePickerRef: any = ref(null);
    const errorData: any = reactive({
      errors: [],
    });

    // to store and show when the panel was last loaded
    const lastTriggeredAt = ref(null);
    const handleLastTriggeredAtUpdate = (data: any) => {
      lastTriggeredAt.value = data;
    };

    const showAddToDashboardDialog = ref(false);

    // refresh interval v-model
    const refreshInterval = ref(0);

    const metaData = ref(null);
    const showViewPanel = ref(false);
    const metaDataValue = (metadata: any) => {
      // console.time("metaDataValue");
      metaData.value = metadata;
      // console.timeEnd("metaDataValue");
    };

    const currentDashboardData: any = reactive({
      data: {},
    });

    // this is used to activate the watcher only after on mounted
    let isPanelConfigWatcherActivated = false;
    const isPanelConfigChanged = ref(false);

    onUnmounted(async () => {
      // clear a few things
      resetDashboardPanelData();
    });

    onMounted(async () => {
      errorData.errors = [];

      editMode.value = false;
      resetDashboardPanelDataAndAddTimeField();

      // for metrics page, use stream type as metric
      dashboardPanelData.data.queries[0].fields.stream_type = "metrics";
      // need to remove the xy filters
      removeXYFilters();

      // set the default query type as promql for metrics
      dashboardPanelData.data.queryType = "promql";
      dashboardPanelData.data.queries[0].customQuery = true;

      // set the show query bar by default for metrics page
      dashboardPanelData.layout.showQueryBar = true;

      chartData.value = {};
      // set the value of the date time after the reset
      updateDateTime(selectedDate.value);

      // let it call the wathcers and then mark the panel config watcher as activated
      await nextTick();
      isPanelConfigWatcherActivated = true;
    });

    const isInitialDashboardPanelData = () => {
      return (
        dashboardPanelData.data.description == "" &&
        !dashboardPanelData.data.config.unit &&
        !dashboardPanelData.data.config.unit_custom &&
        dashboardPanelData.data.queries[0].fields.x.length == 0 &&
        dashboardPanelData.data.queries[0].fields?.breakdown?.length == 0 &&
        dashboardPanelData.data.queries[0].fields.y.length == 0 &&
        dashboardPanelData.data.queries[0].fields.z.length == 0 &&
        dashboardPanelData.data.queries[0].fields.filter.conditions.length ==
          0 &&
        dashboardPanelData.data.queries.length == 1
      );
    };

    const isOutDated = computed(() => {
      //check that is it addpanel initial call
      if (isInitialDashboardPanelData() && !editMode.value) return false;
      //compare chartdata and dashboardpaneldata and variables data as well
      return !isEqual(chartData.value, dashboardPanelData.data);
    });

    watch(isOutDated, () => {
      window.dispatchEvent(new Event("resize"));
    });

    watch(
      () => dashboardPanelData.data.type,
      async () => {
        // console.time("watch:dashboardPanelData.data.type");
        await nextTick();
        chartData.value = JSON.parse(JSON.stringify(dashboardPanelData.data));
        // console.timeEnd("watch:dashboardPanelData.data.type");
      },
    );

    watch(selectedDate, () => {
      // console.time("watch:selectedDate");
      updateDateTime(selectedDate.value);
      // console.timeEnd("watch:selectedDate");
    });

    // resize the chart when config panel is opened and closed
    watch(
      () => dashboardPanelData.layout.isConfigPanelOpen,
      () => {
        // console.time("watch:dashboardPanelData.layout.isConfigPanelOpen");
        window.dispatchEvent(new Event("resize"));
        // console.timeEnd("watch:dashboardPanelData.layout.isConfigPanelOpen");
      },
    );

    // resize the chart when query editor is opened and closed
    watch(
      () => dashboardPanelData.layout.showQueryBar,
      (newValue) => {
        // console.time("watch:dashboardPanelData.layout.showQueryBar");
        if (!newValue) {
          dashboardPanelData.layout.querySplitter = 41;
        } else {
          if (expandedSplitterHeight.value !== null) {
            dashboardPanelData.layout.querySplitter =
              expandedSplitterHeight.value;
          }
        }
        // console.timeEnd("watch:dashboardPanelData.layout.showQueryBar");
      },
    );

    const runQuery = () => {
      // console.time("runQuery");
      if (!isValid(true)) {
        return;
      }

      // copy the data object excluding the reactivity
      chartData.value = JSON.parse(JSON.stringify(dashboardPanelData.data));
      // refresh the date time based on current time if relative date is selected
      dateTimePickerRef.value && dateTimePickerRef.value.refresh();
      updateDateTime(selectedDate.value);
      // console.timeEnd("runQuery");
    };

    const updateDateTime = (value: object) => {
      if (selectedDate.value && dateTimePickerRef?.value) {
        const date = dateTimePickerRef.value?.getConsumableDateTime();

        dashboardPanelData.meta.dateTime = {
          start_time: new Date(date.startTime),
          end_time: new Date(date.endTime),
        };
      }
    };

    //watch dashboardpaneldata when changes, isUpdated will be true
    watch(
      () => dashboardPanelData.data,
      () => {
        // console.time("watch:dashboardPanelData.data");
        if (isPanelConfigWatcherActivated) {
          isPanelConfigChanged.value = true;
        }
        // console.timeEnd("watch:dashboardPanelData.data");
      },
      { deep: true },
    );

    //validate the data
    const isValid = (onlyChart = false) => {
      const errors = errorData.errors;
      errors.splice(0);
      const dashboardData = dashboardPanelData;

      // check if name of panel is there
      if (!onlyChart) {
        if (
          dashboardData.data.title == null ||
          dashboardData.data.title.trim() == ""
        ) {
          errors.push("Name of Panel is required");
        }
      }

      // will push errors in errors array
      validatePanel(errors);

      // show all the errors
      // for (let index = 0; index < errors.length; index++) {
      //   $q.notify({
      //     type: "negative",
      //     message: errors[index],
      //     timeout: 5000,
      //   });
      // }

      if (errors.length) {
        showErrorNotification(
          "There are some errors, please fix them and try again",
        );
      }

      if (errors.length) {
        return false;
      } else {
        return true;
      }
    };

    const layoutSplitterUpdated = () => {
      window.dispatchEvent(new Event("resize"));
    };

    const expandedSplitterHeight = ref(null);

    const querySplitterUpdated = (newHeight: any) => {
      window.dispatchEvent(new Event("resize"));
      if (dashboardPanelData.layout.showQueryBar) {
        expandedSplitterHeight.value = newHeight;
      }
    };

    const handleChartApiError = (errorMessage: any) => {
      const errorList = errorData.errors;
      errorList.splice(0);
      errorList.push(errorMessage);
    };

    const onDataZoom = (event: any) => {
      // console.time("onDataZoom");
      const selectedDateObj = {
        start: new Date(event.start),
        end: new Date(event.end),
      };
      // Truncate seconds and milliseconds from the dates
      selectedDateObj.start.setSeconds(0, 0);
      selectedDateObj.end.setSeconds(0, 0);

      // Compare the truncated dates
      if (selectedDateObj.start.getTime() === selectedDateObj.end.getTime()) {
        // Increment the end date by 1 minute
        selectedDateObj.end.setMinutes(selectedDateObj.end.getMinutes() + 1);
      }

      // set it as a absolute time
      dateTimePickerRef?.value?.setCustomDate("absolute", selectedDateObj);
      // console.timeEnd("onDataZoom");
    };

    const updateVrlFunctionFieldList = (fieldList: any) => {
      // extract all panelSchema alias
      const aliasList: any = [];

      // remove panelschema fields from field list

      // add x axis alias
      dashboardPanelData?.data?.queries[
        dashboardPanelData.layout.currentQueryIndex
      ]?.fields?.x?.forEach((it: any) => {
        if (!it.isDerived) {
          aliasList.push(it.alias);
        }
      });

      // add breakdown alias
      dashboardPanelData?.data?.queries[
        dashboardPanelData.layout.currentQueryIndex
      ]?.fields?.breakdown?.forEach((it: any) => {
        if (!it.isDerived) {
          aliasList.push(it.alias);
        }
      });

      // add y axis alias
      dashboardPanelData?.data?.queries[
        dashboardPanelData.layout.currentQueryIndex
      ]?.fields?.y?.forEach((it: any) => {
        if (!it.isDerived) {
          aliasList.push(it.alias);
        }
      });

      // add z axis alias
      dashboardPanelData?.data?.queries[
        dashboardPanelData.layout.currentQueryIndex
      ]?.fields?.z?.forEach((it: any) => {
        if (!it.isDerived) {
          aliasList.push(it.alias);
        }
      });

      // add latitude alias
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.latitude?.alias &&
        !dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.latitude?.isDerived
      ) {
        aliasList.push(
          dashboardPanelData?.data?.queries[
            dashboardPanelData.layout.currentQueryIndex
          ]?.fields?.latitude.alias,
        );
      }

      // add longitude alias
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.longitude?.alias &&
        !dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.longitude?.isDerived
      ) {
        aliasList.push(
          dashboardPanelData?.data?.queries[
            dashboardPanelData.layout.currentQueryIndex
          ]?.fields?.longitude.alias,
        );
      }

      // add weight alias
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.weight?.alias &&
        !dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.weight?.isDerived
      ) {
        aliasList.push(
          dashboardPanelData?.data?.queries[
            dashboardPanelData.layout.currentQueryIndex
          ]?.fields?.weight.alias,
        );
      }

      // add source alias
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.source?.alias &&
        !dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.source?.isDerived
      ) {
        aliasList.push(
          dashboardPanelData?.data?.queries[
            dashboardPanelData.layout.currentQueryIndex
          ]?.fields?.source.alias,
        );
      }

      // add target alias
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.target?.alias &&
        !dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.target?.isDerived
      ) {
        aliasList.push(
          dashboardPanelData?.data?.queries[
            dashboardPanelData.layout.currentQueryIndex
          ]?.fields?.target.alias,
        );
      }

      // add source alias
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.value?.alias &&
        !dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.value?.isDerived
      ) {
        aliasList.push(
          dashboardPanelData?.data?.queries[
            dashboardPanelData.layout.currentQueryIndex
          ]?.fields?.value.alias,
        );
      }

      // remove custom query fields from field list
      dashboardPanelData.meta.stream.customQueryFields.forEach((it: any) => {
        aliasList.push(it.name);
      });

      // rest will be vrl function fields
      fieldList = fieldList
        .filter((field: any) => aliasList.indexOf(field) < 0)
        .map((field: any) => ({ name: field, type: "Utf8" }));

      dashboardPanelData.meta.stream.vrlFunctionFieldList = fieldList;
    };

    const hoveredSeriesState = ref({
      hoveredSeriesName: "",
      panelId: -1,
      dataIndex: -1,
      seriesIndex: -1,
      hoveredTime: null,
      setHoveredSeriesName: function (name: string) {
        hoveredSeriesState.value.hoveredSeriesName = name ?? "";
      },
      setIndex: function (
        dataIndex: number,
        seriesIndex: number,
        panelId: any,
        hoveredTime?: any,
      ) {
        hoveredSeriesState.value.dataIndex = dataIndex ?? -1;
        hoveredSeriesState.value.seriesIndex = seriesIndex ?? -1;
        hoveredSeriesState.value.panelId = panelId ?? -1;
        hoveredSeriesState.value.hoveredTime = hoveredTime ?? null;
      },
    });

    // used provide and inject to share data between components
    // it is currently used in panelschemarendered, chartrenderer, convertpromqldata(via panelschemarenderer), and convertsqldata
    provide("hoveredSeriesState", hoveredSeriesState);

    // [START] cancel running queries

    //reactive object for loading state of variablesData and panels
    const variablesAndPanelsDataLoadingState = reactive({
      variablesData: {},
      panels: {},
      searchRequestTraceIds: {},
    });

    // provide variablesAndPanelsDataLoadingState to share data between components
    provide(
      "variablesAndPanelsDataLoadingState",
      variablesAndPanelsDataLoadingState,
    );

    const searchRequestTraceIds = computed(() => {
      const searchIds = Object.values(
        variablesAndPanelsDataLoadingState.searchRequestTraceIds,
      ).filter((item: any) => item.length > 0);

      return searchIds.flat() as string[];
    });
    const { traceIdRef, cancelQuery } = useCancelQuery();

    const cancelAddPanelQuery = () => {
      traceIdRef.value = searchRequestTraceIds.value;
      cancelQuery();
    };

    const disable = ref(false);

    watch(variablesAndPanelsDataLoadingState, () => {
      const panelsValues = Object.values(
        variablesAndPanelsDataLoadingState.panels,
      );
      disable.value = panelsValues.some((item: any) => item === true);
    });

    const addToDashboard = () => {
      const errors: any = [];
      // will push errors in errors array
      validatePanel(errors);

      if (errors.length) {
        // set errors into errorData
        errorData.errors = errors;
        showErrorNotification(
          "There are some errors, please fix them and try again",
        );
        return;
      } else {
        showAddToDashboardDialog.value = true;
      }
    };

    const addPanelToDashboard = () => {
      showAddToDashboardDialog.value = false;
    };

    // [END] cancel running queries
    return {
      t,
      updateDateTime,
      runQuery,
      layoutSplitterUpdated,
      expandedSplitterHeight,
      querySplitterUpdated,
      dashboardPanelData,
      chartData,
      editMode,
      selectedDate,
      errorData,
      handleChartApiError,
      currentDashboardData,
      resetAggregationFunction,
      isOutDated,
      store,
      dateTimePickerRef,
      showViewPanel,
      metaDataValue,
      metaData,
      onDataZoom,
      updateVrlFunctionFieldList,
      lastTriggeredAt,
      handleLastTriggeredAtUpdate,
      searchRequestTraceIds,
      cancelAddPanelQuery,
      disable,
      config,
      showAddToDashboardDialog,
      addPanelToDashboard,
      addToDashboard,
      refreshInterval,
    };
  },
});
</script>

<style lang="scss" scoped>
.layout-panel-container {
  display: flex;
  flex-direction: column;
}

.dashboard-icons {
  height: 32px;
}

.splitter {
  height: 4px;
  width: 100%;
}
.splitter-vertical {
  width: 4px;
  height: 100%;
}
.splitter-enabled {
  background-color: #ffffff00;
  transition: 0.3s;
  transition-delay: 0.2s;
}

.splitter-enabled:hover {
  background-color: orange;
}

:deep(.query-editor-splitter .q-splitter__separator) {
  background-color: transparent !important;
}
</style>
