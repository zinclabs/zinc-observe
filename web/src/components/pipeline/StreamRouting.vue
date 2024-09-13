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
    class="full-width stream-routing-section"
    :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
  >
    <div class="stream-routing-title q-pb-sm q-pl-md">
      {{ t("pipeline.routing") }}
    </div>
    <q-separator />

    <div class="stream-routing-container q-px-md q-pt-md q-pr-xl">
      <q-form ref="routeFormRef" @submit="saveRouting">
        <div
          data-test="stream-routing-name-input"
          class="o2-input"
          style="padding-top: 12px"
        >
          <q-input
            v-model="streamRoute.name"
            :label="t('function.stream_name') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            v-bind:readonly="isUpdating"
            v-bind:disable="isUpdating"
            :rules="[
              (val: any, rules: any) =>
                !!val
                  ? isValidStreamName ||
                    `Use alphanumeric and '+=,.@-_' characters only, without spaces.`
                  : t('common.nameRequired'),
            ]"
            tabindex="0"
            style="width: 400px"
            :error-message="
              streamRoute.name && isValidStreamName
                ? 'Stream name already exists'
                : ''
            "
            :error="!isValidName"
            @update:model-value="validateStreamName"
          />
        </div>

        <div class="q-gutter-sm">
          <q-radio
            data-test="add-alert-realtime-alert-radio"
            v-bind:readonly="isUpdating"
            v-bind:disable="isUpdating"
            v-model="streamRoute.is_real_time"
            :checked="!streamRoute.is_real_time"
            :val="true"
            :label="t('alerts.realTime')"
            class="q-ml-none"
          />
          <q-radio
            data-test="add-alert-scheduled-alert-radio"
            v-bind:readonly="isUpdating"
            v-bind:disable="isUpdating"
            v-model="streamRoute.is_real_time"
            :checked="streamRoute.is_real_time"
            :val="false"
            :label="t('alerts.standard')"
            class="q-ml-none"
          />
        </div>

        <div
          v-if="streamRoute.is_real_time"
          class="q-py-sm showLabelOnTop text-bold text-h7"
          data-test="add-alert-query-input-title"
        >
          <real-time-alert
            v-if="streamRoute.is_real_time"
            :columns="filteredColumns"
            :conditions="streamRoute.conditions"
            @field:add="addField"
            @field:remove="removeField"
          />
        </div>
        <div v-else>
          <div
            data-test="stream-route-stream-type-select"
            class="stream-route-stream-type o2-input q-mr-sm q-my-md"
            style="padding-top: 0"
          >
            <q-select
              v-model="streamRoute.destination.stream_type"
              :options="streamTypes"
              :label="t('alerts.streamType') + ' *'"
              :popup-content-style="{ textTransform: 'lowercase' }"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm showLabelOnTop no-case"
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
            :showTimezoneWarning="showTimezoneWarning"
            v-model:trigger="streamRoute.trigger_condition"
            v-model:sql="streamRoute.query_condition.sql"
            v-model:query_type="streamRoute.query_condition.type"
            v-model:aggregation="streamRoute.query_condition.aggregation"
            v-model:isAggregationEnabled="isAggregationEnabled"
            @field:add="addField"
            @field:remove="removeField"
            @validate-sql="validateSqlQuery"
            class="q-mt-sm"
          />

          <div class="q-mt-md">
            <div class="text-bold">{{ t("alerts.additionalVariables") }}</div>
            <variables-input
              class="o2-input"
              :variables="streamRoute.context_attributes"
              @add:variable="addVariable"
              @remove:variable="removeVariable"
            />
          </div>
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
            v-if="isUpdating"
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
import { computed, defineAsyncComponent, onMounted, ref, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import RealTimeAlert from "../alerts/RealTimeAlert.vue";
import {
  getTimezoneOffset,
  getUUID,
  getTimezonesByOffset,
} from "@/utils/zincutils";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import useStreams from "@/composables/useStreams";
import ConfirmDialog from "../ConfirmDialog.vue";
import { useQuasar } from "quasar";
import ScheduledPipeline from "@/components/pipeline/ScheduledPipeline.vue";
import useQuery from "@/composables/useQuery";
import searchService from "@/services/search";
import { convertDateToTimestamp } from "@/utils/date";

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
  name: string;
  conditions: RouteCondition[];
  destination: {
    // should be part of payload
    org_id: string;
    stream_name: string;
    stream_type: string;
  };
  is_real_time: boolean;
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
    frequency: number;
    cron: string;
    timezone: string;
  };
  context_attributes: any;
  description: string;
  enabled: boolean;
  tz_offset?: number;
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

const isValidName: Ref<boolean> = ref(true);

const isAggregationEnabled = ref(false);

const routeFormRef = ref<any>(null);

const showTimezoneWarning = ref(false);

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

const getDefaultStreamRoute = () => {
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

onMounted(() => {
  if (props.editingRoute) {
    isUpdating.value = true;

    streamRoute.value = JSON.parse(
      JSON.stringify(props.editingRoute),
    ) as StreamRoute;

    if (!streamRoute.value.is_real_time) {
      // If aggregation was present enable aggregation toggle
      isAggregationEnabled.value =
        !!props.editingRoute.query_condition.aggregation;

      if (!streamRoute.value.trigger_condition?.timezone) {
        if (streamRoute.value.tz_offset === 0 || !streamRoute.value.tz_offset) {
          streamRoute.value.trigger_condition.timezone = "UTC";
        } else {
          getTimezonesByOffset(streamRoute.value.tz_offset as number).then(
            (res: any) => {
              if (res.length > 1) showTimezoneWarning.value = true;
              streamRoute.value.trigger_condition.timezone = res[0];
            },
          );
        }
      }

      // If context attributes are present, convert them to array
      streamRoute.value.context_attributes = Object.keys(
        streamRoute.value.context_attributes,
      ).map((attr: string) => {
        return {
          key: attr,
          value: streamRoute.value.context_attributes[attr],
          id: getUUID(),
        };
      });
    }
  }

  originalStreamRouting.value = JSON.parse(JSON.stringify(streamRoute.value));

  updateStreamFields();
});

const streamTypes = ["logs", "enrichment_tables"];

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
  return roleNameRegex.test(streamRoute.value.name);
});

const updateStreamFields = async () => {
  let streamCols: any = [];
  const streams: any = await getStream(
    props.streamName,
    props.streamType,
    true,
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

const addField = () => {
  if (streamRoute.value.is_real_time) {
    streamRoute.value.conditions.push({
      column: "",
      operator: "",
      value: "",
      id: getUUID(),
    });
  }
};

const removeField = (field: any) => {
  if (streamRoute.value.is_real_time) {
    streamRoute.value.conditions = streamRoute.value.conditions.filter(
      (_field: any) => _field.id !== field.id,
    );
  }
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
};

// TODO OK : Add check for duplicate routing name
const saveRouting = async () => {
  isValidName.value = true;

  if (!isUpdating.value) validateStreamName();

  if (!isValidName.value) {
    return;
  }

  if (!streamRoute.value.is_real_time) {
    if (!scheduledAlertRef.value.validateInputs()) {
      return false;
    }
  }

  try {
    await validateSqlQueryPromise.value;
  } catch (e) {
    return false;
  }

  routeFormRef.value.validate().then((valid: any) => {
    if (!valid) {
      return false;
    }
  });

  // Save routing
  emit("update:node", {
    data: {
      ...getRoutePayload(),
      name: streamRoute.value.name,
    },
    link: nodeLink.value,
  });

  emit("cancel:hideform");
};

const openDeleteDialog = () => {
  dialog.value.show = true;
  dialog.value.title = "Delete Node";
  dialog.value.message = "Are you sure you want to delete stream routing?";
  dialog.value.okCallback = deleteRoute;
};

const deleteRoute = () => {
  emit("delete:node", {
    data: {
      ...props.editingRoute,
      name: props.editingRoute.name,
    },
    type: "streamRoute",
  });

  emit("delete:node", {
    data: {
      ...props.editingRoute,
      name: props.editingRoute.name + ":" + "condition",
    },
    type: "condition",
  });

  emit("cancel:hideform");
};

const validateStreamName = () => {
  isValidName.value = true;
  Object.values(props.streamRoutes).forEach((route: any) => {
    if (
      route.name === streamRoute.value.name ||
      route.name === props.streamName
    ) {
      isValidName.value = false;
    }
  });
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
      (_variable: any) => _variable.id !== variable.id,
    );
};

const getRoutePayload = () => {
  let payload = JSON.parse(JSON.stringify(streamRoute.value));

  if (payload.uuid) delete payload.uuid;

  if (payload.is_real_time) {
    payload = {
      name: payload.name,
      conditions: payload.conditions,
      is_real_time: payload.is_real_time,
    };
  } else {
    // Deleting uuid from payload as it was added for reference of frontend
    payload.destination.org_id = store.state.selectedOrganization.identifier;

    payload.destination.stream_name = payload.name;

    payload.context_attributes = {};

    payload.query_condition.type = payload.is_real_time
      ? "custom"
      : streamRoute.value.query_condition.type;

    streamRoute.value.context_attributes.forEach((attr: any) => {
      if (attr.key?.trim() && attr.value?.trim())
        payload.context_attributes[attr.key] = attr.value;
    });

    payload.trigger_condition.period = Number(
      streamRoute.value.trigger_condition.period,
    );

    payload.trigger_condition.frequency = Number(
      streamRoute.value.trigger_condition.frequency,
    );

    payload.description = streamRoute.value.description.trim();

    if (
      !isAggregationEnabled.value ||
      streamRoute.value.query_condition.type !== "custom"
    ) {
      payload.query_condition.aggregation = null;
    }

    if (payload.query_condition.aggregation?.having) {
      delete payload.query_condition?.aggregation?.having;
    }

    payload.tz_offset = getTimezoneOffset();

    if (payload.trigger_condition.frequency_type == "cron") {
      const now = new Date();

      // Get the day, month, and year from the date object
      const day = String(now.getDate()).padStart(2, "0");
      const month = String(now.getMonth() + 1).padStart(2, "0"); // January is 0!
      const year = now.getFullYear();

      // Combine them in the DD-MM-YYYY format
      const date = `${day}-${month}-${year}`;

      // Get the hours and minutes, ensuring they are formatted with two digits
      const hours = String(now.getHours()).padStart(2, "0");
      const minutes = String(now.getMinutes()).padStart(2, "0");

      // Combine them in the HH:MM format
      const time = `${hours}:${minutes}`;

      const convertedDateTime = convertDateToTimestamp(
        date,
        time,
        payload.trigger_condition.timezone,
      );

      payload.tz_offset = convertedDateTime.offset;
    }

    delete payload?.conditions;

    delete payload?.query_condition?.conditions;
  }

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
</style>
