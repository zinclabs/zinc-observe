<template>
    <div class="pipeline-view-tooltip container">
      <VueFlow 
      ref="vueFlowRef"
        v-model:nodes="lockedNodes"
        v-model:edges="pipelineObj.currentSelectedPipeline.edges"
      :options="{ readOnly: true }"
      :default-viewport="{ zoom: 0 }"

       >
       <DropzoneBackground
        :style="{
          backgroundColor:  '#e7f3ff',
          transition: 'background-color 0.2s ease',
        }"
      >
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
    </div>
  </template>
  
  <script>
  import { getImageURL } from "@/utils/zincutils";
import DropzoneBackground from "@/plugins/pipelines/DropzoneBackground.vue";
  import { defineComponent, computed, onBeforeMount } from 'vue';
  import { VueFlow } from "@vue-flow/core";
  import { ref, onMounted, onActivated, watch } from "vue";
import { ControlButton, Controls } from '@vue-flow/controls'
import CustomNode from '@/plugins/pipelines/CustomNode.vue';
/* import the required styles */
import "@vue-flow/core/dist/style.css";
import '@vue-flow/controls/dist/style.css'
import useDragAndDrop from '@/plugins/pipelines/useDnD';
const streamImage = getImageURL("images/pipeline/stream.svg");

const functionImage = getImageURL("images/pipeline/function.svg");
const streamOutputImage = getImageURL("images/pipeline/outputStream.svg");
const streamRouteImage = getImageURL("images/pipeline/route.svg");
const conditionImage = getImageURL("images/pipeline/condition.svg");
const queryImage = getImageURL("images/pipeline/query.svg");
  
  export default defineComponent({
    props: {
      pipeline: Object
    },
    components: { VueFlow, CustomNode, Controls,ControlButton,DropzoneBackground },
    setup(props) {
      const {
      pipelineObj,
    } = useDragAndDrop();
      const vueFlowRef = ref(null);
      // Computed properties for nodes and edges
      const lockedNodes = computed(() => {
        return props.pipeline.nodes.map(node => ({
          ...node,
          type: node.io_type
        }));
      });
  
      const edges = computed(() => {
        return props.pipeline.edges || [];
      });

      onMounted(async () => {

        
        pipelineObj.nodeTypes = [
  {
    label: "Source",
    icon: "input",
    isSectionHeader: true,
  },
  {
    label: "Stream",
    subtype: "stream",
    io_type: "input",
    icon: "img:" + streamImage,
    tooltip: "Source: Stream Node",
    isSectionHeader: false,
  },
  {
    label: "Query",
    subtype: "query",
    io_type: "input",
    icon: "img:" + queryImage,
    tooltip: "Source: Query Node",
    isSectionHeader: false,
  },
  {
    label: "Transform",
    icon: "processing",
    isSectionHeader: true,
  },
  {
    label: "Function",
    subtype: "function",
    io_type: "default",
    icon: "img:" + functionImage,
    tooltip: "Function Node",
    isSectionHeader: false,
  },
  {
    label: "Conditions",
    subtype: "condition",
    io_type: "default",
    icon: "img:" + conditionImage,
    tooltip: "Condition Node",
    isSectionHeader: false,
  },
  {
    label: "Destination",
    icon: "input",
    isSectionHeader: true,
  },
  {
    label: "Stream",
    subtype: "stream",
    io_type: "output",
    icon: "img:" + streamOutputImage,
    tooltip: "Destination: Stream Node",
    isSectionHeader: false,
  },
];
        pipelineObj.currentSelectedPipeline = props.pipeline;

        setTimeout(() => {
          if(vueFlowRef.value)
          vueFlowRef.value.fitView({ padding: 0.1});
        }, 100);

      })
  
      // Return the computed properties
      return {
        lockedNodes,
        edges,
        vueFlowRef,
        pipelineObj,
        streamImage,
      };

    }
  });
  </script>
  
  <style >
  .pipeline-view-tooltip {
    width: 500px; /* Adjust the width */
    height: 300px; /* Adjust the height */
    overflow: auto;
  }

 
.o2vf_node {
  width: auto;

  .vue-flow__node {
    padding: 0px;
    width: auto;
  }

  .o2vf_node_input,
  .vue-flow__node-input {
    background-color: #c8d6f5;
    border-color: 1px solid #2c6b2f;
    color: black;
  }

  .o2vf_node_output,
  .vue-flow__node-output {
    background-color: #8fd4b8;
    border-color: 1px solid #3b6f3f;
    color: black;
  }

  .o2vf_node_default,
  .vue-flow__node-default {
    background-color: #efefef;
    border-color: 1px solid #171e25;
    color: black;
  }
}


  </style>

  
  