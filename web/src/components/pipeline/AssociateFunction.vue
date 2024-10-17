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
    data-test="add-stream-routing-section"
    class="full-width full-height"
    :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
  >
    <div class="stream-routing-title q-pb-sm q-pl-md">
      {{ t("pipeline.associateFunction") }}
    </div>
    <q-separator />
    <div v-if="loading">
      <q-spinner
        v-if="loading"
        color="primary"
        size="40px"
        style="
          position: absolute;
          top: 50%;
          left: 50%;
          transform: translate(-50%, -50%);
        "
      />
    </div>
    <div v-else class="stream-routing-container full-width full-height q-pa-md">
      <q-toggle
        data-test="create-function-toggle"
        class="q-mb-sm"
        :label="isUpdating ? 'Edit function' : 'Create new function'"
        v-model="createNewFunction"
      />
      <q-form @submit="saveFunction">
        <div
          v-if="!createNewFunction"
          class="flex justify-start items-center full-width"
          style="padding-top: 0px"
        >
          <div
            data-test="associate-function-select-function-input"
            class="alert-stream-type o2-input q-mr-sm full-width"
            style="padding-top: 0"
          >
            <q-select
              v-model="selectedFunction"
              :options="filteredFunctions"
              :label="t('function.selectFunction') + ' *'"
              :popup-content-style="{ textTransform: 'lowercase' }"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm showLabelOnTop no-case"
              stack-label
              outlined
              filled
              dense
              :rules="[(val: any) => !!val || 'Field is required!']"
              style="min-width: 220px"
              v-bind:readonly="isUpdating"
              v-bind:disable="isUpdating"
              :error-message="
                selectedFunction ? 'Function is already associated' : ''
              "
              :error="functionExists"
            />
          </div>

          <div
            data-test="associate-function-order-input"
            class="o2-input full-width"
            style="padding-top: 12px"
          >
            <q-input
              v-model="functionOrder"
              :label="t('function.order') + ' *'"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              type="number"
              :rules="[
                (val: any) => (!!val && val > -1) || 'Field is required!',
              ]"
              tabindex="0"
              style="min-width: 220px"
            />
          </div>
        </div>

        <div v-if="createNewFunction" class="pipeline-add-function">
          <AddFunction
            ref="addFunctionRef"
            :model-value="functionData"
            :is-updated="isUpdating"
            @update:list="onFunctionCreation"
            @cancel:hideform="cancelFunctionCreation"
          />
        </div>

        <div
          class="flex justify-start full-width"
          :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
        >
          <q-btn
            data-test="associate-function-cancel-btn"
            class="text-bold"
            :label="t('alerts.cancel')"
            text-color="light-text"
            padding="sm md"
            no-caps
            @click="openCancelDialog"
          />
          <q-btn
            data-test="associate-function-save-btn"
            :label="t('alerts.save')"
            class="text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            no-caps
            type="submit"
          />
          <q-btn
            v-if="isUpdating"
            data-test="associate-function-delete-btn"
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
  ref,
  type Ref,
  defineEmits,
  onBeforeMount,
  watch,
  nextTick,
  defineAsyncComponent,
} from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import ConfirmDialog from "../ConfirmDialog.vue";

interface RouteCondition {
  column: string;
  operator: string;
  value: any;
  id: string;
}

interface StreamRoute {
  sourceStreamName: string;
  destinationStreamName: string;
  sourceStreamType: string;
  destinationStreamType: string;
  conditions: RouteCondition[];
}

const AddFunction = defineAsyncComponent(
  () => import("../functions/AddFunction.vue")
);

const props = defineProps({
  defaultOrder: {
    type: Number,
    required: false,
    default: 1,
  },
  functionData: {
    type: Object,
    required: false,
    default: () => {
      return null;
    },
  },
  loading: {
    type: Boolean,
    required: false,
    default: false,
  },
  functions: {
    type: Array,
    required: true,
    default: () => {
      return [];
    },
  },
  associatedFunctions: {
    type: Array,
    required: true,
    default: () => {
      return [];
    },
  },
});

const emit = defineEmits([
  "update:node",
  "cancel:hideform",
  "delete:node",
  "add:function",
]);

const { t } = useI18n();

const addFunctionRef: any = ref(null);

const isUpdating = ref(false);

const selectedFunction = ref("");

const functionOrder = ref(props.defaultOrder);

const filteredFunctions: Ref<any[]> = ref([]);

const createNewFunction = ref(false);

const store = useStore();

const functionExists = ref(false);

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

watch(
  () => props.functions,
  (newVal) => {
    filteredFunctions.value = [...newVal];
  },
  {
    deep: true,
    immediate: true,
  }
);

onBeforeMount(() => {
  filteredFunctions.value = [...props.functions];

  if (props.functionData) {
    isUpdating.value = true;
    selectedFunction.value = props.functionData.name;
    functionOrder.value = props.functionData.order;
  }
});

const openCancelDialog = () => {
  if (
    selectedFunction.value === (props.functionData?.name || "") &&
    functionOrder.value === (props.functionData?.order || 1)
  ) {
    emit("cancel:hideform");
    return;
  }

  dialog.value.show = true;
  dialog.value.title = "Discard Changes";
  dialog.value.message = "Are you sure you want to cancel changes?";
  dialog.value.okCallback = () => emit("cancel:hideform");
};

const openDeleteDialog = () => {
  dialog.value.show = true;
  dialog.value.title = "Delete Node";
  dialog.value.message =
    "Are you sure you want to delete function association?";
  dialog.value.okCallback = deleteFunction;
};

const saveFunction = () => {
  functionExists.value = false;

  if (createNewFunction.value) {
    addFunctionRef.value.onSubmit();
    return;
  }

  if (
    !isUpdating.value &&
    selectedFunction.value &&
    props.associatedFunctions.includes(selectedFunction.value)
  ) {
    functionExists.value = true;
    return;
  }

  emit("update:node", {
    data: { name: selectedFunction.value, order: functionOrder.value },
    link: nodeLink.value,
  });
  emit("cancel:hideform");
};

const onFunctionCreation = async (_function: any) => {
  // Assign newly created function to the block
  createNewFunction.value = false;
  emit("add:function", _function);
  await nextTick();
  selectedFunction.value = _function.name;
};

const cancelFunctionCreation = () => {
  emit("cancel:hideform");
};

const saveUpdatedLink = (link: { from: string; to: string }) => {
  nodeLink.value = link;
};

const deleteFunction = () => {
  emit("delete:node", { data: props.functionData, type: "function" });
  emit("cancel:hideform");
};
</script>

<style scoped>
.stream-routing-title {
  font-size: 20px;
  padding-top: 16px;
}
.stream-routing-container {
  border-radius: 8px;
  /* box-shadow: 0px 0px 10px 0px #d2d1d1; */
}
</style>

<style lang="scss">
.pipeline-add-function {
  .add-function-header,
  .q-separator {
    display: none;
  }

  .add-function-actions {
    display: none;
  }

  .add-function-name-input {
    width: 100%;
    margin-left: 0px !important;

    label {
      width: 100%;
      padding-left: 0;
    }
  }
}
</style>
