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
<div class="container">
   
    <div class="button-group">
      <q-btn
        flat
        round
        @click="zoomIn"
        :class="buttonClass"
      >
        <q-icon name="add" />
      </q-btn>
      <div class="separator"></div>
      <q-btn
        flat
        round
        @click="zoomOut"
        :class="buttonClass"
      >
        <q-icon name="remove" />
        
      </q-btn>
      <div v-show="pipelineObj.dirtyFlag" class="warning-text">
      Unsaved changes detected. Click "Save" to preserve your updates.
    </div>
    </div>
    
  </div>

    <VueFlow
    ref="vueFlowRef"
      v-model:nodes="pipelineObj.currentSelectedPipeline.nodes"
      v-model:edges="pipelineObj.currentSelectedPipeline.edges"
      @node-change="onNodeChange"
      @nodes-change="onNodesChange"
      @edges-change="onEdgesChange"
      @connect="onConnect"
      @dragover="onDragOver"
       :zoom-on-scroll="false"
      :zoom-on-pinch="false"

      @dragleave="onDragLeave"
      :edge-types="edgeTypes"

    >
      <DropzoneBackground
        :style="{
          backgroundColor: isDragOver ? '#e7f3ff' : 'transparent',
          transition: 'background-color 0.2s ease',
        }"
      >
        <p v-if="isDragOver">Drop here</p>
      </DropzoneBackground>
      <template #node-input="{ id, data }">
        <CustomNode :id="id" :data="data" io_type="input" />
      </template>
      <template #node-output="{ id, data }">
        <CustomNode :id="id" :data="data" io_type="output" />
      </template>
      <template #node-default="{ id, data }">
        <CustomNode :id="id" :data="data" io_type="default" />
      </template>
    </VueFlow>
    <!-- Add UI elements or buttons to interact with the methods -->
  </div>
</template>

<script>
import { ref, onMounted } from "vue";
import { VueFlow } from "@vue-flow/core";
// import vueFlowConfig from "./vueFlowConfig";
import CustomNode from "./CustomNode.vue";
import DropzoneBackground from "./DropzoneBackground.vue";
import useDragAndDrop from "./useDnD";

/* import the required styles */
import "@vue-flow/core/dist/style.css";

/* import the default theme (optional) */
import "@vue-flow/core/dist/theme-default.css";
import { useStore } from "vuex";

export default {
  components: { VueFlow, CustomNode, DropzoneBackground },
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
    const store = useStore();
const  buttonClass = () => {
      return this.theme === 'dark' ? 'dark-theme' : 'light-theme';
    };
    onMounted(() => {
      if (vueFlowRef.value) {
        vueFlowRef.value.fitView({ padding: 0.1, includeHiddenNodes: true });
      }
    });
    const vueFlowRef = ref(null);
    const zoomIn = () => {
      vueFlowRef.value.zoomIn();
    };

    const zoomOut = () => {
      vueFlowRef.value.zoomOut();
    };

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
      zoomIn,
      zoomOut,
      vueFlowRef,
      buttonClass
    };
  },
};
</script>

<style scoped>
#graph-container {
  width: 1150px;
  height: 100vh;
}
.container {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.warning-text {
  background-color: #ffc107;
  font-weight: bold;
  padding: 8px;
}

.button-group {
  display: flex;
  align-items: center;
  margin-right: 10px;
}

.dark-theme {
  background-color: #333;
  color: #fff;
}

.light-theme {
  background-color: #fff;
  color: #000;
}

.separator {
  width: 1px;
  height: 24px;
  background-color: #ccc;
  margin: 0 8px;
}

q-btn {
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
