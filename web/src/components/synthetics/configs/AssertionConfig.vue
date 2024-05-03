<template>
  <template v-for="(assertion, index) in assertions" :key="assertion.id">
    <div class="flex items-center justify-start o2-input" style="width: 900px">
      <div class="flex items-center justify-start" style="width: 820px">
        <div :data-test="`request-define-assertion-${index}`" class="q-mr-sm">
          <q-select
            v-model="assertion.type"
            :options="assertionTypes"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm no-case"
            emit-value
            map-options
            outlined
            filled
            dense
            :rules="[(val: any) => !!val || 'Field is required!']"
            style="width: 180px"
          />
        </div>
        <div
          v-if="
            assertion.type === 'body' ||
            assertion.type === 'body_hash' ||
            assertion.type === 'status_code'
          "
          class="flex items-center justify-start"
        >
          <div
            :data-test="`request-define-assertion-operator-${index}`"
            class="q-mr-sm"
          >
            <q-select
              v-model="assertion.operator"
              :options="assertionOperators[assertion.type]"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm no-case"
              emit-value
              map-options
              stack-label
              outlined
              filled
              dense
              style="width: 200px"
              :rules="[(val: any) => !!val || 'Field is required!']"
            />
          </div>
          <div
            :data-test="`request-define-assertion-value-${index}`"
            class="q-mr-sm q-mt-sm"
          >
            <q-input
              v-model="assertion.value"
              color="input-border"
              bg-color="input-bg"
              class="q-mb-sm"
              stack-label
              outlined
              filled
              dense
              tabindex="0"
              style="width: 200px"
            />
          </div>
        </div>

        <div
          v-if="assertion.type === 'header'"
          class="flex items-center justify-start"
        >
          <div
            :data-test="`request-define-assertion-key-${index}`"
            class="q-mr-sm q-mt-sm"
          >
            <q-input
              v-model="assertion.key"
              color="input-border"
              bg-color="input-bg"
              class="q-mb-sm"
              stack-label
              outlined
              filled
              dense
              tabindex="0"
              style="width: 200px"
            />
          </div>
          <div
            :data-test="`request-define-assertion-operator-${index}`"
            class="q-mr-sm"
          >
            <q-select
              v-model="assertion.operator"
              :options="assertionOperators[assertion.type]"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm no-case"
              emit-value
              map-options
              outlined
              filled
              dense
              :rules="[(val: any) => !!val || 'Field is required!']"
              style="width: 200px"
            />
          </div>
          <div
            :data-test="`request-define-assertion-value-${index}`"
            class="q-mr-sm q-mt-sm"
          >
            <q-input
              v-model="assertion.value"
              color="input-border"
              bg-color="input-bg"
              class="q-mb-sm"
              stack-label
              outlined
              filled
              dense
              tabindex="0"
              style="width: 200px"
            />
          </div>
        </div>

        <div
          v-if="assertion.type === 'response_time'"
          class="flex items-center justify-start"
        >
          <div
            :data-test="`request-define-assertion-timingScope-${index}`"
            class="q-mr-sm"
          >
            <q-select
              v-model="assertion.timingScope"
              :options="timeScopeOptions"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm no-case"
              emit-value
              map-options
              outlined
              filled
              dense
              :rules="[(val: any) => !!val || 'Field is required!']"
              style="width: 200px"
            />
          </div>
          <div
            :data-test="`request-define-assertion-operator-${index}`"
            class="q-mr-sm"
          >
            <q-select
              v-model="assertion.operator"
              :options="assertionOperators[assertion.type]"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm no-case"
              emit-value
              map-options
              outlined
              filled
              dense
              :rules="[(val: any) => !!val || 'Field is required!']"
              style="width: 200px"
            />
          </div>
          <div
            :data-test="`request-define-assertion-value-${index}`"
            class="q-mr-sm q-mt-sm"
          >
            <q-input
              v-model="assertion.value"
              color="input-border"
              bg-color="input-bg"
              class="q-mb-sm"
              outlined
              filled
              dense
              tabindex="0"
              style="width: 200px"
            />
          </div>
        </div>
      </div>

      <div class="col-2 q-ml-none">
        <q-btn
          data-test="alert-variables-delete-variable-btn"
          :icon="outlinedDelete"
          class="q-ml-xs iconHoverBtn"
          :class="store.state?.theme === 'dark' ? 'icon-dark' : ''"
          padding="sm"
          unelevated
          size="sm"
          round
          flat
          :title="t('alert_templates.edit')"
          @click="removeAssertion(assertion)"
        />
        <q-btn
          data-test="alert-variables-add-variable-btn"
          v-if="index === assertions.length - 1"
          icon="add"
          class="q-ml-xs iconHoverBtn"
          :class="store.state?.theme === 'dark' ? 'icon-dark' : ''"
          padding="sm"
          unelevated
          size="sm"
          round
          flat
          :title="t('alert_templates.edit')"
          @click="addAssertion"
        />
      </div>
    </div>
  </template>
</template>

<script setup lang="ts">
import { getUUID } from "@/utils/zincutils";
import { computed, ref, type Ref, type WritableComputedRef } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";

interface Assertion {
  operator: string;
  type: string;
  value: string;
  key?: string;
  timingScope?: string;
  id: string;
}

const props = defineProps({
  modelValue: {
    type: Array,
    required: true,
    default: () => [],
  },
});

const emit = defineEmits(["update:modelValue"]);

const assertionTypes = [
  {
    label: "body",
    value: "body",
  },
  {
    label: "body hash",
    value: "body_hash",
  },
  {
    label: "header",
    value: "header",
  },
  {
    label: "response time",
    value: "response_time",
  },
  {
    label: "status code",
    value: "status_code",
  },
];

const { t } = useI18n();

const store = useStore();

const assertions: WritableComputedRef<Assertion[]> = computed({
  get: () => props.modelValue as Assertion[],
  set: (value) => {
    emit("update:modelValue", value);
  },
});

const timeScopeOptions = [
  {
    label: "without DNS",
    value: "withoutDNS",
  },
  {
    label: "including DNS",
    value: "includingDNS",
  },
];

const assertionOperators = {
  body: [
    {
      label: "contains",
      value: "contains",
    },
    {
      label: "does not contains",
      value: "not_contains",
    },
    {
      label: "equals",
      value: "equals",
    },
    {
      label: "does not equals",
      value: "not_equals",
    },
    {
      label: "mathces regex",
      value: "matches_regex",
    },
    {
      label: "does not mathces regex",
      value: "not_matches_regex",
    },
    {
      label: "jsonpath",
      value: "json_path",
    },
    {
      label: "jsonschema",
      value: "json_schema",
    },
    {
      label: "xpath",
      value: "xpath",
    },
  ],
  body_hash: [
    {
      label: "md5",
      value: "md5",
    },
    {
      label: "sha1",
      value: "sha1",
    },
    {
      label: "sha256",
      value: "sha256",
    },
  ],
  header: [
    {
      label: "=",
      value: "=",
    },
    {
      label: "!=",
      value: "!=",
    },
    {
      label: "<",
      value: "<",
    },
    {
      label: "<=",
      value: "<=",
    },
    {
      label: ">",
      value: ">",
    },
    {
      label: ">=",
      value: ">=",
    },
    {
      label: "contains",
      value: "contains",
    },
    {
      label: "does not contains",
      value: "not_contains",
    },
    {
      label: "mathces regex",
      value: "matches_regex",
    },
    {
      label: "does not mathces regex",
      value: "not_matches_regex",
    },
    {
      label: "does not exists",
      value: "does_not_exists",
    },
  ],
  status_code: [
    {
      label: "=",
      value: "=",
    },
    {
      label: "!=",
      value: "!=",
    },
    {
      label: "mathces regex",
      value: "matches_regex",
    },
    {
      label: "does not mathces regex",
      value: "not_matches_regex",
    },
  ],
  response_time: [
    {
      label: "<",
      value: "<",
    },
  ],
};

const addAssertion = () => {
  assertions.value.push({
    operator: "",
    type: "status_code",
    value: "",
    key: "",
    timingScope: "",
    id: getUUID(),
  });
};

const removeAssertion = (assertion: any) => {
  if (assertions.value.length === 1) {
    return;
  }
  assertions.value = assertions.value.filter((a) => a.id !== assertion.id);
};
</script>
<style scoped></style>
