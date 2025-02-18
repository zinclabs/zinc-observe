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

<!-- src/components/PipelineFlow.vue -->
<template>
  <div data-test="pipeline-flow-container" class="container">
     <div data-test="pipeline-flow-unsaved-changes-warning-text" v-show="pipelineObj.dirtyFlag" class="warning-text flex  items-center q-py-xs q-px-sm q-mr-md ">
      <q-icon name="info" class="q-mr-xs " size="16px" />
     Unsaved changes detected. Click "Save" to preserve your updates.
   </div>
   
 </div>

    <VueFlow
    @drop="onDrop"
    ref="vueFlowRef"
      v-model:nodes="pipelineObj.currentSelectedPipeline.nodes"
      v-model:edges="pipelineObj.currentSelectedPipeline.edges"
      @node-change="onNodeChange"
      @nodes-change="onNodesChange"
      @edges-change="onEdgesChange"
      @connect="onConnect"
      @dragover="onDragOver"
      :default-viewport="{ zoom: 1.5 }"
      :min-zoom="0.2"
      :max-zoom="4"
      @dragleave="onDragLeave"
      class="basic-flow"
      
    >

    <!-- <template #edge-button="buttonEdgeProps">
      <EdgeWithButton
        :id="buttonEdgeProps.id"
        :source-x="buttonEdgeProps.sourceX"
        :source-y="buttonEdgeProps.sourceY"
        :target-x="buttonEdgeProps.targetX"
        :target-y="buttonEdgeProps.targetY"
        :source-position="buttonEdgeProps.sourcePosition"
        :target-position="buttonEdgeProps.targetPosition"
        :marker-end="buttonEdgeProps.markerEnd"
        :style="buttonEdgeProps.style"
      />
    </template> -->
    <template #edge-custom="customEdgeProps">
      <CustomEdge
        :id="customEdgeProps.id"
        :source-x="customEdgeProps.sourceX"
        :source-y="customEdgeProps.sourceY"
        :target-x="customEdgeProps.targetX"
        :target-y="customEdgeProps.targetY"
        :source-position="customEdgeProps.sourcePosition"
        :target-position="customEdgeProps.targetPosition"
        :data="customEdgeProps.data"
        :marker-end="customEdgeProps.markerEnd"
        :style="customEdgeProps.style"
        :is-in-view = false
      />
    </template>
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
      <Controls 
      :showInteractive=false

      class="controls-grp"
        position="top-left">
    </Controls>
    </VueFlow>
    <div v-if="isCanvasEmpty" class="empty-text">
      Drag and drop nodes here
    </div>
    <!-- Add UI elements or buttons to interact with the methods -->
</template>

<script>
import { ref, onMounted, onActivated, watch,computed } from "vue";
import { VueFlow, useVueFlow } from "@vue-flow/core";
import { ControlButton, Controls } from '@vue-flow/controls'
// import vueFlowConfig from "./vueFlowConfig";
import CustomNode from "./CustomNode.vue";
import CustomEdge from "./CustomEdge.vue";
import DropzoneBackground from "./DropzoneBackground.vue";
import useDragAndDrop from "./useDnD";
import EdgeWithButton from "./EdgeWithButton.vue";

/* import the required styles */

import { useStore } from "vuex";
const { onInit } = useVueFlow();

export default {
  components: { VueFlow, CustomNode, DropzoneBackground, Controls,ControlButton,EdgeWithButton,CustomEdge
   },
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

    const vueFlowRef = ref(null);
    const isCanvasEmpty = computed(() => pipelineObj.currentSelectedPipeline.nodes.length === 0);

    const { setViewport } = useVueFlow()



    watch(() => pipelineObj.currentSelectedPipeline, (newVal, oldVal) => {
          if(pipelineObj.dirtyFlag){
            pipelineObj.dirtyFlag = false;
          }
        });
    onMounted(async () => {
        setTimeout(() => {
          if (vueFlowRef.value && pipelineObj.currentSelectedPipeline.nodes.length > 4) {
            vueFlowRef.value.fitView({ padding: 0.1});
          }
          else if(vueFlowRef.value){
            vueFlowRef.value.fitView({ padding: 1});
          }
        }, 100);
      });

    
function resetTransform() {
  setViewport({ x: 0, y: 0, zoom: 1 })
}
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
      resetTransform,
      isCanvasEmpty,
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

.warning-text {
  color: #F5A623;
  border: 1px solid #F5A623;
  border-radius: 2px ;
}

  .empty-text {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  color: #888; /* Light text color */
  font-size: 1.5em;
  text-align: center;
  pointer-events: none;
  z-index: 10;
}




</style>
