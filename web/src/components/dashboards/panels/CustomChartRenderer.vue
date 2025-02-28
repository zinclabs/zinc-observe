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
    id="chart"
    @mouseover="handleMouseOver"
    @mouseleave="handleMouseLeave"
    style="height: 100%; width: 100%"
  ></div>
</template>

<script >
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
import { CanvasRenderer, SVGRenderer } from "echarts/renderers";

import DOMPurify from "dompurify";

// Register necessary components
echarts.use([
  TitleComponent,
  TooltipComponent,
  GridComponent,
  ToolboxComponent,
  LegendComponent,
  DataZoomComponent,
  CanvasRenderer,
  SVGRenderer,
]);

export default defineComponent({
  name: "CustomChartRenderer",
  emits: [
    "error",
    "mousemove",
    "mouseout",
    "click",
  ],
  props: {
    data: {
      required: true,
      type: Object,
      default: () => ({}),
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
    let chart = null;

    const hoveredSeriesState = inject("hoveredSeriesState", null);
    const convertStringToFunction = (obj) => {
        if (typeof obj === 'string' && obj.startsWith('function')) {
          try {
            return new Function(`return ${obj}`)(); // Convert string to function
          } catch (error) {
           emit("error",`Error while executing the code: ${error.message}`);
          }
        }

        if (Array.isArray(obj)) {
          return obj.map(item => convertStringToFunction(item)); // Recursively handle arrays
        }

        if (typeof obj === 'object' && obj !== null) {
          const result = {};
          for (const key in obj) {
            if (obj.hasOwnProperty(key)) {
              result[key] = convertStringToFunction(obj[key]); // Recursively handle object properties
            }
          }
          return result;
        }

        return obj; // If it's not a function string or an object, return it as is
      };
      function deepSanitize(obj) {
        if (typeof obj === 'string') {
          return DOMPurify.sanitize(obj);
        } else if (Array.isArray(obj)) {
          return obj.map(deepSanitize);
        } else if (typeof obj === 'object' && obj !== null) {
          return Object.fromEntries(
            Object.entries(obj).map(([key, value]) => [key, deepSanitize(value)])
          );
        }
        return obj;
      }


    const initChart = async () => {
      if (!chartRef.value) return;

      const echartsGL = await import('echarts-gl');

      // Initialize chart
      chart = echarts.init(chartRef.value, undefined, {
        renderer: "canvas",
      });

      try{
      const convertedData = convertStringToFunction(props.data);
      const safeChartOptions = deepSanitize(convertedData);
      chart.setOption(safeChartOptions);

      if (convertedData.o2_events) {
        
        // Add event listeners for custom interactions
        for (const event in convertedData.o2_events) {
          chart.off(event);
          chart.on(event, (params) => convertedData.o2_events[event](params,chart));
        }
      }

      }
      catch(e){
        emit("error","Error while executing the code");
      }




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
      () => props.data,
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
