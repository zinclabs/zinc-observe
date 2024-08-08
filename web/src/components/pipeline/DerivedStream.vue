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
      {{ t("pipeline.derivedStream") }}
    </div>
    <q-separator />

    <div class="stream-routing-container q-px-md q-pt-md q-pr-xl">
      <q-form @submit="saveRouting">
        <div
          data-test="stream-routing-name-input"
          class="o2-input"
          style="padding-top: 12px"
        >
          <q-input
            v-model="derivedStream.name"
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
            style="width: 480px"
            :error-message="
              derivedStream.name && isValidStreamName
                ? 'Stream name already exists'
                : ''
            "
            :error="!isValidName"
            @update:model-value="validateStreamName"
          />
        </div>
        <div class="flex justify-start items-center" style="padding-top: 0px">
          <div
            data-test="add-alert-stream-type-select"
            class="alert-stream-type o2-input q-mr-sm"
            style="padding-top: 0"
          >
            <q-select
              v-model="derivedStream.destination.stream_type"
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
              @update:model-value="updateStreams()"
              :rules="[(val: any) => !!val || 'Field is required!']"
              style="min-width: 220px"
            />
          </div>
          <div
            data-test="add-alert-stream-select"
            class="o2-input"
            style="padding-top: 0"
          >
            <q-select
              v-model="derivedStream.destination.stream_name"
              :options="filteredStreams"
              :label="t('alerts.stream_name') + ' *'"
              :loading="isFetchingStreams"
              :popup-content-style="{ textTransform: 'lowercase' }"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm showLabelOnTop no-case"
              filled
              stack-label
              dense
              use-input
              hide-selected
              fill-input
              :input-debounce="400"
              v-bind:readonly="isUpdating"
              v-bind:disable="isUpdating"
              @filter="filterStreams"
              behavior="menu"
              :rules="[(val: any) => !!val || 'Field is required!']"
              style="min-width: 250px !important; width: 250px !important"
            />
          </div>
        </div>

        <!-- 

        Destination Stream

            Org id

            Stream name

            Stream Type 

        -->

        <real-time-alert
          :columns="filteredColumns"
          :conditions="derivedStream.queryConditions"
          @field:add="addField"
          @field:remove="removeField"
        />

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
import { computed, onMounted, ref, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import RealTimeAlert from "../alerts/RealTimeAlert.vue";
import { getUUID } from "@/utils/zincutils";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import useStreams from "@/composables/useStreams";
import ConfirmDialog from "../ConfirmDialog.vue";
import { useQuasar } from "quasar";

interface QueryCondition {
  column: string;
  operator: string;
  value: any;
  id: string;
}

interface DerivedStream {
  name: string;
  destination: {
    // should be part of payload
    org_id: string;
    stream_name: string;
    stream_type: "logs" | "metrics" | "traces" | "index";
  };
  isRealTime: false;
  queryConditions: QueryCondition[];
  triggerConditions: {};
  contextAttributes: {};
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
  derivedStreams: {
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

const emit = defineEmits(["update:node", "cancel:hideform", "delete:node"]);

const isUpdating = ref(false);

const filteredColumns: any = ref([]);

const isFetchingStreams = ref(false);

const filteredStreams: Ref<any[]> = ref([]);

const streams: any = ref({});

const indexOptions = ref([]);

const originalStreamFields: Ref<any[]> = ref([]);

const isValidName: Ref<boolean> = ref(true);

let existingStreamNames: any;

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

const getDefaultDerivedStream = () => {
  //   return {
  //     name: "",
  //     conditions: [{ column: "", operator: "", value: "", id: getUUID() }],
  //   };
  return {
    name: "",
    destination: {
      org_id: "",
      stream_name: "",
      stream_type: "logs",
    },
    is_real_time: false,
    query_condition: {},
    trigger_condition: {},
    context_attributes: {},
    description: "",
    enabled: true,
  };
};

onMounted(() => {
  if (props.editingRoute) {
    isUpdating.value = true;
    derivedStream.value = JSON.parse(
      JSON.stringify(props.editingRoute),
    ) as DerivedStream;
  }

  originalDerivedStream.value = JSON.parse(JSON.stringify(derivedStream.value));

  existingStreamNames = new Set(
    ...Object.values(props.derivedStreams).map((route: any) => route.name),
    props.streamName,
  );

  updateStreamFields();
});

const streamTypes = ["logs", "metrics", "traces"];

const derivedStream: Ref<DerivedStream> = ref(getDefaultDerivedStream());

const originalDerivedStream: Ref<DerivedStream> = ref(
  getDefaultDerivedStream(),
);

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
  return roleNameRegex.test(derivedStream.value.name);
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
  derivedStream.value.queryConditions.push({
    column: "",
    operator: "=",
    value: "",
    id: getUUID(),
  });
};

const removeField = (field: any) => {
  derivedStream.value.queryConditions =
    derivedStream.value.queryConditions.filter(
      (_field: any) => _field.id !== field.id,
    );
};

const closeDialog = () => {
  emit("cancel:hideform");
};

const openCancelDialog = () => {
  if (
    JSON.stringify(originalDerivedStream.value) ===
    JSON.stringify(derivedStream.value)
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
const saveRouting = () => {
  isValidName.value = true;

  if (!isUpdating.value) validateStreamName();

  if (!isValidName.value) {
    return;
  }

  // Save routing
  emit("update:node", {
    data: {
      ...derivedStream.value,
      name: derivedStream.value.name,
    },
    link: nodeLink.value,
  });

  emit("cancel:hideform");
};

const saveUpdatedLink = (link: { from: string; to: string }) => {
  nodeLink.value = link;
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
    type: "derivedStream",
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
  Object.values(props.derivedStreams).forEach((route: any) => {
    if (
      route.name === derivedStream.value.name ||
      route.name === props.streamName
    ) {
      isValidName.value = false;
    }
  });
};

const updateStreams = () => {};
</script>

<style scoped>
.stream-routing-title {
  font-size: 20px;
  padding-top: 16px;
}
.stream-routing-container {
  width: fit-content;
  border-radius: 8px;
  /* box-shadow: 0px 0px 10px 0px #d2d1d1; */
}
</style>
