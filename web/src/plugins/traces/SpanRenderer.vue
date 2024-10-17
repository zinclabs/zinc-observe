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
  <div class="relative-position q-pt-sm" :style="{ height: '100%' }">
    <template v-if="spans?.length">
      <template v-for="span in spans as any[]" :key="span.spanId"> </template>
    </template>
  </div>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import SpanBlock from "./SpanBlock.vue";

export default defineComponent({
  name: "SpanRenderer",
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
  },
  emits: ["toggleCollapse"],
  setup(props, { emit }) {
    function toggleSpanCollapse(spanId: number | string) {
      emit("toggleCollapse", spanId);
    }

    return {
      toggleSpanCollapse,
    };
  },
  components: { SpanBlock },
});
</script>

<style scoped lang="scss">
.spans-container {
  position: relative;
}
</style>
