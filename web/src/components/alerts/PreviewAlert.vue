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
  <div ref="chartPanelRef" style="height: 100%; position: relative">
    <div style="height: 200px" data-test="alert-preview-chart">
      <p class="sql-preview" v-if="selectedTab === 'sql'">
        Preview is not available in SQL mode
      </p>
      <PanelSchemaRenderer
        v-else-if="chartData"
        :height="6"
        :width="6"
        :panelSchema="chartData"
        :selectedTimeObj="dashboardPanelData.meta.dateTime"
        :variablesData="{}"
        searchType="UI"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import PanelSchemaRenderer from "../dashboards/PanelSchemaRenderer.vue";
import { reactive } from "vue";
import { onBeforeMount } from "vue";
import { cloneDeep } from "lodash-es";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";

const getDefaultDashboardPanelData: any = () => ({
  data: {
    version: 2,
    id: "",
    type: "bar",
    title: "",
    description: "",
    config: {
      show_legends: true,
      legends_position: null,
      unit: null,
      unit_custom: null,
      base_map: {
        type: "osm",
      },
      map_view: {
        zoom: 1,
        lat: 0,
        lng: 0,
      },
    },
    queryType: "sql",
    queries: [
      {
        query: "",
        customQuery: false,
        fields: {
          stream: "",
          stream_type: "logs",
          x: [],
          y: [],
          z: [],
          filter: [],
          latitude: null,
          longitude: null,
          weight: null,
        },
        config: {
          promql_legend: "",
          layer_type: "scatter",
          weight_fixed: 1,
          limit: 0,
          // gauge min and max values
          min: 0,
          max: 100,
        },
      },
    ],
  },
  layout: {
    splitter: 20,
    querySplitter: 20,
    showQueryBar: false,
    isConfigPanelOpen: false,
    currentQueryIndex: 0,
  },
  meta: {
    parsedQuery: "",
    dragAndDrop: {
      dragging: false,
      dragElement: null,
      dragSource: null,
      dragSourceIndex: null,
      currentDragArea: null,
      targetDragIndex: null,
    },
    errors: {
      queryErrors: [],
    },
    editorValue: "",
    dateTime: { start_time: "", end_time: "" },
    filterValue: <any>[],
    stream: {
      selectedStreamFields: [],
      customQueryFields: [],
      functions: [],
      streamResults: <any>[],
      filterField: "",
    },
  },
});

let dashboardPanelData: any = null;

const props = defineProps({
  query: {
    type: String,
    default: "",
  },
  formData: {
    type: Object,
    default: () => ({}),
  },
  isAggregationEnabled: {
    type: Boolean,
    default: false,
  },
  selectedTab: {
    type: String,
    default: "",
  },
});

onBeforeMount(() => {
  dashboardPanelData = reactive({ ...getDefaultDashboardPanelData() });
  dashboardPanelData.data.type = "line";
  dashboardPanelData.data.queryType =
    props.selectedTab === "promql" ? "promql" : "sql";
  dashboardPanelData.data.queries[0].query = props.query;
  dashboardPanelData.data.queries[0].fields.stream = props.formData.stream_name;
  dashboardPanelData.data.queries[0].fields.stream_type =
    props.formData.stream_type;
  dashboardPanelData.data.queries[0].customQuery = true;
});

const chartPanelRef = ref(null);
const chartData = ref({});
const { t } = useI18n();

const store = useStore();

const refreshData = () => {
  const relativeTime = props.formData.trigger_condition.period;

  const endTime = new Date().getTime() * 1000;
  let new_relative_time = 2;
  if (relativeTime < 2) {
    new_relative_time = relativeTime;
  }

  const startTime = endTime - new_relative_time * 60 * 1000000;

  dashboardPanelData.meta.dateTime = {
    start_time: new Date(startTime),
    end_time: new Date(endTime),
  };

  let xAxis = [
    {
      alias: "zo_sql_key",
      color: null,
      column: store.state.zoConfig.timestamp_column || "_timestamp",
      label: "Timestamp",
    },
  ];

  let yAxis = [];

  if (props.isAggregationEnabled) {
    yAxis.push({
      aggregationFunction: props.formData.query_condition.aggregation.function,
      alias: "zo_sql_val",
      color: null,
      column: store.state.zoConfig.timestamp_column || "_timestamp",
      label: t("alerts.numOfEvents"),
    });
  } else {
    yAxis.push({
      aggregationFunction: "count",
      alias: "zo_sql_val",
      color: null,
      column: store.state.zoConfig.timestamp_column || "_timestamp",
      label: t("alerts.numOfEvents"),
    });
  }

  if (
    props.selectedTab === "custom" &&
    props.formData.query_condition?.aggregation?.group_by?.length > 0 &&
    props.formData.query_condition?.aggregation?.group_by[0]?.trim() !== ""
  ) {
    xAxis.push({
      alias: "x_axis_2",
      color: null,
      column: props.formData.query_condition.aggregation.group_by[0],
      label: "x_axis_2",
    });
  }

  if (props.selectedTab === "promql") {
    xAxis = [];
    yAxis = [];
  }

  dashboardPanelData.data.queries[0].fields.x = xAxis;
  dashboardPanelData.data.queries[0].fields.y = yAxis;

  dashboardPanelData.data.queries[0].query = props.query;
  dashboardPanelData.data.queries[0].fields.stream = props.formData.stream_name;
  dashboardPanelData.data.queries[0].fields.stream_type =
    props.formData.stream_type;
  dashboardPanelData.data.queryType =
    props.selectedTab === "promql" ? "promql" : "sql";

  chartData.value = cloneDeep(dashboardPanelData.data);
};

defineExpose({ refreshData });
</script>

<style scoped>
.sql-preview {
  display: flex;
  justify-content: center;
  align-items: center;
  margin-top: 5vh;
}
</style>
