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
  <q-page class="q-pa-none" style="min-height: inherit">
    <div class="row items-center no-wrap q-mx-md q-my-sm">
      <div class="flex items-center">
        <div
          class="flex justify-center items-center q-mr-md cursor-pointer"
          style="
            border: 1.5px solid;
            border-radius: 50%;
            width: 22px;
            height: 22px;
          "
          title="Go Back"
          @click="$emit('cancel:hideform')"
        >
          <q-icon name="arrow_back_ios_new" size="14px" />
        </div>
        <div class="col" data-test="add-template-title">
          <div v-if="isUpdatingTemplate" class="text-h6">
            {{ t("alert_templates.updateTitle") }}
          </div>
          <div v-else class="text-h6">
            {{ t("alert_templates.addTitle") }}
          </div>
        </div>
      </div>
    </div>

    <q-separator style="width: 100%" />

    <q-splitter
      v-model="splitterModel"
      unit="%"
      style="height: calc(100vh - 106px)"
    >
      <template v-slot:before>
        <div class="row q-pa-md">
          <div class="col-12 q-pb-sm q-pt-sm o2-input">
            <q-input
              data-test="add-template-name-input"
              v-model="formData.name"
              :label="t('alerts.name') + ' *'"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              v-bind:readonly="isUpdatingTemplate"
              v-bind:disable="isUpdatingTemplate"
              :rules="[
                (val: any) =>
                  !!val
                    ? isValidResourceName(val) ||
                      `Characters like :, ?, /, #, and spaces are not allowed.`
                    : t('common.nameRequired'),
              ]"
              tabindex="0"
            />
          </div>
          <div class="col-12 q-pb-md">
            <app-tabs
              style="
                border: 1px solid #8a8a8a;
                border-radius: 4px;
                overflow: hidden;
                width: fit-content;
              "
              :tabs="tabs"
              v-model:active-tab="formData.type"
            />
          </div>
          <div v-if="formData.type === 'email'" class="col-12 q-pt-xs o2-input">
            <q-input
              data-test="add-template-email-title-input"
              v-model="formData.title"
              :label="t('alerts.title') + ' *'"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[(val: any) => !!val.trim() || 'Field is required!']"
              tabindex="0"
            />
          </div>
          <div class="col-12 q-pb-md">
            <div
              class="q-pb-sm text-bold"
              data-test="add-template-body-input-title"
            >
              {{ t("alert_templates.body") + " *" }}
            </div>
            <template v-if="formData.type === 'email'">
              <query-editor
                data-test="template-body-editor"
                ref="queryEditorRef"
                editor-id="template-body-editor"
                class="monaco-editor q-mb-md"
                language="markdown"
                v-model:query="formData.body"
              />
            </template>
            <template v-else>
              <query-editor
                data-test="template-body-editor"
                ref="queryEditorRef"
                editor-id="template-body-editor"
                class="monaco-editor q-mb-md"
                language="json"
                v-model:query="formData.body"
              />
            </template>
          </div>
          <div class="col-12 flex justify-center">
            <q-btn
              data-test="add-template-cancel-btn"
              v-close-popup="true"
              class="text-bold"
              :label="t('alerts.cancel')"
              text-color="light-text"
              padding="sm md"
              no-caps
              @click="$emit('cancel:hideform')"
            />
            <q-btn
              data-test="add-template-submit-btn"
              :label="t('alerts.save')"
              class="text-bold no-border q-ml-md"
              color="secondary"
              padding="sm xl"
              @click="saveTemplate"
              no-caps
            />
          </div>
        </div>
      </template>
      <template v-slot:after>
        <div class="q-px-sm q-pt-sm">
          <div class="text-bold q-py-sm q-px-xs text-subtitle2">
            {{ t("alert_templates.variable_guide_header") }}
          </div>
          <q-separator style="width: 100%" />
          <div class="q-py-md q-px-xs">
            <div>org_name, stream_type, stream_name</div>
            <div>alert_name, alert_type</div>
            <div>alert_period, alert_operator, alert_threshold</div>
            <div>alert_count, alert_agg_value</div>
            <div>alert_start_time, alert_end_time, alert_url</div>
            <div>alert_trigger_time, alert_trigger_time_str</div>
            <div><b>rows</b> multiple lines of row template</div>
            <div><b>All of the stream fields are variables.</b></div>
            <div>{rows:N} {var:N} used to limit rows or string length.</div>
          </div>
          <div class="q-pb-md q-px-xs">
            <div class="text-bold text-body-1 q-pb-sm">
              {{ t("alert_templates.variable_usage_examples") }}:
            </div>
            <div
              v-for="(template, index) in sampleTemplates"
              class="q-pb-md"
              :key="template.name"
              :data-test="`add-template-sample-template-${index}`"
            >
              <div class="flex justify-between items-center">
                <div class="q-pb-xs">{{ template.name }}</div>
                <q-icon
                  data-test="add-template-sample-template-copy-btn"
                  class="cursor-pointer"
                  name="content_copy"
                  size="14px"
                  @click="copyTemplateBody(template.body)"
                />
              </div>
              <div
                data-test="add-template-sample-template-text"
                class="add-template q-px-sm rounded-borders"
              >
                <pre class="example-template-body q-my-0">
                    {{ template.body }}
                  </pre
                >
              </div>
            </div>
          </div>
        </div>
      </template>
    </q-splitter>
  </q-page>
</template>
<script lang="ts" setup>
import {
  ref,
  onMounted,
  defineProps,
  onBeforeMount,
  onActivated,
  defineEmits,
  watch,
  computed,
} from "vue";
import type { Ref } from "vue";
import { useI18n } from "vue-i18n";

import templateService from "@/services/alert_templates";
import { useStore } from "vuex";
import { copyToClipboard, useQuasar } from "quasar";
import type { TemplateData, Template } from "@/ts/interfaces/index";
import { useRouter } from "vue-router";
import { isValidResourceName } from "@/utils/zincutils";
import AppTabs from "@/components/common/AppTabs.vue";
import QueryEditor from "@/components/QueryEditor.vue";

const props = defineProps<{ template: TemplateData | null }>();
const emit = defineEmits(["get:templates", "cancel:hideform"]);
const { t } = useI18n();
const splitterModel: Ref<number> = ref(75);
const formData: Ref<Template> = ref({
  name: "",
  body: "",
  type: "http",
  title: "",
});
const store = useStore();
const q = useQuasar();
const editorRef: any = ref(null);
const isUpdatingTemplate = ref(false);
const sampleTemplates = [
  {
    name: "Slack",
    body: `
{
  "text": "{alert_name} is active"
}`,
  },
  {
    name: "Alert Manager",
    body: `
[
  {
    "labels": {
        "alertname": "{alert_name}",
        "stream": "{stream_name}",
        "organization": "{org_name}",
        "alerttype": "{alert_type}",
        "severity": "critical"
    },
    "annotations": {
        "timestamp": "{timestamp}"
    }
  }
]`,
  },
];
onActivated(() => setupTemplateData());
onBeforeMount(() => {
  setupTemplateData();
});

const baseTabStyle = {
  width: "fit-content",
  padding: "4px 14px",
  border: "none !important",
};

const tabs = computed(() => [
  {
    label: "Web Hook",
    value: "http",
    style: {
      ...baseTabStyle,
      background: formData.value.type === "http" ? "#5960B2" : "",
      color: formData.value.type === "http" ? "#ffffff !important" : "",
    },
  },
  {
    label: "Email",
    value: "email",
    style: {
      ...baseTabStyle,
      background: formData.value.type === "email" ? "#5960B2" : "#ffffff",
      color: formData.value.type === "email" ? "#ffffff !important" : "",
    },
  },
]);

const setupTemplateData = () => {
  const params = router.currentRoute.value.query;
  if (props.template) {
    isUpdatingTemplate.value = true;
    formData.value.name = props.template.name;
    formData.value.body = props.template.body;
    formData.value.type = props.template.type;
    formData.value.title = props.template.title;
  }

  if (params.type) {
    formData.value.type = params.type as "email" | "http";
  }
};

const isTemplateBodyValid = () => {
  try {
    JSON.parse(formData.value.body);
    return true;
  } catch (e) {
    q.notify({
      type: "negative",
      message: "Please enter valid JSON in template body",
      timeout: 1500,
    });
    return false;
  }
};

const router = useRouter();

const isTemplateFilled = () =>
  formData.value.name.trim().trim().length &&
  formData.value.body.trim().trim().length;

const saveTemplate = () => {
  if (!isTemplateFilled()) {
    q.notify({
      type: "negative",
      message: "Please fill required fields",
      timeout: 1500,
    });
    return;
  }

  // Here checking is template body json valid
  if (formData.value.type !== "email" && !isTemplateBodyValid()) return;

  const dismiss = q.notify({
    spinner: true,
    message: "Please wait...",
    timeout: 2000,
  });

  if (isUpdatingTemplate.value) {
    templateService
      .update({
        org_identifier: store.state.selectedOrganization.identifier,
        template_name: formData.value.name,
        data: {
          name: formData.value.name.trim(),
          body: formData.value.body,
          type: formData.value.type,
          title: formData.value.title,
        },
      })
      .then(() => {
        dismiss();
        emit("get:templates");
        emit("cancel:hideform");
        q.notify({
          type: "positive",
          message: `Template Saved Successfully.`,
        });
      })
      .catch((err) => {
      if(err.response?.status == 403){
          return;
        }
        dismiss();
        q.notify({
          type: "negative",
          message: err.response?.data?.error || err.response?.data?.message,
        });
      });
  } else {
    {
      templateService
        .create({
          org_identifier: store.state.selectedOrganization.identifier,
          template_name: formData.value.name,
          data: {
            name: formData.value.name.trim(),
            body: formData.value.body,
            type: formData.value.type,
            title: formData.value.title,
          },
        })
        .then(() => {
          dismiss();
          emit("get:templates");
          emit("cancel:hideform");
          q.notify({
            type: "positive",
            message: `Template Saved Successfully.`,
          });
        })
        .catch((err) => {
          if(err.response?.status == 403){
            return;
          }
          dismiss();
          q.notify({
            type: "negative",
            message: err.response?.data?.error || err.response?.data?.message,
          });
        });
    }
  }
};
const copyTemplateBody = (text: any) => {
  copyToClipboard(JSON.parse(JSON.stringify(text))).then(() =>
    q.notify({
      type: "positive",
      message: "Content Copied Successfully!",
      timeout: 1000,
    })
  );
};
</script>
<style lang="scss" scoped>
.monaco-editor {
  width: 100%;
  min-height: 310px !important;
  border-radius: 5px;
  // padding-bottom: 14px;
  resize: vertical;
  overflow: auto;
}

.example-template-body {
  font-size: 10px;
}

.add-template {
  background: rgba(0, 0, 0, 0.07);
}
</style>
