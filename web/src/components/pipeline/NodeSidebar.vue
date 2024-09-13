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

<script>
import useDragAndDrop from "@/plugins/pipelines/useDnD";

export default {
  props: {
    nodeTypes: Array,
    hasInputType: Boolean,
  },
  setup(props) {
    const { onDragStart, pipelineObj } = useDragAndDrop();
    return { node_types: props.nodeTypes, onDragStart, pipelineObj };
  },
};
</script>

<template>
  <div class="nodes">
    <div
      v-for="node in node_types"
      :key="node.io_type"
      class="o2vf_node q-gutter-md q-pb-md"
      :draggable="node.io_type == 'input' && pipelineObj.hasInputNode ? false : true"
      @dragstart="onDragStart($event, node)"
    >
      <q-btn
        borderless
        :class="`o2vf_node_${node.io_type}`"
        flat
        size="md"
        class="q-pa-none btn-fixed-width"
        align="left"
        style="width: 170px"
      >
        <q-tooltip>
          <div style="text-transform: capitalize">{{ node.tooltip }}</div>
        </q-tooltip>
        <q-icon left size="1em"
:name="node.icon" class="q-ma-sm" />
        <q-separator vertical class="q-mr-sm" />
        <div>{{ node.label }}</div>
      </q-btn>
    </div>
  </div>
</template>
