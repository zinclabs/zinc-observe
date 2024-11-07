
/* Copyright 2023 OpenObserve Inc.

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
*/

import { b64EncodeUnicode, getUUID } from "@/utils/zincutils";
import { useVueFlow,MarkerType  } from "@vue-flow/core";
import { watch, reactive ,computed , ref} from "vue";
const functionImage = getImageURL("images/pipeline/function.svg");
const streamImage = getImageURL("images/pipeline/stream.svg");
const streamOutputImage = getImageURL("images/pipeline/outputStream.svg");
const streamRouteImage = getImageURL("images/pipeline/route.svg");
const conditionImage = getImageURL("images/pipeline/condition.svg");
const queryImage = getImageURL("images/pipeline/query.svg");
import { getImageURL } from "@/utils/zincutils";
import { Notify , useQuasar} from "quasar";




const dialogObj = {
  show: false,
  name: "",
  title: "",
  message: "",
  data: null,
};

const defaultPipelineObj = {
  name: "",
  description: "",
  source: {
    source_type: "realtime",
  },
  nodes:<any> [],
  edges:<any> [],
  org: "",
  
};

const defaultObject = {
  pipelineDirectionTopBottom: false,
  dirtyFlag: false,
  isEditPipeline: false,
  isEditNode: false,
  nodesChange:false,
  edgesChange:false,
  draggedNode:<any> null,
  isDragOver: false,
  isDragging: false,
  hasInputNode: false,
  currentSelectedNodeID: "",
  currentSelectedNodeData : <any> {
    stream_type: "logs",
    stream_name: "",
    data: {},
    type:"",
  },
  dialog: dialogObj,
  nodeTypes: [],
  currentSelectedPipeline: defaultPipelineObj,
  pipelineWithoutChange: defaultPipelineObj,
  functions: {},
  previousNodeOptions:<any>[],
  userSelectedNode:<any>{},
  userClickedNode : <any>{},
};

const pipelineObj = reactive(Object.assign({}, defaultObject));

export default function useDragAndDrop() {
  const $q = useQuasar();

  const { screenToFlowCoordinate, onNodesInitialized, updateNode, addEdges  } =
    useVueFlow();

    watch(
      () => pipelineObj.isDragging,
      (dragging) => {
        document.body.style.userSelect = dragging ? "none" : "";
      }
    );

  function hasInputNodeFn() {
    pipelineObj.hasInputNode = pipelineObj.currentSelectedPipeline.nodes.some(
      (node :any) => node.io_type === "input",
    );
  }

  function onDragStart(event:any, node:any) {
 
    if (event.dataTransfer) {
      event.dataTransfer.setData("application/vueflow", node.io_type);
      event.dataTransfer.effectAllowed = "move";
    }

    pipelineObj.draggedNode = node;
    pipelineObj.isDragging = true;
    pipelineObj.currentSelectedNodeData = null;


    document.addEventListener("drop", onDragEnd);
  }

  /**
   * Handles the drag over event.
   *
   * @param {DragEvent} event
   */
  function onDragOver(event:any) {
    event.preventDefault();

    if (pipelineObj.draggedNode?.io_type) {
      pipelineObj.isDragOver = true;

      if (event.dataTransfer) {
        event.dataTransfer.dropEffect = "move";
      }
    }
  }

  function onDragLeave() {
    pipelineObj.isDragOver = false;
  }

  function onDragEnd() {
    pipelineObj.isDragging = false;
    pipelineObj.isDragOver = false;
    document.removeEventListener("drop", onDragEnd);
  }

  /**
   * Handles the drop event.
   *
   * @param {DragEvent} event
   */
  function onDrop(event:any ,offSet:any = {x:0,y:0}) {
    const position = screenToFlowCoordinate({
      x: event.clientX + offSet.x,
      y: event.clientY + offSet.y,
    });

    const nodeId = getUUID();

    const newNode = {
      id: nodeId,
      type: pipelineObj.draggedNode.io_type || "default",
      io_type: pipelineObj.draggedNode.io_type || "default",
      position,
      data: { label: nodeId, node_type: pipelineObj.draggedNode.subtype },
    };

    /**
     * Align node position after drop, so it's centered to the mouse
     *
     * We can hook into events even in a callback, and we can remove the event listener after it's been called.
     */
    const { off } = onNodesInitialized(() => {
      updateNode(nodeId, (node) => ({
        position: {
          x: node.position.x - node.dimensions.width / 2,
          y: node.position.y - node.dimensions.height / 2,
        },
      }));

      off();
    });

    pipelineObj.currentSelectedNodeData = newNode;
    pipelineObj.dialog.name = newNode.data.node_type;
    pipelineObj.dialog.show = true;
    pipelineObj.isEditNode = false;
  }

  function onNodeChange(changes:any) {

    console.log("Node change", changes);
  }

  function onNodesChange(changes:any) {
    hasInputNodeFn();

  }

  function onEdgesChange(changes:any) {
    if(pipelineObj.isEditPipeline == true ){
      pipelineObj.dirtyFlag = true;

    }
    if(changes.length > 0){
      pipelineObj.edgesChange = true;
    }
    console.log("Edges change", changes);
  }

  function onConnect(connection:any) {
    if(connection.sourceHandle === "input" && connection.targetHandle === "input" || connection.sourceHandle === "output" && connection.targetHandle === "output"){
      $q.notify({
        message: "Same type of edges / nodes cannot be connected",
        color: "negative",
        position: "bottom",
        timeout: 3000,
      
    });
      return;
    }
    const isConnectionAlreadyAvailable = pipelineObj.currentSelectedPipeline.edges.find((previousEdge:any) => previousEdge.targetNode.id === connection.target);
    if(isConnectionAlreadyAvailable){
      $q.notify({
        message: "Only one Incoming Edge to the node is allowed",
        color: "negative",
        position: "bottom",
        timeout: 3000,
      
    });
      return;
    }

    const isCycle = detectCycle(pipelineObj.currentSelectedPipeline.edges, connection);
    if(isCycle){
      console.log($q,"qusar")
      $q.notify({
        message: "Adding this edge will create a cycle in the pipeline",
        color: "negative",
        position: "bottom",
        timeout: 3000,
      
    });
    return;
    }
    // Add new connection (edge) to edges array
    const newEdge = {
      id: `e${connection.source}-${connection.target}`,
      source: connection.source,
      target: connection.target,
      
      markerEnd: {
        type: MarkerType.ArrowClosed,
        width: 20,  // Increase arrow width
        height: 20, // Increase arrow height

      },
      type: 'button',      
      style:{
        strokeWidth: 2,
      },
      animated:true,


    };

    pipelineObj.currentSelectedPipeline.edges = [
      ...pipelineObj.currentSelectedPipeline.edges,
      newEdge,
    ]; // Update edges state
  }

  const detectCycle = (edges:any, newConnection:any) => {
    // Step 1: Build the adjacency list from the current edges
    const graph = <any> {};
    edges.forEach((edge:any) => {
        if (!graph[edge.sourceNode.id]) graph[edge.sourceNode.id] = [];
        graph[edge.sourceNode.id].push(edge.targetNode.id);
    });

    // Add the new connection to the adjacency list
    if (!graph[newConnection.source]) graph[newConnection.source] = [];
    graph[newConnection.source].push(newConnection.target);

    // Step 2: Define the DFS function to detect cycles
    const dfs = (node:any, visited:any, recStack:any) => {
        if (!visited.has(node)) {
            // Mark the node as visited and add to recursion stack
            visited.add(node);
            recStack.add(node);

            // Traverse all neighbors (adjacent nodes)
            const neighbors = graph[node] || [];
            for (let neighbor of neighbors) {
                if (!visited.has(neighbor) && dfs(neighbor, visited, recStack)) {
                    return true; // Cycle found
                } else if (recStack.has(neighbor)) {
                    return true; // Cycle found in recursion stack
                }
            }
        }

        // Remove the node from the recursion stack after exploration
        recStack.delete(node);
        return false;
    };

    // Step 3: Run DFS from the source node of the new connection
    const visited = new Set();
    const recStack = new Set();

    if (dfs(newConnection.source, visited, recStack)) {
        return true; // Cycle detected
    }

    return false; // No cycle detected
};

  function validateConnection({ source, target, sourceHandle, targetHandle }:any) {
    // Example validation rules
    const sourceNode = pipelineObj.currentSelectedPipeline.nodes.find(
      (node:any) => node.id === source,
    );
    const targetNode = pipelineObj.currentSelectedPipeline.nodes.find(
      (node:any) => node.id === target,
    );

    // Input-only node (cannot be the source of a connection)
    if (sourceNode.type === "input") {
      return false;
    }

    // Output-only node (cannot be the target of a connection)
    if (targetNode.type === "output") {
      return false;
    }

    return true; // Allow connection for 'both' nodes
  }

  function addNode(newNode:any) {

    if(pipelineObj.isEditNode){
      if(pipelineObj.userSelectedNode == null){
        pipelineObj.currentSelectedPipeline.edges = pipelineObj.currentSelectedPipeline.edges.filter((edge:any) => edge.targetNode.id !== pipelineObj.currentSelectedNodeData.id);
      }
    }
    if(pipelineObj.userClickedNode && pipelineObj.currentSelectedNodeData.id && !pipelineObj.userSelectedNode?.id ){



      const newEdge = {
        id: `e${pipelineObj.userClickedNode}-${pipelineObj.currentSelectedNodeData.id}`,
        source:  pipelineObj.userClickedNode,
        target:pipelineObj.currentSelectedNodeData.id,
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 20,  // Increase arrow width
          height: 20, // Increase arrow height
        },
        type: 'button',
        
        style:{
          strokeWidth: 2,
        },
        animated:true,


      };
      //not required because we create new nodes every time we click on shortcuts
      // const isCycle = detectCycle(pipelineObj.currentSelectedPipeline.edges, newEdge);
      // if(isCycle){
      //   $q.notify({
      //     message: "Adding this edge will create a cycle in the pipeline",
      //     color: "negative",
      //     position: "bottom",
      //     timeout: 3000,
        
      // });
      // return;
      // }
      pipelineObj.currentSelectedPipeline.edges = [
        ...pipelineObj.currentSelectedPipeline.edges,
        newEdge,
      ];
      pipelineObj.userClickedNode = {};
      pipelineObj.userSelectedNode = {};
    }

if(pipelineObj.currentSelectedNodeData.id && pipelineObj.userSelectedNode?.id){
  const newEdge = {
    id: `e${pipelineObj.userSelectedNode.id}-${pipelineObj.currentSelectedNodeData.id}`,
    source:  pipelineObj.userSelectedNode.id,
    target:pipelineObj.currentSelectedNodeData.id,
      markerEnd: {
        type: MarkerType.ArrowClosed,
        width: 20,  // Increase arrow width
        height: 20, // Increase arrow height
      },
    type: 'button',
    
    style:{
      strokeWidth: 2,
    },
    animated:true,



  };


  

  const isCycle = detectCycle(pipelineObj.currentSelectedPipeline.edges, newEdge);
  if(isCycle){
    $q.notify({
      message: "Adding this edge will create a cycle in the pipeline",
      color: "negative",
      position: "bottom",
      timeout: 3000,
    
  });
  return;
  }
  const targetEdgeIfExist = pipelineObj.currentSelectedPipeline.edges.find((previousEdge:any) => previousEdge.targetNode.id === pipelineObj.currentSelectedNodeData.id);

    // If targetEdgeIfExist is found, filter it out from the edges array

  if (targetEdgeIfExist) {
    pipelineObj.currentSelectedPipeline.edges = pipelineObj.currentSelectedPipeline.edges.filter(
      (edge:any) => edge.targetNode.id !== targetEdgeIfExist.targetNode.id
    );
  }
  // Add the new edge to the edges array
  pipelineObj.currentSelectedPipeline.edges = [
    ...pipelineObj.currentSelectedPipeline.edges,
    newEdge,
  ];
  // Update edges state


}

    if(pipelineObj.isEditPipeline == true ){
      pipelineObj.dirtyFlag = true;
      pipelineObj.nodesChange = true;
    }
    pipelineObj.previousNodeOptions.push(pipelineObj.currentSelectedNodeData); 
    let currentSelectedNode = pipelineObj.currentSelectedNodeData;
    if (pipelineObj.isEditNode == true && currentSelectedNode.id != "") {
      if (currentSelectedNode) {
        currentSelectedNode.data = { ...currentSelectedNode.data, ...newNode };

        //find the index from pipelineObj.currentSelectedPipeline.nodes based on id
        const index = pipelineObj.currentSelectedPipeline.nodes.findIndex(
          (node:any) => node.id === currentSelectedNode.id,
        );

        pipelineObj.currentSelectedPipeline.nodes[index] = currentSelectedNode;
      }
    } else {
      if (currentSelectedNode) {
        currentSelectedNode.data = { ...currentSelectedNode.data, ...newNode };
        pipelineObj.currentSelectedPipeline.nodes = [
          ...pipelineObj.currentSelectedPipeline.nodes,
          currentSelectedNode,
        ];
      }
    }

    
    pipelineObj.isEditNode = false;
    // pipelineObj.currentSelectedNodeData = dialogObj;
    pipelineObj.userClickedNode = {};
    pipelineObj.userSelectedNode = {};
    console.log(pipelineObj.currentSelectedNodeData,"pipelineObj.currentSelectedPipeline")
  }



  const formattedOptions = computed(() => { 
    const groupedOptions :any = {};
    let conditionCounter : number = 1;
  
    // Find the index of the current selected node, if it exists
    const currentIndex = pipelineObj.currentSelectedPipeline.nodes.findIndex(
      (node:any) => node.id === pipelineObj.currentSelectedNodeData?.id
    );
  
    const nodesToProcess = currentIndex !== -1 
    ? pipelineObj.currentSelectedPipeline.nodes.filter((_:any, index:any) => index !== currentIndex) 
    : pipelineObj.currentSelectedPipeline.nodes;

    // const nodesToProcess = pipelineObj.currentSelectedPipeline.nodes;
  
    // Map and group nodes by io_type
    nodesToProcess.forEach((node:any) => {
      // Skip nodes with io_type 'output'
      if (node.io_type === 'output') {
        return;
      }
  
      let label = node.data.label;
      let icon = "";
      let color = "red";
  
      // Determine label, icon, and color based on node properties
      if (node.io_type === 'input') {
        label = node.io_type;
        color = "#c8d6f5";
        icon = streamImage;
      } else if (node.type === 'default' && node.data.node_type === 'function') {
        label = node.data.name;
        color = "#efefef";
        icon = functionImage;
      } else if (node.type === 'default' && node.data.node_type === 'condition') {
        label = `Condition ${conditionCounter++}`;
        color = "#efefef";
        icon = conditionImage;
      }
  
      // Create the option object
      const option = {
        id: node.id,
        label,
        node_type: node.data.node_type,
        io_type: node.io_type,
        icon,
        color,
      };
  
      // Group by io_type
      if (!groupedOptions[node.io_type]) {
        groupedOptions[node.io_type] = [];
      }
      groupedOptions[node.io_type].push(option);
    });
  
    // Helper function to map io_type to custom group names
    const getGroupName = (io_type:any) => {
      switch (io_type) {
        case 'input':
          return 'Source';
        case 'default':
          return 'Transform';
        default:
          return io_type; // fallback to the original io_type if not matched
      }
    };
  
    // Flatten the grouped options into an array with custom group headers
    return Object.entries(groupedOptions).flatMap(([group, options]:any) => {
      return [
        { label: getGroupName(group), isGroup: true }, // Custom Group header
        ...options,
      ];
    });
  });
  
  
  
 
  const filteredOptions = ref(formattedOptions.value);

  const filterOptions = (val:any, update:any) => {
    if (val === '') {
      filteredOptions.value = formattedOptions.value;
    } else {
      const filterFn = (option:any) =>
        option.label.toLowerCase().includes(val.toLowerCase()) ||
        (option.node_type && option.node_type.toLowerCase().includes(val.toLowerCase()));

      const filtered : any = [];
      formattedOptions.value.forEach(option => {
        if (option.isGroup) {
          const groupItems = formattedOptions.value.filter(opt => opt.io_type === option.label && filterFn(opt));
          if (groupItems.length > 0) {
            filtered.push(option);
            filtered.push(...groupItems);
          }
        } else if (filterFn(option)) {
          filtered.push(option);
        }
      });

      filteredOptions.value = filtered;
    }

    update(() => filteredOptions.value);
  };

  const getParentNode = (nodeId:any) => {
    const edge = pipelineObj.currentSelectedPipeline.edges.find(
      (edge:any) => edge.target === nodeId,
    );
    if(edge){
      return pipelineObj.currentSelectedPipeline.nodes.find(
        (node:any) => node.id === edge.source,
      );
    }

  }

  const currentSelectedParentNode = () =>{

    const parentNode = getParentNode(pipelineObj.currentSelectedNodeID);
    if(parentNode){
      const currentParentNode = formattedOptions.value.find(
        (node)=> node?.id === parentNode.id
      )
      if(currentParentNode){
        return currentParentNode;
      }
    }

    else {
      return null;
    }
  }


  
  

  function editNode(updatedNode:any) {
    console.log(updateNode,"update Node")
    const index = pipelineObj.currentSelectedPipeline.nodes.findIndex(
      (node:any) => node.id === updatedNode.id,
    );
    if (index !== -1) {
      pipelineObj.currentSelectedPipeline.nodes[index] = {
        ...pipelineObj.currentSelectedPipeline.nodes[index],
        ...updatedNode,
      };
    }
  }
  const comparePipelinesById = (pipeline1:any, pipeline2:any) => {
    const compareIds = (items1:any, items2:any) => {
      const extractAndSortIds = (items:any) =>
        items.map((item:any) => item.id).sort();

      const ids1 = extractAndSortIds(items1);
      const ids2 = extractAndSortIds(items2);
      console.log(ids1,ids2)
  
      return JSON.stringify(ids1) === JSON.stringify(ids2);
    };
    const nodesEqual = compareIds(pipeline1.nodes, pipeline2.nodes);
  
    return nodesEqual;
  };

  function deletePipelineNode(nodeId:any) {
    pipelineObj.currentSelectedPipeline.nodes =
      pipelineObj.currentSelectedPipeline.nodes.filter(
        (node:any) => node.id !== nodeId,
      );

      pipelineObj.previousNodeOptions =
      pipelineObj.previousNodeOptions.filter(
        (node:any) => node.id !== nodeId,
      );

    pipelineObj.currentSelectedPipeline.edges =
      pipelineObj.currentSelectedPipeline.edges.filter(
        (edge:any) => edge.source !== nodeId && edge.target !== nodeId,
      );
    pipelineObj.currentSelectedNodeData = null;
    hasInputNodeFn();

    const arePipelinesEqualById = comparePipelinesById(
      pipelineObj.currentSelectedPipeline,
      pipelineObj.pipelineWithoutChange
    );
    if(arePipelinesEqualById == true && pipelineObj.edgesChange == false && pipelineObj.isEditPipeline == true){
      pipelineObj.dirtyFlag = false;
    }
    if(arePipelinesEqualById == false && pipelineObj.isEditPipeline == true){
      pipelineObj.dirtyFlag = true;
    }
    
    console.log(pipelineObj.currentSelectedPipeline,"edges")
  }

  const resetPipelineData = () => {
    pipelineObj.currentSelectedPipeline = JSON.parse(JSON.stringify(defaultPipelineObj));
    pipelineObj.currentSelectedNodeData = JSON.parse(JSON.stringify(dialogObj));
    pipelineObj.isEditPipeline = false;
    pipelineObj.isEditNode = false;
    pipelineObj.dirtyFlag = false;
    pipelineObj.hasInputNode = false;
    pipelineObj.draggedNode = null;
  };
 

  return {
    pipelineObj,
    onDragStart,
    onDragLeave,
    onDragOver,
    onDrop,
    onNodeChange,
    onNodesChange,
    onEdgesChange,
    onConnect,
    validateConnection,
    addNode,
    editNode,
    deletePipelineNode,
    resetPipelineData,
    comparePipelinesById,
    formattedOptions,
    filteredOptions,
    filterOptions,
    getParentNode,
    currentSelectedParentNode,
  };
}
