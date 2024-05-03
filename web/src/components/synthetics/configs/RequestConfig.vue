<template>
  <div class="request-config o2-input">
    <div class="flex items-center">
      <div class="request-type-selector">
        <q-select
          v-model="requestTypeValue"
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
          v-model="requestUrlValue"
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
      <div class="request-submit-btn">
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
      <app-tabs
        :active-tab="activeConfigTab"
        :tabs="requestConfigTabs"
        @update:active-tab="updateActiveTab"
      />
      <q-separator />
      <div class="relative-position q-mt-sm config-content-container">
        <transition name="slide" mode="out-in">
          <div class="config-content" v-if="activeConfigTab === 'params'">
            <div class="q-mb-sm text-subtitle2 text-bold text-grey-8">
              Query Params
            </div>

            <div class="flex">
              <div
                class="params-key-title q-ml-xs q-mr-xs text-subtitle2 text-grey-9"
              >
                Key
              </div>
              <div
                class="params-value-title q-ml-xs text-subtitle2 text-grey-9"
              >
                Value
              </div>
            </div>
            <variables-input
              :variables="queryParams"
              @add:variable="addQueryParam"
              @remove:variable="(tab) => removeQueryParam(tab)"
            />
          </div>
          <div
            class="config-content flex"
            v-else-if="activeConfigTab === 'authorization'"
          >
            <div class="request-type-selector q-mr-lg">
              <q-select
                v-model="authMeta.type"
                :options="authTypes"
                :label="t('common.type') + ' *'"
                color="input-border"
                bg-color="input-bg"
                class="q-py-sm showLabelOnTop no-case"
                emit-value
                map-options
                stack-label
                outlined
                filled
                dense
                :rules="[(val: any) => !!val || 'Field is required!']"
              />
            </div>
            <div v-if="authMeta.type === 'basic'">
              <div class="request-url">
                <q-input
                  v-model="authMeta.basic.username"
                  color="input-border"
                  bg-color="input-bg"
                  :label="t('user.name') + ' *'"
                  class="showLabelOnTop q-mb-md"
                  placeholder="https://example.com"
                  stack-label
                  outlined
                  filled
                  dense
                  tabindex="0"
                />
              </div>
              <div class="request-url">
                <q-input
                  v-model="authMeta.basic.password"
                  color="input-border"
                  bg-color="input-bg"
                  :label="t('user.password') + ' *'"
                  class="showLabelOnTop q-mb-sm"
                  type="password"
                  stack-label
                  outlined
                  filled
                  dense
                  tabindex="0"
                />
              </div>
            </div>

            <div v-if="authMeta.type === 'bearer'">
              <div class="request-url">
                <q-input
                  v-model="authMeta.bearer.token"
                  color="input-border"
                  bg-color="input-bg"
                  :label="t('common.token') + ' *'"
                  class="showLabelOnTop q-mb-md"
                  stack-label
                  outlined
                  filled
                  dense
                  tabindex="0"
                />
              </div>
            </div>
          </div>
          <div class="config-content" v-else-if="activeConfigTab === 'headers'">
            <div class="q-mb-sm text-bold text-grey-8">Headers</div>

            <variables-input
              :variables="requestHeaders"
              @add:variable="addHeader"
              @remove:variable="(tab) => removeHeader(tab)"
            />
          </div>
          <div class="config-content" v-else-if="activeConfigTab === 'body'">
            <div class="request-body-type-selector q-mr-lg">
              <q-select
                v-model="bodyMeta.type"
                :options="bodyTypes"
                :label="t('synthetics.bodyType') + ' *'"
                color="input-border"
                bg-color="input-bg"
                input-class="no-case"
                options-selected-class="no-case"
                class="q-py-sm showLabelOnTop no-case"
                stack-label
                map-options
                emit-value
                outlined
                filled
                dense
                :rules="[(val: any) => !!val || 'Field is required!']"
              />
            </div>

            <div class="query-editor">
              <QueryEditor
                :key="bodyMeta.type"
                style="height: 300px; width: 100%"
                editorId="synthetics-request-body-editor"
                v-model:query="bodyMeta.content"
                :language="editorLanguage"
                @update:query="onQueryUpdate"
              />
            </div>
          </div>
        </transition>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import VariablesInput from "@/components/alerts/VariablesInput.vue";
import AppTabs from "@/components/common/AppTabs.vue";
import { getUUID } from "@/utils/zincutils";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import QueryEditor from "../QueryEditor.vue";

const props = defineProps({
  modelValue: {
    type: Object,
    required: true,
    default: () => ({}),
  },
});

const { t } = useI18n();

const emit = defineEmits(["update:modelValue"]);

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

const bodyTypes = [
  {
    label: "none",
    value: "none",
  },
  {
    label: "Raw",
    value: "raw",
  },
  {
    label: "Form Data",
    value: "form_data",
  },
  {
    label: "x-www-form-urlencoded",
    value: "x-www-form-urlencoded",
  },
  {
    label: "GraphQL",
    value: "graphql",
  },
  {
    label: "Text-Plain",
    value: "text_plain",
  },
  {
    label: "Text-JSON",
    value: "text_json",
  },
  {
    label: "Text-HTML",
    value: "text_html",
  },
  {
    label: "Text-XML",
    value: "text_xml",
  },
  {
    label: "Text-JavaScript",
    value: "text_javaScript",
  },
];

const request = ref({
  type: "GET",
  url: "",
  params: [
    {
      id: getUUID(),
      key: "",
      value: "",
    },
  ],
  headers: [
    {
      id: getUUID(),
      key: "",
      value: "",
    },
  ],
  auth: {
    type: "basic",
    basic: {
      username: "",
      password: "",
    },
    bearer: {
      token: "",
    },
  },
  body: {
    type: "raw",
    content: "",
  },
});

const authTypes = [
  {
    label: "Basic",
    value: "basic",
  },
  {
    label: "Bearer",
    value: "bearer",
  },
  {
    label: "OAuth 2.0",
    value: "oauth_2.0",
  },
];

const requestConfigTabs = [
  {
    value: "params",
    label: t("synthetics.params"),
    style: {
      width: "100px",
      margin: "0 10px",
    },
  },
  {
    value: "authorization",
    label: t("synthetics.authorization"),
    style: {
      width: "120px",
      margin: "0 10px",
    },
  },
  {
    value: "headers",
    label: t("synthetics.headers"),
    style: {
      width: "110px",
      margin: "0 10px",
    },
  },
  {
    value: "body",
    label: t("synthetics.body"),
    style: {
      width: "100px",
      margin: "0 10px",
    },
  },
];

const activeConfigTab = ref<string>("params");

const updateActiveTab = (tab: string) => {
  activeConfigTab.value = tab;
};

const editorLanguage = ref<string>("json");

watch(
  () => request.value.body.type,
  (newVal) => {
    if (newVal === "graphql") {
      editorLanguage.value = "graphql";
    }

    if (newVal === "text_plain") {
      editorLanguage.value = "markdown";
    }

    if (newVal === "text_json") {
      editorLanguage.value = "json";
    }

    if (newVal === "text_html") {
      editorLanguage.value = "html";
    }

    if (newVal === "text_xml") {
      editorLanguage.value = "xml";
    }

    if (newVal === "text_javaScript") {
      editorLanguage.value = "javascript";
    }
  }
);

const requestTypeValue = computed({
  get: () => props.modelValue.type,
  set: (val) => {
    emit("update:modelValue", { ...props.modelValue, type: val });
  },
});

const requestUrlValue = computed({
  get: () => props.modelValue.url,
  set: (val) => {
    emit("update:modelValue", { ...props.modelValue, url: val });
  },
});

const queryParams = computed({
  get: () => props.modelValue.params,
  set: (val) => {
    emit("update:modelValue", { ...props.modelValue, params: val });
  },
});

const requestHeaders = computed({
  get: () => props.modelValue.headers,
  set: (val) => {
    console.log(val);
    emit("update:modelValue", { ...props.modelValue, headers: val });
  },
});

const authMeta = computed({
  get: () => props.modelValue.auth,
  set: (val) => {
    emit("update:modelValue", {
      ...props.modelValue,
      auth: val,
    });
  },
});

const bodyMeta = computed({
  get: () => props.modelValue.body,
  set: (val) => {
    emit("update:modelValue", {
      ...props.modelValue,
      body: val,
    });
  },
});

const addQueryParam = () => {
  queryParams.value.push({
    id: getUUID(),
    key: "",
    value: "",
  });
};

const removeQueryParam = (tab: { id: string }) => {
  queryParams.value = queryParams.value.filter(
    (param: { id: string }) => param.id !== tab.id
  );
};

const addHeader = () => {
  requestHeaders.value.push({
    id: getUUID(),
    key: "",
    value: "",
  });
};

const removeHeader = (tab: { id: string }) => {
  requestHeaders.value = requestHeaders.value.filter(
    (param: { id: string }) => param.id !== tab.id
  );
};

const onQueryUpdate = (query: string) => {
  console.log(query);
};
</script>

<style scoped lang="scss">
.config-content-container {
  height: 380px;
  overflow-y: auto;
}
.config-content {
  height: 200px;
  width: 100%;
  padding: 10px;
  box-sizing: border-box;
  position: absolute; /* Position absolutely within the parent */
  top: 0;
  left: 0;
}

.slide-enter-active,
.slide-leave-active {
  transition: transform 0.1s ease-in-out;
}

.slide-enter-from,
.slide-leave-to {
  opacity: 0;
  transform: translateY(-5px); /* Start from or move to the right */
}

.slide-enter-to,
.slide-leave-from {
  opacity: 1;
  transform: translateY(0); /* Move to or start from the center */
}

.request-type-selector {
  width: 120px;
}

.request-body-type-selector {
  width: 250px;
}

.request-url {
  width: 400px;
}

.params-key-title,
.params-value-title {
  width: 250px;
}

.query-editor {
  height: 100px;
  width: 100%;
}
</style>

<style lang="scss">
.request-config {
  .variables-value-input {
    width: 400px;
  }

  .request-body-type-selector {
    .q-field--labeled.showLabelOnTop.q-select
      .q-field__control-container
      .q-field__native
      > :first-child {
      text-transform: none;
    }
  }

  .request-submit-btn {
    margin-top: 18px;
    button {
      padding: 6px 32px !important;
      border-top-left-radius: 0;
      border-bottom-left-radius: 0;
    }
  }
}
</style>
