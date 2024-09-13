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

<!-- src/components/PipelineFlow.vue -->
<template>
  <div id="graph-container"
class="dnd-flow" @drop="onDrop">
    <VueFlow
      v-model:nodes="pipelineObj.currentSelectedPipeline.nodes"
      v-model:edges="pipelineObj.currentSelectedPipeline.edges"
      @node-change="onNodeChange"
      @nodes-change="onNodesChange"
      @edges-change="onEdgesChange"
      @connect="onConnect"
      @dragover="onDragOver"
      @dragleave="onDragLeave"
    >
      <DropzoneBackground
        :style="{
          backgroundColor: isDragOver ? '#e7f3ff' : 'transparent',
          transition: 'background-color 0.2s ease',
        }"
      >
        <p v-if="isDragOver">Drop here</p>
      </DropzoneBackground>
      <template #node-input="{ id, io_type, data }">
        <CustomNode :id="id" :data="data" io_type="input" />
      </template>
      <template #node-output="{ id, io_type, data }">
        <CustomNode :id="id" :data="data" io_type="output" />
      </template>
      <template #node-default="{ id, io_type, data }">
        <CustomNode :id="id" :data="data" io_type="default" />
      </template>
    </VueFlow>
    <!-- Add UI elements or buttons to interact with the methods -->
  </div>
</template>

<script>
import { ref } from "vue";
import { VueFlow } from "@vue-flow/core";
// import vueFlowConfig from "./vueFlowConfig";
import CustomNode from "./CustomNode.vue";
import CustomQuery from "./CustomQuery.vue";
import DropzoneBackground from "./DropzoneBackground.vue";
import useDragAndDrop from "./useDnD";

/* import the required styles */
import "@vue-flow/core/dist/style.css";

/* import the default theme (optional) */
import "@vue-flow/core/dist/theme-default.css";

export default {
  components: { VueFlow, CustomNode, DropzoneBackground, CustomQuery },
  setup() {
    const {
      onDragOver,
      onDrop,
      onDragLeave,
      isDragOver,
      onNodeChange,
      onNodesChange,
      onEdgesChange,
      onConnect,
      validateConnection,
      pipelineObj,
    } = useDragAndDrop();

    return {
      pipelineObj,
      onDragOver,
      onDrop,
      onDragLeave,
      isDragOver,
      onNodeChange,
      onNodesChange,
      onEdgesChange,
      onConnect,
      validateConnection,
    };
  },
};
</script>

<style scoped>
#graph-container {
  width: 1150px;
  height: 1000px;
}
</style>
