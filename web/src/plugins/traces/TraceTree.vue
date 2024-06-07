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
  <template v-for="(span, index) in spans as any[]" :key="span.spanId">
    <div
      :style="{
        position: 'relative',
        width: '100%',
        overflow: 'visible',
        flexWrap: 'nowrap',
      }"
      class="flex"
    >
      <div :style="{ width: leftWidth + 'px' }">
        <div
          :style="{
            height: '100%',
            margin: `0 0px 0 ${
              span.hasChildSpans
                ? span.style.left
                : parseInt(span.style.left) +
                  spanDimensions.collapseWidth +
                  'px'
            }`,
          }"
          class="flex items-start justify-start ellipsis"
          :title="span.operationName"
        >
          <div
            class="flex no-wrap q-pt-sm full-width relative-position operation-name-container"
            :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
            :style="{ height: '30px' }"
            @mouseover="() => (spanHoveredIndex = index)"
            @mouseout="() => (spanHoveredIndex = -1)"
          >
            <div
              class="absolute view-logs-container"
              :class="spanHoveredIndex === index ? 'show' : ''"
            >
              <q-btn
                class="q-mx-xs view-span-logs"
                :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
                size="10px"
                icon="search"
                dense
                no-caps
                :title="t('traces.viewLogs')"
                @click.stop="viewSpanLogs(span)"
              >
                <!-- <span class="text view-logs-btn-text">View Logs</span> -->
              </q-btn>
            </div>
            <div
              v-if="span.hasChildSpans"
              :style="{
                width: spanDimensions.collapseWidth + 'px',
                height: spanDimensions.collapseHeight + 'px',
              }"
              class="q-pt-xs flex justify-center items-center collapse-container cursor-pointer"
              @click.stop="toggleSpanCollapse(span.spanId)"
            >
              <q-icon
                dense
                round
                flat
                name="expand_more"
                class="collapse-btn"
                :style="{
                  rotate: collapseMapping[span.spanId] ? '0deg' : '270deg',
                }"
              />
            </div>
            <div
              class="ellipsis q-pl-xs cursor-pointer"
              :style="{
                paddingLeft: '4px',
                borderLeft: `3px solid ${span.style.color}`,
              }"
              @click="selectSpan(span.spanId)"
            >
              <q-icon
                v-if="span.spanStatus === 'ERROR'"
                name="error"
                class="text-red-6 q-mr-xs"
                title="Error Span"
              />
              <span class="text-subtitle2 text-bold q-mr-sm">
                {{ span.serviceName }}
              </span>
              <span
                class="text-body2"
                :class="
                  store.state.theme === 'dark'
                    ? 'text-grey-5'
                    : 'text-blue-grey-9'
                "
                >{{ span.operationName }}</span
              >
            </div>
          </div>
          <div
            :style="{
              backgroundColor: span.style.backgroundColor,
              height: `calc(100% - 30px)`,
              borderLeft: `3px solid ${span.style.color}`,
              marginLeft: span.hasChildSpans ? '14px' : '0',
              width: '100%',
            }"
          ></div>
        </div>
      </div>

      <span-block
        :span="span"
        :depth="depth"
        :baseTracePosition="baseTracePosition"
        :styleObj="{
          position: 'absolute',
          top: span.style.top,
          left: span.style.left,
          height: '60px',
        }"
        :style="{
          width: `calc(100% - ${leftWidth}px)`,
        }"
        :spanDimensions="spanDimensions"
        :isCollapsed="collapseMapping[span.spanId]"
        :spanData="spanMap[span.spanId]"
        @toggle-collapse="toggleSpanCollapse"
        @select-span="selectSpan"
        @mouseover="() => (spanHoveredIndex = index)"
        @mouseout="() => (spanHoveredIndex = -1)"
        @view-logs="viewSpanLogs(span)"
      />
    </div>
  </template>
</template>

<script lang="ts">
import { defineComponent, onBeforeMount, ref } from "vue";
import { getImageURL } from "@/utils/zincutils";
import useTraces from "@/composables/useTraces";
import { useStore } from "vuex";
import SpanBlock from "./SpanBlock.vue";
import type { Ref } from "vue";
import { useI18n } from "vue-i18n";
import { b64EncodeStandard } from "@/utils/zincutils";
import { useRouter } from "vue-router";

export default defineComponent({
  name: "TraceTree",
  props: {
    spans: {
      type: Array,
      default: () => [],
    },
    isCollapsed: {
      type: Boolean,
      default: false,
    },
    collapseMapping: {
      type: Object,
      default: () => {},
    },
    baseTracePosition: {
      type: Object,
      default: () => null,
    },
    depth: {
      type: Number,
      default: 0,
    },
    spanDimensions: {
      type: Object,
      default: () => {},
    },
    spanMap: {
      type: Object,
      default: () => ({}),
    },
    leftWidth: {
      type: Number,
      default: 0,
    },
  },
  emits: ["toggleCollapse", "selectSpan"],
  setup(props, { emit }) {
    const { searchObj } = useTraces();
    const store = useStore();

    const { t } = useI18n();

    const spanHoveredIndex = ref(-1);

    const router = useRouter();

    function toggleSpanCollapse(spanId: number | string) {
      emit("toggleCollapse", spanId);
    }
    const selectSpan = (spanId: string) => {
      emit("selectSpan", spanId);
    };

    const viewSpanLogs = (span: any) => {
      const stream: string =
        searchObj.data.traceDetails.selectedLogStreams.join(",");

      // 1 minute before and after the span
      const from = span.startTimeMs * 1000 - 60000000;
      const to = span.endTimeMs * 1000 + 60000000;
      const refresh = 0;
      const query = b64EncodeStandard(
        `span_id='${span.spanId}' AND trace_id='${searchObj.data.traceDetails.selectedTrace.trace_id}'`
      );

      router.push({
        path: "/logs",
        query: {
          stream_type: "logs",
          stream,
          from,
          to,
          refresh,
          sql_mode: "false",
          query,
          org_identifier: store.state.selectedOrganization.identifier,
          show_histogram: "true",
          type: "trace_explorer",
          quick_mode: "false",
        },
      });
    };

    return {
      toggleSpanCollapse,
      getImageURL,
      selectSpan,
      store,
      viewSpanLogs,
      t,
      spanHoveredIndex,
    };
  },
  components: { SpanBlock },
});
</script>

<style scoped lang="scss">
.view-logs-container {
  top: 7px;
  right: 0;
}
.spans-container {
  position: relative;
}

.collapse-btn {
  width: 14px;
  height: auto;
  opacity: 0.6;
}

.operation-name-container {
  .view-logs-container {
    visibility: hidden;
  }
  .view-logs-container {
    &.show {
      visibility: visible !important;
    }
  }
}
</style>
<style lang="scss">
.view-logs-btn-text {
  visibility: visible;
}
.view-span-logs {
  background-color: inherit;
  .view-logs-btn-text {
    visibility: hidden;
    width: 0px;
    transition: width 0.3s ease-in;
  }
  &:hover .view-logs-btn-text {
    visibility: visible;
    width: auto;
  }
}
</style>
