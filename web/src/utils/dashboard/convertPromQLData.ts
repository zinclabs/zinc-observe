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

import moment from "moment";
import {
  formatDate,
  formatUnitValue,
  getUnitValue,
} from "./convertDataIntoUnitValue";
import { utcToZonedTime } from "date-fns-tz";
import { calculateGridPositions } from "./calculateGridForSubPlot";

// const colorArray = [
//   '#0056b3', '#003e80', '#00294d', '#001326',
//   '#28a745', '#1f8e3e', '#156331', '#0e3f24', '#072717',
//   '#dc3545', '#a52a35', '#751e26', '#4e141a', '#27090d',
//   '#fd7e14', '#d46a11', '#a9560e', '#803f0a', '#551f06',
//   '#ffc107', '#d9a106', '#b28605', '#8b6a04', '#634d03',
//   '#6f42c1', '#57348f', '#412663', '#2a1940', '#160c20',
//   '#e83e8c', '#b82d66', '#8c1f49', '#631433', '#3f0a1d'
// ];

// [
//   "#99cadd",
//   "#a9bdd5",
//   "#d6a9bc",
//   "#c691a4",
//   "#b87c94",
//   "#ffcb20",
//   "#ff3b3b",
//   "#ff8d0c",
//   "#42a6ff",
//   "#ffb3ba",
//   "#ffdfba",
//   "#ffffba",
//   "#baffc9",
//   "#bae1ff",
//   "#a8e6cf",
//   "#dcedc1",
//   "#ffd3b6",
//   "#ffaaa5",
//   "#ff8b94",
//   "#ffbf00",
//   "#ffcf40",
//   "#ffdc73",
//   "#b88c8c",
//   "#ddadad",
//   "#d6c7c7",
//   "#9fb9bf",
//   "#aec8ce",
// ];

// [
//   "#E24D42", // Red
//   "#1F78C1", // Blue
//   "#BA43A9", // Purple
//   "#705DA0", // Violet
//   "#466803", // Green
//   "#508642", // Dark Green
//   "#447EBC", // Dark Blue
//   "#C15C17", // Brown
//   "#890F02", // Dark Red
//   "#757575", // Grey
//   "#70DBED", // Light Blue
//   "#6ED0E0", // Turquoise
//   "#B9D7D9", // Light Grey
//   "#D683CE", // Light Purple
//   "#E5AC0E", // Orange
//   "#AEA2E0", // Lavender
//   "#5195CE", // Bright Blue
//   "#D9BF77", // Beige
//   "#FADE2A", // Yellow
//   "#2F575E", // Dark Turquoise
//   "#99440A", // Dark Brown
//   "#58140C", // Maroon
//   "#052B51", // Navy
//   "#511749", // Dark Violet
//   "#3F2B5B", // Dark Indigo
//   "#E0F9D7", // Light Green
//   "#FCEACA", // Light Yellow
//   "#CFFAFF", // Sky Blue
//   "#F9E2D2", // Light Pink
//   "#FCE2DE", // Pink
//   "#BADFF4", // Light Bright Blue
//   "#F9D9F9", // Light Lavender
//   "#DEDAF7"  // Pale Lavender
// ]

const colorArrayBySeries = [
  "#5470c6",
  "#91cc75",
  "#fac858",
  "#ee6666",
  "#73c0de",
  "#3ba272",
  "#fc8452",
  "#9a60b4",
  "#dc1e1e",
  "#9086e9",
  "#27e162",
  "#e08de6",
  "#de6086",
  "rgba(161,225,200,0.48)",
  "#ec8ceb",
  "#99e9f1",
  "#eec59c",
];

const colorArrayByValue = [
  "#0ac670",
  "#08b968",
  "#08aa60",
  "#079f59",
  "#069a55",
];

function getIndexForName(name: any, colorArray: any) {
  let index = 0;
  for (let i = 0; i < 15; i++) {
    const charCode = name.charCodeAt(i) || 0;
    index += charCode * (i + 1);
  }
  return index % colorArray.length;
}

function nameToColor(name: any, colorArray: any) {
  const index = getIndexForName(name, colorArray);
  return colorArray[index];
}

/**
 * Converts PromQL data into a format suitable for rendering a chart.
 *
 * @param {any} panelSchema - the panel schema object
 * @param {any} searchQueryData - the search query data
 * @param {any} store - the store object
 * @return {Object} - the option object for rendering the chart
 */
export const convertPromQLData = (
  panelSchema: any,
  searchQueryData: any,
  store: any,
  chartPanelRef: any,
  hoveredSeriesState: any
) => {
  // if no data than return it
  if (
    !Array.isArray(searchQueryData) ||
    searchQueryData.length === 0 ||
    !searchQueryData[0] ||
    !panelSchema
  ) {
    return { options: null };
  }

  // flag to check if the data is time seriesc
  let isTimeSeriesFlag = true;

  const legendPosition = getLegendPosition(
    panelSchema?.config?.legends_position
  );

  const legendConfig: any = {
    show: panelSchema.config?.show_legends,
    type: "scroll",
    orient: legendPosition,
    padding: [10, 20, 10, 10],
    tooltip: {
      show: true,
      padding: 10,
      textStyle: {
        fontSize: 12,
      },
      formatter: (params: any) => {
        hoveredSeriesState?.value?.setHoveredSeriesName(params?.name);
        return params?.name;
      },
    },
    textStyle: {
      width: 150,
      overflow: "truncate",
      rich: {
        a: {
          fontWeight: "bold",
        },
        b: {
          fontStyle: "normal",
        },
      },
    },
    formatter: (name: any) => {
      return name == hoveredSeriesState?.value?.hoveredSeriesName
        ? "{a|" + name + "}"
        : "{b|" + name + "}";
    },
  };

  // Additional logic to adjust the legend position
  if (legendPosition === "vertical") {
    legendConfig.left = null; // Remove left positioning
    legendConfig.right = 0; // Apply right positioning
    legendConfig.top = "center"; // Apply bottom positioning
  } else {
    legendConfig.left = "0"; // Apply left positioning
    legendConfig.top = "bottom"; // Apply bottom positioning
  }

  const options: any = {
    backgroundColor: "transparent",
    legend: legendConfig,
    // color: colorArrayByValue,
    grid: {
      containLabel: panelSchema.config?.axis_width == null ? true : false,
      //based on config width set grid
      left: panelSchema.config?.axis_width ?? 5,
      right: 20,
      top: "15",
      bottom:
        legendConfig.orient === "horizontal" && panelSchema.config?.show_legends
          ? panelSchema.config?.axis_width == null
            ? 30
            : 50
          : panelSchema.config?.axis_width == null
          ? 5
          : "25",
    },
    tooltip: {
      show: true,
      trigger: "axis",
      textStyle: {
        color: store.state.theme === "dark" ? "#fff" : "#000",
        fontSize: 12,
      },
      enterable: true,
      backgroundColor:
        store.state.theme === "dark" ? "rgba(0,0,0,1)" : "rgba(255,255,255,1)",
      extraCssText: "max-height: 200px; overflow: auto; max-width: 500px",
      formatter: function (name: any) {
        // show tooltip for hovered panel only for other we only need axis so just return empty string
        if (
          hoveredSeriesState?.value &&
          hoveredSeriesState?.value?.panelId != panelSchema.id
        )
          return "";
        if (name.length == 0) return "";

        const date = new Date(name[0].data[0]);

        // get the current series index from name
        const currentSeriesIndex = name.findIndex(
          (it: any) =>
            it.seriesName == hoveredSeriesState?.value?.hoveredSeriesName
        );

        // swap current hovered series index to top in tooltip
        const temp = name[0];
        name[0] = name[currentSeriesIndex != -1 ? currentSeriesIndex : 0];
        name[currentSeriesIndex != -1 ? currentSeriesIndex : 0] = temp;

        let hoverText = name.map((it: any) => {
          // check if the series is the current series being hovered
          // if have than bold it
          if (it?.seriesName == hoveredSeriesState?.value?.hoveredSeriesName)
            return `<strong>${it.marker} ${it.seriesName} : ${formatUnitValue(
              getUnitValue(
                it.data[1],
                panelSchema.config?.unit,
                panelSchema.config?.unit_custom,
                panelSchema.config?.decimals
              )
            )} </strong>`;
          // else normal text
          else
            return `${it.marker} ${it.seriesName} : ${formatUnitValue(
              getUnitValue(
                it.data[1],
                panelSchema.config?.unit,
                panelSchema.config?.unit_custom,
                panelSchema.config?.decimals
              )
            )}`;
        });

        return `${formatDate(date)} <br/> ${hoverText.join("<br/>")}`;
      },
      axisPointer: {
        show: true,
        type: "cross",
        label: {
          fontSize: 12,
          precision: panelSchema.config?.decimals,
          show: true,
          formatter: function (name: any) {
            if (name.axisDimension == "y")
              return formatUnitValue(
                getUnitValue(
                  name.value,
                  panelSchema.config?.unit,
                  panelSchema.config?.unit_custom,
                  panelSchema.config?.decimals
                )
              );
            const date = new Date(name.value);
            return `${formatDate(date)}`;
          },
        },
      },
    },
    xAxis: {
      type: "time",
      axisLine: {
        show: panelSchema.config?.axis_border_show || false,
      },
      splitLine: {
        show: true,
        lineStyle: {
          opacity: 0.5,
        },
      },
      axisLabel: {
        // hide axis label if overlaps
        hideOverlap: true,
      },
    },
    yAxis: {
      type: "value",
      axisLabel: {
        formatter: function (name: any) {
          return formatUnitValue(
            getUnitValue(
              name,
              panelSchema.config?.unit,
              panelSchema.config?.unit_custom,
              panelSchema.config?.decimals
            )
          );
        },
      },
      axisLine: {
        show: panelSchema.config?.axis_border_show || false,
      },
      splitLine: {
        show: true,
        lineStyle: {
          opacity: 0.5,
        },
      },
    },
    toolbox: {
      orient: "vertical",
      show: !["pie", "donut", "metric", "gauge"].includes(panelSchema.type),
      showTitle: false,
      tooltip: {
        show: false,
      },
      itemSize: 0,
      itemGap: 0,
      // it is used to hide toolbox buttons
      bottom: "100%",
      feature: {
        dataZoom: {
          filterMode: "none",
          yAxisIndex: "none",
        },
      },
    },
    series: [],
  };
  // to pass grid index in gauge chart
  let gaugeIndex = 0;

  // for gauge chart we need total no. of gauge to calculate grid positions
  let totalLength = 0;
  // for gauge chart, it contains grid array, single chart height and width, no. of charts per row and no. of columns
  let gridDataForGauge: any = {};

  if (panelSchema.type === "gauge") {
    // calculate total length of all metrics
    searchQueryData.forEach((metric: any) => {
      if (metric.result && Array.isArray(metric.result)) {
        totalLength += metric.result.length;
      }
    });

    // create grid array based on chart panel width, height and total no. of gauge
    gridDataForGauge = calculateGridPositions(
      chartPanelRef.value.offsetWidth,
      chartPanelRef.value.offsetHeight,
      totalLength
    );

    //assign grid array to gauge chart options
    options.grid = gridDataForGauge.gridArray;
  }

  options.series = searchQueryData.map((it: any, index: number) => {
    switch (panelSchema.type) {
      case "bar":
      case "line":
      case "area":
      case "scatter":
      case "area-stacked": {
        switch (it?.resultType) {
          case "matrix": {
            const seriesObj = it?.result?.map((metric: any) => {
              const values = metric.values.sort(
                (a: any, b: any) => a[0] - b[0]
              );
              return {
                name: getPromqlLegendName(
                  metric.metric,
                  panelSchema.queries[index].config.promql_legend
                ),
                itemStyle: {
                  color: nameToColor(
                    getPromqlLegendName(
                      metric.metric,
                      panelSchema.queries[index].config.promql_legend
                    ),
                    colorArrayBySeries
                  ),
                },
                // colorBy: "data",
                // if utc then simply return the values by removing z from string
                // else convert time from utc to zoned
                // used slice to remove Z from isostring to pass as a utc
                data: values.map((value: any) => [
                  store.state.timezone != "UTC"
                    ? utcToZonedTime(value[0] * 1000, store.state.timezone)
                    : new Date(value[0] * 1000).toISOString().slice(0, -1),
                  value[1],
                ]),
                ...getPropsByChartTypeForSeries(panelSchema.type),
              };
            });

            return seriesObj;
          }
          case "vector": {
            const traces = it?.result?.map((metric: any) => {
              const values = [metric.value];
              return {
                name: JSON.stringify(metric.metric),
                x: values.map((value: any) =>
                  moment(value[0] * 1000).toISOString(true)
                ),
                y: values.map((value: any) => value[1]),
              };
            });
            return traces;
          }
        }
      }
      case "gauge": {
        // we doesnt required to hover timeseries for gauge chart
        isTimeSeriesFlag = false;

        const series = it?.result?.map((metric: any) => {
          const values = metric.values.sort((a: any, b: any) => a[0] - b[0]);
          gaugeIndex++;
          return {
            ...getPropsByChartTypeForSeries(panelSchema.type),
            min: panelSchema?.queries[index]?.config?.min || 0,
            max: panelSchema?.queries[index]?.config?.max || 100,

            //which grid will be used
            gridIndex: gaugeIndex - 1,
            // radius, progress and axisline width will be calculated based on grid width and height
            radius: `${
              Math.min(
                gridDataForGauge.gridWidth,
                gridDataForGauge.gridHeight
              ) /
                2 -
              5
            }px`,
            progress: {
              show: true,
              width: `${
                Math.min(
                  gridDataForGauge.gridWidth,
                  gridDataForGauge.gridHeight
                ) / 6
              }`,
            },
            axisLine: {
              lineStyle: {
                width: `${
                  Math.min(
                    gridDataForGauge.gridWidth,
                    gridDataForGauge.gridHeight
                  ) / 6
                }`,
              },
            },
            title: {
              fontSize: 10,
              offsetCenter: [0, "70%"],
              // width: upto chart width
              width: `${gridDataForGauge.gridWidth}`,
              overflow: "truncate",
            },

            // center of gauge
            // x: left + width / 2,
            // y: top + height / 2,
            center: [
              `${
                parseFloat(options.grid[gaugeIndex - 1].left) +
                parseFloat(options.grid[gaugeIndex - 1].width) / 2
              }%`,
              `${
                parseFloat(options.grid[gaugeIndex - 1].top) +
                parseFloat(options.grid[gaugeIndex - 1].height) / 2
              }%`,
            ],
            data: [
              {
                name: getPromqlLegendName(
                  metric.metric,
                  panelSchema.queries[index].config.promql_legend
                ),
                // taking first value for gauge
                value: values[0][1],
                detail: {
                  formatter: function (value: any) {
                    const unitValue = getUnitValue(
                      value,
                      panelSchema.config?.unit,
                      panelSchema.config?.unit_custom,
                      panelSchema.config?.decimals
                    );
                    return unitValue.value + unitValue.unit;
                  },
                },
              },
            ],
            detail: {
              valueAnimation: true,
              offsetCenter: [0, 0],
              fontSize: 12,
            },
          };
        });
        options.dataset = { source: [[]] };
        options.tooltip = {
          show: true,
          trigger: "item",
          textStyle: {
            color: store.state.theme === "dark" ? "#fff" : "#000",
            fontSize: 12,
          },
          valueFormatter: (value: any) => {
            // unit conversion
            return formatUnitValue(
              getUnitValue(
                value,
                panelSchema.config?.unit,
                panelSchema.config?.unit_custom,
                panelSchema.config?.decimals
              )
            );
          },
          enterable: true,
          backgroundColor:
            store.state.theme === "dark"
              ? "rgba(0,0,0,1)"
              : "rgba(255,255,255,1)",
          extraCssText: "max-height: 200px; overflow: auto; max-width: 500px",
        };
        options.angleAxis = {
          show: false,
        };
        options.radiusAxis = {
          show: false,
        };
        options.polar = {};
        options.xAxis = [];
        options.yAxis = [];
        return series;
      }
      case "metric": {
        // we doesnt required to hover timeseries for gauge chart
        isTimeSeriesFlag = false;

        switch (it?.resultType) {
          case "matrix": {
            const series = it?.result?.map((metric: any) => {
              const values = metric.values.sort(
                (a: any, b: any) => a[0] - b[0]
              );
              const unitValue = getUnitValue(
                values[values.length - 1][1],
                panelSchema.config?.unit,
                panelSchema.config?.unit_custom,
                panelSchema.config?.decimals
              );
              return {
                ...getPropsByChartTypeForSeries(panelSchema.type),
                renderItem: function (params: any) {
                  return {
                    type: "text",
                    style: {
                      text:
                        (parseFloat(unitValue?.value)?.toFixed(
                          panelSchema.config.decimals ?? 2
                        ) ?? 0) + unitValue.unit,
                      fontSize: Math.min(params.coordSys.cx / 2, 90), //coordSys is relative. so that we can use it to calculate the dynamic size
                      fontWeight: 500,
                      align: "center",
                      verticalAlign: "middle",
                      x: params.coordSys.cx,
                      y: params.coordSys.cy,
                      fill: store.state.theme == "dark" ? "#fff" : "#000",
                    },
                  };
                },
              };
            });
            options.dataset = { source: [[]] };
            options.tooltip = {
              show: false,
            };
            options.angleAxis = {
              show: false,
            };
            options.radiusAxis = {
              show: false,
            };
            options.polar = {};
            options.xAxis = [];
            options.yAxis = [];
            return series;
          }
          case "vector": {
            const traces = it?.result?.map((metric: any) => {
              const values = [metric.value];
              return {
                name: JSON.stringify(metric.metric),
                value: metric?.value?.length > 1 ? metric.value[1] : "",
                ...getPropsByChartTypeForSeries(panelSchema.type),
              };
            });
            return traces;
          }
        }
        break;
      }
      default: {
        return [];
      }
    }
  });

  options.series = options.series.flat();

  const calculateWidthText = (text: string): number => {
    if (!text) return 0;

    const span = document.createElement("span");
    document.body.appendChild(span);

    span.style.font = "sans-serif";
    span.style.fontSize = "12px";
    span.style.height = "auto";
    span.style.width = "auto";
    span.style.top = "0px";
    span.style.position = "absolute";
    span.style.whiteSpace = "no-wrap";
    span.innerHTML = text;

    const width = Math.ceil(span.clientWidth);
    span.remove();
    return width;
  };

  //from this maxValue want to set the width of the chart based on max value is greater than 30% than give default legend width other wise based on max value get legend width
  //only check for vertical side only
  if (
    legendConfig.orient == "vertical" &&
    panelSchema.config?.show_legends &&
    panelSchema.type != "gauge" &&
    panelSchema.type != "metric"
  ) {
    let legendWidth;

    if (
      panelSchema.config.legend_width &&
      !isNaN(parseFloat(panelSchema.config.legend_width.value))
      // ["px", "%"].includes(panelSchema.config.legend_width.unit)
    ) {
      if (panelSchema.config.legend_width.unit === "%") {
        // If in percentage, calculate percentage of the chartPanelRef width
        const percentage = panelSchema.config.legend_width.value / 100;
        legendWidth = chartPanelRef.value?.offsetWidth * percentage;
      } else {
        // If in pixels, use the provided value
        legendWidth = panelSchema.config.legend_width.value;
      }
    } else {
      const maxValue = options.series.reduce((max: any, it: any) => {
        return max.length < it?.name?.length ? it?.name : max;
      }, "");

      // If legend_width is not provided or has invalid format, calculate it based on other criteria
      legendWidth =
        Math.min(
          chartPanelRef.value?.offsetWidth / 3,
          calculateWidthText(maxValue) + 60
        ) ?? 20;
    }

    options.grid.right = legendWidth;
    options.legend.textStyle.width = legendWidth - 55;
  }

  //check if is there any data else filter out axis or series data
  if (!options?.series?.length && !options?.xAxis?.length) {
    return {
      options: {
        series: [],
        xAxis: [],
      },
    };
  }

  // allowed to zoom, only if timeseries
  options.toolbox.show = options.toolbox.show && isTimeSeriesFlag;

  // promql query will be always timeseries except gauge and metric text chart.
  return {
    options,
    extras: { panelId: panelSchema?.id, isTimeSeries: isTimeSeriesFlag },
  };
};

/**
 * Retrieves the legend name for a given metric and label.
 *
 * @param {any} metric - The metric object containing the values for the legend name placeholders.
 * @param {string} label - The label template for the legend name. If null or empty, the metric object will be converted to a JSON string and returned.
 * @return {string} The legend name with the placeholders replaced by the corresponding values from the metric object.
 */
const getPromqlLegendName = (metric: any, label: string) => {
  if (label) {
    let template = label || "";
    const placeholders = template.match(/\{([^}]+)\}/g);

    // Step 2: Iterate through each placeholder
    placeholders?.forEach(function (placeholder: any) {
      // Step 3: Extract the key from the placeholder
      const key = placeholder.replace("{", "").replace("}", "");

      // Step 4: Retrieve the corresponding value from the JSON object
      const value = metric[key];

      // Step 5: Replace the placeholder with the value in the template
      if (value) {
        template = template.replace(placeholder, value);
      }
    });
    return template;
  } else {
    return JSON.stringify(metric);
  }
};

/**
 * Determines the position of the legend based on the provided legendPosition.
 *
 * @param {string} legendPosition - The desired position of the legend. Possible values are "bottom" or "right".
 * @return {string} The position of the legend. Possible values are "horizontal" or "vertical".
 */
const getLegendPosition = (legendPosition: string) => {
  switch (legendPosition) {
    case "bottom":
      return "horizontal";
    case "right":
      return "vertical";
    default:
      return "horizontal";
  }
};

/**
 * Returns the props object based on the given chart type.
 *
 * @param {string} type - The chart type.
 * @return {object} The props object for the given chart type.
 */
const getPropsByChartTypeForSeries = (type: string) => {
  switch (type) {
    case "bar":
      return {
        type: "bar",
        emphasis: { focus: "series" },
        lineStyle: { width: 1.5 },
      };
    case "line":
      return {
        type: "line",
        emphasis: { focus: "series" },
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 1.5 },
      };
    case "scatter":
      return {
        type: "scatter",
        emphasis: { focus: "series" },
        symbolSize: 5,
      };
    case "pie":
      return {
        type: "pie",
        emphasis: { focus: "series" },
        lineStyle: { width: 1.5 },
      };
    case "donut":
      return {
        type: "pie",
        emphasis: { focus: "series" },
        lineStyle: { width: 1.5 },
      };
    case "h-bar":
      return {
        type: "bar",
        orientation: "h",
        emphasis: { focus: "series" },
        lineStyle: { width: 1.5 },
      };
    case "area":
      return {
        type: "line",
        emphasis: { focus: "series" },
        smooth: true,
        areaStyle: {},
        showSymbol: false,
        lineStyle: { width: 1.5 },
      };
    case "stacked":
      return {
        type: "bar",
        emphasis: { focus: "series" },
        lineStyle: { width: 1.5 },
      };
    case "area-stacked":
      return {
        type: "line",
        smooth: true,
        stack: "Total",
        areaStyle: {},
        showSymbol: false,
        emphasis: {
          focus: "series",
        },
        lineStyle: { width: 1.5 },
      };
    case "gauge":
      return {
        type: "gauge",
        startAngle: 205,
        endAngle: -25,
        pointer: {
          show: false,
        },
        axisTick: {
          show: false,
        },
        splitLine: {
          show: false,
        },
        axisLabel: {
          show: false,
        },
      };
    case "metric":
      return {
        type: "custom",
        coordinateSystem: "polar",
      };
    case "h-stacked":
      return {
        type: "bar",
        emphasis: { focus: "series" },
        orientation: "h",
        lineStyle: { width: 1.5 },
      };
    default:
      return {
        type: "bar",
      };
  }
};
