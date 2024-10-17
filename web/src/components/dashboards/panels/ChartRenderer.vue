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
    data-test="chart-renderer"
    ref="chartRef"
    id="chart1"
    @mouseover="
      () => {
        // if hoveredSeriesState is not null then set panelId
        if (hoveredSeriesState)
          hoveredSeriesState.panelId = data?.extras?.panelId;
      }
    "
    @mouseleave="
      () => {
        // if hoveredSeriesState is not null then set -1
        if (hoveredSeriesState) hoveredSeriesState.setIndex(-1, -1, -1, null);
      }
    "
    style="height: 100%; width: 100%"
  ></div>
</template>

<script lang="ts">
// Find the index of the nearest value in a sorted array
// used in timeseries hovering across all charts to find nearest dataindex
function findNearestIndex(sortedArray: any, target: any) {
  let left = 0;
  let right = sortedArray.length - 1;
  let nearestIndex = -1;

  while (left <= right) {
    const mid = Math.floor((left + right) / 2);

    if (sortedArray[mid][0] === target) {
      return mid; // Found the target at index mid
    }

    if (
      nearestIndex === -1 ||
      Math.abs(sortedArray[mid][0] - target) <
        Math.abs(sortedArray[nearestIndex][0] - target)
    ) {
      nearestIndex = mid; // Update nearestIndex if current element is closer to the target
    }

    if (sortedArray[mid][0] < target) {
      left = mid + 1; // Target is in the right half
    } else {
      right = mid - 1; // Target is in the left half
    }
  }

  return nearestIndex; // Return the index of the nearest value
}

import {
  defineComponent,
  ref,
  onMounted,
  watch,
  onUnmounted,
  nextTick,
  onActivated,
  inject,
} from "vue";
import { useStore } from "vuex";
import * as echarts from "echarts/core";
import {
  BarChart,
  LineChart,
  CustomChart,
  GaugeChart,
  PieChart,
  ScatterChart,
  HeatmapChart,
  SankeyChart,
  TreeChart,
  GraphChart,
} from "echarts/charts";
import {
  TitleComponent,
  TooltipComponent,
  GridComponent,
  ToolboxComponent,
  DatasetComponent,
  LegendComponent,
  PolarComponent,
  VisualMapComponent,
  DataZoomComponent,
  MarkLineComponent,
} from "echarts/components";
import { LabelLayout, UniversalTransition } from "echarts/features";
import { CanvasRenderer, SVGRenderer } from "echarts/renderers";
import type {
  BarSeriesOption,
  LineSeriesOption,
  CustomSeriesOption,
  GaugeSeriesOption,
  PieSeriesOption,
  ScatterSeriesOption,
  HeatmapSeriesOption,
  SankeySeriesOption,
  TreeSeriesOption,
} from "echarts/charts";
import type { ComposeOption } from "echarts/core";
import type {
  TitleComponentOption,
  TooltipComponentOption,
  GridComponentOption,
  ToolboxComponentOption,
  DatasetComponentOption,
  LegendComponentOption,
  PolarComponentOption,
  VisualMapComponentOption,
  DataZoomComponentOption,
  MarkLineComponentOption,
} from "echarts/components";

type ECOption = ComposeOption<
  | BarSeriesOption
  | LineSeriesOption
  | CustomSeriesOption
  | GaugeSeriesOption
  | PieSeriesOption
  | ScatterSeriesOption
  | HeatmapSeriesOption
  | SankeySeriesOption
  | TreeSeriesOption
  | TitleComponentOption
  | TooltipComponentOption
  | GridComponentOption
  | ToolboxComponentOption
  | DatasetComponentOption
  | LegendComponentOption
  | PolarComponentOption
  | VisualMapComponentOption
  | DataZoomComponentOption
  | MarkLineComponentOption
>;

echarts.use([
  TitleComponent,
  TooltipComponent,
  GridComponent,
  ToolboxComponent,
  DatasetComponent,
  LegendComponent,
  PolarComponent,
  VisualMapComponent,
  DataZoomComponent,
  MarkLineComponent,
  BarChart,
  LineChart,
  CustomChart,
  GaugeChart,
  PieChart,
  ScatterChart,
  HeatmapChart,
  SankeyChart,
  TreeChart,
  GraphChart,
  LabelLayout,
  UniversalTransition,
  CanvasRenderer,
  SVGRenderer,
]);

export default defineComponent({
  name: "ChartRenderer",
  emits: [
    "updated:chart",
    "click",
    "updated:dataZoom",
    "error",
    "mouseover",
    "mousemove",
    "mouseout",
  ],
  props: {
    data: {
      required: true,
      type: Object,
      default: () => ({ options: {} }),
    },
    renderType: {
      type: String,
      default: "canvas",
    },
  },
  setup(props: any, { emit }) {
    const chartRef: any = ref(null);
    let chart: any;
    const store = useStore();
    const windowResizeEventCallback = async () => {
      try {
        await nextTick();
        await nextTick();
        chart?.resize();
      } catch (e) {}
    };

    // currently hovered series state
    const hoveredSeriesState: any = inject("hoveredSeriesState", null);

    const mouseHoverEffectFn = (params: any) => {
      // if chart type is pie then set seriesName and seriesIndex from data and dataIndex
      // seriesName and seriesIndex will used in the same function
      if (params?.componentSubType === "pie") {
        params.seriesName = params?.data?.name;
        params.seriesIndex = params?.dataIndex;
      }

      // if sankey chart then do not set seriesName and seriesIndex
      if (params?.componentSubType === "sankey") {
        params.seriesName = "";
        params.seriesIndex = -1;
      }

      // set current hovered series name in state
      hoveredSeriesState?.value?.setHoveredSeriesName(params?.seriesName ?? "");

      // Below logic is to scroll legend upto current series index
      // which creates wrong legend highlight issue in tooltip
      // so commented out

      // scroll legend upto current series index
      // const legendOption = chart?.getOption()?.legend[0];
      // if (legendOption) {
      // legendOption.scrollDataIndex = params?.seriesIndex || 0;
      // chart?.setOption({ legend: [legendOption] });
      // chart?.dispatchAction({
      //   type: "legendScroll",
      //   scrollDataIndex: params?.seriesIndex || 0,
      //   legendId: params?.seriesId,
      // });
      // }
    };

    const mouseOutEffectFn = () => {
      // reset current hovered series name in state
      hoveredSeriesState?.value?.setHoveredSeriesName("");
    };

    const legendSelectChangedFn = (params: any) => {
      // check if all series are selected (all will be false)
      if (
        Object.values(params.selected).every((value: any) => value === false)
      ) {
        // set all series to true
        Object.keys(params.selected).forEach((name: any) => {
          params.selected[name] = true;
        });

        // select only selected series
      } else {
        // set all false except selected series
        Object.keys(params.selected).forEach((name: any) => {
          params.selected[name] = params.name === name ? true : false;
        });
      }

      // get legend
      const legendOption = chart?.getOption()?.legend[0];

      if (legendOption) {
        // set selected array
        legendOption.selected = params.selected;

        // set options with selected object
        chart?.setOption({ legend: [legendOption] });
      }
    };

    // restore chart and select datazoom button
    const restoreChart = () => {
      chart?.dispatchAction({
        type: "restore",
      });
      // we need that toolbox datazoom button initially selected
      chart?.dispatchAction({
        type: "takeGlobalCursor",
        key: "dataZoomSelect",
        dataZoomSelectActive: true,
      });
    };

    const chartInitialSetUp = () => {
      chart?.on("mousemove", (params: any) => {
        emit("mousemove", params);
        mouseHoverEffectFn(params);
      });
      chart?.on("mouseout", (params: any) => {
        emit("mouseout", params);
        mouseOutEffectFn();
      });
      chart?.on("globalout", () => {
        mouseHoverEffectFn({});
        hoveredSeriesState?.value?.setIndex(-1, -1, -1, null);
        hoveredSeriesState?.value?.setHoveredSeriesName("");
      });

      chart?.on("legendselectchanged", legendSelectChangedFn);
      chart?.on("highlight", (params: any) => {
        // reset hovered series name on downplay
        // hoveredSeriesState?.value?.setHoveredSeriesName("");

        // downplay event will only called by currently hovered panel else it will go into infinite loop
        // and chart must be timeseries chart
        if (
          props.data.extras?.panelId == hoveredSeriesState?.value?.panelId &&
          props?.data?.extras?.isTimeSeries === true
        ) {
          const seriesIndex = params?.batch?.[0]?.seriesIndex;
          const dataIndex = Math.max(params?.batch?.[0]?.dataIndex, 0);

          // set current hovered series name in state
          if (chart?.getOption()?.series[seriesIndex]?.data[dataIndex]) {
            hoveredSeriesState?.value?.setIndex(
              dataIndex,
              seriesIndex,
              props?.data?.extras?.panelId || -1,
              chart?.getOption()?.series[seriesIndex]?.data[dataIndex][0]
            );
          }
        }
      });
      emit("updated:chart", {
        start: chart?.getOption()?.dataZoom[0]?.startValue || 0,
        end: chart?.getOption()?.dataZoom[0]?.endValue || 0,
      });

      //on dataZoom emit an event of start x and end x
      chart?.on("dataZoom", function (params: any) {
        //if batch then emit dataZoom event
        if (params?.batch) {
          emit("updated:dataZoom", {
            start: params?.batch[0]?.startValue || 0,
            end: params?.batch[0]?.endValue || 0,
          });
          restoreChart();
        }
        //else if daatazoom then emit dataZoom event
        else if (chart?.getOption()?.dataZoom) {
          emit("updated:chart", {
            start: chart?.getOption()?.dataZoom[0]?.startValue || 0,
            end: chart?.getOption()?.dataZoom[0]?.endValue || 0,
          });
        }
      });
      chart?.on("click", function (params: any) {
        emit("click", params);
      });

      chart?.on("mouseover", function (params: any) {
        emit("mouseover", params);
      });

      window.removeEventListener("resize", windowResizeEventCallback);
      window.addEventListener("resize", windowResizeEventCallback);

      // we need that toolbox datazoom button initially selected
      chart?.dispatchAction({
        type: "takeGlobalCursor",
        key: "dataZoomSelect",
        dataZoomSelectActive: true,
      });
    };

    // dispatch tooltip action for all charts
    watch(
      () => [
        hoveredSeriesState?.value?.seriesIndex,
        hoveredSeriesState?.value?.dataIndex,
        hoveredSeriesState?.value?.panelId,
      ],
      () => {
        // if hovered series is not same as currently hovered series
        // and hovered series is not -1
        // and chart is time series
        if (
          isChartVisible &&
          props?.data?.extras?.panelId &&
          props?.data?.extras?.panelId != hoveredSeriesState?.value?.panelId &&
          hoveredSeriesState?.value?.panelId != -1 &&
          props?.data?.extras?.isTimeSeries === true
        ) {
          // need to check index should not be greater than series length
          const hoveredSeriesIndex =
            chart?.getOption()?.series.length >
            hoveredSeriesState?.value?.seriesIndex
              ? hoveredSeriesState?.value?.seriesIndex
              : 0;
          let hoveredSeriesDataIndex = hoveredSeriesState?.value?.dataIndex;

          // if hovered series dataindex is not there
          // or check hovered time is not at the same index in the current chart (ie if at same index then not need to find nearest index)
          if (
            !chart?.getOption()?.series[hoveredSeriesIndex]?.data[
              hoveredSeriesDataIndex
            ] ||
            chart?.getOption()?.series[hoveredSeriesIndex]?.data[
              hoveredSeriesDataIndex
            ][0] != hoveredSeriesState.value?.hoveredTime
          ) {
            hoveredSeriesDataIndex = findNearestIndex(
              chart?.getOption()?.series[hoveredSeriesIndex]?.data ?? [],
              hoveredSeriesState?.value?.hoveredTime
            );
          }

          chart?.dispatchAction({
            type: "showTip",
            seriesIndex: hoveredSeriesIndex,
            dataIndex: hoveredSeriesDataIndex,
          });
        }

        // if state is -1 then restore chart
        if (
          hoveredSeriesState.value?.dataIndex == -1 &&
          hoveredSeriesState.value?.seriesIndex == -1 &&
          hoveredSeriesState.value?.panelId == -1 &&
          hoveredSeriesState.value?.hoveredTime == null
        ) {
          restoreChart();
        }
      }
    );

    watch(
      () => hoveredSeriesState?.value?.hoveredSeriesName,
      () => {
        chart?.dispatchAction({
          type: "highlight",
          seriesName: hoveredSeriesState?.value?.hoveredSeriesName,
        });
      }
    );

    watch(
      () => store.state.theme,
      (newTheme) => {
        const theme = newTheme === "dark" ? "dark" : "light";
        chart?.dispose();
        chart = echarts.init(chartRef.value, theme, {
          renderer: props.renderType,
        });
        const options = props.data.options || {};

        // change color and background color of tooltip
        options.tooltip &&
          options.tooltip.textStyle &&
          (options.tooltip.textStyle.color =
            theme === "dark" ? "#fff" : "#000");
        options.tooltip &&
          (options.tooltip.backgroundColor =
            theme === "dark" ? "rgba(0,0,0,1)" : "rgba(255,255,255,1)");
        options.animation = false;
        try {
          chart?.setOption(options, true);
          chart?.setOption({ animation: true });
        } catch (e) {
          emit("error", e);
        }

        chartInitialSetUp();
      }
    );

    onMounted(async () => {
      try {
        await nextTick();
        await nextTick();
        await nextTick();
        await nextTick();
        await nextTick();
        await nextTick();
        await nextTick();
        const theme = store.state.theme === "dark" ? "dark" : "light";
        if (chartRef.value) {
          chart = echarts.init(chartRef.value, theme, {
            renderer: props.renderType,
          });
        }
        chart?.setOption(props?.data?.options || {}, true);
        chartInitialSetUp();
      } catch (e) {
        emit("error", e);
      }
    });
    onUnmounted(() => {
      window.removeEventListener("resize", windowResizeEventCallback);
    });

    // observer for chart visibility
    let isChartVisibleObserver: any;

    // flag for chart visibility
    let isChartVisible: any = false;

    onMounted(() => {
      // chart visibility observer
      isChartVisibleObserver = new IntersectionObserver((entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            // chart is visible
            isChartVisible = true;
          } else {
            // chart is not visible
            isChartVisible = false;
          }
        });
      });

      if (chartRef.value) {
        // observe chart
        isChartVisibleObserver.observe(chartRef.value);
      }
    });

    onUnmounted(() => {
      if (chartRef.value) {
        // unobserve chart
        isChartVisibleObserver.unobserve(chartRef.value);
      }
    });

    //need to resize chart on activated
    onActivated(() => {
      windowResizeEventCallback();

      // we need that toolbox datazoom button initially selected
      chart?.dispatchAction({
        type: "takeGlobalCursor",
        key: "dataZoomSelect",
        dataZoomSelectActive: true,
      });
    });

    watch(
      () => props.data.options,
      async () => {
        try {
          await nextTick();
          chart?.resize();
          try {
            chart?.setOption(props?.data?.options || {}, true);
          } catch (error) {}

          // we need that toolbox datazoom button initially selected
          // for that we required to dispatch an event
          // while dispatching an event we need to pass a datazoomselectactive as true
          // this action is available in the echarts docs in list of brush actions
          chart?.dispatchAction({
            type: "takeGlobalCursor",
            key: "dataZoomSelect",
            dataZoomSelectActive: true,
          });
          windowResizeEventCallback();
        } catch (e) {
          emit("error", e);
        }
      },
      { deep: true }
    );
    return { chartRef, hoveredSeriesState };
  },
});
</script>
