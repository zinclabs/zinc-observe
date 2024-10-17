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

<!-- eslint-disable vue/no-unused-components -->
<template>
  <div style="height: 100%; width: 100%">
    <q-separator></q-separator>
    <div class="row" style="height: 100%">
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
            <div class="col scroll" style="height: 100%; overflow-y: auto">
              <div class="column" style="height: 100%">
                <div class="col-auto q-pa-sm">
                  <span class="text-weight-bold">{{ t("panel.fields") }}</span>
                </div>
                <div class="col" style="width: 100%; height: 100%">
                  <FieldList :editMode="true" />
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
            <div class="row" style="height: 100%">
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
                      ? [0, 400]
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
                      <DashboardQueryBuilder :dashboardData="{}" />
                      <q-separator />

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
                          was not updated automatically. Click on the "Run
                          Query" button to run the query again
                        </div>
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
                            @metadata-update="metaDataValue"
                            :key="dashboardPanelData.data.type"
                            :panelSchema="chartData"
                            :selectedTimeObj="dashboardPanelData.meta.dateTime"
                            :variablesData="{}"
                            @updated:vrl-function-field-list="
                              updateVrlFunctionFieldList
                            "
                            :width="6"
                            @error="handleChartApiError"
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
                </q-splitter>
              </div>
              <q-separator vertical />
              <div class="col-auto">
                <PanelSidebar
                  :title="t('dashboard.configLabel')"
                  v-model="dashboardPanelData.layout.isConfigPanelOpen"
                >
                  <ConfigPanel :dashboardPanelData="dashboardPanelData" />
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
import { defineComponent, ref, watch, defineAsyncComponent } from "vue";
import PanelSidebar from "@/components/dashboards/addPanel/PanelSidebar.vue";
import ChartSelection from "@/components/dashboards/addPanel/ChartSelection.vue";
import FieldList from "@/components/dashboards/addPanel/FieldList.vue";

import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import DashboardQueryBuilder from "@/components/dashboards/addPanel/DashboardQueryBuilder.vue";
import useDashboardPanelData from "@/composables/useDashboardPanel";
import DashboardErrorsComponent from "@/components/dashboards/addPanel/DashboardErrors.vue";
import PanelSchemaRenderer from "@/components/dashboards/PanelSchemaRenderer.vue";
import { provide } from "vue";
import { toRefs } from "vue";
import { inject } from "vue";
import { computed } from "vue";
import { isEqual } from "lodash-es";
import { onActivated } from "vue";
import useNotifications from "@/composables/useNotifications";

const ConfigPanel = defineAsyncComponent(() => {
  return import("@/components/dashboards/addPanel/ConfigPanel.vue");
});

const CustomHTMLEditor = defineAsyncComponent(() => {
  return import("@/components/dashboards/addPanel/CustomHTMLEditor.vue");
});

const CustomMarkdownEditor = defineAsyncComponent(() => {
  return import("@/components/dashboards/addPanel/CustomMarkdownEditor.vue");
});

const AddToDashboard = defineAsyncComponent(() => {
  return import("./../metrics/AddToDashboard.vue");
});

export default defineComponent({
  name: "VisualizeLogsQuery",
  props: {
    visualizeChartData: {
      type: Object,
      required: true,
    },
    errorData: {
      type: Object,
      required: true,
    },
  },
  components: {
    ChartSelection,
    FieldList,
    DashboardQueryBuilder,
    DashboardErrorsComponent,
    PanelSidebar,
    ConfigPanel,
    PanelSchemaRenderer,
    CustomHTMLEditor,
    CustomMarkdownEditor,
    AddToDashboard,
  },
  emits: ["handleChartApiError"],
  setup(props, { emit }) {
    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "logs"
    );
    const { t } = useI18n();
    const store = useStore();
    const { dashboardPanelData, resetAggregationFunction, validatePanel } =
      useDashboardPanelData(dashboardPanelDataPageKey);
    const metaData = ref(null);
    const metaDataValue = (metadata: any) => {
      metaData.value = metadata;
    };
    const { showErrorNotification } = useNotifications();

    const { visualizeChartData }: any = toRefs(props);
    const chartData = ref(visualizeChartData.value);

    const showAddToDashboardDialog = ref(false);

    watch(
      () => visualizeChartData.value,
      async () => {
        // await nextTick();
        chartData.value = JSON.parse(JSON.stringify(visualizeChartData.value));
      }
    );

    const isOutDated = computed(() => {
      //compare chartdata and dashboardpaneldata
      return !isEqual(chartData.value, dashboardPanelData.data);
    });

    watch(isOutDated, () => {
      window.dispatchEvent(new Event("resize"));
    });

    // resize the chart when config panel is opened and closed
    watch(
      () => dashboardPanelData.layout.isConfigPanelOpen,
      () => {
        window.dispatchEvent(new Event("resize"));
      }
    );

    // resize the chart when query editor is opened and closed
    watch(
      () => dashboardPanelData.layout.showQueryBar,
      (newValue) => {
        if (!newValue) {
          dashboardPanelData.layout.querySplitter = 41;
        } else {
          if (expandedSplitterHeight.value !== null) {
            dashboardPanelData.layout.querySplitter =
              expandedSplitterHeight.value;
          }
        }
      }
    );

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
      emit("handleChartApiError", errorMessage);
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

    const addToDashboard = () => {
      const errors: any = [];
      // will push errors in errors array
      validatePanel(errors);

      if (errors.length) {
        // set errors into errorData
        props.errorData.errors = errors;
        showErrorNotification(
          "There are some errors, please fix them and try again"
        );
        return;
      } else {
        showAddToDashboardDialog.value = true;
      }
    };

    const addPanelToDashboard = () => {
      showAddToDashboardDialog.value = false;
    };

    onActivated(() => {
      dashboardPanelData.layout.querySplitter = 20;
    });

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
          ]?.fields?.latitude.alias
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
          ]?.fields?.longitude.alias
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
          ]?.fields?.weight.alias
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
          ]?.fields?.source.alias
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
          ]?.fields?.target.alias
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
          ]?.fields?.value.alias
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

    return {
      t,
      layoutSplitterUpdated,
      expandedSplitterHeight,
      querySplitterUpdated,
      dashboardPanelData,
      handleChartApiError,
      resetAggregationFunction,
      store,
      metaDataValue,
      metaData,
      chartData,
      showAddToDashboardDialog,
      addPanelToDashboard,
      addToDashboard,
      isOutDated,
      updateVrlFunctionFieldList,
    };
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
