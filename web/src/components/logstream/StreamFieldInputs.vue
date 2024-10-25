<template>
  <div data-test="add-stream-fields-section">
    <div v-if="showHeader" data-test="alert-conditions-text" class="text-bold">
      Fields *
    </div>
    <template v-if="!fields.length">
      <q-btn
        data-test="add-stream-add-field-btn"
        color="primary"
        class="q-mt-sm text-bold add-field"
        label="Add Field"
        size="sm"
        icon="add"
        style="
          border-radius: 4px;
          text-transform: capitalize;
          background: #f2f2f2 !important;
          color: #000 !important;
        "
        @click="addApiHeader"
      />
    </template>
    <template v-else>
      <div
        v-for="(field, index) in (fields as any)"
        :key="field.uuid"
        class="flex justify-start items-end q-col-gutter-sm q-pb-sm"
        :data-test="`alert-conditions-${index + 1}`"
      >
        <div data-test="add-stream-field-name-input" class="q-ml-none o2-input">
          <q-input
            v-model="field.name"
            :placeholder="t('common.name') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm"
            stack-label
            outlined
            filled
            dense
            :rules="[(val: any) => !!val.trim() || 'Field is required!']"
            tabindex="0"
            style="min-width: 350px"
          />
        </div>
        <!-- <div
          v-if="visibleInputs.type"
          data-test="alert-conditions-operator-select"
          class="q-ml-none o2-input"
        >
          <q-select
            v-model="field.type"
            :options="fieldTypes"
            :popup-content-style="{ textTransform: 'capitalize' }"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm"
            stack-label
            outlined
            filled
            dense
            :rules="[(val: any) => !!val || 'Field is required!']"
            style="min-width: 120px"
            @update:model-value="emits('input:update', 'conditions', field)"
          />
        </div> -->
        <div
          v-if="visibleInputs.index_type"
          data-test="add-stream-field-type-select-input"
          class="q-ml-none flex items-end o2-input"
        >
          <q-select
            v-model="field.index_type"
            :options="streamIndexType"
            :popup-content-style="{ textTransform: 'capitalize' }"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm stream-schema-index-select"
            multiple
            :max-values="2"
            map-options
            :option-disable="(_option: any) => disableOptions(field, _option)"
            emit-value
            clearable
            stack-label
            outlined
            filled
            dense
            :rules="[(val: any) => !!val || 'Field is required!']"
            style="width: 300px"
            @update:model-value="emits('input:update', 'conditions', field)"
          />
        </div>
        <div
          class="q-ml-none alerts-condition-action"
          style="margin-bottom: 12px"
        >
          <q-btn
            data-test="add-stream-delete-field-btn"
            :icon="outlinedDelete"
            class="q-ml-xs iconHoverBtn"
            :class="store.state?.theme === 'dark' ? 'icon-dark' : ''"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            :title="t('alert_templates.edit')"
            @click="deleteApiHeader(field, index)"
            style="min-width: auto"
          />
          <q-btn
            data-test="add-stream-add-field-btn"
            v-if="index === fields.length - 1"
            icon="add"
            class="q-ml-xs iconHoverBtn"
            :class="store.state?.theme === 'dark' ? 'icon-dark' : ''"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            :title="t('alert_templates.edit')"
            @click="addApiHeader()"
            style="min-width: auto"
          />
        </div>
      </div>
    </template>
  </div>
</template>

<script lang="ts" setup>
import { defineProps, ref } from "vue";
import { useI18n } from "vue-i18n";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";
import { useStore } from "vuex";

defineProps({
  fields: {
    type: Array,
    default: () => [],
    required: true,
  },
  showHeader: {
    type: Boolean,
    default: true,
  },
  visibleInputs: {
    type: Object,
    default: () => ({
      name: true,
      type: true,
      index_type: true,
    }),
  },
});

const emits = defineEmits(["add", "remove", "input:update"]);

const streamIndexType = [
  { label: "Full text search", value: "fullTextSearchKey" },
  { label: "Secondary index", value: "secondaryIndexKey" },
  { label: "Bloom filter", value: "bloomFilterKey" },
  { label: "KeyValue partition", value: "keyPartition" },
  { label: "Prefix partition", value: "prefixPartition" },
  { label: "Hash partition (8 Buckets)", value: "hashPartition_8" },
  { label: "Hash partition (16 Buckets)", value: "hashPartition_16" },
  { label: "Hash partition (32 Buckets)", value: "hashPartition_32" },
  { label: "Hash partition (64 Buckets)", value: "hashPartition_64" },
  { label: "Hash partition (128 Buckets)", value: "hashPartition_128" },
];

const fieldTypes = [
  {
    label: "String",
    value: "Utf8",
  },
  {
    label: "Integer",
    value: "Int64",
  },
  {
    label: "Float",
    value: "Float64",
  },
];

const store = useStore();

const { t } = useI18n();

const deleteApiHeader = (field: any, index: number) => {
  emits("remove", field, index);
  emits("input:update", "conditions", field);
};

const addApiHeader = () => {
  emits("add");
};

const disableOptions = (schema: any, option: any) => {
  let selectedHashPartition = "";

  let selectedIndices = "";

  for (let i = 0; i < (schema?.index_type || []).length; i++) {
    if (schema.index_type[i].includes("hashPartition")) {
      selectedHashPartition = schema.index_type[i];
    }
    selectedIndices += schema.index_type[i];
  }
  if(selectedIndices.includes('prefixPartition') && option.value.includes('keyPartition')){
        return true;
      }
  if(selectedIndices.includes('keyPartition') && option.value.includes('prefixPartition')){
    return true;
  }
  if (
    selectedIndices.includes("hashPartition") &&
    selectedHashPartition !== option.value &&
    (option.value.includes("hashPartition") ||
      option.value.includes("keyPartition") || option.value.includes("prefixPartition"))

  )
    return true;
  if (
    ( selectedIndices.includes("keyPartition") || selectedIndices.includes("prefixPartition"))&&
    option.value.includes("hashPartition")
  )
    return true;

  return false;
};
</script>

<style lang="scss">
.add-field {
  .q-icon {
    margin-right: 4px !important;
    font-size: 15px !important;
  }
}

.alerts-condition-action {
  .q-btn {
    &.icon-dark {
      filter: none !important;
    }
  }
}
</style>
