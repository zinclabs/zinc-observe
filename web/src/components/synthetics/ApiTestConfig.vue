<template>
  <div style="height: calc(100vh - 114px); overflow-y: auto">
    <div class="row items-center no-wrap q-mx-md q-my-sm">
      <div class="flex items-center">
        <div
          data-test="add-report-back-btn"
          class="flex justify-center items-center q-mr-md cursor-pointer"
          style="
            border: 1.5px solid;
            border-radius: 50%;
            width: 22px;
            height: 22px;
          "
          title="Go Back"
          @click="router.back()"
        >
          <q-icon name="arrow_back_ios_new" size="14px" />
        </div>
        <div v-if="isEditingTest" class="text-h6" data-test="add-report-title">
          {{ apiConfig.name }}
        </div>
        <div v-else class="text-h6" data-test="add-report-title">
          {{ t("synthetics.addApiTest") }}
        </div>
      </div>
    </div>
    <q-separator />
    <div class="q-ma-md">
      <q-stepper
        v-model="step"
        vertical
        color="primary"
        animated
        class="q-mt-md"
        header-nav
      >
        <q-form @submit="saveTest">
          <q-step
            data-test="api-test-define-request"
            :name="1"
            prefix="1"
            title="Define Request"
            :done="step > 1"
          >
            <recorder-request-config
              v-if="isBrowserTest"
              v-model="apiConfig.request"
            />
            <request-config
              v-else
              class="q-mt-md"
              v-model="apiConfig.request"
              :advanced="true"
              @update:model-value="onRequestConfigUpdate"
            />

            <q-stepper-navigation>
              <div class="q-ml-sm">
                <q-btn
                  data-test="add-report-step2-continue-btn"
                  @click="step = 2"
                  color="secondary"
                  label="Continue"
                  no-caps
                />
              </div>
            </q-stepper-navigation>
          </q-step>

          <q-step
            data-test="api-test-define-assertions"
            :name="2"
            prefix="2"
            :title="isBrowserTest ? 'Test Editor' : 'Define Assertion'"
            :done="step > 2"
          >
            <recorder-config v-if="isBrowserTest" class="q-mb-md" />
            <assertion-config
              v-else
              class="q-mt-md"
              v-model="apiConfig.assertions"
              @update:model-value="onRequestConfigUpdate"
            />

            <div
              class="tw-flex tw-items-center tw-justify-start q-ml-sm q-mt-lg"
            >
              <q-btn
                data-test="add-report-step2-continue-btn"
                @click="step = 3"
                color="secondary"
                label="Continue"
                no-caps
              />
              <q-btn
                data-test="add-report-step2-back-btn"
                flat
                @click="step = 1"
                color="primary"
                label="Back"
                class="q-ml-sm"
                no-caps
              />
            </div>
          </q-step>

          <q-step
            data-test="api-test-define-retry-condition"
            :name="3"
            prefix="3"
            title="Define Retry Condition"
            :done="step > 3"
          >
            <retry-config
              class="q-mt-md"
              v-model="apiConfig.retry"
              @update:model-value="onRequestConfigUpdate"
            />

            <div
              class="tw-flex tw-items-center tw-justify-start q-ml-sm q-mt-lg"
            >
              <q-btn
                data-test="add-report-step2-continue-btn"
                @click="step = 4"
                color="secondary"
                label="Continue"
                no-caps
              />
              <q-btn
                data-test="add-report-step2-back-btn"
                flat
                @click="step = 2"
                color="primary"
                label="Back"
                class="q-ml-sm"
                no-caps
              />
            </div>
          </q-step>

          <q-step
            data-test="add-report-select-schedule-step"
            :name="4"
            prefix="4"
            title="Schedule"
            :done="step > 4"
            class="q-mt-md"
          >
            <schedule-config
              class="q-mt-md"
              v-model="apiConfig.schedule"
              @update:model-value="onRequestConfigUpdate"
            />
            <div
              class="tw-flex tw-items-center tw-justify-start q-ml-sm q-mt-lg"
            >
              <q-btn
                data-test="add-report-step2-continue-btn"
                @click="step = 5"
                color="secondary"
                label="Continue"
                no-caps
              />
              <q-btn
                data-test="add-report-step2-back-btn"
                flat
                @click="step = 3"
                color="primary"
                label="Back"
                class="q-ml-sm"
                no-caps
              />
            </div>
          </q-step>
          <q-step
            data-test="add-report-select-schedule-step"
            :name="5"
            title="Alert"
            prefix="5"
            :done="step > 5"
            class="q-mt-md"
          >
            <alert-config
              v-model="apiConfig.alert"
              @update:model-value="onRequestConfigUpdate"
            />
            <q-btn
              data-test="add-report-step2-back-btn"
              flat
              @click="step = 4"
              color="primary"
              label="Back"
              class="q-ml-sm"
              no-caps
            />
          </q-step>
        </q-form>
      </q-stepper>
    </div>
  </div>
  <div
    class="flex justify-end q-px-md q-py-sm full-width"
    style="position: sticky; bottom: 0px; z-index: 2"
    :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
    :style="{
      'box-shadow':
        store.state.theme === 'dark'
          ? 'rgb(45 45 45) 0px -4px 7px 0px'
          : 'rgb(240 240 240) 0px -4px 7px 0px',
    }"
  >
    <q-btn
      data-test="add-report-cancel-btn"
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
  </div>
  <ConfirmDialog
    v-model="dialog.show"
    :title="dialog.title"
    :message="dialog.message"
    @update:ok="dialog.okCallback"
    @update:cancel="dialog.show = false"
  />
</template>

<script setup lang="ts">
import RequestConfig from "@/components/synthetics/configs/RequestConfig.vue";
import AssertionConfig from "@/components/synthetics/configs/AssertionConfig.vue";
import RetryConfig from "@/components/synthetics/configs/RetryConfig.vue";
import ScheduleConfig from "@/components/synthetics/configs/ScheduleConfig.vue";
import AlertConfig from "@/components/synthetics/configs/AlertConfig.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import syntheticsService from "@/services/synthetics";
import RecorderConfig from "@/components/synthetics/configs/RecorderConfig.vue";
import RecorderRequestConfig from "@/components/synthetics/configs/RecorderRequestConfig.vue";

import { computed, onBeforeMount, ref, watch } from "vue";
import { getUUID } from "@/utils/zincutils";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";

const testType = ref("http");

const step = ref<number>(1);

const store = useStore();

const { t } = useI18n();

const router = useRouter();

const isEditingTest = ref(false);

const dialog = ref({
  show: false,
  title: "",
  message: "",
  okCallback: () => {},
});

const apiConfig = ref({
  name: "Test1",
  type: "",
  request: {
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
  },
  assertions: [
    {
      operator: "",
      type: "body",
      value: "",
      key: "",
      timingScope: "",
      id: getUUID(),
    },
  ],
  retry: {
    count: 0,
    delay: 0,
  },
  schedule: {
    interval: 1,
    type: "once",
    cron: "",
    custom: {
      interval: 1,
      frequency: "hours",
      period: "hours",
    },
    start: {
      date: "",
      time: "",
      timezone: "",
    },
    timezoneOffset: "",
  },
  alert: {
    type: "email",
    emails: "",
    message: "",
    title: "",
  },
  lastTriggeredAt: 0,
  createdAt: "",
  updatedAt: "",
  owner: "",
  lastEditedBy: "",
});

const originalApiConfig = ref(JSON.stringify(apiConfig.value));

const isBrowserTest = computed(() => testType.value === "browser");

onBeforeMount(() => {
  testType.value = (router.currentRoute.value.params.type || "http") as string;
  syntheticsService
    .getTest(
      store.state.selectedOrganization.identifier,
      router.currentRoute.value.params.name as string,
    )
    .then((response) => {
      if (response.data) {
        isEditingTest.value = true;
        apiConfig.value = response.data;
        originalApiConfig.value = JSON.stringify(response.data);
      }
    })
    .catch((error) => {
      console.log(error);
    });
});

watch(
  () => apiConfig.value.request,
  (newVal) => {},
  {
    deep: true,
  },
);
const onRequestConfigUpdate = (config: any) => {};

const saveReport = () => {
  console.log("save report", apiConfig.value);
};

const goToTests = () => {
  router.replace({
    name: "synthetics",
    query: {
      org_identifier: store.state.selectedOrganization.identifier,
    },
  });
};

const openCancelDialog = () => {
  if (originalApiConfig.value === JSON.stringify(apiConfig.value)) {
    goToTests();
    return;
  }
  dialog.value.show = true;
  dialog.value.title = "Discard Changes";
  dialog.value.message = "Are you sure you want to cancel changes?";
  dialog.value.okCallback = goToTests;
};

const saveTest = () => {
  syntheticsService
    .updateTest(store.state.selectedOrganization.identifier, apiConfig.value)
    .then((response) => {})
    .catch((error) => {
      console.log(error);
    });
};
</script>

<style scoped></style>
