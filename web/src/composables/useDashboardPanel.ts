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

import { reactive, computed } from "vue";
import StreamService from "@/services/stream";
import { useStore } from "vuex";
import { useQuasar } from "quasar";

const colors = [
  "#5960b2",
  "#c23531",
  "#2f4554",
  "#61a0a8",
  "#d48265",
  "#91c7ae",
  "#749f83",
  "#ca8622",
  "#bda29a",
  "#6e7074",
  "#546570",
  "#c4ccd3",
];

const getDefaultDashboardPanelData: any = () => ({
  data: {
    version: 3,
    id: "",
    type: "bar",
    title: "",
    description: "",
    config: {
      show_legends: true,
      legends_position: null,
      unit: null,
      unit_custom: null,
      decimals: 2,
      axis_width: null,
      axis_border_show: false,
      legend_width: {
        value: null,
        unit: "px",
      },
      base_map: {
        type: "osm",
      },
      map_view: {
        zoom: 1,
        lat: 0,
        lng: 0,
      },
      map_symbol_style: {
        size: "by Value",
        size_by_value: {
          min: 1,
          max: 100,
        },
        size_fixed: 2,
      },
      drilldown: [],
      connect_nulls: false,
      wrap_table_cells: false,
      color: {
        mode: "palette-classic-by-series",
        fixedColor: ["#53ca53"],
        seriesBy: "last",
      },
    },
    htmlContent: "",
    markdownContent: "",
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
          source: null,
          target: null,
          value: null,
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

let dashboardPanelData = reactive({ ...getDefaultDashboardPanelData() });

const useDashboardPanelData = () => {
  const store = useStore();
  const $q = useQuasar();

  const cleanupDraggingFields = () => {
    dashboardPanelData.meta.dragAndDrop.currentDragArea = null;
    dashboardPanelData.meta.dragAndDrop.targetDragIndex = -1;
    dashboardPanelData.meta.dragAndDrop.dragging = false;
    dashboardPanelData.meta.dragAndDrop.dragElement = null;
    dashboardPanelData.meta.dragAndDrop.dragSource = null;
    dashboardPanelData.meta.dragAndDrop.dragSourceIndex = null;
  };

  // get default queries
  const getDefaultQueries = () => {
    return getDefaultDashboardPanelData().data.queries;
  };

  const addQuery = () => {
    const queryType =
      dashboardPanelData.data.queryType === "sql" ? "sql" : "promql";
    const newQuery: any = {
      query: "",
      customQuery: queryType === "promql",
      fields: {
        stream:
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream,
        stream_type:
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream_type,
        x: [],
        y: [],
        z: [],
        filter: [],
        latitude: null,
        longitude: null,
        weight: null,
        source: null,
        target: null,
        value: null,
      },
      config: {
        promql_legend: "",
        layer_type: "scatter",
        weight_fixed: 1,
      },
    };
    dashboardPanelData.data.queries.push(newQuery);
  };

  const removeQuery = (index: number) => {
    dashboardPanelData.data.queries.splice(index, 1);
  };

  const resetDashboardPanelData = () => {
    Object.assign(dashboardPanelData, getDefaultDashboardPanelData());
  };

  const generateLabelFromName = (name: string) => {
    return name
      .replace(/[\_\-\s\.]/g, " ")
      .split(" ")
      .map((string) => string.charAt(0).toUpperCase() + string.slice(1))
      .filter((it) => it)
      .join(" ");
  };

  const promqlMode = computed(
    () => dashboardPanelData.data.queryType == "promql"
  );

  const isAddXAxisNotAllowed = computed((e: any) => {
    switch (dashboardPanelData.data.type) {
      case "pie":
      case "donut":
      case "heatmap":
      case "gauge":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.x.length >= 1
        );
      case "metric":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.x.length >= 0
        );
      case "table":
        return false;
      default:
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.x.length >= 2
        );
    }
  });

  const isAddYAxisNotAllowed = computed((e: any) => {
    switch (dashboardPanelData.data.type) {
      case "pie":
      case "donut":
      case "gauge":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.y.length >= 1
        );
      case "metric":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.y.length >= 1
        );
      case "area-stacked":
      case "stacked":
      case "heatmap":
      case "h-stacked":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.y.length >= 1
        );
      default:
        return false;
    }
  });

  const isAddZAxisNotAllowed = computed((e: any) => {
    switch (dashboardPanelData.data.type) {
      case "heatmap":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.z.length >= 1
        );
      default:
        return false;
    }
  });

  const addXAxisItem = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x = [];
    }

    if (isAddXAxisNotAllowed.value) {
      return;
    }

    // check for existing field
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x.find((it: any) => it.column == row.name)
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x.push({
        label: !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery
          ? generateLabelFromName(row.name)
          : row.name,
        alias: !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery
          ? "x_axis_" +
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.x.length +
              1)
          : row.name,
        column: row.name,
        color: null,
        aggregationFunction:
          row.name == store.state.zoConfig.timestamp_column
            ? "histogram"
            : null,
        sortBy:
          row.name == store.state.zoConfig.timestamp_column
            ? dashboardPanelData.data.type == "table"
              ? "DESC"
              : "ASC"
            : null,
      });
    }

    updateArrayAlias();
  };

  const addYAxisItem = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y = [];
    }

    if (isAddYAxisNotAllowed.value) {
      return;
    }

    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y.find((it: any) => it?.column == row.name)
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y.push({
        label: !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery
          ? generateLabelFromName(row.name)
          : row.name,
        alias: !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery
          ? "y_axis_" +
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.y.length +
              1)
          : row.name,
        column: row.name,
        color: getNewColorValue(),
        aggregationFunction:
          dashboardPanelData.data.type == "heatmap" ? null : "count",
      });
    }
    updateArrayAlias();
  };

  const addZAxisItem = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z = [];
    }

    if (isAddZAxisNotAllowed.value) {
      return;
    }

    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z.find((it: any) => it.column == row.name)
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z.push({
        label: !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery
          ? generateLabelFromName(row.name)
          : row.name,
        alias: !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery
          ? "z_axis_" +
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.z.length +
              1)
          : row.name,
        column: row.name,
        color: getNewColorValue(),
        aggregationFunction: "count",
      });
    }
    updateArrayAlias();
  };

  const addLatitude = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.latitude
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.latitude = {
        label: generateLabelFromName(row.name),
        alias: "latitude",
        column: row.name,
        color: getNewColorValue(),
        aggregationFunction: null, // You can set the appropriate aggregation function here
      };
    }
  };

  const addLongitude = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.longitude
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.longitude = {
        label: generateLabelFromName(row.name),
        alias: "longitude",
        column: row.name,
        color: getNewColorValue(),
        aggregationFunction: null, // You can set the appropriate aggregation function here
      };
    }
  };

  const addWeight = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.weight
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.weight = {
        label: generateLabelFromName(row.name),
        alias: "weight",
        column: row.name,
        color: getNewColorValue(),
        aggregationFunction: "count", // You can set the appropriate aggregation function here
      };
    }
  };

  const addSource = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.source
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.source = {
        label: generateLabelFromName(row.name),
        alias: "source",
        column: row.name,
        color: getNewColorValue(),
        aggregationFunction: null, // You can set the appropriate aggregation function here
      };
    }
  };

  const addTarget = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.target
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.target = {
        label: generateLabelFromName(row.name),
        alias: "target",
        column: row.name,
        color: getNewColorValue(),
        aggregationFunction: null, // You can set the appropriate aggregation function here
      };
    }
  };

  const addValue = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.value
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.value = {
        label: generateLabelFromName(row.name),
        alias: "value",
        column: row.name,
        color: getNewColorValue(),
        aggregationFunction: "sum", // You can set the appropriate aggregation function here
      };
    }
  };

  // get new color value based on existing color from the chart
  const getNewColorValue = () => {
    const YAxisColor = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.y.map((it: any) => it?.color);
    let newColor = colors.filter((el: any) => !YAxisColor.includes(el));
    if (!newColor.length) {
      newColor = colors;
    }
    return newColor[0];
  };

  const resetAggregationFunction = () => {
    switch (dashboardPanelData.data.type) {
      case "heatmap":
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y.forEach((itemY: any) => {
          itemY.aggregationFunction = null;
        });
        dashboardPanelData.data.queries?.forEach((query: any) => {
          query.fields.latitude = null;
          query.fields.longitude = null;
          query.fields.weight = null;
          query.fields.source = null;
          query.fields.target = null;
          query.fields.value = null;
        });
        if (dashboardPanelData.data.queryType === "sql") {
          dashboardPanelData.layout.currentQueryIndex = 0;
          dashboardPanelData.data.queries =
            dashboardPanelData.data.queries.slice(0, 1);
        }
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.markdownContent = "";
        break;

      case "area":
      case "area-stacked":
      case "bar":
      case "line":
      case "scatter":
      case "pie":
      case "donut":
      case "h-bar":
      case "stacked":
      case "h-stacked":
      case "metric":
      case "table":
      case "gauge":
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y.forEach((itemY: any) => {
          if (itemY.aggregationFunction === null) {
            itemY.aggregationFunction = "count";
          }
        });
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z = [];
        // we have multiple queries for geomap, so if we are moving away, we need to reset
        // the values of lat, lng and weight in all the queries
        dashboardPanelData.data.queries?.forEach((query: any) => {
          query.fields.latitude = null;
          query.fields.longitude = null;
          query.fields.weight = null;
          query.fields.source = null;
          query.fields.target = null;
          query.fields.value = null;
        });
        if (dashboardPanelData.data.queryType === "sql") {
          dashboardPanelData.layout.currentQueryIndex = 0;
          dashboardPanelData.data.queries =
            dashboardPanelData.data.queries.slice(0, 1);
        }
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.markdownContent = "";
        break;
      case "geomap":
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.x = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z = [];
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.markdownContent = "";
        dashboardPanelData.data.queries?.forEach((query: any) => {
          query.fields.source = null;
          query.fields.target = null;
          query.fields.value = null;
        });
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.limit = 0;
        break;
      case "html":
        dashboardPanelData.data.queries = getDefaultQueries();
        dashboardPanelData.data.markdownContent = "";
        dashboardPanelData.data.queryType = "";
        break;
      case "markdown":
        dashboardPanelData.data.queries = getDefaultQueries();
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.queryType = "";
        break;
      case "sankey":
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.x = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.filter = [];
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.markdownContent = "";
        dashboardPanelData.data.queries?.forEach((query: any) => {
          query.fields.latitude = null;
          query.fields.longitude = null;
          query.fields.weight = null;
        });
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.limit = 0;
      default:
        break;
    }

    // aggregation null then count
  };

  // update X or Y axis aliases when new value pushes into the X and Y axes arrays
  const updateArrayAlias = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields?.x?.forEach(
      (it: any, index: any) =>
        (it.alias = !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery
          ? "x_axis_" + (index + 1)
          : it?.column)
    );
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields?.y?.forEach(
      (it: any, index: any) =>
        (it.alias = !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery
          ? "y_axis_" + (index + 1)
          : it?.column)
    );
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields?.z?.forEach(
      (it: any, index: any) =>
        (it.alias = !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery
          ? "z_axis_" + (index + 1)
          : it?.column)
    );
  };

  const removeXAxisItem = (name: string) => {
    const index = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.x.findIndex((it: any) => it.column == name);
    if (index >= 0) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x.splice(index, 1);
    }
  };

  const removeYAxisItem = (name: string) => {
    const index = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.y.findIndex((it: any) => it.column == name);
    if (index >= 0) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y.splice(index, 1);
    }
  };

  const removeZAxisItem = (name: string) => {
    const index = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.z.findIndex((it: any) => it.column == name);
    if (index >= 0) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z.splice(index, 1);
    }
  };

  const removeFilterItem = (name: string) => {
    const index = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.filter.findIndex((it: any) => it.column == name);
    if (index >= 0) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.filter.splice(index, 1);
    }
  };

  const removeLatitude = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.latitude = null;
  };

  const removeLongitude = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.longitude = null;
  };

  const removeWeight = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.weight = null;
  };

  const removeSource = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.source = null;
  };

  const removeTarget = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.target = null;
  };

  const removeValue = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.value = null;
  };

  const addFilteredItem = (name: string) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.filter
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.filter = [];
    }

    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.filter.find((it: any) => it.column == name)
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.filter.push({
        type: "list",
        values: [],
        column: name,
        operator: null,
        value: null,
      });
    }

    if (!dashboardPanelData.meta.filterValue) {
      dashboardPanelData.meta.filterValue = [];
    }

    // remove any existing data
    const find = dashboardPanelData.meta.filterValue.findIndex(
      (it: any) => it.column == name
    );
    if (find >= 0) {
      dashboardPanelData.meta.filterValue.splice(find, 1);
    }

    StreamService.fieldValues({
      org_identifier: store.state.selectedOrganization.identifier,
      stream_name:
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream,
      start_time: new Date(
        dashboardPanelData.meta.dateTime["start_time"].toISOString()
      ).getTime(),
      end_time: new Date(
        dashboardPanelData.meta.dateTime["end_time"].toISOString()
      ).getTime(),
      fields: [name],
      size: 10,
      type: dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.stream_type,
    })
      .then((res: any) => {
        dashboardPanelData.meta.filterValue.push({
          column: name,
          value: res?.data?.hits?.[0]?.values
            .map((it: any) => it.zo_sql_key)
            .filter((it: any) => it),
        });
      })
      .catch((error: any) => {
        const errorDetailValue =
          error.response?.data.error_detail ||
          error.response?.data.message ||
          "Something went wrong!";
        const trimmedErrorMessage =
          errorDetailValue.length > 300
            ? errorDetailValue.slice(0, 300) + " ..."
            : errorDetailValue;
        $q.notify({
          type: "negative",
          message: trimmedErrorMessage,
          timeout: 5000,
        });
      });
  };

  const loadFilterItem = (name: any) => {
    StreamService.fieldValues({
      org_identifier: store.state.selectedOrganization.identifier,
      stream_name:
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream,
      start_time: new Date(
        dashboardPanelData.meta.dateTime["start_time"].toISOString()
      ).getTime(),
      end_time: new Date(
        dashboardPanelData.meta.dateTime["end_time"].toISOString()
      ).getTime(),
      fields: [name],
      size: 10,
      type: dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.stream_type,
    })
      .then((res: any) => {
        const find = dashboardPanelData.meta.filterValue.findIndex(
          (it: any) => it.column == name
        );
        if (find >= 0) {
          dashboardPanelData.meta.filterValue.splice(find, 1);
        }
        dashboardPanelData.meta.filterValue.push({
          column: name,
          value: res?.data?.hits?.[0]?.values
            .map((it: any) => it.zo_sql_key)
            .filter((it: any) => it)
            .map((it: any) => String(it)),
        });
      })
      .catch((error: any) => {
        const errorDetailValue =
          error.response?.data.error_detail ||
          error.response?.data.message ||
          "Something went wrong!";
        const trimmedErrorMessage =
          errorDetailValue.length > 300
            ? errorDetailValue.slice(0, 300) + " ..."
            : errorDetailValue;
        $q.notify({
          type: "negative",
          message: trimmedErrorMessage,
          timeout: 5000,
        });
      });
  };

  const removeXYFilters = () => {
    if (
      promqlMode.value ||
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].customQuery == false
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x.splice(
        0,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.x.length
      );
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y.splice(
        0,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y.length
      );
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z.splice(
        0,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z.length
      );
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.filter.splice(
        0,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.filter.length
      );
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.latitude = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.longitude = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.weight = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.source = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.target = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.value = null;
    }
  };

  // This function updates the x and y fields of a custom query in the dashboard panel data
  const updateXYFieldsForCustomQueryMode = () => {
    // Check if the custom query is enabled and PromQL mode is disabled
    if (
      !promqlMode.value &&
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].customQuery == true
    ) {
      // Loop through each custom query field in the dashboard panel data's stream meta
      dashboardPanelData.meta.stream.customQueryFields.forEach(
        (it: any, index: number) => {
          // Get the name of the current custom query field
          const { name } = it;

          // Determine the current field type based on the name
          let field;
          if (name === "latitude") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.latitude;
          } else if (name === "longitude") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.longitude;
          } else if (name === "weight") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.weight;
          } else if (name === "source") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.source;
          } else if (name === "target") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.target;
          } else if (name === "value") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.value;
          } else {
            // For other field types (x, y, z), determine the type and index as before
            let currentFieldType;

            if (
              index <
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.x.length
            ) {
              currentFieldType = "x";
            } else if (
              index <
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.x.length +
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.y.length
            ) {
              currentFieldType = "y";
            } else {
              currentFieldType = "z";
            }

            if (currentFieldType === "x") {
              field =
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.x[index];
            } else if (currentFieldType === "y") {
              field =
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.y[
                  index -
                    dashboardPanelData.data.queries[
                      dashboardPanelData.layout.currentQueryIndex
                    ].fields.x.length
                ];
            } else {
              field =
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.z[
                  index -
                    dashboardPanelData.data.queries[
                      dashboardPanelData.layout.currentQueryIndex
                    ].fields.x.length -
                    dashboardPanelData.data.queries[
                      dashboardPanelData.layout.currentQueryIndex
                    ].fields.y.length
                ];
            }

            // If the current field is a y or z field, set the aggregation function to "count"
            if (currentFieldType === "y" || currentFieldType === "z") {
              field.aggregationFunction = "count";
            }
          }

          // Update the properties of the current field
          field.alias = name; // Set the alias to the name of the custom query field
          field.column = name; // Set the column to the name of the custom query field
          field.color = null; // Reset the color to null
        }
      );
    }
  };

  // this updates the fields when you switch from the auto to custom
  const updateXYFieldsOnCustomQueryChange = (oldCustomQueryFields: any) => {
    // Create a copy of the old custom query fields array
    const oldArray = oldCustomQueryFields;
    // Create a deep copy of the new custom query fields array
    const newArray = JSON.parse(
      JSON.stringify(dashboardPanelData.meta.stream.customQueryFields)
    );

    // Check if the length of the old and new arrays are the same
    if (oldArray.length == newArray.length) {
      // Create an array to store the indexes of changed fields
      const changedIndex: any = [];
      // Iterate through the new array
      newArray.forEach((obj: any, index: any) => {
        const { name } = obj;
        // Check if the name of the field at the same index in the old array is different
        if (oldArray[index].name != name) {
          changedIndex.push(index);
        }
      });
      // Check if there is only one changed field
      if (changedIndex.length == 1) {
        const oldName = oldArray[changedIndex[0]]?.name;

        let fieldIndex = dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.x?.findIndex((it: any) => it?.alias == oldName);
        // Check if the field is in the x fields array
        if (fieldIndex >= 0) {
          const newName = newArray[changedIndex[0]]?.name;
          const field =
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.x[fieldIndex];

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }
        // Check if the field is in the y fields array
        fieldIndex = dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y?.findIndex((it: any) => it?.alias == oldName);
        if (fieldIndex >= 0) {
          const newName = newArray[changedIndex[0]]?.name;
          const field =
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.y[fieldIndex];

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }
        // Check if the field is in the z fields array
        fieldIndex = dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z?.findIndex((it: any) => it?.alias == oldName);
        if (fieldIndex >= 0) {
          const newName = newArray[changedIndex[0]]?.name;
          const field =
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.z[fieldIndex];

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the latitude fields
        let field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.latitude;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the longitude fields array
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.longitude;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the weight fields array
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.weight;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the source fields array
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.source;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the target fields array
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.target;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the value fields array
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.value;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }
      }
    }
  };

  return {
    dashboardPanelData,
    resetDashboardPanelData,
    updateArrayAlias,
    addXAxisItem,
    addYAxisItem,
    addZAxisItem,
    addLatitude,
    addLongitude,
    addWeight,
    addSource,
    addTarget,
    addValue,
    removeXAxisItem,
    removeYAxisItem,
    removeZAxisItem,
    removeFilterItem,
    removeLatitude,
    removeLongitude,
    removeWeight,
    removeSource,
    removeTarget,
    removeValue,
    addFilteredItem,
    loadFilterItem,
    removeXYFilters,
    updateXYFieldsForCustomQueryMode,
    updateXYFieldsOnCustomQueryChange,
    isAddXAxisNotAllowed,
    isAddYAxisNotAllowed,
    isAddZAxisNotAllowed,
    promqlMode,
    addQuery,
    removeQuery,
    resetAggregationFunction,
    cleanupDraggingFields,
    getDefaultQueries,
  };
};
export default useDashboardPanelData;
