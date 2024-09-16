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

<script setup>
import {
  Handle,
  Position,
  useHandleConnections,
  useNodesData,
} from "@vue-flow/core";

import useDragAndDrop from "./useDnD";

const props = defineProps({
  id: {
    type: String,
  },
  data: {
    type: Object,
  },
  io_type: {
    type: String,
  },
});

function getIcon(searchTerm, ioType) {
  const node = this.pipelineObj.nodeTypes.find(
    (node) => node.subtype === searchTerm && node.io_type === ioType
  );
  return node ? node.icon : undefined;
}
function getTooltip(searchTerm) {
  const node = this.pipelineObj.nodeTypes.find(
    (node) => node.subtype === searchTerm,
  );
  return node ? node.tooltip : "";
}
const { onDragStart, pipelineObj } = useDragAndDrop();

const connections = useHandleConnections({
  useHandleConnections: true,
});

const nodesData = useNodesData(() => connections.value[0]?.source);
</script>

<template>
  <!-- Input Handle (Target) -->
  <div class="o2vf_node">
    <Handle
      v-if="io_type == 'output' || io_type === 'default'"
      id="input"
      type="target"
      position="top"
      :style="{ filter: 'invert(100%)' }"
    />
    <div
      v-if="data.node_type == 'function'"
      :class="`o2vf_node_${type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
        width: 170px;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
      "
    >
      <div class="icon-container" style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon :name="getIcon(data.node_type, io_type)"
size="1em" class="q-ma-sm" />
      </div>

      <!-- Separator -->
      <q-separator vertical class="q-mr-sm" />

      <!-- Label -->
      <div class="container">
        <div
          class="row"
          align="left"
          style="
            text-align: left;
            text-wrap: wrap;
            width: auto;
            text-overflow: ellipsis;
          "
        >
          {{ data.name }}
        </div>
        <div class="row">
          <div style="text-transform: capitalize">
            Run
            {{
              data.afterFlattening ? "After Flattening" : "Before Flattening"
            }}
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="data.node_type == 'stream'"
      :class="`o2vf_node_${io_type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
        width: 170px;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
      "
    >
      <div class="icon-container" style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon :name="getIcon(data.node_type, io_type)"
size="1em" class="q-ma-sm" />
      </div>

      <!-- Separator -->
      <q-separator vertical class="q-mr-sm" />

      <!-- Label -->
      <div class="container">
        <div
          class="row"
          style="
            text-align: left;
            text-wrap: wrap;
            width: auto;
            text-overflow: ellipsis;
          "
        >
          {{ data.stream_type }} - {{ data.stream_name }}
        </div>
      </div>
    </div>

    <div
      v-if="data.node_type == 'query'"
      :class="`o2vf_node_${io_type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
        width: 170px;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
      "
    >
      <div class="icon-container" style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon :name="getIcon(data.node_type, io_type)"
size="1em" class="q-ma-sm" />
      </div>

      <!-- Separator -->
      <q-separator vertical class="q-mr-sm" />

      <!-- Label -->
      <div class="container">
        <div
          class="row"
          style="
            text-align: left;
            text-wrap: wrap;
            width: auto;
            text-overflow: ellipsis;
          "
        >
          {{ data.stream_type }} - {{ data.stream_name }}
        </div>
      </div>
    </div>
    <Handle
      v-if="io_type === 'input' || io_type === 'default'"
      id="output"
      type="source"
      position="bottom"
      :style="{ filter: 'invert(100%)' }"
    />
  </div>
</template>

<style lang="scss">
.vue-flow__node-custom {
  padding: 10px;
  border-radius: 3px;
  width: 150px;
  font-size: 12px;
  text-align: center;
  border-width: 1px;
  border-style: solid;
  color: var(--vf-node-text);
  background-color: var(--vf-node-bg);
  border-color: var(--vf-node-color);
}
</style>
