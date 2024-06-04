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

<template>
  <div
    class="trace-details"
    :style="{
      width: '97vw !important',
      background: store.state.theme === 'dark' ? '#181a1b' : '#ffffff',
    }"
  >
    <div
      class="row q-px-sm"
      v-if="traceTree.length && !searchObj.data.traceDetails.loading"
    >
      <div
        class="q-py-sm q-px-sm flex items-end justify-between col-12 toolbar"
      >
        <div class="flex items-end justify-start">
          <div class="text-h6 q-mr-lg">
            {{ traceTree[0]["operationName"] }}
          </div>
          <div class="q-pb-xs q-mr-lg flex items-center">
            <div>Trace ID: {{ spanList[0]["trace_id"] }}</div>
            <q-icon
              class="q-ml-xs text-grey-8 cursor-pointer trace-copy-icon"
              size="12px"
              name="content_copy"
              title="Copy"
              @click="copyTraceId"
            />
          </div>

          <div class="q-pb-xs">Spans: {{ spanList.length }}</div>
        </div>
        <div class="flex items-center">
          <q-btn
            data-test="logs-search-bar-share-link-btn"
            class="q-mr-sm download-logs-btn q-px-sm"
            size="sm"
            icon="share"
            round
            flat
            no-outline
            :title="t('search.shareLink')"
            @click="shareLink"
          />
          <q-btn v-close-popup="true" round flat icon="cancel" size="md" />
        </div>
      </div>
      <q-separator style="width: 100%" />
      <div class="col-12 flex justify-between items-end q-pr-sm q-pt-sm">
        <div
          class="trace-chart-btn flex items-center no-wrap cursor-pointer q-mb-sm"
          @click="toggleTimeline"
        >
          <q-icon
            name="expand_more"
            :class="!isTimelineExpanded ? 'rotate-270' : ''"
            size="22px"
            class="cursor-pointer text-grey-10"
          />
          <div class="text-subtitle2 text-bold">
            {{
              activeVisual === "timeline"
                ? "Trace Timeline"
                : "Trace Service Map"
            }}
          </div>
        </div>

        <div
          v-if="isTimelineExpanded"
          class="rounded-borders"
          style="border: 1px solid #cacaca; padding: 2px"
        >
          <template v-for="visual in traceVisuals" :key="visual.value">
            <q-btn
              :color="visual.value === activeVisual ? 'primary' : ''"
              :flat="visual.value === activeVisual ? false : true"
              dense
              no-caps
              size="11px"
              class="q-px-sm visual-selection-btn"
              @click="activeVisual = visual.value"
            >
              <q-icon><component :is="visual.icon" /></q-icon>
              {{ visual.label }}</q-btn
            >
          </template>
        </div>
      </div>
      <div
        v-show="isTimelineExpanded"
        class="col-12"
        :key="isTimelineExpanded.toString()"
      >
        <ChartRenderer
          v-if="activeVisual === 'timeline'"
          class="trace-details-chart"
          id="trace_details_gantt_chart"
          :data="ChartData"
          @updated:chart="updateChart"
          style="height: 200px"
        />
        <ChartRenderer v-else :data="traceServiceMap" style="height: 200px" />
      </div>
      <q-separator style="width: 100%" class="q-mb-sm" />
      <div
        class="histogram-spans-container"
        :class="[
          isSidebarOpen ? 'histogram-container' : 'histogram-container-full',
          isTimelineExpanded ? '' : 'full',
        ]"
      >
        <trace-header
          :baseTracePosition="baseTracePosition"
          :splitterWidth="leftWidth"
        />
        <div class="relative-position full-height">
          <div
            class="trace-tree-container"
            :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
          >
            <div class="q-pt-sm position-relative">
              <div
                :style="{
                  width: '1px',
                  left: `${leftWidth}px`,
                  backgroundColor:
                    store.state.theme === 'dark' ? '#3c3c3c' : '#ececec',
                  zIndex: 999,
                  top: '-28px',
                  height: 'calc(100% + 30px) !important',
                  cursor: 'col-resize',
                }"
                class="absolute full-height"
                @mousedown="startResize"
              />
              <trace-tree
                :collapseMapping="collapseMapping"
                :spans="spanPositionList"
                :baseTracePosition="baseTracePosition"
                :spanDimensions="spanDimensions"
                :spanMap="spanMap"
                :leftWidth="leftWidth"
                class="trace-tree"
                @toggle-collapse="toggleSpanCollapse"
              />
            </div>
          </div>
        </div>
      </div>
      <q-separator vertical />
      <div
        v-if="isSidebarOpen && selectedSpanId"
        class="histogram-sidebar"
        :class="isTimelineExpanded ? '' : 'full'"
      >
        <trace-details-sidebar
          :span="spanMap[selectedSpanId as string]"
          @close="closeSidebar"
        />
      </div>
    </div>
    <div
      v-else-if="searchObj.data.traceDetails.loading"
      class="flex column items-center justify-center"
      :style="{ height: '100%' }"
    >
      <q-spinner-hourglass color="primary" size="3em" :thickness="2" />
      <div class="q-pt-sm">Fetching your trace.</div>
    </div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  type Ref,
  onMounted,
  watch,
  defineAsyncComponent,
} from "vue";
import { cloneDeep } from "lodash-es";
import SpanRenderer from "./SpanRenderer.vue";
import useTraces from "@/composables/useTraces";
import { computed } from "vue";
import TraceDetailsSidebar from "./TraceDetailsSidebar.vue";
import TraceTree from "./TraceTree.vue";
import TraceHeader from "./TraceHeader.vue";
import { useStore } from "vuex";
import { formatTimeWithSuffix, getImageURL } from "@/utils/zincutils";
import TraceTimelineIcon from "@/components/icons/TraceTimelineIcon.vue";
import ServiceMapIcon from "@/components/icons/ServiceMapIcon.vue";
import {
  convertTimelineData,
  convertTraceServiceMapData,
} from "@/utils/traces/convertTraceData";
import { throttle } from "lodash-es";
import { copyToClipboard, useQuasar } from "quasar";
import { useI18n } from "vue-i18n";

export default defineComponent({
  name: "TraceDetails",
  props: {
    traceId: {
      type: String,
      default: "",
    },
  },
  components: {
    SpanRenderer,
    TraceDetailsSidebar,
    TraceTree,
    TraceHeader,
    TraceTimelineIcon,
    ServiceMapIcon,
    ChartRenderer: defineAsyncComponent(
      () => import("@/components/dashboards/panels/ChartRenderer.vue")
    ),
  },
  emits: ["shareLink"],
  setup(props, { emit }) {
    const traceTree: any = ref([]);
    const spanMap: any = ref({});
    const { searchObj } = useTraces();
    const baseTracePosition: any = ref({});
    const collapseMapping: any = ref({});
    const traceRootSpan: any = ref(null);
    const spanPositionList: any = ref([]);
    const splitterModel = ref(25);
    const timeRange: any = ref({ start: 0, end: 0 });
    const store = useStore();
    const traceServiceMap: any = ref({});
    const spanDimensions = {
      height: 30,
      barHeight: 8,
      textHeight: 25,
      gap: 15,
      collapseHeight: "14",
      collapseWidth: 14,
      connectorPadding: 2,
      paddingLeft: 8,
      hConnectorWidth: 20,
      dotConnectorWidth: 6,
      dotConnectorHeight: 6,
      colors: ["#b7885e", "#1ab8be", "#ffcb99", "#f89570", "#839ae2"],
    };

    const { t } = useI18n();

    const $q = useQuasar();

    const traceVisuals = [
      { label: "Timeline", value: "timeline", icon: TraceTimelineIcon },
      { label: "Service Map", value: "service_map", icon: ServiceMapIcon },
    ];

    const activeVisual = ref("timeline");

    const traceChart = ref({
      data: [],
    });

    const ChartData: any = ref({});

    const leftWidth: Ref<number> = ref(250);
    const initialX: Ref<number> = ref(0);
    const initialWidth: Ref<number> = ref(0);

    const throttledResizing = ref<any>(null);

    const spanList: any = computed(() => {
      return searchObj.data.traceDetails.spanList;
    });

    const isTimelineExpanded = ref(false);

    onMounted(() => {
      buildTracesTree();
    });

    watch(
      () => spanList.value.length,
      () => {
        if (spanList.value.length) {
          buildTracesTree();
        } else traceTree.value = [];
      },
      { immediate: true }
    );

    const isSidebarOpen = computed(() => {
      return searchObj.data.traceDetails.showSpanDetails;
    });

    const selectedSpanId = computed(() => {
      return searchObj.data.traceDetails.selectedSpanId;
    });

    const calculateTracePosition = () => {
      const tics = [];
      baseTracePosition.value["durationMs"] = timeRange.value.end;
      baseTracePosition.value["startTimeMs"] =
        traceTree.value[0].startTimeMs + timeRange.value.start;
      const quarterMs = (timeRange.value.end - timeRange.value.start) / 4;
      let time = timeRange.value.start;
      for (let i = 0; i <= 4; i++) {
        tics.push({
          value: Number(time.toFixed(2)),
          label: `${formatTimeWithSuffix(time * 1000)}`,
          left: i === 0 ? "-1px" : `${25 * i}%`,
        });
        time += quarterMs;
      }
      baseTracePosition.value["tics"] = tics;
    };

    // Find out spans who has reference_parent_span_id as span_id of first span in sampleTrace
    async function buildTracesTree() {
      if (!spanList.value?.length) return;

      spanMap.value = {};
      traceTree.value = [];
      spanPositionList.value = [];
      collapseMapping.value = {};
      let lowestStartTime: number = spanList.value[0].start_time;
      let highestEndTime: number = spanList.value[0].end_time;

      if (!spanList.value?.length) return;

      spanList.value.forEach((spanData: any) => {
        spanMap.value[spanData.span_id] = spanData;
      });

      const formattedSpanMap: any = {};

      spanList.value.forEach((spanData: any) => {
        formattedSpanMap[spanData.span_id] = getFormattedSpan(spanData);
      });

      for (let i = 0; i < spanList.value.length; i++) {
        if (spanList.value[i].start_time < lowestStartTime) {
          lowestStartTime = spanList.value[i].start_time;
        }
        if (spanList.value[i].end_time > highestEndTime) {
          highestEndTime = spanList.value[i].end_time;
        }

        const span = formattedSpanMap[spanList.value[i].span_id];

        span.style.color = searchObj.meta.serviceColors[span.serviceName];

        span.style.backgroundColor = adjustOpacity(span.style.color, 0.2);

        span.index = i;

        collapseMapping.value[span.spanId] = true;

        if (!span.parentId) {
          traceTree.value.push(span);
        } else if (!formattedSpanMap[span.parentId]) {
          traceTree.value.push(span);
        } else if (span.parentId && formattedSpanMap[span.parentId]) {
          const parentSpan = formattedSpanMap[span.parentId];
          if (!parentSpan.spans) parentSpan.spans = [];
          parentSpan.spans.push(span);
        }
      }

      traceTree.value[0].lowestStartTime =
        converTimeFromNsToMs(lowestStartTime);
      traceTree.value[0].highestEndTime = converTimeFromNsToMs(highestEndTime);
      traceTree.value[0].style.color =
        searchObj.meta.serviceColors[traceTree.value[0].serviceName];

      traceTree.value.forEach((span: any) => {
        addSpansPositions(span, 0);
      });

      timeRange.value.end = 0;
      timeRange.value.start = 0;

      calculateTracePosition();
      buildTraceChart();
      buildServiceTree();
    }

    let index = 0;
    const addSpansPositions = (span: any, depth: number) => {
      if (!span.index) index = 0;
      span.depth = depth;
      spanPositionList.value.push(
        Object.assign(span, {
          style: {
            color: span.style.color,
            backgroundColor: span.style.backgroundColor,
            top: index * spanDimensions.height + "px",
            left: spanDimensions.gap * depth + "px",
          },
          hasChildSpans: !!span.spans.length,
          currentIndex: index,
        })
      );
      if (collapseMapping.value[span.spanId]) {
        if (span.spans.length) {
          span.spans.forEach((childSpan: any) => {
            index = index + 1;
            childSpan.totalSpans = addSpansPositions(childSpan, depth + 1);
          });
          span.totalSpans = span.spans.reduce(
            (acc: number, span: any) =>
              acc + ((span?.spans?.length || 0) + (span?.totalSpans || 0)),
            0
          );
        }
        return (span?.spans?.length || 0) + (span?.totalSpans || 0);
      } else {
        return 0;
      }
    };

    function adjustOpacity(hexColor: string, opacity: number) {
      // Ensure opacity is between 0 and 1
      opacity = Math.max(0, Math.min(1, opacity));

      // Convert opacity to a hex value
      const opacityHex = Math.round(opacity * 255)
        .toString(16)
        .padStart(2, "0");

      // Append the opacity hex value to the original hex color
      return hexColor + opacityHex;
    }

    const buildServiceTree = () => {
      const serviceTree: any[] = [];
      let maxDepth = 0;
      let maxHeight: number[] = [0];
      const getService = (
        span: any,
        currentColumn: any[],
        serviceName: string,
        depth: number,
        height: number
      ) => {
        maxHeight[depth] =
          maxHeight[depth] === undefined ? 1 : maxHeight[depth] + 1;
        if (serviceName !== span.serviceName) {
          const children: any[] = [];
          currentColumn.push({
            name: `${span.serviceName} \n (${span.durationMs}ms)`,
            parent: serviceName,
            duration: span.durationMs,
            children: children,
            itemStyle: {
              color: searchObj.meta.serviceColors[span.serviceName],
            },
            emphasis: {
              disabled: true,
            },
          });
          if (span.spans && span.spans.length) {
            span.spans.forEach((_span: any) =>
              getService(_span, children, span.serviceName, depth + 1, height)
            );
          } else {
            if (maxDepth < depth) maxDepth = depth;
          }
          return;
        }
        if (span.spans && span.spans.length) {
          span.spans.forEach((span: any) =>
            getService(span, currentColumn, serviceName, depth + 1, height)
          );
        } else {
          if (maxDepth < depth) maxDepth = depth;
        }
      };
      traceTree.value.forEach((span: any) => {
        getService(span, serviceTree, "", 1, 1);
      });
      traceServiceMap.value = convertTraceServiceMapData(
        cloneDeep(serviceTree),
        maxDepth
      );
    };

    // Convert span object to required format
    // Converting ns to ms
    const getFormattedSpan = (span: any) => {
      return {
        [store.state.zoConfig.timestamp_column]:
          span[store.state.zoConfig.timestamp_column],
        startTimeMs: converTimeFromNsToMs(span.start_time),
        endTimeMs: converTimeFromNsToMs(span.end_time),
        durationMs: Number((span.duration / 1000).toFixed(4)), // This key is standard, we use for calculating width of span block. This should always be in ms
        durationUs: Number(span.duration.toFixed(4)), // This key is used for displaying duration in span block. We convert this us to ms, s in span block
        idleMs: convertTime(span.idle_ns),
        busyMs: convertTime(span.busy_ns),
        spanId: span.span_id,
        operationName: span.operation_name,
        serviceName: span.service_name,
        spanStatus: span.span_status,
        spanKind: getSpanKind(span.span_kind.toString()),
        parentId: span.reference_parent_span_id,
        spans: [],
        index: 0,
        style: {
          color: "",
        },
      };
    };

    const convertTime = (time: number) => {
      return Number((time / 1000000).toFixed(2));
    };

    const converTimeFromNsToMs = (time: number) => {
      const nanoseconds = time;
      const milliseconds = Math.floor(nanoseconds / 1000000);
      const date = new Date(milliseconds);
      return date.getTime();
    };

    const getSpanKind = (spanKind: string) => {
      const spanKindMapping: { [key: string]: string } = {
        "1": "Client",
        "2": "Server",
        "3": "Producer",
        "4": "Consumer",
        "5": "Internal",
      };
      return spanKindMapping[spanKind];
    };

    const closeSidebar = () => {
      searchObj.data.traceDetails.showSpanDetails = false;
      searchObj.data.traceDetails.selectedSpanId = null;
    };
    const toggleSpanCollapse = (spanId: number | string) => {
      collapseMapping.value[spanId] = !collapseMapping.value[spanId];
      index = 0;
      spanPositionList.value = [];
      traceTree.value.forEach((span: any) => {
        addSpansPositions(span, 0);
      });
    };
    const buildTraceChart = () => {
      const data: any = [];
      for (let i = spanPositionList.value.length - 1; i > -1; i--) {
        const absoluteStartTime =
          spanPositionList.value[i].startTimeMs -
          traceTree.value[0].lowestStartTime;

        data.push({
          x0: absoluteStartTime,
          x1: Number(
            (absoluteStartTime + spanPositionList.value[i].durationMs).toFixed(
              4
            )
          ),
          fillcolor: spanPositionList.value[i].style.color,
        });
      }
      traceChart.value.data = data;
      ChartData.value = convertTimelineData(traceChart);
    };
    const updateChart = (data: any) => {
      timeRange.value.start = data.start || 0;
      timeRange.value.end = data.end || 0;
      calculateTracePosition();
    };
    const mockServiceMap = [
      {
        name: "Service A",
        color: "#000000",
        duration: "10",
        children: [
          {
            name: "Service B",
            color: "#000000",
            duration: "10",

            children: [
              {
                name: "Service c",
                color: "#000000",
                duration: "10",

                children: [
                  {
                    name: "Service F",
                    color: "#000000",
                    duration: "10",

                    children: [
                      {
                        name: "Service G",
                        color: "#000000",
                        duration: "10",

                        children: [
                          {
                            name: "Service H",
                            color: "#000000",
                            duration: "10",

                            children: [
                              {
                                name: "Service H",
                                color: "#000000",
                                duration: "10",

                                children: [
                                  {
                                    name: "Service H",
                                    color: "#000000",
                                    duration: "10",

                                    children: [
                                      {
                                        name: "Service H H H H H H H H H H H H H H H H H",
                                        color: "#000000",
                                        duration: "10",
                                      },
                                    ],
                                  },
                                ],
                              },
                            ],
                          },
                        ],
                      },
                    ],
                  },
                ],
              },
              {
                name: "Service E",
                color: "#000000",
                duration: "10",
              },
            ],
          },
          { name: "Service D", color: "#000000", duration: "10" },
          { name: "Service D", color: "#000000", duration: "10" },
        ],
      },
      {
        name: "Service X",
        color: "#000000",
        duration: "10",
      },
      {
        name: "Service Y",
        color: "#000000",
        duration: "10",
        children: [
          {
            name: "Service YA",
            color: "#000000",
            duration: "10",
          },
        ],
      },
    ];

    onMounted(() => {
      throttledResizing.value = throttle(resizing, 50);
    });

    const startResize = (event: any) => {
      initialX.value = event.clientX;
      initialWidth.value = leftWidth.value;

      window.addEventListener("mousemove", throttledResizing.value);
      window.addEventListener("mouseup", stopResize);
      document.body.classList.add("no-select");
    };

    const resizing = (event: any) => {
      const deltaX = event.clientX - initialX.value;
      leftWidth.value = initialWidth.value + deltaX;
    };

    const stopResize = () => {
      window.removeEventListener("mousemove", throttledResizing.value);
      window.removeEventListener("mouseup", stopResize);
      document.body.classList.remove("no-select");
    };

    const toggleTimeline = () => {
      isTimelineExpanded.value = !isTimelineExpanded.value;
    };

    const copyTraceId = () => {
      $q.notify({
        type: "positive",
        message: "Trace ID copied to clipboard",
        timeout: 2000,
      });
      copyToClipboard(spanList.value[0]["trace_id"]);
    };

    const shareLink = () => {
      emit("shareLink");
    };

    return {
      t,
      traceTree,
      collapseMapping,
      traceRootSpan,
      baseTracePosition,
      searchObj,
      spanList,
      isSidebarOpen,
      selectedSpanId,
      spanMap,
      closeSidebar,
      toggleSpanCollapse,
      spanPositionList,
      spanDimensions,
      splitterModel,
      ChartData,
      traceChart,
      updateChart,
      traceServiceMap,
      mockServiceMap,
      activeVisual,
      traceVisuals,
      getImageURL,
      store,
      leftWidth,
      startResize,
      isTimelineExpanded,
      toggleTimeline,
      copyToClipboard,
      copyTraceId,
      shareLink,
    };
  },
});
</script>

<style scoped lang="scss">
$sidebarWidth: 60%;
$seperatorWidth: 2px;
$toolbarHeight: 50px;
$traceHeaderHeight: 30px;
$traceChartHeight: 210px;

$traceChartCollapseHeight: 42px;

.toolbar {
  height: $toolbarHeight;
}
.trace-details {
  height: 100vh;
  overflow: auto;
}
.histogram-container-full {
  width: 100%;
}
.histogram-container {
  width: calc(100% - $sidebarWidth - $seperatorWidth);
}

.histogram-sidebar {
  width: $sidebarWidth;
  height: calc(100vh - $toolbarHeight - $traceChartHeight - 44px);
  overflow-y: auto;
  overflow-x: hidden;

  &.full {
    height: calc(100vh - $toolbarHeight - 8px - 44px);
  }
}

.histogram-spans-container {
  height: calc(100vh - $toolbarHeight - $traceChartHeight - 44px);
  overflow-y: auto;
  position: relative;
  overflow-x: hidden;

  &.full {
    height: calc(100vh - $toolbarHeight - 8px - 44px);
  }
}

.trace-tree-container {
  overflow: auto;
}

.trace-chart-btn {
  cursor: pointer;
  padding-right: 8px;
  border-radius: 2px;
  padding-top: 2px;
  padding-bottom: 2px;

  &:hover {
    background-color: rgba($primary, 0.9);
    color: #ffffff;

    .q-icon {
      color: #ffffff !important;
    }
  }
}
</style>
<style lang="scss">
.trace-details {
  .q-splitter__before,
  .q-splitter__after {
    overflow: revert !important;
  }

  .q-splitter__before {
    z-index: 999 !important;
  }

  .trace-details-chart {
    .rangeslider-slidebox {
      fill: #7076be !important;
    }
    .rangeslider-mask-max,
    .rangeslider-mask-min {
      fill: #d2d2d2 !important;
      fill-opacity: 1 !important;
    }
  }

  .visual-selection-btn {
    .q-icon {
      padding-right: 5px;
      font-size: 15px;
    }
  }
}

.no-select {
  user-select: none !important;
  -moz-user-select: none !important;
  -webkit-user-select: none !important;
  -ms-user-select: none !important;
}

.trace-copy-icon {
  &:hover {
    &.q-icon {
      text-shadow: 0px 2px 8px rgba(0, 0, 0, 0.5);
    }
  }
}
</style>
