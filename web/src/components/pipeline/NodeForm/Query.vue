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
    data-test="add-stream-query-routing-section"
    class="full-width stream-routing-section"
    :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
  >
    <div class="stream-routing-title q-pb-sm q-pl-md">
      {{ t("pipeline.query") }}
    </div>
    <q-separator />

    <div class="stream-routing-container q-px-md q-pt-md q-pr-xl">
      <q-form ref="queryFormRef" @submit="saveQueryData">
        <div>
          <div
            data-test="stream-route-stream-type-select"
            class="stream-route-stream-type o2-input"
            style="padding-top: 0"
          >
            <q-select
              v-model="streamRoute.stream_type"
              :options="streamTypes"
              :label="t('alerts.streamType') + ' *'"
              :popup-content-style="{ textTransform: 'lowercase' }"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop no-case"
              stack-label
              outlined
              filled
              dense
              v-bind:readonly="isUpdating"
              v-bind:disable="isUpdating"
              :rules="[(val: any) => !!val || 'Field is required!']"
              style="width: 400px"
            />
          </div>
          <scheduled-pipeline
            ref="scheduledAlertRef"
            :columns="filteredColumns"
            :conditions="[]"
            :alertData="streamRoute"
            :disableThreshold="true"
            :disableVrlFunction="true"
            :isValidSqlQuery="isValidSqlQuery"
            :disableQueryTypeSelection="true"
            v-model:trigger="streamRoute.trigger_condition"
            v-model:sql="streamRoute.query_condition.sql"
            v-model:query_type="streamRoute.query_condition.type"
            v-model:aggregation="streamRoute.query_condition.aggregation"
            v-model:isAggregationEnabled="isAggregationEnabled"
            @validate-sql="validateSqlQuery"
            class="q-mt-sm"
          />
        </div>

        <div
          class="flex justify-start full-width"
          :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
        >
      
          <q-btn
            data-test="stream-routing-query-cancel-btn"
            class="text-bold"
            :label="t('alerts.cancel')"
            text-color="light-text"
            padding="sm md"
            no-caps
            @click="openCancelDialog"
          />
          <q-btn
            data-test="stream-routing-query-save-btn"
            :label="t('alerts.save')"
            class="text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            no-caps
            type="submit"
          />
          <q-btn
          v-if="pipelineObj.isEditNode"
            data-test="stream-routing-query-delete-btn"
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
  onActivated,
} from "vue";
import { useI18n } from "vue-i18n";
import { getUUID } from "@/utils/zincutils";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import useStreams from "@/composables/useStreams";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import { useQuasar } from "quasar";
import ScheduledPipeline from "@/components/pipeline/NodeForm/ScheduledPipeline.vue";
import useQuery from "@/composables/useQuery";
import searchService from "@/services/search";
import useDragAndDrop from "@/plugins/pipelines/useDnD";

const VariablesInput = defineAsyncComponent(
  () => import("@/components/alerts/VariablesInput.vue")
);

interface RouteCondition {
  column: string;
  operator: string;
  value: any;
  id: string;
}

interface StreamRoute {
  name: string;
  stream_type: string;
  conditions: RouteCondition[];
  query_condition: {
    sql: string;
    type: string;
    aggregation: {
      group_by: string[];
    } | null;
  };
  trigger_condition: {
    period: number;
    frequency_type: string;
    frequency: string;
    cron: string;
    timezone: any;

  };
  context_attributes: any;
  description: string;
  enabled: boolean;
}

const props = defineProps({
  streamName: {
    type: String,
    required: true,
  },
  streamType: {
    type: String,
    required: true,
  },
  editingRoute: {
    type: Object,
    required: false,
    default: () => null,
  },
  streamRoutes: {
    type: Object,
    required: true,
    default: () => ({}),
  },
});

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

const isAggregationEnabled = ref(false);

const queryFormRef = ref<any>(null);

const { addNode, pipelineObj , deletePipelineNode} = useDragAndDrop();

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

const getDefaultStreamRoute : any = () => {
  if (pipelineObj.isEditNode) {
    return pipelineObj.currentSelectedNodeData.data;
  }
  return {
    name: "",
    conditions: [{ column: "", operator: "", value: "", id: getUUID() }],
    stream_type: "logs",
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

onMounted(() => {
  
  if (pipelineObj.isEditNode) {
    streamRoute.value = pipelineObj.currentSelectedNodeData?.data as StreamRoute;
  }

  originalStreamRouting.value = JSON.parse(JSON.stringify(streamRoute.value));

  updateStreamFields();
});

onActivated(() => {
  if (pipelineObj.isEditNode) {
    streamRoute.value = pipelineObj.currentSelectedNodeData?.data as StreamRoute;
  }

  originalStreamRouting.value = JSON.parse(JSON.stringify(streamRoute.value));
});

const streamTypes = ["logs"];

const streamRoute: Ref<StreamRoute> = ref(getDefaultStreamRoute());

const originalStreamRouting: Ref<StreamRoute> = ref(getDefaultStreamRoute());

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
      (column: any) => column.toLowerCase().indexOf(value) > -1
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
  return roleNameRegex.test(streamRoute.value.name);
});

const updateStreamFields = async () => {
  let streamCols: any = [];
  const streams: any = await getStream(
    props.streamName,
    props.streamType,
    true
  );

  if (streams && Array.isArray(streams.schema)) {
    streamCols = streams.schema.map((column: any) => ({
      label: column.name,
      value: column.name,
      type: column.type,
    }));
  }
  originalStreamFields.value = [...streamCols];
  filteredColumns.value = [...streamCols];
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
const saveQueryData = async () => {
  if (!scheduledAlertRef.value.validateInputs()) {
    return false;
  }

  try {
    await validateSqlQuery();
    await validateSqlQueryPromise.value;

  } catch (e) {
    return false;
  }

  queryFormRef.value.validate().then((valid: any) => {
    if (!valid) {
      return false;
    }
  });

  const formData = streamRoute.value;
  if(typeof formData.trigger_condition.period === 'string') {
    formData.trigger_condition.period = parseInt(formData.trigger_condition.period);
  }
  let queryPayload = {
    node_type: "query", // required
    stream_type: formData.stream_type, // required
    org_id: store.state.selectedOrganization.identifier, // required
    query_condition: {
      // same as before
      type: formData.query_condition.type,
      conditions: null,
      sql: formData.query_condition.sql,
      promql: null,
      promql_condition: null,
      aggregation: formData.query_condition.aggregation,
      vrl_function: null,
      search_event_type: "DerivedStream",

    },
    trigger_condition: {
      // same as before
      period: formData.trigger_condition.period || 1,
      operator: "=",
      threshold: 0,
      frequency: parseInt(formData.trigger_condition.frequency),
      cron: formData.trigger_condition.cron,
      frequency_type: formData.trigger_condition.frequency_type,
      silence: 0,
      timezone: formData.trigger_condition.timezone,
    },
  };
  addNode(queryPayload);
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
  deletePipelineNode (pipelineObj.currentSelectedNodeID);


  emit("cancel:hideform");
};

const addVariable = () => {
  streamRoute.value.context_attributes.push({
    key: "",
    value: "",
    id: getUUID(),
  });
};

const removeVariable = (variable: any) => {
  streamRoute.value.context_attributes =
    streamRoute.value.context_attributes.filter(
      (_variable: any) => _variable.id !== variable.id
    );
};

const validateSqlQuery = () => {

  const query = buildQueryPayload({
    sqlMode: true,
    streamName: streamRoute.value.name as string,
  });

  delete query.aggs;

  query.query.sql = streamRoute.value.query_condition.sql;


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
        if (  err) {
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
</style>
