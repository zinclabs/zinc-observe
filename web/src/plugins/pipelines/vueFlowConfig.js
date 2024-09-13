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

// src/vueFlowConfig.js
// import { createVueFlow } from "@vue-flow/core";
import CustomNode from "./CustomNode.vue"; // Adjust the path to your custom node

// const initialNodes = [
//   {
//     id: "custom-node",
//     type: "custom", // Use 'custom' for custom nodes
//     component: CustomNode, // Register your custom component here
//     position: { x: 100, y: 100 },
//     data: {
//       /* Custom data if needed */
//     },
//   },
//   // Add other built-in node types as needed
// ];

// Initial configuration for nodes and edges
// const initialNodes = [
//   { id: "1", type: "input", label: "Node 1", position: { x: 250, y: 5 } },
//   { id: "2", label: "Node 2", position: { x: 100, y: 100 } },
//   { id: "3", label: "Node 3", position: { x: 400, y: 100 } },
//   { id: "4", label: "Node 4", position: { x: 400, y: 200 } },
// ];

// const initialEdges = [
//   { id: "e1-2", source: "1", target: "2", animated: true },
//   { id: "e1-3", source: "1", target: "3" },
// ];

// // Create VueFlow instance
// const vueFlow = createVueFlow({
//   nodes: initialNodes,
//   edges: initialEdges,
// });

// Define methods to manage nodes and edges
// const methods = {
//   // Method to add a new node
//   addNode(newNode) {
//     vueFlow.nodes.push(newNode);
//   },

//   // Method to update an existing node by id
//   updateNode(id, updatedNode) {
//     const index = vueFlow.nodes.findIndex((node) => node.id === id);
//     if (index !== -1) {
//       vueFlow.nodes[index] = { ...vueFlow.nodes[index], ...updatedNode };
//     }
//   },

//   // Method to remove a node by id
//   removeNode(id) {
//     vueFlow.nodes = vueFlow.nodes.filter((node) => node.id !== id);
//     // Also remove any edges connected to the removed node
//     vueFlow.edges = vueFlow.edges.filter(
//       (edge) => edge.source !== id && edge.target !== id,
//     );
//   },

//   // Method to add a new edge
//   addEdge(newEdge) {
//     vueFlow.edges.push(newEdge);
//   },

//   // Method to update an existing edge by id
//   updateEdge(id, updatedEdge) {
//     const index = vueFlow.edges.findIndex((edge) => edge.id === id);
//     if (index !== -1) {
//       vueFlow.edges[index] = { ...vueFlow.edges[index], ...updatedEdge };
//     }
//   },

//   // Method to remove an edge by id
//   removeEdge(id) {
//     vueFlow.edges = vueFlow.edges.filter((edge) => edge.id !== id);
//   },
// };

// // Export the vueFlow instance and methods
// export default {
//   methods,
// };
