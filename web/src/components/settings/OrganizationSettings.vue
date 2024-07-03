<template>
  <div class="q-px-md q-pt-md q-pb-md">
    <div class="text-body1 text-bold">
      {{ t("settings.logDetails") }}
    </div>
  </div>

  <div class="q-mx-md q-mb-md">
    <div
      data-test="add-role-rolename-input-btn"
      class="trace-id-field-name o2-input q-mb-sm"
    >
      <q-input
        v-model.trim="traceIdFieldName"
        :label="t('settings.traceIdFieldName') + ' *'"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        outlined
        stack-label
        filled
        dense
        :rules="[
          (val, rules) =>
            !!val
              ? isValidTraceField ||
                `Use alphanumeric and '+=,.@-_' characters only, without spaces.`
              : t('common.nameRequired'),
        ]"
      >
        <template v-slot:hint>
          Use alphanumeric and '+=,.@-_' characters only, without spaces.
        </template>
      </q-input>
    </div>

    <div
      data-test="add-role-rolename-input-btn"
      class="span-id-field-name o2-input"
    >
      <q-input
        v-model.trim="spanIdFieldName"
        :label="t('settings.spanIdFieldName') + ' *'"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        :rules="[
          (val, rules) =>
            !!val
              ? isValidSpanField ||
                `Use alphanumeric and '+=,.@-_' characters only, without spaces.`
              : t('common.nameRequired'),
        ]"
        @update:model-value="updateFieldName('span')"
      >
        <template v-slot:hint>
          Use alphanumeric and '+=,.@-_' characters only, without spaces.
        </template>
      </q-input>
    </div>

    <div class="flex justify-start q-mt-lg">
      <q-btn
        data-test="add-alert-cancel-btn"
        v-close-popup="true"
        class="q-mb-md text-bold"
        :label="t('alerts.cancel')"
        text-color="light-text"
        padding="sm md"
        no-caps
        @click="$emit('cancel:hideform')"
      />
      <q-btn
        data-test="add-alert-submit-btn"
        :label="t('alerts.save')"
        class="q-mb-md text-bold no-border q-ml-md"
        color="secondary"
        padding="sm xl"
        no-caps
        @click="saveOrgSettings"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import organizations from "@/services/organizations";
import { useStore } from "vuex";

const { t } = useI18n();
const traceIdFieldName = ref("trace_id");
const spanIdFieldName = ref("span_id");
const store = useStore();

const isValidSpanField = ref(true);
const isValidTraceField = ref(true);

const isValidRoleName = computed(() => {
  const roleNameRegex = /^[a-zA-Z0-9+=,.@_-]+$/;
  // Check if the role name is valid
  return roleNameRegex.test(traceIdFieldName.value);
});

const validateFieldName = (value: string) => {
  const roleNameRegex = /^[a-zA-Z0-9+=,.@_-]+$/;
  // Check if the role name is valid
  return roleNameRegex.test(value);
};

const updateFieldName = (fieldName: string) => {
  if (fieldName === "span")
    isValidSpanField.value = validateFieldName(spanIdFieldName.value);

  if (fieldName === "trace")
    isValidTraceField.value = validateFieldName(traceIdFieldName.value);
};

const saveOrgSettings = async () => {
  try {
    await organizations.post_organization_settings(
      store.state.selectedOrganization.identifier,
      {
        trace_id_field_name: traceIdFieldName.value,
        span_id_field_name: spanIdFieldName.value,
      }
    );

    store.dispatch("setOrganizationSettings", {
      ...store.state?.organizationData?.organizationSettings,
      trace_id_field_name: traceIdFieldName.value,
      span_id_field_name: spanIdFieldName.value,
    });
  } catch (e) {
    console.log("Error saving organization settings");
  }
};
</script>

<style scoped lang="scss">
.trace-id-field-name,
.span-id-field-name {
  width: 400px;
}
</style>
