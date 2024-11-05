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

<template>
  <div
    data-test="add-condition-condition-section"
    class="full-width stream-routing-section"
    :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
  >
    <div class="stream-routing-title q-pb-sm q-pl-md">
      {{ t("pipeline.conditionTitle") }}
    </div>
    <q-separator />

    <div class="stream-routing-container q-px-md q-pt-md q-pr-xl">
      <q-form ref="routeFormRef" @submit="saveCondition">
        <div
          class="q-py-sm showLabelOnTop text-bold text-h7"
          data-test="add-alert-query-input-title"
        >
        <div>
        <div class="previous-drop-down">
      <q-select
          color="input-border"
          class="q-py-sm showLabelOnTop no-case tw-w-full "
          stack-label
          outlined
          filled
          dense
          v-model="selected"
          :options="filteredOptions"
          use-input
          input-debounce="300"
          @filter="filterOptions"


          label="Select Previous Node"
          clearable
        >
        <template v-slot:option="scope">
  <q-item
    v-bind="scope.itemProps"
    v-if="!scope.opt.isGroup"
    class="full-width"
    :style="{ backgroundColor: scope.opt.color  }"
    style="color: black;"
  >              
    <q-item-section avatar class="w-full">
      <q-img
        :src="scope.opt.icon"
        style="width: 24px; height: 24px"
      />
    </q-item-section>
    
    <div class="flex tw-justify-between tw-w-full"  >
      <q-item-section>
        <q-item-label v-html="scope.opt.label"></q-item-label>
      </q-item-section>
      <q-item-section>
        <q-item-label class="tw-ml-auto" v-html="scope.opt.node_type"></q-item-label>
      </q-item-section>
    </div>
  </q-item>

  <!-- Render non-selectable group headers -->
  <q-item v-else   :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'">
    <q-item-section >
      <q-item-label v-html="scope.opt.label" />
    </q-item-section>
  </q-item>
</template>

        </q-select>
      </div>
  </div>
          <real-time-alert
            :columns="filteredColumns"
            :conditions="streamRoute.conditions"
            @field:add="addField"
            @field:remove="removeField"
            :enableNewValueMode="true" 
          />

   
        </div>

        <div
          class="flex justify-start q-mt-lg q-py-sm full-width"
          :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
        >
          <q-btn
            data-test="stream-routing-cancel-btn"
            class="text-bold"
            :label="t('alerts.cancel')"
            text-color="light-text"
            padding="sm md"
            no-caps
            @click="openCancelDialog"
          />
          <q-btn
            data-test="add-report-save-btn"
            :label="t('alerts.save')"
            class="text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            no-caps
            type="submit"
          />
          <q-btn
          v-if="pipelineObj.isEditNode"
            data-test="stream-routing-delete-btn"
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
import {
  computed,
  defineAsyncComponent,
  onMounted,
  ref,
  type Ref,
  onBeforeMount,
  watch,
} from "vue";
import { useI18n } from "vue-i18n";
import RealTimeAlert from "../../alerts/RealTimeAlert.vue";
import {
  getTimezoneOffset,
  getUUID,
  getTimezonesByOffset,
} from "@/utils/zincutils";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import useStreams from "@/composables/useStreams";
import ConfirmDialog from "../../ConfirmDialog.vue";
import { useQuasar } from "quasar";
import useQuery from "@/composables/useQuery";
import searchService from "@/services/search";
import { convertDateToTimestamp } from "@/utils/date";
import useDragAndDrop from "@/plugins/pipelines/useDnD";


const VariablesInput = defineAsyncComponent(
  () => import("@/components/alerts/VariablesInput.vue"),
);

interface RouteCondition {
  column: string;
  operator: string;
  value: any;
  id: string;
}

interface StreamRoute {
  conditions: RouteCondition[];
  name: string;
  query_condition: any | null;

}

const { t } = useI18n();

const q = useQuasar();

const router = useRouter();

const store = useStore();

const { getStream, getStreams } = useStreams();

const { buildQueryPayload } = useQuery();

const emit = defineEmits(["update:node", "cancel:hideform", "delete:node"]);

const isUpdating = ref(false);

const filteredColumns: any = ref([]);

const isValidSqlQuery = ref(true);

const validateSqlQueryPromise = ref<Promise<unknown>>();

const scheduledAlertRef = ref<any>(null);

const filteredStreams: Ref<any[]> = ref([]);

const indexOptions = ref([]);

const originalStreamFields: Ref<any[]> = ref([]);

const isValidName: Ref<boolean> = ref(true);

const isAggregationEnabled = ref(false);

const routeFormRef = ref<any>(null);

const showTimezoneWarning = ref(false);

const { addNode, pipelineObj , deletePipelineNode, formattedOptions,   filteredOptions, filterOptions,getParentNode ,currentSelectedParentNode} = useDragAndDrop();

const selected = ref(null);
watch(selected, (newValue:any) => {
      pipelineObj.userSelectedNode = newValue; 
});
let parser: any;

const nodeLink = ref({
  from: "",
  to: "",
});

const dialog = ref({
  show: false,
  title: "",
  message: "",
  okCallback: () => {},
});

const getDefaultStreamRoute  : any = () => {
  if (pipelineObj.isEditNode) {
    return pipelineObj.currentSelectedNodeData.data;
  }
  return {
    name: "",
    conditions: [{ column: "", operator: "", value: "", id: getUUID() }],
    destination: {
      org_id: "",
      stream_name: "",
      stream_type: "logs",
    },
    is_real_time: true,
    query_condition: {
      sql: "",
      type: "sql",
      aggregation: null,
    },
    trigger_condition: {
      period: 15,
      frequency_type: "minutes",
      cron: "",
      frequency: 15,
      timezone: "UTC",
    },
    context_attributes: [
      {
        key: "",
        value: "",
        id: getUUID(),
      },
    ],
    description: "",
    enabled: true,
  };
};

onBeforeMount(async () => {
  await importSqlParser();
});

onMounted(async () => {
  await importSqlParser();
  getFields();
  if(pipelineObj.isEditNode){
    const selectedParentNode = currentSelectedParentNode();
    if(selectedParentNode){
      selected.value = selectedParentNode;

    }
  }
  else{
    if(pipelineObj.userSelectedNode){
      const currentSelectedNode = formattedOptions.value.find(
        (node)=> node?.id === pipelineObj.userSelectedNode.label
      )
      if(currentSelectedNode?.node_type){
        selected.value = currentSelectedNode;

      }
    }
    else{
      selected.value = null;
      pipelineObj.userSelectedNode = {};
    }
  }
});

const importSqlParser = async () => {
  const useSqlParser: any = await import("@/composables/useParser");
  const { sqlParser }: any = useSqlParser.default();
  parser = await sqlParser();
};

const streamTypes = ["logs", "enrichment_tables"];

const streamRoute:  Ref<StreamRoute> = ref (getDefaultStreamRoute());

const originalStreamRouting:  Ref<StreamRoute> = ref(getDefaultStreamRoute());

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

const filterStreams = (val: string, update: any) => {
  filteredStreams.value = filterColumns(indexOptions.value, val, update);
};

const isValidStreamName = computed(() => {
  const roleNameRegex = /^[a-zA-Z0-9+=,.@_-]+$/;
  // Check if the role name is valid
  return roleNameRegex.test(streamRoute.value?.name);
});

const updateStreamFields = async (streamName : any, streamType : any) => {

  let streamCols: any = [];
  const streams: any = await getStream(streamName, streamType, true);

  if (streams && Array.isArray(streams.schema)) {
    streamCols = streams.schema.map((column: any) => ({
      label: column.name,
      value: column.name,
      type: column.type,
    }));
  }
  originalStreamFields.value = [...originalStreamFields.value, ...streamCols];
  filteredColumns.value = [...filteredColumns.value, ...streamCols];
};

const getFields = async () => {
  try {
    // find input node
    const inputStreamNode : any = pipelineObj.currentSelectedPipeline.nodes.find(
      (node: any) => node.io_type === "input" && node.data.node_type === "stream",
    );


    const inputQueryNode :any = pipelineObj.currentSelectedPipeline.nodes.find(
      (node: any) => node.io_type === "input" && node.data.node_type === "query",
    );
    if (inputStreamNode ) {
      updateStreamFields(
        inputStreamNode.data?.stream_name.value || inputStreamNode.data?.stream_name,
        inputStreamNode.data?.stream_type,
      );
    } else {
      const filteredQuery: any = inputQueryNode?.data?.query_condition.sql
        .split("\n")
        .filter((line: string) => line.length > 0 && !line.trim().startsWith("--")) // Only process non-empty lines
        .join("\n");
        if(filteredQuery){
          const parsedSql = parser.astify(filteredQuery);
          if (parsedSql && parsedSql.from) {
            const streamNames = parsedSql.from.map((item : any) => item.table);
            for (const streamName of streamNames) {
              await updateStreamFields(streamName, inputQueryNode?.data?.stream_type);
            }
          }
        }
      
    }
  } catch (e) {
    console.error(e);
  }
};

const addField = () => {
    streamRoute.value.conditions.push({
      column: "",
      operator: "",
      value: "",
      id: getUUID(),
    });
};

const removeField = (field: any) => {
    streamRoute.value.conditions = streamRoute.value.conditions.filter(
      (_field: any) => _field.id !== field.id,
    );
};

const closeDialog = () => {
  emit("cancel:hideform");
};

const openCancelDialog = () => {
  if (
    JSON.stringify(originalStreamRouting.value) ===
    JSON.stringify(streamRoute.value)
  ) {
    closeDialog();
    return;
  }

  dialog.value.show = true;
  dialog.value.title = "Discard Changes";
  dialog.value.message = "Are you sure you want to cancel routing changes?";
  dialog.value.okCallback = closeDialog;
  pipelineObj.userClickedNode = {};
  pipelineObj.userSelectedNode = {};  

};

// TODO OK : Add check for duplicate routing name
const saveCondition = async () => {
  let payload = getConditionPayload();
  if(payload.conditions.length === 0){
    q.notify({
      type: "negative",
      message: "Please add atleast one condition",
      timeout: 3000,
    });
    return;
  }
  let conditionData = {
    node_type: "condition",
    conditions: payload.conditions,
  };
  addNode(conditionData);

  emit("cancel:hideform");
};

const openDeleteDialog = () => {
  dialog.value.show = true;
  dialog.value.title = "Delete Node";
  dialog.value.message = "Are you sure you want to delete stream routing?";
  dialog.value.okCallback = deleteRoute;
};

const deleteRoute = () => {
  // emit("delete:node", {
  //   data: {
  //     ...props.editingRoute,
  //     name: props.editingRoute.name,
  //   },
  //   type: "streamRoute",
  // });

  // emit("delete:node", {
  //   data: {
  //     ...props.editingRoute,
  //     name: props.editingRoute.name + ":" + "condition",
  //   },
  //   type: "condition",
  // });

  deletePipelineNode(pipelineObj.currentSelectedNodeID);

  emit("cancel:hideform");
};

const getConditionPayload = () => {
  let payload = JSON.parse(JSON.stringify(streamRoute.value));

  payload = {
    name: payload.name,
    conditions: payload.conditions,
    is_real_time: payload.is_real_time,
  };

  if (isUpdating.value) {
    payload.updatedAt = new Date().toISOString();
    payload.lastEditedBy = store.state.userInfo.email;
  } else {
    payload.createdAt = new Date().toISOString();
    payload.owner = store.state.userInfo.email;
    payload.lastTriggeredAt = new Date().getTime();
    payload.lastEditedBy = store.state.userInfo.email;
    payload.updatedAt = new Date().toISOString();
  }

  return payload;
};

const validateSqlQuery = () => {
  const query = buildQueryPayload({
    sqlMode: true,
    streamName: streamRoute.value.name as string,
  });

  delete query.aggs;

  query.query.sql = streamRoute.value.query_condition?.sql || '';

  validateSqlQueryPromise.value = new Promise((resolve, reject) => {
    searchService
      .search({
        org_identifier: store.state.selectedOrganization.identifier,
        query,
        page_type: "logs",
      })
      .then((res: any) => {
        isValidSqlQuery.value = true;
        resolve("");
      })
      .catch((err: any) => {
        if (err.response.data.code === 500) {
          isValidSqlQuery.value = false;
          q.notify({
            type: "negative",
            message: "Invalid SQL Query : " + err.response.data.message,
            timeout: 3000,
          });
          reject("");
        } else isValidSqlQuery.value = true;

        resolve("");
      });
  });
};
</script>

<style scoped>
.stream-routing-title {
  font-size: 20px;
  padding-top: 16px;
}
.stream-routing-container {
  width: 720px;
  border-radius: 8px;
  /* box-shadow: 0px 0px 10px 0px #d2d1d1; */
}

.stream-routing-section {
  min-height: 100%;
}
.previous-drop-down{
  width: 600px;
}
</style>
