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
  <template v-for="span in spans as any[]" :key="span.spanId">
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
            class="flex no-wrap q-pt-sm full-width"
            :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
            :style="{ height: '30px' }"
          >
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
      />
    </div>
  </template>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { getImageURL } from "@/utils/zincutils";
import useTraces from "@/composables/useTraces";
import { useStore } from "vuex";
import SpanBlock from "./SpanBlock.vue";

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

    function toggleSpanCollapse(spanId: number | string) {
      emit("toggleCollapse", spanId);
    }
    const selectSpan = (spanId: string) => {
      emit("selectSpan", spanId);
    };

    return {
      toggleSpanCollapse,
      getImageURL,
      selectSpan,
      store,
    };
  },
  components: { SpanBlock },
});
</script>

<style scoped lang="scss">
.spans-container {
  position: relative;
}

.collapse-btn {
  width: 14px;
  height: auto;
  opacity: 0.6;
}
</style>
