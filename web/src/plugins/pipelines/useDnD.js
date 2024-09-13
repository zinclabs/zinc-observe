/* Copyright 2023 Zinc Labs Inc.

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

import { getUUID } from "@/utils/zincutils";
import { useVueFlow } from "@vue-flow/core";
import { watch, reactive } from "vue";

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
    source_type: "realtime"
  },
  nodes: [],
  edges: [],
};

const defaultObject = {
  isEditMode: false,
  draggedNode: null,
  isDragOver: false,
  isDragging: false,
  hasInputNode: false,
  currentSelectedNode: null,
  dialog: dialogObj,
  nodeTypes: null,
  currentSelectedPipeline: defaultPipelineObj,
};

const pipelineObj = reactive(Object.assign({}, defaultObject));

export default function useDragAndDrop() {
  const { screenToFlowCoordinate, onNodesInitialized, updateNode } =
    useVueFlow();

  watch(pipelineObj.isDragging, (dragging) => {
    document.body.style.userSelect = dragging ? "none" : "";
  });

  function hasInputNodeFn() {
    pipelineObj.hasInputNode = pipelineObj.currentSelectedPipeline.nodes.some(
      (node) => node.io_type === "input",
    );
  }

  function onDragStart(event, node) {
    if (event.dataTransfer) {
      event.dataTransfer.setData("application/vueflow", node.io_type);
      event.dataTransfer.effectAllowed = "move";
    }

    pipelineObj.draggedNode = node;
    pipelineObj.isDragging = true;
    pipelineObj.currentSelectedNode = null;

    document.addEventListener("drop", onDragEnd);
  }

  /**
   * Handles the drag over event.
   *
   * @param {DragEvent} event
   */
  function onDragOver(event) {
    event.preventDefault();

    if (pipelineObj.draggedNode.io_type) {
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
  function onDrop(event) {
    const position = screenToFlowCoordinate({
      x: event.clientX,
      y: event.clientY,
    });

    const nodeId = getUUID();

    console.log("pipelineObj.draggedNode", pipelineObj);
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

    pipelineObj.currentSelectedNode = newNode;
    pipelineObj.dialog.name = newNode.data.node_type;
    pipelineObj.dialog.show = true;
  }

  function onNodeChange(changes) {
    console.log("Node change", changes);
  }

  function onNodesChange(changes) {
    hasInputNodeFn();
    console.log("Nodes change", changes);
  }

  function onEdgesChange(changes) {
    console.log("Edges change", changes);
  }

  function onConnect(connection) {
    // Add new connection (edge) to edges array
    const newEdge = {
      id: `e${connection.source}-${connection.target}`,
      source: connection.source,
      target: connection.target,
    };
    pipelineObj.currentSelectedPipeline.edges = [...pipelineObj.currentSelectedPipeline.edges, newEdge]; // Update edges state
  }

  function validateConnection({ source, target, sourceHandle, targetHandle }) {
    // Example validation rules
    const sourceNode = pipelineObj.currentSelectedPipeline.nodes.find((node) => node.id === source);
    const targetNode = pipelineObj.currentSelectedPipeline.nodes.find((node) => node.id === target);

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

  function addNode(newNode) {
    let currentSelectedNode = pipelineObj.currentSelectedNode;
    if (currentSelectedNode) {
      currentSelectedNode.data = { ...currentSelectedNode.data, ...newNode };
      pipelineObj.currentSelectedPipeline.nodes = [...pipelineObj.currentSelectedPipeline.nodes, currentSelectedNode];
    }

    pipelineObj.currentSelectedNode = dialogObj;
  }

  function editNode(updatedNode) {
    const index = pipelineObj.currentSelectedPipeline.nodes.findIndex(
      (node) => node.id === updatedNode.id,
    );
    if (index !== -1) {
      pipelineObj.currentSelectedPipeline.nodes[index] = {
        ...pipelineObj.currentSelectedPipeline.nodes[index],
        ...updatedNode,
      };
    }
  }

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
  };
}
