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

<!-- eslint-disable vue/no-unused-components -->
<template>
  <div style="height: calc(100vh - 57px); overflow-y: auto" class="scroll">
    <div class="flex justify-between items-center q-pa-sm">
      <div class="flex items-center q-table__title q-mr-md">
        <span>
          {{ editMode ? t("panel.editPanel") : t("panel.addPanel") }}
        </span>
        <div>
          <q-input
            data-test="dashboard-panel-name"
            v-model="dashboardPanelData.data.title"
            :label="t('panel.name') + '*'"
            class="q-ml-xl"
            filled
            dense
          />
        </div>
      </div>
      <div class="flex q-gutter-sm">
        <q-btn
          outline
          padding="sm"
          class="q-mr-sm"
          no-caps
          label="Dashboard Tutorial"
          @click="showTutorial"
          data-test="dashboard-panel-tutorial-btn"
        ></q-btn>
        <q-btn
          v-if="!['html', 'markdown'].includes(dashboardPanelData.data.type)"
          outline
          padding="sm"
          class="q-mr-sm"
          no-caps
          icon="info_outline"
          @click="showViewPanel = true"
          data-test="dashboard-panel-data-view-query-inspector-btn"
        >
          <q-tooltip anchor="center left" self="center right"
            >Query Inspector
          </q-tooltip>
        </q-btn>
        <DateTimePickerDashboard
          v-if="selectedDate"
          v-model="selectedDate"
          ref="dateTimePickerRef"
        />
        <q-btn
          class="q-ml-md text-bold"
          outline
          padding="sm lg"
          color="red"
          no-caps
          :label="t('panel.discard')"
          @click="goBackToDashboardList"
          data-test="dashboard-panel-discard"
        />
        <q-btn
          class="q-ml-md text-bold"
          outline
          padding="sm lg"
          no-caps
          :label="t('panel.save')"
          data-test="dashboard-panel-save"
          @click.stop="savePanelData.execute()"
          :loading="savePanelData.isLoading.value"
        />
        <q-btn
          v-if="!['html', 'markdown'].includes(dashboardPanelData.data.type)"
          class="q-ml-md text-bold no-border"
          data-test="dashboard-apply"
          padding="sm lg"
          color="secondary"
          no-caps
          :label="t('panel.apply')"
          @click="runQuery"
        />
      </div>
    </div>
    <q-separator></q-separator>
    <div class="row" style="height: calc(100vh - 115px); overflow-y: auto">
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
              class="col scroll"
              style="height: calc(100vh - 115px); overflow-y: auto"
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
              style="height: calc(100vh - 115px); overflow-y: auto"
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
                      <VariablesValueSelector
                        :variablesConfig="currentDashboardData.data?.variables"
                        :showDynamicFilters="
                          currentDashboardData.data?.variables
                            ?.showDynamicFilters
                        "
                        :selectedTimeDate="dashboardPanelData.meta.dateTime"
                        @variablesData="variablesDataUpdated"
                      />

                      <div
                        v-if="isOutDated"
                        :style="{
                          borderColor: '#c3920d',
                          borderWidth: '1px',
                          borderStyle: 'solid',
                          backgroundColor:
                            store.state.theme == 'dark' ? '#2a1f03' : '#faf2da',
                          padding: '1%',
                          margin: '1%',
                          borderRadius: '5px',
                        }"
                      >
                        <div style="font-weight: 700">
                          Your chart is not up to date
                        </div>
                        <div>
                          Chart configuration has been updated, but the chart
                          was not updated automatically. Click on the "Apply"
                          button to run the query again
                        </div>
                      </div>

                      <div style="flex: 1">
                        <PanelSchemaRenderer
                          @metadata-update="metaDataValue"
                          :key="dashboardPanelData.data.type"
                          :panelSchema="chartData"
                          :selectedTimeObj="dashboardPanelData.meta.dateTime"
                          :variablesData="variablesData"
                          :width="6"
                          @error="handleChartApiError"
                          @updated:data-zoom="onDataZoom"
                        />
                        <q-dialog v-model="showViewPanel">
                          <QueryInspector
                            :metaData="metaData"
                            :data="panelTitle"
                          ></QueryInspector>
                        </q-dialog>
                      </div>
                      <DashboardErrorsComponent :errors="errorData" />
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
                    :variablesData="variablesData"
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
import PanelSidebar from "../../../components/dashboards/addPanel/PanelSidebar.vue";
import ChartSelection from "../../../components/dashboards/addPanel/ChartSelection.vue";
import FieldList from "../../../components/dashboards/addPanel/FieldList.vue";
import { useQuasar } from "quasar";

import { useI18n } from "vue-i18n";
import {
  addPanel,
  getDashboard,
  getPanel,
  updatePanel,
} from "../../../utils/commons";
import { onBeforeRouteLeave, useRoute, useRouter } from "vue-router";
import { useStore } from "vuex";
import DashboardQueryBuilder from "../../../components/dashboards/addPanel/DashboardQueryBuilder.vue";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import DateTimePickerDashboard from "../../../components/DateTimePickerDashboard.vue";
import DashboardErrorsComponent from "../../../components/dashboards/addPanel/DashboardErrors.vue";
import VariablesValueSelector from "../../../components/dashboards/VariablesValueSelector.vue";
import PanelSchemaRenderer from "../../../components/dashboards/PanelSchemaRenderer.vue";
import { useLoading } from "@/composables/useLoading";
import { isEqual } from "lodash-es";
import { provide } from "vue";

const ConfigPanel = defineAsyncComponent(() => {
  return import("../../../components/dashboards/addPanel/ConfigPanel.vue");
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
  name: "AddPanel",
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
    DashboardQueryEditor: defineAsyncComponent(
      () => import("@/components/dashboards/addPanel/DashboardQueryEditor.vue")
    ),
    QueryInspector,
    CustomHTMLEditor,
    CustomMarkdownEditor,
  },
  setup(props) {
    // This will be used to copy the chart data to the chart renderer component
    // This will deep copy the data object without reactivity and pass it on to the chart renderer
    const chartData = ref({});
    const $q = useQuasar();
    const { t } = useI18n();
    const router = useRouter();
    const route = useRoute();
    const store = useStore();
    const {
      dashboardPanelData,
      promqlMode,
      resetDashboardPanelData,
      resetAggregationFunction,
    } = useDashboardPanelData();
    const editMode = ref(false);
    const selectedDate: any = ref(null);
    const dateTimePickerRef: any = ref(null);
    const errorData: any = reactive({
      errors: [],
    });
    let variablesData: any = reactive({});
    const metaData = ref(null);
    const showViewPanel = ref(false);
    const metaDataValue = (metadata: any) => {
      // console.time("metaDataValue");
      metaData.value = metadata;
      // console.timeEnd("metaDataValue");
    };

    //dashboard tutorial link on click
    const showTutorial = () => {
      window.open("https://short.openobserve.ai/dashboard-tutorial");
    };

    const variablesDataUpdated = (data: any) => {
      Object.assign(variablesData, data);
    };
    const currentDashboardData: any = reactive({
      data: {},
    });

    // this is used to activate the watcher only after on mounted
    let isPanelConfigWatcherActivated = false;
    const isPanelConfigChanged = ref(false);

    const savePanelData = useLoading(async () => {
      // console.time("savePanelData");
      const dashboardId = route.query.dashboard + "";
      await savePanelChangesToDashboard(dashboardId);
      // console.timeEnd("savePanelData");
    });

    onUnmounted(async () => {
      // console.time("onUnmounted");
      // clear a few things
      resetDashboardPanelData();

      // remove beforeUnloadHandler event listener
      window.removeEventListener("beforeunload", beforeUnloadHandler);
      // console.timeEnd("onUnmounted");
    });

    onMounted(async () => {
      errorData.errors = [];

      // console.time("onMounted");
      // todo check for the edit more
      if (route.query.panelId) {
        editMode.value = true;
        const panelData = await getPanel(
          store,
          route.query.dashboard,
          route.query.panelId,
          route.query.folder,
          route.query.tab
        );
        Object.assign(
          dashboardPanelData.data,
          JSON.parse(JSON.stringify(panelData))
        );
        await nextTick();
        chartData.value = JSON.parse(JSON.stringify(dashboardPanelData.data));
        updateDateTime(selectedDate.value);
      } else {
        editMode.value = false;
        resetDashboardPanelData();
        chartData.value = {};
        // set the value of the date time after the reset
        updateDateTime(selectedDate.value);
      }
      // console.timeEnd("onMounted");
      // let it call the wathcers and then mark the panel config watcher as activated
      await nextTick();
      isPanelConfigWatcherActivated = true;

      //event listener before unload and data is updated
      window.addEventListener("beforeunload", beforeUnloadHandler);
      // console.time("add panel loadDashboard");
      loadDashboard();
      // console.timeEnd("add panel loadDashboard");
    });

    let list = computed(function () {
      return [toRaw(store.state.currentSelectedDashboard)];
    });

    const currentDashboard = toRaw(store.state.currentSelectedDashboard);

    // const getDashboard = () => {
    //   return currentDashboard.dashboardId;
    // };
    const loadDashboard = async () => {
      // console.time("AddPanel:loadDashboard");
      let data = JSON.parse(
        JSON.stringify(
          await getDashboard(
            store,
            route.query.dashboard,
            route.query.folder ?? "default"
          )
        )
      );
      // console.timeEnd("AddPanel:loadDashboard");

      // console.time("AddPanel:loadDashboard:after");
      currentDashboardData.data = data;
      // if variables data is null, set it to empty list
      if (
        !(
          currentDashboardData.data?.variables &&
          currentDashboardData.data?.variables?.list.length
        )
      ) {
        variablesData.isVariablesLoading = false;
        variablesData.values = [];
      }

      // get default time for dashboard
      // if dashboard has relative time settings
      if ((data?.defaultDatetimeDuration?.type ?? "relative") === "relative") {
        selectedDate.value = {
          valueType: "relative",
          relativeTimePeriod:
            data?.defaultDatetimeDuration?.relativeTimePeriod ?? "15m",
        };
      } else {
        // else, dashboard will have absolute time settings
        selectedDate.value = {
          valueType: "absolute",
          startTime: data?.defaultDatetimeDuration?.startTime,
          endTime: data?.defaultDatetimeDuration?.endTime,
        };
      }
    };

    const isInitailDashboardPanelData = () => {
      return (
        dashboardPanelData.data.description == "" &&
        !dashboardPanelData.data.config.unit &&
        !dashboardPanelData.data.config.unit_custom &&
        dashboardPanelData.data.queries[0].fields.x.length == 0 &&
        dashboardPanelData.data.queries[0].fields.y.length == 0 &&
        dashboardPanelData.data.queries[0].fields.z.length == 0 &&
        dashboardPanelData.data.queries[0].fields.filter.length == 0 &&
        dashboardPanelData.data.queries.length == 1
      );
    };

    const isOutDated = computed(() => {
      //check that is it addpanel initial call
      if (isInitailDashboardPanelData() && !editMode.value) return false;
      //compare chartdata and dashboardpaneldata
      return !isEqual(chartData.value, dashboardPanelData.data);
    });

    watch(isOutDated, () => {
      window.dispatchEvent(new Event("resize"));
    });

    const currentXLabel = computed(() => {
      return dashboardPanelData.data.type == "table"
        ? "First Column"
        : dashboardPanelData.data.type == "h-bar"
        ? "Y-Axis"
        : "X-Axis";
    });

    const currentYLabel = computed(() => {
      return dashboardPanelData.data.type == "table"
        ? "Other Columns"
        : dashboardPanelData.data.type == "h-bar"
        ? "X-Axis"
        : "Y-Axis";
    });

    watch(
      () => dashboardPanelData.data.type,
      async () => {
        // console.time("watch:dashboardPanelData.data.type");
        await nextTick();
        chartData.value = JSON.parse(JSON.stringify(dashboardPanelData.data));
        // console.timeEnd("watch:dashboardPanelData.data.type");
      }
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
      }
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
      }
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
      if (selectedDate.value) {
        dashboardPanelData.meta.dateTime = {
          start_time: new Date(selectedDate.value.startTime),
          end_time: new Date(selectedDate.value.endTime),
        };
      }
    };
    const goBack = () => {
      return router.push({
        path: "/dashboards/view",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
          dashboard: route.query.dashboard,
          folder: route.query.folder,
          tab: route.query.tab ?? currentDashboardData.data.tabs[0].tabId,
        },
      });
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
      { deep: true }
    );

    const beforeUnloadHandler = (e: any) => {
      //check is data updated or not
      if (isPanelConfigChanged.value) {
        // Display a confirmation message
        const confirmMessage = t("dashboard.unsavedMessage"); // Some browsers require a return statement to display the message
        e.returnValue = confirmMessage;
        return confirmMessage;
      }
      return;
    };

    onBeforeRouteLeave((to, from, next) => {
      if (from.path === "/dashboards/add_panel" && isPanelConfigChanged.value) {
        const confirmMessage = t("dashboard.unsavedMessage");
        if (window.confirm(confirmMessage)) {
          // User confirmed, allow navigation
          next();
        } else {
          // User canceled, prevent navigation
          next(false);
        }
      } else {
        // No unsaved changes or not leaving the edit route, allow navigation
        next();
      }
    });
    const panelTitle = computed(() => {
      return { title: dashboardPanelData.data.title };
    });

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

      //check each query is empty or not for promql
      if (dashboardData.data.queryType == "promql") {
        dashboardData.data.queries.map((q: any, index: number) => {
          if (q && q.query == "") {
            errors.push(`Query-${index + 1} is empty`);
          }
        });
      }

      //check each query is empty or not for geomap
      if (dashboardData.data.type == "geomap") {
        dashboardData.data.queries.map((q: any, index: number) => {
          if (q && q.query == "") {
            errors.push(`Query-${index + 1} is empty`);
          }
        });
      }

      //check content should be empty for html
      if (dashboardData.data.type == "html") {
        if (dashboardData.data.htmlContent.trim() == "") {
          errors.push("Please enter your HTML code");
        }
      }

      //check content should be empty for html
      if (dashboardData.data.type == "markdown") {
        if (dashboardData.data.markdownContent.trim() == "") {
          errors.push("Please enter your markdown code");
        }
      }

      if (promqlMode.value) {
        // 1. chart type: only line chart is supported
        const allowedChartTypes = [
          "area",
          "line",
          "bar",
          "scatter",
          "area-stacked",
          "metric",
          "gauge",
          "html",
          "markdown",
        ];
        if (!allowedChartTypes.includes(dashboardPanelData.data.type)) {
          errors.push(
            "Selected chart type is not supported for PromQL. Only line chart is supported."
          );
        }

        // 2. x axis, y axis, filters should be blank
        if (
          dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
            .fields.x.length > 0
        ) {
          errors.push(
            "X-Axis is not supported for PromQL. Remove anything added to the X-Axis."
          );
        }

        if (
          dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
            .fields.y.length > 0
        ) {
          errors.push(
            "Y-Axis is not supported for PromQL. Remove anything added to the Y-Axis."
          );
        }

        if (
          dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
            .fields.filter.length > 0
        ) {
          errors.push(
            "Filters are not supported for PromQL. Remove anything added to the Filters."
          );
        }

        // if(!dashboardPanelData.data.query) {
        //   errors.push("Query should not be empty")
        // }
      } else {
        switch (dashboardPanelData.data.type) {
          case "donut":
          case "pie": {
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.y.length > 1 ||
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.y.length == 0
            ) {
              errors.push("Add one value field for donut and pie charts");
            }

            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.x.length > 1 ||
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.x.length == 0
            ) {
              errors.push("Add one label field for donut and pie charts");
            }

            break;
          }
          case "metric": {
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.y.length > 1 ||
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.y.length == 0
            ) {
              errors.push("Add one value field for metric charts");
            }

            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.x.length
            ) {
              errors.push(
                `${currentXLabel.value} field is not allowed for Metric chart`
              );
            }

            break;
          }
          case "gauge": {
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.y.length != 1
            ) {
              errors.push("Add one value field for gauge chart");
            }
            // gauge can have zero or one label
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.x.length != 1 &&
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.x.length != 0
            ) {
              errors.push(`Add one label field for gauge chart`);
            }

            break;
          }
          case "h-bar":
          case "area":
          case "line":
          case "scatter":
          case "bar": {
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.y.length < 1
            ) {
              errors.push("Add at least one field for the Y-Axis");
            }

            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.x.length > 2 ||
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.x.length == 0
            ) {
              errors.push(`Add one or two fields for the X-Axis`);
            }

            break;
          }
          case "table": {
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.y.length == 0 &&
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.x.length == 0
            ) {
              errors.push("Add at least one field on X-Axis or Y-Axis");
            }

            break;
          }
          case "heatmap": {
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.y.length == 0
            ) {
              errors.push("Add at least one field for the Y-Axis");
            }

            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.x.length == 0
            ) {
              errors.push(`Add one field for the X-Axis`);
            }

            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.z.length == 0
            ) {
              errors.push(`Add one field for the Z-Axis`);
            }

            break;
          }
          case "area-stacked":
          case "stacked":
          case "h-stacked": {
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.y.length > 1 ||
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.y.length == 0
            ) {
              errors.push(
                "Add exactly one field on Y-Axis for stacked and h-stacked charts"
              );
            }
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.x.length != 2
            ) {
              errors.push(
                `Add exactly two fields on the X-Axis for stacked and h-stacked charts`
              );
            }

            break;
          }
          case "geomap": {
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.latitude == null
            ) {
              errors.push("Add one field for the latitude");
            }
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.longitude == null
            ) {
              errors.push("Add one field for the longitude");
            }
            break;
          }

          case "sankey": {
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.source == null
            ) {
              errors.push("Add one field for the source");
            }
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.target == null
            ) {
              errors.push("Add one field for the target");
            }
            if (
              dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
                .fields.value == null
            ) {
              errors.push("Add one field for the value");
            }
            break;
          }
          default:
            break;
        }

        // check if aggregation function is selected or not
        if (!(dashboardData.data.type == "heatmap")) {
          const aggregationFunctionError = dashboardData.data.queries[
            dashboardData.layout.currentQueryIndex
          ].fields.y.filter(
            (it: any) =>
              it.aggregationFunction == null || it.aggregationFunction == ""
          );
          if (
            dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
              .fields.y.length &&
            aggregationFunctionError.length
          ) {
            errors.push(
              ...aggregationFunctionError.map(
                (it: any) =>
                  `${currentYLabel.value}: ${it.column}: Aggregation function required`
              )
            );
          }
        }

        // check if labels are there for y axis items
        const labelError = dashboardData.data.queries[
          dashboardData.layout.currentQueryIndex
        ].fields.y.filter((it: any) => it.label == null || it.label == "");
        if (
          dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
            .fields.y.length &&
          labelError.length
        ) {
          errors.push(
            ...labelError.map(
              (it: any) =>
                `${currentYLabel.value}: ${it.column}: Label required`
            )
          );
        }

        // if there are filters
        if (
          dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
            .fields.filter.length
        ) {
          // check if at least 1 item from the list is selected
          const listFilterError = dashboardData.data.queries[
            dashboardData.layout.currentQueryIndex
          ].fields.filter.filter(
            (it: any) => it.type == "list" && !it.values?.length
          );
          if (listFilterError.length) {
            errors.push(
              ...listFilterError.map(
                (it: any) =>
                  `Filter: ${it.column}: Select at least 1 item from the list`
              )
            );
          }

          // check if condition operator is selected
          const conditionFilterError = dashboardData.data.queries[
            dashboardData.layout.currentQueryIndex
          ].fields.filter.filter(
            (it: any) => it.type == "condition" && it.operator == null
          );
          if (conditionFilterError.length) {
            errors.push(
              ...conditionFilterError.map(
                (it: any) => `Filter: ${it.column}: Operator selection required`
              )
            );
          }

          // check if condition value is selected
          const conditionValueFilterError = dashboardData.data.queries[
            dashboardData.layout.currentQueryIndex
          ].fields.filter.filter(
            (it: any) =>
              it.type == "condition" &&
              !["Is Null", "Is Not Null"].includes(it.operator) &&
              (it.value == null || it.value == "")
          );
          if (conditionValueFilterError.length) {
            errors.push(
              ...conditionValueFilterError.map(
                (it: any) => `Filter: ${it.column}: Condition value required`
              )
            );
          }
        }

        // check if query syntax is valid
        if (
          dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
            .customQuery &&
          dashboardData.meta.errors.queryErrors.length
        ) {
          errors.push("Please add valid query syntax");
        }

        // check if field selection is from the custom query fields when the custom query mode is ON
        if (
          dashboardData.data.queries[dashboardData.layout.currentQueryIndex]
            .customQuery
        ) {
          const customQueryXFieldError = dashboardPanelData.data.queries[
            dashboardData.layout.currentQueryIndex
          ].fields.x.filter(
            (it: any) =>
              !dashboardPanelData.meta.stream.customQueryFields.find(
                (i: any) => i.name == it.column
              )
          );
          if (customQueryXFieldError.length) {
            errors.push(
              ...customQueryXFieldError.map(
                (it: any) =>
                  `Please update X-Axis Selection. Current X-Axis field ${it.column} is invalid`
              )
            );
          }

          const customQueryYFieldError = dashboardPanelData.data.queries[
            dashboardData.layout.currentQueryIndex
          ].fields.y.filter(
            (it: any) =>
              !dashboardPanelData.meta.stream.customQueryFields.find(
                (i: any) => i.name == it.column
              )
          );
          if (customQueryYFieldError.length) {
            errors.push(
              ...customQueryYFieldError.map(
                (it: any) =>
                  `Please update Y-Axis Selection. Current Y-Axis field ${it.column} is invalid`
              )
            );
          }
        } else {
          // check if field selection is from the selected stream fields when the custom query mode is OFF
          const customQueryXFieldError = dashboardPanelData.data.queries[
            dashboardData.layout.currentQueryIndex
          ].fields.x.filter(
            (it: any) =>
              !dashboardPanelData.meta.stream.selectedStreamFields.find(
                (i: any) => i.name == it.column
              )
          );
          if (customQueryXFieldError.length) {
            errors.push(
              ...customQueryXFieldError.map(
                (it: any) =>
                  `Please update X-Axis Selection. Current X-Axis field ${it.column} is invalid for selected stream`
              )
            );
          }

          const customQueryYFieldError = dashboardPanelData.data.queries[
            dashboardData.layout.currentQueryIndex
          ].fields.y.filter(
            (it: any) =>
              !dashboardPanelData.meta.stream.selectedStreamFields.find(
                (i: any) => i.name == it.column
              )
          );
          if (customQueryYFieldError.length) {
            errors.push(
              ...customQueryYFieldError.map(
                (it: any) =>
                  `Please update Y-Axis Selection. Current Y-Axis field ${it.column} is invalid for selected stream`
              )
            );
          }
        }
      }

      // show all the errors
      // for (let index = 0; index < errors.length; index++) {
      //   $q.notify({
      //     type: "negative",
      //     message: errors[index],
      //     timeout: 5000,
      //   });
      // }

      if (errors.length) {
        $q.notify({
          type: "negative",
          message: "There are some errors, please fix them and try again",
          timeout: 5000,
        });
      }

      if (errors.length) {
        return false;
      } else {
        return true;
      }
    };

    const savePanelChangesToDashboard = async (dashId: string) => {
      if (!isValid()) {
        return;
      }

      try {
        // console.time("savePanelChangesToDashboard");
        if (editMode.value) {
          const errorMessageOnSave = await updatePanel(
            store,
            dashId,
            dashboardPanelData.data,
            route.query.folder ?? "default",
            route.query.tab ?? currentDashboardData.data.tabs[0].tabId
          );
          if (errorMessageOnSave instanceof Error) {
            errorData.errors.push(
              "Error saving panel configuration : " + errorMessageOnSave.message
            );
            return;
          }
        } else {
          const panelId =
            "Panel_ID" + Math.floor(Math.random() * (99999 - 10 + 1)) + 10;

          dashboardPanelData.data.id = panelId;
          chartData.value = JSON.parse(JSON.stringify(dashboardPanelData.data));

          const errorMessageOnSave = await addPanel(
            store,
            dashId,
            dashboardPanelData.data,
            route.query.folder ?? "default",
            route.query.tab ?? currentDashboardData.data.tabs[0].tabId
          );
          if (errorMessageOnSave instanceof Error) {
            errorData.errors.push(
              "Error saving panel configuration  : " +
                errorMessageOnSave.message
            );
            return;
          }
        }
        // console.timeEnd("savePanelChangesToDashboard");

        isPanelConfigWatcherActivated = false;
        isPanelConfigChanged.value = false;

        await nextTick();
        return router.push({
          path: "/dashboards/view",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
            dashboard: dashId,
            folder: route.query.folder ?? "default",
            tab: route.query.tab ?? currentDashboardData.data.tabs[0].tabId,
          },
        });
      } catch (error: any) {
        $q.notify({
          type: "negative",
          message:
            error?.message ??
            (editMode.value
              ? "Error while updating panel"
              : "Error while creating panel"),
          timeout: 2000,
        });
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
        hoveredTime?: any
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

    return {
      t,
      updateDateTime,
      goBack,
      savePanelChangesToDashboard,
      runQuery,
      layoutSplitterUpdated,
      expandedSplitterHeight,
      querySplitterUpdated,
      currentDashboard,
      list,
      dashboardPanelData,
      chartData,
      editMode,
      selectedDate,
      errorData,
      handleChartApiError,
      variablesDataUpdated,
      currentDashboardData,
      variablesData,
      savePanelData,
      resetAggregationFunction,
      isOutDated,
      store,
      dateTimePickerRef,
      showViewPanel,
      metaDataValue,
      metaData,
      panelTitle,
      onDataZoom,
      showTutorial,
    };
  },
  methods: {
    goBackToDashboardList(evt: any, row: any) {
      this.goBack();
    },
  },
});
</script>

<style lang="scss" scoped>
.layout-panel-container {
  display: flex;
  flex-direction: column;
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
