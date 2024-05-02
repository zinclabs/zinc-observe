<template>
  <div>Add Api Test</div>
  <q-stepper
    v-model="step"
    vertical
    color="primary"
    animated
    class="q-mt-md"
    header-nav
  >
    <q-step
      data-test="api-test-define-request"
      :name="1"
      title="Define Request"
      :done="step > 1"
    >
      <request-config class="q-mt-md" />

      <q-stepper-navigation>
        <q-btn
          data-test="add-report-step2-continue-btn"
          @click="step = 2"
          color="secondary"
          label="Continue"
          no-caps
        />
      </q-stepper-navigation>
    </q-step>

    <q-step
      data-test="api-test-define-assertions"
      :name="2"
      title="Define Assertion"
      :done="step > 2"
    >
      <assertion-config class="q-mt-md" />
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
    </q-step>

    <q-step
      data-test="api-test-define-retry-condition"
      :name="3"
      title="Define Retry Condition"
      :done="step > 3"
    >
      <retry-config class="q-mt-md" />
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
    </q-step>

    <q-step
      data-test="add-report-select-schedule-step"
      :name="4"
      title="Schedule"
      icon="schedule"
      :done="step > 4"
      class="q-mt-md"
    >
      <schedule-config class="q-mt-md" />
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
    </q-step>
    <q-step
      data-test="add-report-select-schedule-step"
      :name="5"
      title="Alert"
      icon="alert"
      :done="step > 5"
      class="q-mt-md"
    >
      <alert-config />
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
  </q-stepper>
</template>

<script setup lang="ts">
import RequestConfig from "@/components/synthetics/configs/RequestConfig.vue";
import AssertionConfig from "@/components/synthetics/configs/AssertionConfig.vue";
import RetryConfig from "@/components/synthetics/configs/RetryConfig.vue";
import ScheduleConfig from "@/components/synthetics/configs/ScheduleConfig.vue";
import AlertConfig from "@/components/synthetics/configs/AlertConfig.vue";

import { ref } from "vue";

const step = ref<number>(1);
</script>

<style scoped></style>
