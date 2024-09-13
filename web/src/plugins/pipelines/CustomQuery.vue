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
import {
  Handle,
  Position,
  useHandleConnections,
  useNodesData,
} from "@vue-flow/core";

import Query from "@/components/pipeline/Query.vue";

import useDragAndDrop from "./useDnD";

export default {
  components: {
    Query,
  },
  props: {
    id: String,
    data: Object,
    type: String,
  },
  setup(props) {
    const { onDragStart } = useDragAndDrop();

    const connections = useHandleConnections({
      useHandleConnections: true,
    });

    const nodesData = useNodesData(() => connections.value[0]?.source);
  },
};
</script>

<template>
  <div>
    Query
    <!-- Output Handle (Source) -->
    <Handle
      id="output"
      type="source"
      position="bottom"
      :style="{ filter: 'invert(100%)' }"
    />
  </div>
</template>

<style lang="scss">
.vue-flow__node-query {
  padding: 10px;
  border-radius: 3px;
  width: auto;
  font-size: 12px;
  text-align: center;
  border-width: 1px;
  border-style: solid;
  color: var(--vf-node-text);
  background-color: var(--vf-node-bg);
  border-color: var(--vf-node-color);
}
</style>
