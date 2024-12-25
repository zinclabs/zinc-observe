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
  <div class="cipher-keys-add-akeyless-type">
    <!-- Add input filed for base URL, access id with URL validation from q-input-->
    <q-input
      data-test="add-cipher-key-name-input"
      v-model="formData.key.store.akeyless.base_url"
      :label="t('cipherKey.name') + ' *'"
      color="input-border"
      bg-color="input-bg"
      class="showLabelOnTop full-width"
      stack-label
      outlined
      filled
      :rules="[(val: any) => !!val || t('validation.required')]"
    />
    <!-- Add input filed for access id with URL validation from q-input-->
    <q-input
      data-test="add-cipher-key-access-id-input"
      v-model="formData.key.store.akeyless.access_id"
      :label="t('cipherKey.accessId') + ' *'"
      color="input-border"
      bg-color="input-bg"
      class="showLabelOnTop full-width"
      stack-label
      outlined
      filled
      :rules="[
        (val: any) => !!val || t('validation.required'),
        (val: string) =>
          /^[a-zA-Z0-9-]*$/.test(val) || t('validation.invalidAccessId'),
      ]"
    />
    <q-label class="q-mb-md"
color="primary" tag="h2"
      >{{ t("cipherKey.secret") }}:</q-label
    >
    <div resize="vertical" class="q-mb-md">
      <query-editor
        data-test="cipher-key-secret-sql-editor"
        ref="queryEditorRef"
        editor-id="alerts-query-editor"
        class="monaco-editor"
        :debounceTime="300"
        v-model:query="query"
        language="json"
        :class="query == '' && queryEditorPlaceholderFlag ? 'empty-query' : ''"
        @update:query="updateQueryValue"
        @focus="queryEditorPlaceholderFlag = false"
        @blur="onBlurQueryEditor"
      />
    </div>
  </div>
</template>

<script lang="ts" setup>
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { defineAsyncComponent } from "vue";

const query = ref("");
const { t } = useI18n();
const queryEditorPlaceholderFlag = ref(true);
const props = defineProps({
  formData: Object,
});

const updateQueryValue = (value: string) => {
  query.value = value;
};

const onBlurQueryEditor = () => {
  if (query.value === "") {
    queryEditorPlaceholderFlag.value = true;
  }
};

const QueryEditor = defineAsyncComponent(
  () => import("@/components/QueryEditor.vue"),
);
</script>

<style lang="scss">
.cipher-keys-add-akeyless-type {
  .monaco-editor {
    width: 500px !important;
    height: 100px !important;
    border: 1px solid $border-color;
  }
}
</style>
