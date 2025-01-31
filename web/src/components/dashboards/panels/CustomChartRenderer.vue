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

<!-- eslint-disable vue/v-on-event-hyphenation -->
<!-- eslint-disable vue/attribute-hyphenation -->
<template>
  <div
    data-test="chart-renderer"
    ref="chartRef"
    id="chart1"
    @mouseover="handleMouseOver"
    @mouseleave="handleMouseLeave"
    style="height: 100%; width: 100%"
  ></div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onMounted,
  watch,
  onUnmounted,
  nextTick,
  inject,
} from "vue";
import * as echarts from "echarts";

// Import all components and renderers once for generic usage
import {
  TitleComponent,
  TooltipComponent,
  GridComponent,
  ToolboxComponent,
  LegendComponent,
  DataZoomComponent,
} from "echarts/components";
import {
  BarChart,
  LineChart,
  PieChart,
  ScatterChart,
  HeatmapChart,
} from "echarts/charts";
import { CanvasRenderer, SVGRenderer } from "echarts/renderers";

// Register necessary components
echarts.use([
  TitleComponent,
  TooltipComponent,
  GridComponent,
  ToolboxComponent,
  LegendComponent,
  DataZoomComponent,
  BarChart,
  LineChart,
  PieChart,
  ScatterChart,
  HeatmapChart,
  CanvasRenderer,
  SVGRenderer,
]);

export default defineComponent({
  name: "ChartRenderer",
  props: {
    data: {
      required: true,
      type: Object,
      default: () => ({ option: {} }),
    },
    renderType: {
      type: String,
      default: "canvas",
    },
    height: {
      type: String,
      default: "100%",
    },
  },
  setup(props, { emit }) {
    const chartRef = ref(null);
    let chart: echarts.ECharts | null = null;

    const hoveredSeriesState = inject("hoveredSeriesState", null);

    const initChart = async () => {
      if (!chartRef.value) return;

      // Destroy the existing chart instance if any
      if (chart) {
        chart.dispose();
      }

      // Initialize chart
      chart = echarts.init(chartRef.value, undefined, {
        renderer: "canvas",
      });

      // Apply option from props
      chart.setOption(props?.data?.option ?? {});

      // Add event listeners for generic interactions
      chart.on("mousemove", (params) => emit("mousemove", params));
      chart.on("mouseout", () => emit("mouseout"));
      chart.on("legendselectchanged", (params) =>
        emit("legendChanged", params),
      );
    };

    const handleResize = async () => {
      await nextTick();
      chart?.resize();
    };

    const handleMouseOver = () => {
      if (hoveredSeriesState)
        hoveredSeriesState.panelId = props.data.extras?.panelId;
    };

    const handleMouseLeave = () => {
      // if (hoveredSeriesState) hoveredSeriesState.setIndex(-1, -1, -1, null);
    };

    onMounted(() => {
      initChart();

      // Watch for window resize
      window.addEventListener("resize", handleResize);
    });
    watch(
      () => props.data.option,
      async () => {
        await initChart(); // Re-initialize chart when option change
      },
      { deep: true },
    );

    onUnmounted(() => {
      chart?.dispose();
      window.removeEventListener("resize", handleResize);
    });

    return { chartRef, handleMouseOver, handleMouseLeave };
  },
});
</script>
