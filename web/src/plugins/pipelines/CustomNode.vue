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
import { Handle } from "@vue-flow/core";
import useDragAndDrop from "./useDnD";
import { defineEmits, ref } from "vue";
import { useI18n } from "vue-i18n";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

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

const emit = defineEmits(["delete:node"]);

const { t } = useI18n();

const { pipelineObj, deletePipelineNode } = useDragAndDrop();

const editNode = (id) => {
  //from id find the node from pipelineObj.currentSelectedPipelineData.nodes
  const fullNode = pipelineObj.currentSelectedPipeline.nodes.find(
    (node) => node.id === id
  );
  pipelineObj.isEditNode = true;
  pipelineObj.currentSelectedNodeData = fullNode;
  pipelineObj.currentSelectedNodeID = id;
  pipelineObj.dialog.name = fullNode.data.node_type;
  pipelineObj.dialog.show = true;
};

const deleteNode = (id) => {
  openCancelDialog(id);
};
const  functionInfo = (data) =>  {
  console.log(data,"data i am ")
  console.log(pipelineObj.functions[data.name],"fun to be displayed")

      return pipelineObj.functions[data.name] || null;
  }

const confirmDialogMeta = ref({
  show: false,
  title: "",
  message: "",
  data: null,
  onConfirm: () => {},
});

const openCancelDialog = (id) => {
  confirmDialogMeta.value.show = true;
  confirmDialogMeta.value.title = t("common.delete");
  confirmDialogMeta.value.message = "Are you sure you want to delete node?";
  confirmDialogMeta.value.onConfirm = () => {
    deletePipelineNode(id);
  };
};

const resetConfirmDialog = () => {
  confirmDialogMeta.value.show = false;
  confirmDialogMeta.value.title = "";
  confirmDialogMeta.value.message = "";
  confirmDialogMeta.value.onConfirm = () => {};
};

function getIcon(data, ioType) {
  console.log(data,"data in getIcon")
  const searchTerm = data.node_type;

  const node = this.pipelineObj.nodeTypes.find(
    (node) => node.subtype === searchTerm && node.io_type === ioType,
  );
  return node ? node.icon : undefined;
}
</script>

<template>
  <!-- Input Handle (Target) -->
  <div class="o2vf_node">
    <Handle
      v-if="io_type == 'output' || io_type === 'default'"
      id="input"
      type="target"
      :position="pipelineObj.pipelineDirectionTopBottom == true ? 'top' : 'left'"
      :style="{ filter: 'invert(100%)' }"
    />
    <div
      v-if="data.node_type == 'function'"
      :class="`o2vf_node_${type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
      padding: 5px 2px;
        width: fit-content;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
      "
    >
    <q-tooltip :style="{ maxWidth: '300px', whiteSpace: 'pre-wrap' }">
  <div>
    <strong>Name:</strong> {{ functionInfo(data).name }}<br />
    <strong>Definition:</strong><br />
    <div style="border: 1px solid lightgray; padding: 4px; border-radius: 1px ;">
      {{ functionInfo(data).function }}
    </div>
  </div>
</q-tooltip>

      <div class="icon-container " style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon
          :name="getIcon(data, io_type)"
          size="1em"
          class="q-ma-sm"
        />
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
              data.after_flatten ? "After Flattening" : "Before Flattening"
            }}
          </div>
        </div>
      </div>
      <div class="float-right tw-pl-2">
        <q-btn
          flat
          round
          dense
          icon="edit"
          size="0.8em"
          @click="editNode(id)"
        />
        <q-btn
          flat
          round
          dense
          icon="delete"
          size="0.8em"
          @click="deleteNode(id)"
        />
      </div>
    </div>

    <div
      v-if="data.node_type == 'stream'"
      :class="`o2vf_node_${io_type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
        width: fit-content;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
        padding: 5px 2px;

      "
    >
      <div class="icon-container" style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon
          :name="getIcon(data, io_type)"
          size="1em"
          class="q-ma-sm"
        />
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
          {{ data.stream_type }} -{{   data.stream_name.label }}
        </div>
      </div>
      <div class="float-right tw-pl-2">
        <q-btn
          flat
          round
          dense
          icon="edit"
          size="0.8em"
          @click="editNode(id)"
        />
        <q-btn
          flat
          round
          dense
          icon="delete"
          size="0.8em"
          @click="deleteNode(id)"
        />
      </div>
    </div>

    <div
      v-if="data.node_type == 'query'"
      :class="`o2vf_node_${io_type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
        width: fit-content;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
        padding: 5px 2px;

      "
    >
    <q-tooltip :style="{ maxWidth: '300px', whiteSpace: 'pre-wrap' }">
  <div>
    <strong>SQL:</strong> <pre style="max-width: 200px ; text-wrap: wrap;">{{ data.query_condition.sql }}</pre><br />
    <strong>Period:</strong> {{ data.trigger_condition.period }}<br />
    <strong>Frequency:</strong> {{ data.trigger_condition.frequency }} {{ data.trigger_condition.frequency_type }}<br />
    <strong>Operator:</strong> {{ data.trigger_condition.operator }}<br />
    <strong>Threshold:</strong> {{ data.trigger_condition.threshold }}<br />
    <strong>Cron:</strong> {{ data.trigger_condition.cron || 'None' }}<br />
    <strong>Silence:</strong> {{ data.trigger_condition.silence }}
  </div>
</q-tooltip>
      <div class="icon-container" style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon
          :name="getIcon(data, io_type)"
          size="1em"
          class="q-ma-sm"
        />
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

      <div class="float-right">
        <q-btn
          flat
          round
          dense
          icon="edit"
          size="0.8em"
          @click="editNode(id)"
          style="margin-right: 5px"
        />
        <q-btn
          flat
          round
          dense
          icon="delete"
          size="0.8em"
          @click="deleteNode(id)"
        />
      </div>
    </div>

    <div
      v-if="data.node_type == 'condition'"
      :class="`o2vf_node_${io_type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
        width: fit-content;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
      "
    >
    <q-tooltip>
      <div v-for="(condition, index) in data.conditions" :key="index">
        <div>
          <strong>Column:</strong> {{ condition.column }}
        </div>
        <div>
          <strong>Operator:</strong> {{ condition.operator }}
        </div>
        <div>
          <strong>Value:</strong> {{ condition.value }}
        </div>
        <q-separator v-if="index < data.conditions.length - 1" />
      </div>
    </q-tooltip>
      <div class="icon-container" style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon
          :name="getIcon(data, io_type)"
          size="1em"
          class="q-ma-sm"
        />
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
          Condition - {{ data.condition }}
        </div>
      </div>

      <div class="float-right">
        <q-btn
          flat
          round
          dense
          icon="edit"
          size="0.8em"
          @click="editNode(id)"
          style="margin-right: 5px"
        />
        <q-btn
          flat
          round
          dense
          icon="delete"
          size="0.8em"
          @click="deleteNode(id)"
        />
      </div>
    </div>
    <Handle
      v-if="io_type === 'input' || io_type === 'default'"
      id="output"
      type="source"
      :position="pipelineObj.pipelineDirectionTopBottom == true ? 'bottom' : 'right'"
      :style="{ filter: 'invert(100%)' }"
    />
  </div>

  <confirm-dialog
    :title="confirmDialogMeta.title"
    :message="confirmDialogMeta.message"
    @update:ok="confirmDialogMeta.onConfirm()"
    @update:cancel="resetConfirmDialog"
    v-model="confirmDialogMeta.show"
  />
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
