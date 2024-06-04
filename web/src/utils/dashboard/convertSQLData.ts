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

/**
 * Converts SQL data into a format suitable for rendering a chart.
 *
 * @param {any} panelSchema - the panel schema object
 * @param {any} searchQueryData - the search query data
 * @param {any} store - the store object
 * @return {Object} - the options object for rendering the chart
 */
import { utcToZonedTime } from "date-fns-tz";
import {
  formatDate,
  formatUnitValue,
  getUnitValue,
} from "./convertDataIntoUnitValue";
import { calculateGridPositions } from "./calculateGridForSubPlot";
import { isGivenFieldInOrderBy } from "../query/sqlUtils";
export const convertSQLData = async (
  panelSchema: any,
  searchQueryData: any,
  store: any,
  chartPanelRef: any,
  hoveredSeriesState: any
) => {
  // console.time("convertSQLData");

  // if no data than return it
  if (
    !Array.isArray(searchQueryData) ||
    searchQueryData.length === 0 ||
    !searchQueryData[0] ||
    !panelSchema.queries[0].fields.x ||
    !panelSchema.queries[0].fields.y
  ) {
    // console.timeEnd("convertSQLData");
    return { options: null };
  }

  // flag to check if the data is time series
  let isTimeSeriesFlag = false;

  // get the x axis key
  const getXAxisKeys = () => {
    return panelSchema?.queries[0]?.fields?.x?.length
      ? panelSchema?.queries[0]?.fields?.x.map((it: any) => it.alias)
      : [];
  };

  // get the y axis key
  const getYAxisKeys = () => {
    return panelSchema?.queries[0]?.fields?.y?.length
      ? panelSchema?.queries[0]?.fields?.y.map((it: any) => it.alias)
      : [];
  };

  // get the z axis key
  const getZAxisKeys = () => {
    return panelSchema?.queries[0]?.fields?.z?.length
      ? panelSchema?.queries[0]?.fields?.z.map((it: any) => it.alias)
      : [];
  };

  // get the axis data using key
  const getAxisDataFromKey = (key: string) => {
    let data =
      searchQueryData[0]?.filter((item: any) => {
        return (
          xAxisKeys.every((key: any) => item[key] != null) &&
          yAxisKeys.every((key: any) => item[key] != null)
        );
      }) || [];

    // if data is not there use {} as a default value
    const keys = Object.keys((data.length && data[0]) || {}); // Assuming there's at least one object

    const keyArrays: any = {};

    for (const key of keys) {
      keyArrays[key] = data.map((obj: any) => obj[key]);
    }

    let result = keyArrays[key] || [];

    return result;
  };

  // Step 1: Get the X-Axis key
  const xAxisKeys = getXAxisKeys();

  // Step 2: Get the Y-Axis key
  const yAxisKeys = getYAxisKeys();

  const zAxisKeys = getZAxisKeys();

  const legendPosition = getLegendPosition(
    panelSchema.config?.legends_position
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
      width: 100,
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
    grid: {
      containLabel: panelSchema.config?.axis_width == null ? true : false,
      left: panelSchema.config?.axis_width ?? 30,
      right: 20,
      top: "15",
      bottom:
        legendConfig.orient === "horizontal" && panelSchema.config?.show_legends
          ? panelSchema.config?.axis_width == null
            ? 50
            : 60
          : panelSchema.config?.axis_width == null
          ? 35
          : "40",
    },
    tooltip: {
      trigger: "axis",
      textStyle: {
        color: store.state.theme === "dark" ? "#fff" : "#000",
        fontSize: 12,
      },
      enterable: true,
      backgroundColor:
        store.state.theme === "dark" ? "rgba(0,0,0,1)" : "rgba(255,255,255,1)",
      extraCssText: "max-height: 200px; overflow: auto; max-width: 400px",
      axisPointer: {
        type: "cross",
        label: {
          show: true,
          fontsize: 12,
          precision: panelSchema.config?.decimals,
          formatter: function (params: any) {
            let lineBreaks = "";
            if (
              panelSchema.type === "h-bar" ||
              panelSchema.type === "h-stacked"
            ) {
              if (params.axisDimension == "x")
                return formatUnitValue(
                  getUnitValue(
                    params.value,
                    panelSchema.config?.unit,
                    panelSchema.config?.unit_custom,
                    panelSchema.config?.decimals
                  )
                );

              //we does not required any linebreaks for h-stacked because we only use one x axis
              if (panelSchema.type === "h-stacked")
                return params.value.toString();
              for (
                let i = 0;
                i < xAxisKeys.length - params.axisIndex - 1;
                i++
              ) {
                lineBreaks += " \n \n";
              }
              params.value = params.value.toString();
              return `${lineBreaks}  ${params.value}`;
            }
            if (params.axisDimension == "y")
              return formatUnitValue(
                getUnitValue(
                  params.value,
                  panelSchema.config?.unit,
                  panelSchema.config?.unit_custom,
                  panelSchema.config?.decimals
                )
              );
            for (let i = 0; i < xAxisKeys.length - params.axisIndex - 1; i++) {
              lineBreaks += " \n \n";
            }
            params.value = params.value.toString();
            return `${lineBreaks}  ${params.value}`;
          },
        },
      },
      formatter: function (name: any) {
        // show tooltip for hovered panel only for other we only need axis so just return empty string
        if (
          hoveredSeriesState?.value &&
          panelSchema.id &&
          hoveredSeriesState?.value?.panelId != panelSchema.id
        )
          return "";
        if (name.length == 0) return "";

        // if hovered series is not null
        // then swap the hovered series to top in tooltip
        if (hoveredSeriesState?.value?.hoveredSeriesName) {
          // get the current series index from name
          const currentSeriesIndex = name.findIndex(
            (it: any) =>
              it.seriesName == hoveredSeriesState?.value?.hoveredSeriesName
          );

          // swap current hovered series index to top in tooltip
          const temp = name[0];
          name[0] = name[currentSeriesIndex != -1 ? currentSeriesIndex : 0];
          name[currentSeriesIndex != -1 ? currentSeriesIndex : 0] = temp;
        }

        const hoverText: string[] = [];
        name.forEach((it: any) => {
          // if value is not null, show in tooltip
          if (it.value != null) {
            // check if the series is the current series being hovered
            // if have than bold it
            if (it?.seriesName == hoveredSeriesState?.value?.hoveredSeriesName)
              hoverText.push(
                `<strong>${it.marker} ${it.seriesName} : ${formatUnitValue(
                  getUnitValue(
                    it.value,
                    panelSchema.config?.unit,
                    panelSchema.config?.unit_custom,
                    panelSchema.config?.decimals
                  )
                )} </strong>`
              );
            // else normal text
            else
              hoverText.push(
                `${it.marker} ${it.seriesName} : ${formatUnitValue(
                  getUnitValue(
                    it.value,
                    panelSchema.config?.unit,
                    panelSchema.config?.unit_custom,
                    panelSchema.config?.decimals
                  )
                )}`
              );
          }
        });

        return `${name[0].name} <br/> ${hoverText.join("<br/>")}`;
      },
    },
    xAxis: xAxisKeys?.map((key: any, index: number) => {
      const data = getAxisDataFromKey(key);

      //unique value index array
      const arr: any = [];
      for (let i = 0; i < data.length; i++) {
        if (i == 0 || data[i] != data[i - 1]) arr.push(i);
      }

      return {
        type: "category",
        position: panelSchema.type == "h-bar" ? "left" : "bottom",
        // inverse data for h-stacked and h-bar
        inverse: ["h-stacked", "h-bar"].includes(panelSchema.type) ? true : false,
        name: index == 0 ? panelSchema.queries[0]?.fields?.x[index]?.label : "",
        nameLocation: "middle",
        nameGap: 9 * (xAxisKeys.length - index + 1),
        nameTextStyle: {
          fontWeight: "bold",
          fontSize: 14,
        },
        axisLabel: {
          interval:
            panelSchema.type == "h-stacked"
              ? "auto"
              : index == xAxisKeys.length - 1
              ? "auto"
              : function (i: any) {
                  return arr.includes(i);
                },
          overflow: index == xAxisKeys.length - 1 ? "none" : "truncate",
          // hide axis label if overlaps
          hideOverlap: true,
          width: 100,
          margin: 18 * (xAxisKeys.length - index - 1) + 5,
        },
        splitLine: {
          show: true,
        },
        axisLine: {
          show: panelSchema.config?.axis_border_show || false,
        },
        axisTick: {
          show: xAxisKeys.length == 1 ? false : true,
          align: "left",
          alignWithLabel: false,
          length: 20 * (xAxisKeys.length - index),
          interval:
            panelSchema.type == "h-stacked"
              ? "auto"
              : function (i: any) {
                  return arr.includes(i);
                },
        },
        data: data,
      };
    }),
    yAxis: {
      type: "value",
      name:
        panelSchema.queries[0]?.fields?.y?.length == 1
          ? panelSchema.queries[0]?.fields?.y[0]?.label
          : "",
      nameLocation: "middle",
      nameGap:
        calculateWidthText(
          panelSchema.type == "h-bar" || panelSchema.type == "h-stacked"
            ? largestLabel(getAxisDataFromKey(yAxisKeys[0]))
            : formatUnitValue(
                getUnitValue(
                  largestLabel(getAxisDataFromKey(yAxisKeys[0])),
                  panelSchema.config?.unit,
                  panelSchema.config?.unit_custom,
                  panelSchema.config?.decimals
                )
              )
        ) + 8,
      nameTextStyle: {
        fontWeight: "bold",
        fontSize: 14,
      },
      axisLabel: {
        formatter: function (value: any) {
          return formatUnitValue(
            getUnitValue(
              value,
              panelSchema.config?.unit,
              panelSchema.config?.unit_custom,
              panelSchema.config?.decimals
            )
          );
        },
      },
      splitLine: {
        show: true,
      },
      axisLine: {
        show: panelSchema.config?.axis_border_show || false,
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
          yAxisIndex: "none",
        },
      },
    },
    series: [],
  };
  const defaultSeriesProps = getPropsByChartTypeForSeries(panelSchema.type);

  // Now set the series values as per the chart data
  // Override any configs if required as per the chart type
  switch (panelSchema.type) {
    case "area-stacked":
    case "line":
    case "area":
    case "scatter": {
      //if area stacked then continue
      //or if area or line or scatter, then check x axis length
      if (
        panelSchema.type == "area-stacked" ||
        ((panelSchema.type == "line" ||
          panelSchema.type == "area" ||
          panelSchema.type == "scatter") &&
          panelSchema.queries[0].fields.x.length == 2)
      ) {
        options.xAxis = options.xAxis.slice(0, 1);
        options.tooltip.axisPointer.label = {
          show: true,
          label: {
            fontsize: 12,
            precision: panelSchema.config?.decimals,
          },
          formatter: function (params: any) {
            if (params.axisDimension == "y")
              return formatUnitValue(
                getUnitValue(
                  params.value,
                  panelSchema.config?.unit,
                  panelSchema.config?.unit_custom,
                  panelSchema.config?.decimals
                )
              );
            return params.value.toString();
          },
        };
        options.xAxis[0].axisLabel = {};
        options.xAxis[0].axisTick = {};
        options.xAxis[0].nameGap = 20;

        // get the unique value of the first xAxis's key
        options.xAxis[0].data = Array.from(
          new Set(getAxisDataFromKey(xAxisKeys[0]))
        );
        // options.xAxis[0].data = Array.from(new Set(options.xAxis[0].data));

        // stacked with xAxis's second value
        // allow 2 xAxis and 1 yAxis value for stack chart
        // get second x axis key
        const key1 = xAxisKeys[1];
        // get the unique value of the second xAxis's key
        const stackedXAxisUniqueValue = [
          ...new Set(searchQueryData[0].map((obj: any) => obj[key1])),
        ].filter((it) => it);

        options.series = yAxisKeys
          .map((yAxis: any) => {
            const yAxisName = panelSchema?.queries[0]?.fields?.y.find(
              (it: any) => it.alias == yAxis
            ).label;
            return stackedXAxisUniqueValue?.map((key: any) => {
              // queryData who has the xaxis[1] key as well from xAxisUniqueValue.
              const data = searchQueryData[0].filter(
                (it: any) => it[key1] == key
              );
              const seriesObj = {
                //only append if yaxiskeys length is more than 1
                name:
                  yAxisKeys.length == 1 ? key : key + " (" + yAxisName + ")",
                ...defaultSeriesProps,
                // config to connect null values
                connectNulls: panelSchema.config?.connect_nulls ?? false,
                data: options.xAxis[0].data.map(
                  (it: any) =>
                    data.find((it2: any) => it2[xAxisKeys[0]] == it)?.[yAxis] ??
                    null
                ),
              };
              return seriesObj;
            });
          })
          .flat();
      } else if (panelSchema.type == "line" || panelSchema.type == "area") {
        //if x and y length is not 2 and 1 respectively then do following
        options.series = yAxisKeys?.map((key: any) => {
          const seriesObj = {
            name: panelSchema?.queries[0]?.fields?.y.find(
              (it: any) => it.alias == key
            )?.label,
            color:
              panelSchema.queries[0]?.fields?.y.find(
                (it: any) => it.alias == key
              )?.color || "#5960b2",
            opacity: 0.8,
            ...defaultSeriesProps,
            // config to connect null values
            connectNulls: panelSchema.config?.connect_nulls ?? false,
            data: getAxisDataFromKey(key),
          };
          return seriesObj;
        });
        // scatter chart with single x and y axis(single or multiple)
      } else {
        options.tooltip.formatter = function (name: any) {
          // show tooltip for hovered panel only for other we only need axis so just return empty string
          if (
            hoveredSeriesState?.value &&
            panelSchema.id &&
            hoveredSeriesState?.value?.panelId != panelSchema.id
          )
            return "";
          if (name.length == 0) return "";

          // if hovered series is not null
          // then swap the hovered series to top in tooltip
          if (hoveredSeriesState?.value?.hoveredSeriesName) {
            // get the current series index from name
            const currentSeriesIndex = name.findIndex(
              (it: any) =>
                it.seriesName == hoveredSeriesState?.value?.hoveredSeriesName
            );
            // swap current hovered series index to top in tooltip
            const temp = name[0];
            name[0] = name[currentSeriesIndex != -1 ? currentSeriesIndex : 0];
            name[currentSeriesIndex != -1 ? currentSeriesIndex : 0] = temp;
          }
          const hoverText: string[] = [];
          name.forEach((it: any) => {
            if (it.data != null) {
              // check if the series is the current series being hovered
              // if have than bold it
              if (
                it?.seriesName == hoveredSeriesState?.value?.hoveredSeriesName
              )
                hoverText.push(
                  `<strong>${it.marker} ${it.seriesName} : ${formatUnitValue(
                    getUnitValue(
                      it.data,
                      panelSchema.config?.unit,
                      panelSchema.config?.unit_custom,
                      panelSchema.config?.decimals
                    )
                  )} </strong>`
                );
              // else normal text
              else
                hoverText.push(
                  `${it.marker} ${it.seriesName} : ${formatUnitValue(
                    getUnitValue(
                      it.data,
                      panelSchema.config?.unit,
                      panelSchema.config?.unit_custom,
                      panelSchema.config?.decimals
                    )
                  )}`
                );
            }
          });
          return `${name[0].name} <br/> ${hoverText.join("<br/>")}`;
        };
        options.series = yAxisKeys?.map((key: any) => {
          const seriesObj = {
            name: panelSchema?.queries[0]?.fields?.y.find(
              (it: any) => it.alias == key
            )?.label,
            color:
              panelSchema.queries[0]?.fields?.y.find(
                (it: any) => it.alias == key
              )?.color || "#5960b2",
            opacity: 0.8,
            ...defaultSeriesProps,
            // config to connect null values
            connectNulls: panelSchema.config?.connect_nulls ?? false,
            data: getAxisDataFromKey(key),
          };
          return seriesObj;
        });
      }
      break;
    }
    case "bar": {
      options.series = yAxisKeys?.map((key: any) => {
        const seriesObj = {
          name: panelSchema?.queries[0]?.fields?.y.find(
            (it: any) => it.alias == key
          )?.label,
          color:
            panelSchema.queries[0]?.fields?.y.find((it: any) => it.alias == key)
              ?.color || "#5960b2",
          opacity: 0.8,
          ...defaultSeriesProps,
          data: getAxisDataFromKey(key),
        };
        return seriesObj;
      });
      break;
    }
    case "h-bar": {
      //generate trace based on the y axis keys
      options.series = yAxisKeys?.map((key: any) => {
        const seriesObj = {
          name: panelSchema?.queries[0]?.fields?.y.find(
            (it: any) => it.alias == key
          )?.label,
          color:
            panelSchema.queries[0]?.fields?.y.find((it: any) => it.alias == key)
              ?.color || "#5960b2",
          opacity: 0.8,
          ...defaultSeriesProps,
          data: getAxisDataFromKey(key),
        };
        return seriesObj;
      });
      //swap x and y axis
      const temp = options.xAxis;
      options.xAxis = options.yAxis;
      options.yAxis = temp;

      options.yAxis.forEach((it: any) => {
        it.nameGap = calculateWidthText(
          xAxisKeys.reduce(
            (str: any, it: any) => str + largestLabel(getAxisDataFromKey(it)),
            ""
          )
        );
      });
      (options.xAxis.name =
        panelSchema.queries[0]?.fields?.y?.length >= 1
          ? panelSchema.queries[0]?.fields?.y[0]?.label
          : ""),
        (options.xAxis.nameGap = 20);
      break;
    }
    case "pie": {
      options.tooltip = {
        trigger: "item",
        textStyle: {
          color: store.state.theme === "dark" ? "#fff" : "#000",
          fontSize: 12,
        },
        backgroundColor:
          store.state.theme === "dark"
            ? "rgba(0,0,0,1)"
            : "rgba(255,255,255,1)",
        formatter: function (name: any) {
          // show tooltip for hovered panel only for other we only need axis so just return empty string
          if (
            hoveredSeriesState?.value &&
            panelSchema.id &&
            hoveredSeriesState?.value?.panelId != panelSchema.id
          )
            return "";
          return `${name.marker} ${name.name} : <b>${formatUnitValue(
            getUnitValue(
              name.value,
              panelSchema.config?.unit,
              panelSchema.config?.unit_custom,
              panelSchema.config?.decimals
            )
          )}</b>`;
        },
      };
      //generate trace based on the y axis keys
      options.series = yAxisKeys?.map((key: any) => {
        const seriesObj = {
          ...defaultSeriesProps,
          data: getAxisDataFromKey(key).map((it: any, i: number) => {
            return { value: it, name: options?.xAxis[0]?.data[i] };
          }),
          label: {
            show: true,
            formatter: "{d}%", // {b} represents name, {c} represents value {d} represents percent
            position: "inside", // You can adjust the position of the labels
            fontSize: 10,
          },
          percentPrecision: panelSchema.config?.decimals ?? 2,
        };
        return seriesObj;
      });

      options.xAxis = [];
      options.yAxis = [];
      break;
    }
    case "donut": {
      options.tooltip = {
        trigger: "item",
        textStyle: {
          color: store.state.theme === "dark" ? "#fff" : "#000",
          fontSize: 12,
        },
        backgroundColor:
          store.state.theme === "dark"
            ? "rgba(0,0,0,1)"
            : "rgba(255,255,255,1)",
        formatter: function (name: any) {
          // show tooltip for hovered panel only for other we only need axis so just return empty string
          if (
            hoveredSeriesState?.value &&
            panelSchema.id &&
            hoveredSeriesState?.value?.panelId != panelSchema.id
          )
            return "";
          return `${name.marker} ${name.name} : <b>${formatUnitValue(
            getUnitValue(
              name.value,
              panelSchema.config?.unit,
              panelSchema.config?.unit_custom,
              panelSchema.config?.decimals
            )
          )}<b/>`;
        },
      };
      //generate trace based on the y axis keys
      options.series = yAxisKeys?.map((key: any) => {
        const seriesObj = {
          ...defaultSeriesProps,
          data: getAxisDataFromKey(key).map((it: any, i: number) => {
            return { value: it, name: options?.xAxis[0]?.data[i] };
          }),
          label: {
            show: true,
            formatter: "{d}%", // {b} represents name, {c} represents value {d} represents percent
            position: "inside", // You can adjust the position of the labels
            fontSize: 10,
          },
          percentPrecision: panelSchema.config?.decimals ?? 2,
        };
        return seriesObj;
      });

      options.xAxis = [];
      options.yAxis = [];
      break;
    }
    case "stacked": {
      options.xAxis[0].data = Array.from(
        new Set(getAxisDataFromKey(xAxisKeys[0]))
      );
      options.xAxis = options.xAxis.slice(0, 1);
      options.tooltip.axisPointer.label = {
        show: true,
        label: {
          fontsize: 12,
          precision: panelSchema.config?.decimals,
        },
        formatter: function (params: any) {
          if (params.axisDimension == "y")
            return formatUnitValue(
              getUnitValue(
                params.value,
                panelSchema.config?.unit,
                panelSchema.config?.unit_custom,
                panelSchema.config?.decimals
              )
            );
          return params.value.toString();
        },
      };
      options.xAxis[0].axisLabel.margin = 5;
      options.xAxis[0].axisLabel = {};
      options.xAxis[0].axisTick = {};
      options.xAxis[0].nameGap = 20;

      // stacked with xAxis's second value
      // allow 2 xAxis and 1 yAxis value for stack chart
      // get second x axis key
      const key1 = xAxisKeys[1];
      // get the unique value of the second xAxis's key
      const stackedXAxisUniqueValue = [
        ...new Set(searchQueryData[0].map((obj: any) => obj[key1])),
      ].filter((it) => it);

      options.series = stackedXAxisUniqueValue?.map((key: any) => {
        // queryData who has the xaxis[1] key as well from xAxisUniqueValue.
        const data = searchQueryData[0].filter((it: any) => it[key1] == key);
        const seriesObj = {
          name: key,
          ...defaultSeriesProps,
          data: options.xAxis[0].data.map(
            (it: any) =>
              data.find((it2: any) => it2[xAxisKeys[0]] == it)?.[
                yAxisKeys[0]
              ] ?? null
          ),
        };
        return seriesObj;
      });
      break;
    }
    case "heatmap": {
      // get first x axis key
      const key0 = xAxisKeys[0];
      // get the unique value of the first xAxis's key
      let xAxisZerothPositionUniqueValue = [
        ...new Set(searchQueryData[0].map((obj: any) => obj[key0])),
      ].filter((it) => it);

      // get second x axis key
      const key1 = yAxisKeys[0];
      // get the unique value of the second xAxis's key
      const xAxisFirstPositionUniqueValue = [
        ...new Set(searchQueryData[0].map((obj: any) => obj[key1])),
      ].filter((it) => it);

      const yAxisKey0 = zAxisKeys[0];
      const zValues: any = xAxisFirstPositionUniqueValue.map((first: any) => {
        // queryData who has the xaxis[1] key as well from xAxisUniqueValue.
        const data = searchQueryData[0].filter((it: any) => it[key1] == first);

        return xAxisZerothPositionUniqueValue.map((zero: any) => {
          return data.find((it: any) => it[key0] == zero)?.[yAxisKey0] || "-";
        });
      });

      (options.visualMap = {
        min: 0,
        max: zValues.reduce(
          (a: any, b: any) =>
            Math.max(
              a,
              b.reduce((c: any, d: any) => Math.max(c, +d || 0), 0)
            ),
          0
        ),
        calculable: true,
        orient: "horizontal",
        left: "center",
      }),
        (options.series = [
          {
            ...defaultSeriesProps,
            name: panelSchema?.queries[0]?.fields?.y[0].label,
            data: zValues
              .map((it: any, index: any) => {
                return xAxisZerothPositionUniqueValue.map(
                  (i: any, j: number) => {
                    return [j, index, it[j]];
                  }
                );
              })
              .flat(),
            label: {
              show: true,
              fontSize: 12,
              formatter: (params: any) => {
                return (
                  formatUnitValue(
                    getUnitValue(
                      params.value[2],
                      panelSchema.config?.unit,
                      panelSchema.config?.unit_custom,
                      panelSchema.config?.decimals
                    )
                  ) || params.value[2]
                );
              },
            },
          },
        ]);
      // option.yAxis.data=xAxisFirstPositionUniqueValue;
      (options.tooltip = {
        position: "top",
        textStyle: {
          color: store.state.theme === "dark" ? "#fff" : "#000",
          fontSize: 12,
        },
        backgroundColor:
          store.state.theme === "dark"
            ? "rgba(0,0,0,1)"
            : "rgba(255,255,255,1)",
        formatter: (params: any) => {
          // show tooltip for hovered panel only for other we only need axis so just return empty string
          if (
            hoveredSeriesState?.value &&
            panelSchema.id &&
            hoveredSeriesState?.value?.panelId != panelSchema.id
          )
            return "";
          // we have value[1] which return yaxis index
          // it is used to get y axis data
          return `${
            options?.yAxis?.data[params?.value[1]] || params?.seriesName
          } <br/> ${params?.marker} ${params?.name} : ${
            formatUnitValue(
              getUnitValue(
                params?.value[2],
                panelSchema?.config?.unit,
                panelSchema?.config?.unit_custom,
                panelSchema?.config?.decimals
              )
            ) || params.value[2]
          }`;
        },
      }),
        (options.tooltip.axisPointer = {
          type: "cross",
          label: {
            fontsize: 12,
            precision: panelSchema.config?.decimals,
          },
        });
      // if auto sql
      if (panelSchema?.queries[0]?.customQuery == false) {
        // check if x axis has histogram or not
        // for heatmap we only have one field in x axis event we have used find fn

        const field = panelSchema.queries[0].fields?.x.find(
          (it: any) =>
            it.aggregationFunction == "histogram" &&
            it.column == store.state.zoConfig.timestamp_column
        );
        // if histogram
        if (field) {
          // convert time string to selected timezone
          xAxisZerothPositionUniqueValue = xAxisZerothPositionUniqueValue.map(
            (it: any) => {
              return formatDate(utcToZonedTime(it + "Z", store.state.timezone));
            }
          );
        }
        // else custom sql
      } else {
        // sampling data to know whether data is timeseries or not
        const sample = xAxisZerothPositionUniqueValue.slice(
          0,
          Math.min(20, xAxisZerothPositionUniqueValue.length)
        );
        // if timeseries
        if (isTimeSeries(sample)) {
          // convert time string to selected timezone
          xAxisZerothPositionUniqueValue = xAxisZerothPositionUniqueValue.map(
            (it: any) => {
              return formatDate(utcToZonedTime(it + "Z", store.state.timezone));
            }
          );
        }
      }

      options.grid.bottom = 60;
      (options.xAxis = [
        {
          type: "category",
          data: xAxisZerothPositionUniqueValue,
          splitArea: {
            show: true,
          },
        },
      ]),
        [
          (options.yAxis = {
            type: "category",
            data: xAxisFirstPositionUniqueValue,
            splitArea: {
              show: true,
            },
          }),
        ];
      options.legend.show = false;
      break;
    }
    case "h-stacked": {
      options.xAxis[0].data = Array.from(
        new Set(getAxisDataFromKey(xAxisKeys[0]))
      );
      options.xAxis = options.xAxis.slice(0, 1);

      // stacked with xAxis's second value
      // allow 2 xAxis and 1 yAxis value for stack chart
      // get second x axis key
      const key1 = xAxisKeys[1];
      // get the unique value of the second xAxis's key
      const stackedXAxisUniqueValue = [
        ...new Set(searchQueryData[0].map((obj: any) => obj[key1])),
      ].filter((it) => it);

      options.series = stackedXAxisUniqueValue?.map((key: any) => {
        // queryData who has the xaxis[1] key as well from xAxisUniqueValue.
        const data = searchQueryData[0].filter((it: any) => it[key1] == key);
        const seriesObj = {
          name: key,
          ...defaultSeriesProps,
          data: options.xAxis[0].data.map(
            (it: any) =>
              data.find((it2: any) => it2[xAxisKeys[0]] == it)?.[
                yAxisKeys[0]
              ] ?? null
          ),
        };
        return seriesObj;
      });

      const temp = options.xAxis;
      options.xAxis = options.yAxis;
      options.yAxis = temp;
      options.yAxis.map((it: any) => {
        it.nameGap = calculateWidthText(largestLabel(it.data)) + 20;
      });
      options.xAxis.nameGap = 20;
      break;
    }
    case "metric": {
      const key1 = yAxisKeys[0];
      const yAxisValue = getAxisDataFromKey(key1);
      const unitValue = getUnitValue(
        yAxisValue.length > 0 ? yAxisValue[0] : 0,
        panelSchema.config?.unit,
        panelSchema.config?.unit_custom,
        panelSchema.config?.decimals
      );
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
      options.series = [
        {
          ...defaultSeriesProps,
          renderItem: function (params: any) {
            return {
              type: "text",
              style: {
                text:
                  (parseFloat(unitValue?.value)?.toFixed(
                    panelSchema?.config?.decimals ?? 2
                  ) ?? 0) + unitValue?.unit,
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
        },
      ];
      break;
    }
    case "gauge": {
      const key1 = yAxisKeys[0];
      const yAxisValue = getAxisDataFromKey(key1);
      // used for gague name
      const xAxisValue = getAxisDataFromKey(xAxisKeys[0]);

      // create grid array based on chart panel width, height and total no. of gauge
      const gridDataForGauge = calculateGridPositions(
        chartPanelRef.value.offsetWidth,
        chartPanelRef.value.offsetHeight,
        yAxisValue.length
      );

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
      // for each gague we have seperate grid
      options.grid = gridDataForGauge.gridArray;

      options.series = yAxisValue.map((it: any, index: any) => {
        return {
          ...defaultSeriesProps,
          min: panelSchema?.queries[0]?.config?.min || 0,
          max: panelSchema?.queries[0]?.config?.max || 100,

          //which grid will be used
          gridIndex: index,
          // radius, progress and axisline width will be calculated based on grid width and height
          radius: `${
            Math.min(gridDataForGauge.gridWidth, gridDataForGauge.gridHeight) /
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
              parseFloat(options.grid[index].left) +
              parseFloat(options.grid[index].width) / 2
            }%`,
            `${
              parseFloat(options.grid[index].top) +
              parseFloat(options.grid[index].height) / 2
            }%`,
          ],

          data: [
            {
              // gauge name may have or may not have
              name: xAxisValue[index] ?? "",
              value: it,
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
      break;
    }
    default: {
      break;
    }
  }

  // auto SQL: if x axis has time series
  if (
    panelSchema.type != "h-bar" &&
    panelSchema.type != "h-stacked" &&
    panelSchema.type != "heatmap" &&
    panelSchema.type != "metric" &&
    panelSchema.type != "pie" &&
    panelSchema.type != "donut" &&
    panelSchema?.queries[0]?.customQuery == false &&
    Array.isArray(options.xAxis) &&
    options.xAxis.length > 0 &&
    options.xAxis[0].data.length > 0
  ) {
    // auto SQL: if x axis has time series(aggregation function is histogram)
    const field = panelSchema.queries[0].fields?.x.find(
      (it: any) =>
        it.aggregationFunction == "histogram" &&
        it.column == store.state.zoConfig.timestamp_column
    );

    const timestampField = panelSchema.queries[0].fields?.x.find(
      (it: any) =>
        !it.aggregationFunction &&
        it.column == store.state.zoConfig.timestamp_column
    );

    //if x axis has time series
    if (field || timestampField) {
      // set timeseries flag as a true
      isTimeSeriesFlag = true;

      // if timezone is UTC then simply return x axis value which will be in UTC (note that need to remove Z from timezone string)
      // else check if xaxis value is interger(ie time will be in milliseconds)
      // if yes then return to convert into other timezone
      // if no then create new datetime object and get in milliseconds using getTime method
      options?.series?.forEach((seriesObj: any) => {
        // if value field is not present in the data than use null
        if (field) {
          seriesObj.data = seriesObj?.data?.map((it: any, index: any) => [
            utcToZonedTime(
              new Date(options.xAxis[0].data[index] + "Z").getTime(),
              store.state.timezone
            ),
            it ?? null,
          ]);
        } else if (timestampField) {
          seriesObj.data = seriesObj?.data?.map((it: any, index: any) => [
            utcToZonedTime(
              new Date(options.xAxis[0].data[index]).getTime() / 1000,
              store.state.timezone
            ),
            it ?? null,
          ]);
        }
      });
      options.xAxis[0].type = "time";
      options.xAxis[0].data = [];
      options.tooltip.formatter = function (name: any) {
        // show tooltip for hovered panel only for other we only need axis so just return empty string
        if (
          hoveredSeriesState?.value &&
          panelSchema.id &&
          hoveredSeriesState?.value?.panelId != panelSchema.id
        )
          return "";
        if (name.length == 0) return "";

        const date = new Date(name[0].data[0]);

        // if hovered series is not null
        // then swap the hovered series to top in tooltip
        if (hoveredSeriesState?.value?.hoveredSeriesName) {
          // get the current series index from name
          const currentSeriesIndex = name.findIndex(
            (it: any) =>
              it.seriesName == hoveredSeriesState?.value?.hoveredSeriesName
          );

          // swap current hovered series index to top in tooltip
          const temp = name[0];
          name[0] = name[currentSeriesIndex != -1 ? currentSeriesIndex : 0];
          name[currentSeriesIndex != -1 ? currentSeriesIndex : 0] = temp;
        }

        const hoverText: string[] = [];
        name.forEach((it: any) => {
          if (it.data[1] != null) {
            // check if the series is the current series being hovered
            // if have than bold it
            if (it?.seriesName == hoveredSeriesState?.value?.hoveredSeriesName)
              hoverText.push(
                `<strong>${it.marker} ${it.seriesName} : ${formatUnitValue(
                  getUnitValue(
                    it.data[1],
                    panelSchema.config?.unit,
                    panelSchema.config?.unit_custom,
                    panelSchema.config?.decimals
                  )
                )} </strong>`
              );
            // else normal text
            else
              hoverText.push(
                `${it.marker} ${it.seriesName} : ${formatUnitValue(
                  getUnitValue(
                    it.data[1],
                    panelSchema.config?.unit,
                    panelSchema.config?.unit_custom,
                    panelSchema.config?.decimals
                  )
                )}`
              );
          }
        });

        return `${formatDate(date)} <br/> ${hoverText.join("<br/>")}`;
      };
      options.tooltip.axisPointer = {
        type: "cross",
        label: {
          fontsize: 12,
          precision: panelSchema.config?.decimals,
          formatter: function (params: any) {
            if (params.axisDimension == "y")
              return formatUnitValue(
                getUnitValue(
                  params.value,
                  panelSchema.config?.unit,
                  panelSchema.config?.unit_custom,
                  panelSchema.config?.decimals
                )
              );
            return Number.isInteger(params.value)
              ? formatDate(new Date(params.value))
              : params.value;
          },
        },
        formatter: function (params: any) {
          const date = new Date(params[0].value[0]);
          return formatDate(date).toString();
        },
      };
    }
  }

  //custom SQL: check if it is timeseries or not
  if (
    panelSchema.type != "h-bar" &&
    panelSchema.type != "h-stacked" &&
    panelSchema.type != "heatmap" &&
    panelSchema.type != "metric" &&
    panelSchema.type != "pie" &&
    panelSchema.type != "donut" &&
    panelSchema?.queries[0]?.customQuery == true &&
    Array.isArray(options.xAxis) &&
    options.xAxis.length > 0 &&
    options.xAxis[0].data.length > 0
  ) {
    const sample = options.xAxis[0].data.slice(
      0,
      Math.min(20, options.xAxis[0].data.length)
    );

    const isTimeSeriesData = isTimeSeries(sample);
    const isTimeStampData = isTimeStamp(sample);

    if (isTimeSeriesData || isTimeStampData) {
      // set timeseries flag as a true
      isTimeSeriesFlag = true;

      options?.series?.forEach((seriesObj: any) => {
        // if value field is not present in the data than use null
        if (isTimeSeriesData) {
          seriesObj.data = seriesObj?.data?.map((it: any, index: any) => [
            utcToZonedTime(
              new Date(options.xAxis[0].data[index] + "Z").getTime(),
              store.state.timezone
            ),
            it ?? null,
          ]);
        } else if (isTimeStampData) {
          seriesObj.data = seriesObj?.data?.map((it: any, index: any) => [
            utcToZonedTime(
              new Date(options.xAxis[0].data[index]).getTime() / 1000,
              store.state.timezone
            ),
            it ?? null,
          ]);
        }
      });
      options.xAxis[0].type = "time";
      options.xAxis[0].data = [];
      options.tooltip.formatter = function (name: any) {
        // show tooltip for hovered panel only for other we only need axis so just return empty string
        if (
          hoveredSeriesState?.value &&
          panelSchema.id &&
          hoveredSeriesState?.value?.panelId != panelSchema.id
        )
          return "";
        if (name.length == 0) return "";

        const date = new Date(name[0].data[0]);

        // if hovered series is not null
        // then swap the hovered series to top in tooltip
        if (hoveredSeriesState?.value?.hoveredSeriesName) {
          // get the current series index from name
          const currentSeriesIndex = name.findIndex(
            (it: any) =>
              it.seriesName == hoveredSeriesState?.value?.hoveredSeriesName
          );

          // swap current hovered series index to top in tooltip
          const temp = name[0];
          name[0] = name[currentSeriesIndex != -1 ? currentSeriesIndex : 0];
          name[currentSeriesIndex != -1 ? currentSeriesIndex : 0] = temp;
        }

        const hoverText: string[] = [];
        name.forEach((it: any) => {
          if (it.data[1] != null) {
            // check if the series is the current series being hovered
            // if have than bold it
            if (it?.seriesName == hoveredSeriesState?.value?.hoveredSeriesName)
              hoverText.push(
                `<strong>${it.marker} ${it.seriesName} : ${formatUnitValue(
                  getUnitValue(
                    it.data[1],
                    panelSchema.config?.unit,
                    panelSchema.config?.unit_custom,
                    panelSchema.config?.decimals
                  )
                )} </strong>`
              );
            // else normal text
            else
              hoverText.push(
                `${it.marker} ${it.seriesName} : ${formatUnitValue(
                  getUnitValue(
                    it.data[1],
                    panelSchema.config?.unit,
                    panelSchema.config?.unit_custom,
                    panelSchema.config?.decimals
                  )
                )}`
              );
          }
        });

        return `${formatDate(date)} <br/> ${hoverText.join("<br/>")}`;
      };
      options.tooltip.axisPointer = {
        type: "cross",
        label: {
          fontsize: 12,
          precision: panelSchema.config?.decimals,
          formatter: function (params: any) {
            if (params.axisDimension == "y")
              return formatUnitValue(
                getUnitValue(
                  params.value,
                  panelSchema.config?.unit,
                  panelSchema.config?.unit_custom,
                  panelSchema.config?.decimals
                )
              );
            return formatDate(new Date(params?.value)).toString();
          },
        },
        formatter: function (params: any) {
          const date = new Date(params[0].value[0]);
          return formatDate(date).toString();
        },
      };
    }
  }

  // stacked chart sort by y axis
  // ignore if time series chart
  if (
    ["stacked", "h-stacked", "area-stacked"].includes(panelSchema?.type) &&
    isTimeSeriesFlag == false
  ) {

    // get x axis object
    // for h-stacked, categorical axis is y axis
    // for stacked and area-stacked, categorical axis is x axis
    const xAxisObj =
      panelSchema.type == "h-stacked" ? options.yAxis : options.xAxis;

    // check if order by uses y axis field
    // will return null if not exist
    // will return ASC or DESC if exist
    const isYAxisExistInOrderBy = await isGivenFieldInOrderBy(
      panelSchema?.queries[0]?.query ?? "",
      yAxisKeys[0]
    );
    if (isYAxisExistInOrderBy) {
      // Calculate the total for each series and combine it with the corresponding x-axis label
      let totals = new Map();
      for (let i = 0; i < xAxisObj[0]?.data?.length; i++) {
        let total = options?.series?.reduce(
          (sum: number, currentSeries: any) => sum + currentSeries?.data[i] ?? 0,
          0
        );
        totals.set(i, { label: xAxisObj[0]?.data[i], total });
      }

      // Sort the indices by total in the specified order
      // ASC for ascending, DESC for descending
      let sortedIndices = Array.from(totals.keys()).sort((a, b) => {
        let diff = totals.get(a).total - totals.get(b).total;
        return isYAxisExistInOrderBy == "ASC" ? diff : -diff;
      });

      // Create new sorted arrays for the x-axis labels and series
      xAxisObj[0].data = sortedIndices.map((i) => totals.get(i).label);
      options.series = options?.series?.map((currentSeries: any) => {
        currentSeries.data = sortedIndices?.map((i) => currentSeries?.data[i]);
        return currentSeries;
      });
    }
  }

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
      let maxValue: string;
      if (panelSchema.type === "pie" || panelSchema.type === "donut") {
        maxValue = options.series[0].data.reduce((max: any, it: any) => {
          return max.length < it?.name?.length ? it?.name : max;
        }, "");
      } else {
        maxValue = options.series.reduce((max: any, it: any) => {
          return max.length < it?.name?.length ? it?.name : max;
        }, "");
      }

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
  // for metric we does not have data field
  if (panelSchema.type != "metric") {
    options.series = options.series.filter((it: any) => it.data?.length);
    if (panelSchema.type == "h-bar" || panelSchema.type == "h-stacked") {
      options.xAxis = options.series.length ? options.xAxis : {};
    } else {
      options.yAxis = options.series.length ? options.yAxis : {};
    }
  }

  // allowed to zoom, only if timeseries
  options.toolbox.show = options.toolbox.show && isTimeSeriesFlag;

  // console.timeEnd("convertSQLData");
  return {
    options,
    extras: { panelId: panelSchema?.id, isTimeSeries: isTimeSeriesFlag },
  };
};

/**
 * Returns the position format for the legend.
 *
 * @param {string} legendPosition - The desired position of the legend. Possible values are "bottom" and "right".
 * @return {string} The format of the legend position. Possible values are "horizontal" and "vertical".
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

const isTimeSeries = (sample: any) => {
  const iso8601Pattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$/;
  return sample.every((value: any) => {
    return iso8601Pattern.test(value);
  });
};

//Check if the sample is timestamp
const isTimeStamp = (sample: any) => {
  const microsecondsPattern = /^\d{16}$/;
  return sample.every((value: any) =>
    microsecondsPattern.test(value.toString())
  );
};

/**
 * Calculates the width of a given text.
 * Useful to calculate nameGap for the left axis
 *
 * @param {string} text - The text to calculate the width of.
 * @return {number} The width of the text in pixels.
 */
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

/**
 * Finds the largest label in the given data array.
 *
 * @param {any[]} data - An array of data.
 * @return {any} The largest label in the data array.
 */
const largestLabel = (data: any) =>
  data.reduce((largest: any, label: any) => {
    return label?.toString().length > largest?.toString().length
      ? label
      : largest;
  }, "");

/**
 * Retrieves the properties for a given chart type and returns them as an object.
 *
 * @param {string} type - The type of chart.
 * @return {Object} - The properties for the given chart type.
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
        areaStyle: null,
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
        avoidLabelOverlap: false,
        emphasis: {
          itemStyle: {
            shadowBlur: 10,
            shadowOffsetX: 0,
            shadowColor: "rgba(0, 0, 0, 0.5)",
          },
          label: {
            show: true,
          },
        },
        label: {
          show: true,
        },
        radius: "80%",
        lineStyle: { width: 1.5 },
      };
    case "donut":
      return {
        type: "pie",
        radius: ["50%", "80%"],
        avoidLabelOverlap: false,
        label: {
          show: true,
          // position: 'center'
        },
        emphasis: {
          label: {
            show: true,
            fontSize: 12,
            fontWeight: "bold",
          },
        },
        labelLine: {
          show: false,
        },
        lineStyle: { width: 1.5 },
      };
    case "h-bar":
      return {
        type: "bar",
        emphasis: { focus: "series" },
        lineStyle: { width: 1.5 },
      };
    case "area":
      return {
        type: "line",
        smooth: true,
        emphasis: { focus: "series" },
        areaStyle: {},
        showSymbol: false,
        lineStyle: { width: 1.5 },
      };
    case "stacked":
      return {
        type: "bar",
        stack: "total",
        emphasis: {
          focus: "series",
        },
        lineStyle: { width: 1.5 },
      };
    case "heatmap":
      return {
        type: "heatmap",
        label: {
          show: true,
        },
        emphasis: {
          itemStyle: {
            shadowBlur: 10,
            shadowColor: "rgba(0, 0, 0, 0.5)",
          },
        },
        lineStyle: { width: 1.5 },
      };
    case "area-stacked":
      return {
        type: "line",
        smooth: true,
        stack: "Total",
        areaStyle: {},
        emphasis: {
          focus: "series",
        },
        showSymbol: false,
        lineStyle: { width: 1.5 },
      };
    case "metric":
      return {
        type: "custom",
        coordinateSystem: "polar",
      };
    case "h-stacked":
      return {
        type: "bar",
        stack: "total",
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
        progress: {
          show: true,
          width: 10,
        },
        pointer: {
          show: false,
        },
        axisLine: {
          lineStyle: {
            width: 10,
          },
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
    default:
      return {
        type: "bar",
      };
  }
};
