<template>
  <div class="q-my-sm q-px-sm">
    <div
      data-test="add-report-share-title-input"
      class="report-name-input o2-input"
    >
      <q-input
        v-model="alert.title"
        :label="t('reports.title') + ' *'"
        color="input-border"
        bg-color="input-bg"
        class="showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        :rules="[(val: any) => !!val.trim() || 'Field is required!']"
        tabindex="0"
        style="width: 400px"
      />
    </div>

    <div
      data-test="add-report-share-recipients-input"
      class="report-name-input o2-input"
    >
      <q-input
        v-model="alert.emails"
        :label="t('reports.recipients') + ' *'"
        color="input-border"
        bg-color="input-bg"
        class="showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        :rules="[(val: any) => /^([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})(\s*[;,]\s*([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}))*$/.test(val) || 'Add valid emails!']"
        tabindex="0"
        style="width: 100%"
        borderless
        :placeholder="t('user.inviteByEmail')"
      />
    </div>
    <div data-test="add-report-share-message-section">
      <div style="font-size: 14px" class="text-bold text-grey-8">Message</div>

      <div data-test="add-report-share-message-input">
        <q-input v-model="alert.message" filled type="textarea" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const alert = ref({
  type: "email",
  emails: "",
  message: "",
  title: "",
});
</script>

<style scoped></style>
