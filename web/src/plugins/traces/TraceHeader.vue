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
    :style="{
      height: '30px',
      border:
        store.state.theme === 'dark'
          ? '1px solid #3c3c3c'
          : '1px solid #ececec',
    }"
    style="top: 0; z-index: 999; position: sticky"
    class="flex justify-start items-center header-bg bg-grey-2"
    :class="store.state.theme === 'dark' ? 'bg-grey-9' : 'bg-grey-2'"
  >
    <div
      class="flex justify-start items-center no-wrap row q-px-sm"
      :style="{
        width: splitterWidth + 'px',
      }"
    >
      Operation Name
    </div>
    <div
      class="flex justify-start items-center no-wrap row relative-position"
      :style="{
        width: `calc(100% - ${splitterWidth}px)`,
      }"
    >
      <div class="col-3 text-caption q-pl-xs">
        {{ baseTracePosition.tics[0].label }}
      </div>
      <div class="col-3 text-caption q-pl-xs">
        {{ baseTracePosition.tics[1].label }}
      </div>
      <div class="col-3 text-caption q-pl-xs">
        {{ baseTracePosition.tics[2].label }}
      </div>
      <div class="col-3 text-caption flex justify-between items-center q-px-xs">
        <div>{{ baseTracePosition.tics[3].label }}</div>
        <div>{{ baseTracePosition.tics[4].label }}</div>
      </div>
      <div
        v-for="(tick, index) in baseTracePosition['tics']"
        class="trace-tic"
        :key="tick.value + index"
        :style="{
          position: 'absolute',
          left: tick.left,
          top: '-3px',
          width: '1px',
          backgroundColor: store.state.theme === 'dark' ? '#3c3c3c' : '#cacaca',
          zIndex: index === 0 ? '5' : '1',
          height: '26px',
        }"
      ></div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import { useStore } from "vuex";

export default defineComponent({
  name: "TraceNavbar",
  props: {
    baseTracePosition: {
      type: Object,
      default: () => null,
    },
    spanDimensions: {
      type: Object,
      default: () => {},
    },
    splitterWidth: {
      type: Number,
      default: 0,
    },
  },
  setup() {
    const store = useStore();
    return {
      store,
    };
  },
});
</script>

<style scoped lang="scss">
$toolbarHeight: 50px;
$traceHeaderHeight: 30px;
$traceChartHeight: 250px;

.spans-container {
  position: relative;
}

.collapse-btn {
  width: 10px;
  height: 10px;
}
.trace-tic {
  height: calc(100vh - $toolbarHeight - 5px - $traceChartHeight);
}

.header-bg {
  background-color: color-mix(in srgb, currentColor 5%, transparent);
}
</style>
