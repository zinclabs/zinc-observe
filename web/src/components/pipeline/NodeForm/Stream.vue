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

<template>
  <div
    data-test="add-stream-routing-section"
    class="full-width full-height"
    :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
  >
    <div class="stream-routing-title q-pb-sm q-pl-md">
      {{ t("pipeline.streamTitle") }}
    </div>
    <q-separator />
    <div class="stream-routing-container full-width q-pa-md">
      <q-form @submit="saveStream">

        <div class="flex justify-start items-center" style="padding-top: 0px">
          <div
            data-test="add-alert-stream-type-select"
            class="alert-stream-type o2-input q-mr-sm full-width"
            style="padding-top: 0"
          >

            <q-select
              v-model="stream_type"
              :options="streamTypes"
              :label="t('alerts.streamType') + ' *'"
              :popup-content-style="{ textTransform: 'lowercase' }"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm showLabelOnTop no-case full-width"
              stack-label
              outlined
              filled
              dense

              @update:model-value="updateStreams()"
              :rules="[(val: any) => !!val || 'Field is required!']"
            />
          </div>
          <div
            data-test="add-alert-stream-select"
            class="o2-input full-width"
            style="padding-top: 0"
          >
          <q-select
            v-model="stream_name"
            :options="filteredStreams"
             option-label="label"
              option-value="value"
            :label="t('alerts.stream_name') + ' *'"
            :loading="isFetchingStreams"
            :popup-content-style="{ textTransform: 'lowercase' }"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm showLabelOnTop no-case full-width"
            filled
            stack-label
            dense
            use-input
            hide-selected
            fill-input
            :input-debounce="400"
            @filter="filterStreams"
            behavior="menu"
            :rules="[(val: any) => !!val || 'Field is required!']"
            :option-disable="option => option.isDisable"
            />
          </div>
        </div>

        <div
          class="flex justify-start full-width"
          :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
        >
          <q-btn
            data-test="associate-stream-cancel-btn"
            class="text-bold"
            :label="t('alerts.cancel')"
            text-color="light-text"
            padding="sm md"
            no-caps
            @click="openCancelDialog"
          />
          <q-btn
            data-test="associate-stream-save-btn"
            :label="t('alerts.save')"
            class="text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            no-caps
            type="submit"
          />
          <q-btn
            v-if="pipelineObj.isEditNode"
            data-test="associate-stream-delete-btn"
            :label="t('pipeline.deleteNode')"
            class="text-bold no-border q-ml-md"
            color="negative"
            padding="sm xl"
            no-caps
            @click="openDeleteDialog"
          />
        </div>
      </q-form>
    </div>
  </div>
  <confirm-dialog
    v-model="dialog.show"
    :title="dialog.title"
    :message="dialog.message"
    @update:ok="dialog.okCallback"
    @update:cancel="dialog.show = false"
  />
</template>
<script lang="ts" setup>
import { ref, type Ref, defineEmits, onMounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import ConfirmDialog from "../../ConfirmDialog.vue";
import useDragAndDrop from "@/plugins/pipelines/useDnD";
import useStreams from "@/composables/useStreams";
import pipelineService from "@/services/pipelines";
const emit = defineEmits(["cancel:hideform"]);

const { t } = useI18n();

const store = useStore();

const { addNode, pipelineObj, deletePipelineNode } = useDragAndDrop();

const { getStreams } = useStreams();

const filteredStreams: Ref<string[]> = ref([]);
const isFetchingStreams = ref(false);
const indexOptions = ref([]);
const schemaList = ref([]);
const streams: any = ref({});
const usedStreams: any = ref([]);
const streamTypes = ["logs", "metrics", "traces"];
const stream_name = ref(pipelineObj.currentSelectedNodeData?.data.stream_name);
const stream_type = ref("logs");
watch(stream_name, async (newVal) => {
  console.log(stream_name.value, "stream_name");
});
onMounted(async () => {
  await getUsedStreamsList();
  await getStreamList();
});
async function getUsedStreamsList() {
    const org_identifier = store.state.selectedOrganization.identifier;
  await pipelineService.getPipelineStreams(org_identifier)
    .then((res: any) => {
      usedStreams.value[stream_type.value] = res.data.list;
    })
}
async function getStreamList() {
  const streamType = pipelineObj.currentSelectedNodeData.hasOwnProperty("stream_type")
    ? pipelineObj.currentSelectedNodeData.stream_type
    : "logs";
  
  
  isFetchingStreams.value = true;
  
  try {
    console.log(stream_type.value,"in  fun")
    const res : any = await getStreams(stream_type.value, false);
    
    if (res.list.length > 0 && pipelineObj.currentSelectedNodeData.type === "input") {
      res.list.forEach((stream) => {
        stream.isDisable = usedStreams.value[streamType].some(
          (usedStream) => usedStream.stream_name === stream.name
        );
      });
    }
    
    streams.value[streamType] = res.list;
    schemaList.value = res.list;
    indexOptions.value = res.list.map((data) => data.name);
    console.log(streams.value, "streams");
  } finally {
    isFetchingStreams.value = false;
  }
}

// watch(stream_type.value || pipelineObj.currentSelectedNodeData.type, async (newVal) => {
//   await getStreamList();
// });
const updateStreams = () => {
  getStreamList();
  console.log(stream_type.value,"stream_type")
  // pipelineObj.currentSelectedNodeData.data.stream_type = stream_type.value;
  
};



const dialog = ref({
  show: false,
  title: "",
  message: "",
  okCallback: () => {},
});

const openCancelDialog = () => {
  dialog.value.show = true;
  dialog.value.title = "Discard Changes";
  dialog.value.message = "Are you sure you want to cancel changes?";
  dialog.value.okCallback = () => emit("cancel:hideform");
};

const openDeleteDialog = () => {
  dialog.value.show = true;
  dialog.value.title = "Delete Node";
  dialog.value.message =
    "Are you sure you want to delete stream association?";
  dialog.value.okCallback = deleteNode;
};

const deleteNode = () => {
  deletePipelineNode(pipelineObj.currentSelectedNodeID);
  emit("cancel:hideform");
};

const saveStream = () => {
  const streamNodeData = {
    stream_type: stream_type,
    stream_name: stream_name,
    org_id: store.state.selectedOrganization.identifier,
    node_type: "stream",
  };
  addNode(streamNodeData);

  emit("cancel:hideform");
};

const filterStreams = (val: string, update: any) => {
  const streamType = pipelineObj.currentSelectedNodeData.stream_type || 'logs';
  if(pipelineObj.currentSelectedNodeData.type === 'input') {
    const filtered = streams.value[streamType].filter((stream) => {
    return stream.name.toLowerCase().includes(val.toLowerCase());
  }).map((stream) => ({
    label: stream.name,
    value: stream.name,  // Use a unique identifier if needed
    isDisable: stream.isDisable
  }));
  filteredStreams.value = filtered;
  }
  else{
    const filtered = streams.value[streamType].filter((stream) => {
    return stream.name.toLowerCase().includes(val.toLowerCase());
  }).map((stream) => ({
    label: stream.name,
    value: stream.name,  // Use a unique identifier if needed
    isDisable: false
  }));
  filteredStreams.value = filtered;

  }

  
  update();
};


const filterColumns = (options: any[], val: String, update: Function) => {
  let filteredOptions: any[] = [];
  if (val === "") {
    update(() => {
      filteredOptions = [...options];
    });
    return filteredOptions;
  }
  update(() => {
    const value = val.toLowerCase();
    filteredOptions = options.filter(
      (column: any) => column.toLowerCase().indexOf(value) > -1,
    );
  });
  return filteredOptions;
};
</script>

<style scoped>
.stream-routing-title {
  font-size: 20px;
  padding-top: 16px;
}
</style>
