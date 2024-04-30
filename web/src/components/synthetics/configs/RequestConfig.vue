<template>
  <div>
    <div class="flex items-center o2-input">
      <div class="request-type-selector">
        <q-select
          v-model="request.type"
          :options="requestTypeOptions"
          :label="t('common.url') + ' *'"
          color="input-border"
          bg-color="input-bg"
          class="q-py-sm showLabelOnTop no-case"
          stack-label
          outlined
          filled
          dense
          :rules="[(val: any) => !!val || 'Field is required!']"
        />
      </div>
      <div class="request-url">
        <q-input
          v-model="request.url"
          color="input-border"
          bg-color="input-bg"
          class="showLabelOnTop q-mb-sm"
          placeholder="https://example.com"
          stack-label
          outlined
          filled
          dense
          tabindex="0"
        />
      </div>
      <div class="q-mt-md">
        <q-btn
          data-test="add-alert-submit-btn"
          :label="t('common.send')"
          class="text-bold no-border"
          color="secondary"
          padding="sm xl"
          type="submit"
          no-caps
        />
      </div>
    </div>
    <div>
      <app-tabs />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import AppTabs from "src/components/common/AppTabs.vue";

const { t } = useI18n();

// create array of strings
const requestTypeOptions = [
  "GET",
  "POST",
  "PUT",
  "DELETE",
  "PATCH",
  "HEAD",
  "OPTIONS",
];

const request = ref({
  type: "GET",
  url: "",
});

const selectedRequestType = ref<string>("GET");
</script>

<style scoped lang="scss">
.request-type-selector {
  width: 120px;
}

.request-url {
  width: 400px;
}
</style>
