<!-- Copyright 2023 Zinc Labs Inc.

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
  <div data-test="add-report-section" class="full-width create-report-page">
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
        <div
          v-if="isEditingReport"
          class="text-h6"
          data-test="add-report-title"
        >
          {{ t("reports.update") }}
        </div>
        <div v-else class="text-h6" data-test="add-report-title">
          {{ t("reports.add") }}
        </div>
      </div>
    </div>
    <q-separator />
    <div class="flex" style="height: calc(100vh - 162px); overflow: auto">
      <div ref="addAlertFormRef" class="q-px-lg q-my-md" style="width: 1024px">
        <q-form
          class="create-report-form"
          ref="addReportFormRef"
          @submit="onSubmit"
        >
          <div
            data-test="add-report-name-input"
            class="report-name-input o2-input q-px-sm"
            style="padding-top: 12px"
          >
            <q-input
              v-model.trim="formData.name"
              :label="t('alerts.name') + ' *'"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              v-bind:readonly="isEditingReport"
              v-bind:disable="isEditingReport"
              :rules="[
                (val: any) =>
                  !!val
                    ? isValidResourceName(val) ||
                      `Characters like :, ?, /, #, and spaces are not allowed.`
                    : t('common.nameRequired'),
              ]"
              tabindex="0"
              style="width: 400px"
            >
              <template v-slot:hint>
                Characters like :, ?, /, #, and spaces are not allowed.
              </template>
            </q-input>
          </div>
          <div
            data-test="add-report-description-input"
            class="report-name-input o2-input q-px-sm"
          >
            <q-input
              v-model="formData.description"
              :label="t('reports.description')"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              tabindex="0"
              style="width: 600px"
            />
          </div>

          <div class="tw-flex tw-items-center tw-pt-4">
            <q-toggle
              data-test="report-cached-toggle-btn"
              v-model="isCachedReport"
              :label="t('reports.cachedReport')"
            />
            <q-icon
              name="info_outline"
              class="cursor-pointer q-ml-sm"
              size="16px"
            >
              <q-tooltip
                v-model="showInfoTooltip"
                anchor="center end"
                self="center left"
                class="tw-text-[12px]"
              >
                Note: Cached reports are stored for quick access to dashboards;
                sharing is disabled for these reports.</q-tooltip
              >
            </q-icon>
          </div>

          <q-stepper
            v-model="step"
            vertical
            color="primary"
            animated
            class="q-mt-md"
            header-nav
          >
            <q-step
              data-test="add-report-select-dashboard-step"
              :name="1"
              title="Select Dashboard"
              :icon="outlinedDashboard"
              :done="step > 1"
            >
              <template
                v-for="(dashboard, index) in formData.dashboards"
                :key="dashboard.folder + dashboard.dashboard"
              >
                <div
                  :data-test="`add-report-dashboard-${index}`"
                  class="q-my-sm q-px-sm flex items-center justify-start"
                >
                  <div
                    data-test="add-report-folder-select"
                    class="o2-input q-mr-sm"
                    style="padding-top: 0; width: 30%"
                  >
                    <q-select
                      v-model="dashboard.folder"
                      :options="folderOptions"
                      :label="t('reports.dashboardFolder') + ' *'"
                      :loading="isFetchingFolders"
                      :popup-content-style="{ textTransform: 'none' }"
                      color="input-border"
                      bg-color="input-bg"
                      class="q-py-sm showLabelOnTop no-case"
                      filled
                      stack-label
                      map-options
                      dense
                      emit-value
                      use-input
                      hide-selected
                      fill-input
                      :input-debounce="400"
                      input-style="text-transform: none;"
                      @update:model-value="onFolderSelection(dashboard.folder)"
                      @filter="
                        (...args: any) =>
                          onFilterOptions('folders', args[0], args[1])
                      "
                      behavior="menu"
                      :rules="[(val: any) => !!val || 'Field is required!']"
                      style="
                        min-width: 250px !important;
                        width: 100% !important;
                      "
                    />
                  </div>
                  <div
                    data-test="add-report-dashboard-select"
                    class="o2-input q-mr-sm"
                    style="padding-top: 0; width: 30%"
                  >
                    <q-select
                      v-model="dashboard.dashboard"
                      :options="dashboardOptions"
                      :label="t('reports.dashboard') + ' *'"
                      :loading="isFetchingDashboard || isFetchingFolders"
                      :popup-content-style="{ textTransform: 'none' }"
                      color="input-border"
                      bg-color="input-bg"
                      class="q-py-sm showLabelOnTop no-case"
                      filled
                      emit-value
                      map-options
                      stack-label
                      dense
                      use-input
                      hide-selected
                      fill-input
                      input-style="text-transform: none;"
                      :input-debounce="400"
                      @update:model-value="
                        onDashboardSelection(dashboard.dashboard)
                      "
                      @filter="
                        (...args: any) =>
                          onFilterOptions('dashboards', args[0], args[1])
                      "
                      behavior="menu"
                      :rules="[(val: any) => !!val || 'Field is required!']"
                      style="
                        min-width: 250px !important;
                        width: 100% !important;
                      "
                    />
                  </div>
                  <div
                    data-test="add-report-tab-select"
                    class="o2-input"
                    style="padding-top: 0; width: 30%"
                  >
                    <q-select
                      v-model="dashboard.tabs"
                      :options="dashboardTabOptions"
                      :label="t('reports.dashboardTab') + ' *'"
                      :loading="isFetchingDashboard || isFetchingFolders"
                      :popup-content-style="{ textTransform: 'none' }"
                      color="input-border"
                      bg-color="input-bg"
                      class="q-py-sm showLabelOnTop"
                      filled
                      emit-value
                      map-options
                      stack-label
                      dense
                      use-input
                      hide-selected
                      fill-input
                      input-style="text-transform: none;"
                      :input-debounce="400"
                      :rules="[
                        (val: any) => !!val.length || 'Field is required!',
                      ]"
                      @filter="
                        (...args: any) =>
                          onFilterOptions('tabs', args[0], args[1])
                      "
                      style="
                        min-width: 250px !important;
                        width: 100% !important;
                      "
                    />
                  </div>

                  <div
                    data-test="add-report-timerange-select"
                    class="full-width q-mt-sm"
                  >
                    <div class="q-mb-sm">
                      <div
                        style="font-size: 14px"
                        class="text-bold text-grey-8"
                      >
                        Time Range*
                      </div>
                      <div style="font-size: 12px">
                        Generates report with the data from specified time range
                      </div>
                    </div>
                    <DateTime
                      auto-apply
                      :default-type="dashboard.timerange.type"
                      :default-absolute-time="{
                        startTime: dashboard.timerange.from,
                        endTime: dashboard.timerange.to,
                      }"
                      :default-relative-time="dashboard.timerange.period"
                      data-test="add-report-timerange-dropdown"
                      @on:date-change="updateDateTime"
                    />
                  </div>

                  <div
                    data-test="add-report-variable-select"
                    class="full-width q-mt-md o2-input"
                  >
                    <div
                      data-test="add-report-variables-title"
                      style="font-size: 14px"
                      class="text-bold text-grey-8"
                    >
                      Variables
                    </div>
                    <VariablesInput
                      :variables="dashboard.variables"
                      @add:variable="addDashboardVariable"
                      @remove:variable="removeDashboardVariable"
                    />
                  </div>
                </div>
              </template>
              <q-stepper-navigation>
                <q-btn
                  data-test="add-report-step1-continue-btn"
                  @click="step = 2"
                  color="secondary"
                  label="Continue"
                  no-caps
                />
              </q-stepper-navigation>
            </q-step>

            <q-step
              data-test="add-report-select-schedule-step"
              :name="2"
              title="Schedule"
              icon="schedule"
              :done="step > 2"
              class="q-mt-md"
            >
              <div class="q-my-sm q-px-sm">
                <!-- <div class="flex justify-start items-center q-py-sm">
              <q-icon name="event" class="q-mr-sm" />
              <div style="font-size: 14px">
                The report will be sent immediately after it is saved and will
                be sent every hour.
              </div>
            </div> -->
                <div
                  style="font-size: 14px"
                  class="text-bold text-grey-8 q-mb-sm"
                >
                  Frequency
                </div>
                <div
                  style="
                    border: 1px solid #d7d7d7;
                    width: fit-content;
                    border-radius: 2px;
                  "
                >
                  <template v-for="visual in frequencyTabs" :key="visual.value">
                    <q-btn
                      :data-test="`add-report-schedule-frequency-${visual.value}-btn`"
                      :color="visual.value === frequency.type ? 'primary' : ''"
                      :flat="visual.value === frequency.type ? false : true"
                      dense
                      no-caps
                      size="12px"
                      class="q-px-lg visual-selection-btn"
                      style="padding-top: 4px; padding-bottom: 4px"
                      @click="frequency.type = visual.value"
                    >
                      {{ visual.label }}</q-btn
                    >
                  </template>
                </div>

                <template v-if="frequency.type === 'cron'">
                  <div class="flex items-center justify-start q-mt-md">
                    <div
                      data-test="add-report-schedule-custom-interval-input"
                      class="o2-input q-mr-sm"
                      style="padding-top: 0; width: 320px"
                    >
                      <div class="q-mb-xs text-bold text-grey-8">
                        {{ t("reports.cronExpression") + " *" }}
                        <q-icon
                          :name="outlinedInfo"
                          size="17px"
                          class="q-ml-xs cursor-pointer"
                          :class="
                            store.state.theme === 'dark'
                              ? 'text-grey-5'
                              : 'text-grey-7'
                          "
                        >
                          <q-tooltip anchor="center right" self="center left">
                            <span style="font-size: 14px">
                              Pattern: * * * * * * means every second.
                              <br />
                              Format: [Second (optional) 0-59] [Minute 0-59]
                              [Hour 0-23] [Day of Month 1-31, 'L'] [Month 1-12]
                              [Day of Week 0-7 or '1L-7L', 0 and 7 for Sunday].
                              <br />
                              Use '*' to represent any value, 'L' for the last
                              day/weekday. <br />
                              Example: 0 0 12 * * ? - Triggers at 12:00 PM
                              daily. It specifies second, minute, hour, day of
                              month, month, and day of week, respectively.</span
                            >
                          </q-tooltip>
                        </q-icon>
                      </div>
                      <q-input
                        filled
                        v-model="frequency.cron"
                        color="input-border"
                        bg-color="input-bg"
                        type="text"
                        outlined
                        :rules="[
                          (val: any) =>
                            !!val.length
                              ? cronError.length
                                ? cronError
                                : true
                              : 'Field is required!',
                        ]"
                        dense
                        style="width: 100%"
                      />
                    </div>
                    <div class="o2-input">
                      <q-select
                        data-test="add-report-schedule-start-timezone-select"
                        v-model="scheduling.timezone"
                        :options="filteredTimezone"
                        @blur="
                          timezone =
                            timezone == ''
                              ? Intl.DateTimeFormat().resolvedOptions().timeZone
                              : timezone
                        "
                        use-input
                        @filter="timezoneFilterFn"
                        input-debounce="0"
                        dense
                        filled
                        emit-value
                        fill-input
                        hide-selected
                        :label="t('logStream.timezone') + ' *'"
                        :display-value="`Timezone: ${timezone}`"
                        :rules="[(val: any) => !!val || 'Field is required!']"
                        class="timezone-select showLabelOnTop"
                        stack-label
                        outlined
                        style="width: 300px"
                      />
                    </div>
                  </div>
                </template>
                <template v-else>
                  <div class="q-mt-md tw-flex tw-justify-start tw-items-center">
                    <div
                      class="tw-flex tw-justify-center tw-align-center"
                      style="
                        border: 1px solid #d7d7d7;
                        width: fit-content;
                        border-radius: 2px;
                      "
                    >
                      <template v-for="visual in timeTabs" :key="visual.value">
                        <q-btn
                          :data-test="`add-report-schedule-${visual.value}-btn`"
                          :color="
                            visual.value === selectedTimeTab ? 'primary' : ''
                          "
                          :flat="
                            visual.value === selectedTimeTab ? false : true
                          "
                          dense
                          no-caps
                          size="12px"
                          class="q-px-md visual-selection-btn"
                          style="padding-top: 4px; padding-bottom: 4px"
                          @click="selectedTimeTab = visual.value"
                        >
                          {{ visual.label }}</q-btn
                        >
                      </template>
                    </div>
                    <q-icon
                      name="info_outline"
                      class="cursor-pointer q-ml-sm"
                      size="16px"
                    >
                      <q-tooltip
                        anchor="center end"
                        self="center left"
                        class="tw-text-[12px]"
                      >
                        "Schedule Now" will schedule the report using the
                        current date, time, and timezone.<br />
                        In "Schedule Later" you can customize the date, time,
                        and timezone.
                      </q-tooltip>
                    </q-icon>
                  </div>

                  <div
                    v-if="frequency.type === 'custom'"
                    class="flex items-start justify-start q-mt-md"
                  >
                    <div
                      data-test="add-report-schedule-custom-interval-input"
                      class="o2-input q-mr-sm"
                      style="padding-top: 0; width: 160px"
                    >
                      <q-input
                        filled
                        v-model="frequency.custom.interval"
                        label="Repeat every *"
                        color="input-border"
                        bg-color="input-bg"
                        class="showLabelOnTop"
                        stack-label
                        type="number"
                        outlined
                        dense
                        :rules="[(val: any) => !!val || 'Field is required!']"
                        style="width: 100%"
                      />
                    </div>

                    <div
                      data-test="add-report-schedule-custom-frequency-select"
                      class="o2-input"
                      style="padding-top: 0; width: 160px"
                    >
                      <q-select
                        v-model="frequency.custom.period"
                        :options="customFrequencyOptions"
                        :label="'Frequency *'"
                        :popup-content-style="{ textTransform: 'capitalize' }"
                        color="input-border"
                        bg-color="input-bg"
                        class="q-pt-sm q-pb-none showLabelOnTop no-case"
                        filled
                        emit-value
                        stack-label
                        dense
                        behavior="menu"
                        :rules="[(val: any) => !!val || 'Field is required!']"
                        style="width: 100% !important"
                      />
                    </div>
                  </div>

                  <div
                    data-test="add-report-schedule-send-later-section"
                    v-if="selectedTimeTab === 'scheduleLater'"
                    class="flex items-center justify-start q-mt-md"
                  >
                    <div
                      data-test="add-report-schedule-start-date-input"
                      class="o2-input q-mr-sm"
                    >
                      <q-input
                        filled
                        v-model="scheduling.date"
                        label="Start Date *"
                        color="input-border"
                        bg-color="input-bg"
                        class="showLabelOnTop"
                        :rules="[
                          (val: any) =>
                            /^(0[1-9]|[12]\d|3[01])-(0[1-9]|1[0-2])-\d{4}$/.test(
                              val,
                            ) || 'Date format is incorrect!',
                        ]"
                        stack-label
                        outlined
                        dense
                        style="width: 160px"
                      >
                        <template v-slot:append>
                          <q-icon name="event" class="cursor-pointer">
                            <q-popup-proxy
                              cover
                              transition-show="scale"
                              transition-hide="scale"
                            >
                              <q-date
                                v-model="scheduling.date"
                                mask="DD-MM-YYYY"
                              >
                                <div class="row items-center justify-end">
                                  <q-btn
                                    v-close-popup="true"
                                    label="Close"
                                    color="primary"
                                    flat
                                  />
                                </div>
                              </q-date>
                            </q-popup-proxy>
                          </q-icon>
                        </template>
                      </q-input>
                    </div>
                    <div
                      data-test="add-report-schedule-start-time-input"
                      class="o2-input q-mr-sm"
                    >
                      <q-input
                        filled
                        v-model="scheduling.time"
                        label="Start Time *"
                        color="input-border"
                        bg-color="input-bg"
                        class="showLabelOnTop"
                        mask="time"
                        :rules="['time']"
                        stack-label
                        outlined
                        dense
                        style="width: 160px"
                      >
                        <template v-slot:append>
                          <q-icon name="access_time" class="cursor-pointer">
                            <q-popup-proxy
                              cover
                              transition-show="scale"
                              transition-hide="scale"
                            >
                              <q-time v-model="scheduling.time">
                                <div class="row items-center justify-end">
                                  <q-btn
                                    v-close-popup="true"
                                    label="Close"
                                    color="primary"
                                    flat
                                  />
                                </div>
                              </q-time>
                            </q-popup-proxy>
                          </q-icon>
                        </template>
                      </q-input>
                    </div>
                    <div class="o2-input">
                      <q-select
                        data-test="add-report-schedule-start-timezone-select"
                        v-model="scheduling.timezone"
                        :options="filteredTimezone"
                        @blur="
                          timezone =
                            timezone == ''
                              ? Intl.DateTimeFormat().resolvedOptions().timeZone
                              : timezone
                        "
                        use-input
                        @filter="timezoneFilterFn"
                        input-debounce="0"
                        dense
                        filled
                        emit-value
                        fill-input
                        hide-selected
                        :label="t('logStream.timezone') + ' *'"
                        :display-value="`Timezone: ${timezone}`"
                        :rules="[(val: any) => !!val || 'Field is required!']"
                        class="timezone-select showLabelOnTop"
                        stack-label
                        outlined
                        style="width: 300px"
                      />
                    </div>
                  </div>
                </template>
              </div>

              <q-stepper-navigation>
                <q-btn
                  v-if="!isCachedReport"
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
              </q-stepper-navigation>
            </q-step>

            <q-step
              v-if="!isCachedReport"
              data-test="add-report-share-step"
              :name="3"
              title="Share"
              icon="mail"
              :done="step > 3"
              class="q-mt-md"
            >
              <div class="q-my-sm q-px-sm">
                <div
                  data-test="add-report-share-title-input"
                  class="report-name-input o2-input"
                >
                  <q-input
                    v-model="formData.title"
                    :label="t('reports.title') + ' *'"
                    color="input-border"
                    bg-color="input-bg"
                    class="showLabelOnTop"
                    stack-label
                    outlined
                    filled
                    dense
                    :rules="[
                      (val: any) => !!val.trim() || 'Field is required!',
                    ]"
                    tabindex="0"
                    style="width: 400px"
                  />
                </div>

                <div
                  data-test="add-report-share-recipients-input"
                  class="report-name-input o2-input"
                >
                  <q-input
                    v-model="emails"
                    :label="t('reports.recipients') + ' *'"
                    color="input-border"
                    bg-color="input-bg"
                    class="showLabelOnTop"
                    stack-label
                    outlined
                    filled
                    dense
                    :rules="[
                      (val: any) =>
                        /^([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})(\s*[;,]\s*([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}))*$/.test(
                          val,
                        ) || 'Add valid emails!',
                    ]"
                    tabindex="0"
                    style="width: 100%"
                    borderless
                    :placeholder="t('user.inviteByEmail')"
                  />
                </div>
                <div data-test="add-report-share-message-section">
                  <div style="font-size: 14px" class="text-bold text-grey-8">
                    Message
                  </div>

                  <div data-test="add-report-share-message-input">
                    <q-input
                      v-model="formData.message"
                      filled
                      type="textarea"
                    />
                  </div>
                </div>
              </div>
              <q-stepper-navigation>
                <q-btn
                  data-test="add-report-step3-back-btn"
                  flat
                  @click="step = 2"
                  color="primary"
                  label="Back"
                  class="q-ml-sm"
                  no-caps
                />
              </q-stepper-navigation>
            </q-step>
          </q-stepper>
        </q-form>
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
        @click="saveReport"
      />
    </div>
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
import { ref, nextTick } from "vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import DateTime from "@/components/DateTime.vue";
import {
  getUUID,
  useLocalTimezone,
  isValidResourceName,
} from "@/utils/zincutils";
import VariablesInput from "@/components/alerts/VariablesInput.vue";
import { useStore } from "vuex";
import { outlinedDashboard } from "@quasar/extras/material-icons-outlined";
import dashboardService from "@/services/dashboards";
import { onBeforeMount } from "vue";
import type { Ref } from "vue";
import { DateTime as _DateTime } from "luxon";
import reports from "@/services/reports";
import { useQuasar } from "quasar";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import cronParser from "cron-parser";
import { outlinedInfo } from "@quasar/extras/material-icons-outlined";
import { convertDateToTimestamp } from "@/utils/date";

const props = defineProps({
  report: {
    type: Object,
    default: null,
  },
});

const defaultReport = {
  dashboards: [
    {
      folder: "",
      dashboard: "",
      tabs: "" as string | string[],
      variables: [] as { key: string; value: string; id: string }[],
      timerange: {
        type: "relative",
        period: "30m",
        from: 0,
        to: 0,
      },
    },
  ],
  description: "",
  destinations: [
    {
      email: "",
    },
  ],
  enabled: true,
  media_type: "Pdf",
  name: "",
  title: "",
  message: "",
  orgId: "",
  start: 0,
  frequency: {
    interval: 1,
    type: "once",
    cron: "",
  },
  user: "",
  password: "",
  timezone: "UTC",
  timezoneOffset: 0,
  lastTriggeredAt: null,
  createdAt: "",
  updatedAt: "",
  owner: "",
  lastEditedBy: "",
  report_type: "PDF",
};

const { t } = useI18n();
const router = useRouter();

const isCachedReport = ref(false);

const showInfoTooltip = ref(false);

const originalReportData: Ref<string> = ref("");

const addReportFormRef: Ref<any> = ref(null);

const step = ref(1);

const formData = ref(defaultReport);

const q = useQuasar();

const dialog = ref({
  show: false,
  title: "",
  message: "",
  okCallback: () => {},
});

const timeTabs = [
  {
    label: "Schedule now",
    value: "scheduleNow",
  },
  {
    label: "Schedule later",
    value: "scheduleLater",
  },
];

const frequencyTabs = [
  {
    label: "Cron Job",
    value: "cron",
  },
  {
    label: "Once",
    value: "once",
  },
  {
    label: "Hourly",
    value: "hours",
  },
  {
    label: "Daily",
    value: "days",
  },
  {
    label: "Weekly",
    value: "weeks",
  },
  {
    label: "Monthly",
    value: "months",
  },
  {
    label: "Custom",
    value: "custom",
  },
];

const selectedTimeTab = ref("scheduleNow");

const store = useStore();

const filteredTimezone: any = ref([]);

const folderOptions: Ref<{ label: string; value: string }[]> = ref([]);

const dashboardOptions: Ref<
  { label: string; value: string; tabs: any[]; version: number }[]
> = ref([]);

const dashboardTabOptions: Ref<{ label: string; value: string }[]> = ref([]);

const options: Ref<{ [key: string]: any[] }> = ref({});

const emails = ref("");

const isEditingReport = ref(false);

const isFetchingReport = ref(false);

const cronError = ref("");

const frequency = ref({
  type: "once",
  custom: {
    interval: 1,
    period: "days",
  },
  cron: "",
});

onBeforeMount(async () => {
  await getDashboaordFolders();

  isEditingReport.value = !!router.currentRoute.value.query?.name;

  if (!isEditingReport.value) setInitialReportData();

  if (isEditingReport.value) {
    isEditingReport.value = true;
    isFetchingReport.value = true;

    const reportName: string = (router.currentRoute.value.query?.name ||
      "") as string;

    reports
      .getReport(store.state.selectedOrganization.identifier, reportName)
      .then((res: any) => {
        setupEditingReport(res.data);
        originalReportData.value = JSON.stringify(formData.value);
      })
      .catch((err) => {
        q.notify({
          type: "negative",
          message: err?.data?.message || "Error while fetching report!",
          timeout: 4000,
        });
      })
      .finally(() => {
        isFetchingReport.value = false;
      });
  } else {
    originalReportData.value = JSON.stringify(formData.value);
  }
});

const isFetchingFolders = ref(false);

const isFetchingDashboard = ref(false);

const onSubmit = () => {};

const scheduling = ref({
  date: "",
  time: "",
  timezone: "",
});

const isValidName = computed(() => {
  const roleNameRegex = /^[a-zA-Z0-9+=,.@_-]+$/;
  // Check if the role name is valid
  return roleNameRegex.test(formData.value.name);
});

const setInitialReportData = async () => {
  const queryParams = router.currentRoute.value.query;

  if (queryParams.type === "cached") {
    isCachedReport.value = true;
  } else {
    isCachedReport.value = false;
  }

  if (queryParams.folderId) {
    formData.value.dashboards[0].folder = queryParams.folderId as string;
    await onFolderSelection(queryParams.folderId as string);
  }

  if (queryParams.folderId && queryParams.dashboardId) {
    formData.value.dashboards[0].dashboard = queryParams.dashboardId as string;
    setDashboardTabOptions(queryParams.dashboardId);
  }

  if (queryParams.dashboardId && queryParams.tabId) {
    formData.value.dashboards[0].tabs = queryParams.tabId as string;
  }
};

const onFolderSelection = async (id: string) => {
  formData.value.dashboards.forEach((dashboard: any) => {
    dashboard.dashboard = "";
    dashboard.tabs = "";
  });

  try {
    await setDashboardOptions(id);
    return true;
  } catch (err) {
    return false;
  }
};

const setDashboardOptions = (id: string) => {
  dashboardOptions.value.length = 0;
  isFetchingDashboard.value = true;
  return new Promise((resolve, reject) => {
    dashboardService
      .list(
        0,
        10000,
        "name",
        false,
        "",
        store.state.selectedOrganization.identifier,
        id,
      )
      .then((response: any) => {
        response.data.dashboards
          .map((dash: any) => Object.values(dash).filter((dash) => dash)[0])
          .forEach(
            (dashboard: {
              title: string;
              dashboardId: string;
              tabs: any[];
              version: number;
            }) => {
              dashboardOptions.value.push({
                label: dashboard.title,
                value: dashboard.dashboardId,
                tabs: dashboard?.tabs?.map((tab) => ({
                  label: tab.name,
                  value: tab.tabId,
                })) || [{ label: "Default", value: "default" }],
                version: dashboard.version,
              });
              options.value["dashboards"] = [...dashboardOptions.value];
            },
          );

        resolve(true);
      })
      .catch((err) => reject(true))
      .finally(() => (isFetchingDashboard.value = false));
  });
};

const onDashboardSelection = (dashboardId: any) => {
  formData.value.dashboards.forEach((dashboard: any) => {
    dashboard.tabs = "";
  });
  setDashboardTabOptions(dashboardId);
};

const setDashboardTabOptions = (dashboardId: any) => {
  const defaultTabs = [{ label: "Default", value: "default" }];

  dashboardTabOptions.value =
    dashboardOptions.value.filter(
      (dashboard) => dashboard.value === dashboardId,
    )[0].tabs || defaultTabs;

  options.value["tabs"] = [...dashboardTabOptions.value];
};

const updateDateTime = (datetime: any) => {
  formData.value.dashboards[0].timerange.type =
    datetime.valueType === "relative-custom" ? "relative" : datetime.valueType;

  formData.value.dashboards[0].timerange.from = datetime.startTime;
  formData.value.dashboards[0].timerange.to = datetime.endTime;

  formData.value.dashboards[0].timerange.period =
    datetime.relativeTimePeriod || "30m";
};

const customFrequencyOptions = [
  {
    label: "days",
    value: "days",
  },
  {
    label: "hours",
    value: "hours",
  },
  {
    label: "weeks",
    value: "weeks",
  },
  {
    label: "months",
    value: "months",
  },
];

const currentTimezone =
  useLocalTimezone() || Intl.DateTimeFormat().resolvedOptions().timeZone;
const timezone = ref(currentTimezone);

const timezoneFilterFn = (val: string, update: Function) => {
  filteredTimezone.value = filterColumns(timezoneOptions, val, update);
};

const filterColumns = (options: any[], val: String, update: Function) => {
  let filteredOptions: any[] = [];
  if (val === "") {
    update(() => {
      filteredOptions = [...options];
    });
    return filteredOptions;
  }
  update(() => {
    const value = val.toLowerCase();
    filteredOptions = options.filter(
      (column: any) => column.toLowerCase().indexOf(value) > -1,
    );
  });
  return filteredOptions;
};

// @ts-ignore
let timezoneOptions = Intl.supportedValuesOf("timeZone").map((tz: any) => {
  return tz;
});

const browserTime =
  "Browser Time (" + Intl.DateTimeFormat().resolvedOptions().timeZone + ")";

// Add the UTC option
timezoneOptions.unshift("UTC");
timezoneOptions.unshift(browserTime);

const addDashboardVariable = () => {
  formData.value.dashboards[0].variables.push({
    key: "",
    value: "",
    id: getUUID(),
  });
};

const removeDashboardVariable = (variable: any) => {
  formData.value.dashboards[0].variables =
    formData.value.dashboards[0].variables.filter(
      (_variable: any) => _variable.id !== variable.id,
    );
};

const getDashboaordFolders = () => {
  return new Promise((resolve, reject) => {
    isFetchingFolders.value = true;
    dashboardService
      .list_Folders(store.state.selectedOrganization.identifier)
      .then((res) => {
        res.data.list.forEach((folder: { name: string; folderId: string }) => {
          folderOptions.value.push({
            label: folder.name,
            value: folder.folderId,
          });
          options.value["folders"] = [...folderOptions.value];
        });
        resolve(true);
      })
      .catch((err) => reject(true))
      .finally(() => {
        isFetchingFolders.value = false;
      });
  });
};

const saveReport = async () => {
  // If frequency is cron, then we set the start timestamp as current time and timezone as browser timezone
  const reportPayload = JSON.parse(JSON.stringify(formData.value));

  if (
    selectedTimeTab.value === "scheduleNow" ||
    frequency.value.type === "cron"
  ) {
    const now = new Date();

    // Get the day, month, and year from the date object
    const day = String(now.getDate()).padStart(2, "0");
    const month = String(now.getMonth() + 1).padStart(2, "0"); // January is 0!
    const year = now.getFullYear();

    // Combine them in the DD-MM-YYYY format
    scheduling.value.date = `${day}-${month}-${year}`;

    // Get the hours and minutes, ensuring they are formatted with two digits
    const hours = String(now.getHours()).padStart(2, "0");
    const minutes = String(now.getMinutes()).padStart(2, "0");

    // Combine them in the HH:MM format
    scheduling.value.time = `${hours}:${minutes}`;
  }

  // If the user has selected to schedule now, we set the timezone to the current timezone
  if (selectedTimeTab.value === "scheduleNow") {
    scheduling.value.timezone = timezone.value;
  }

  const convertedDateTime = convertDateToTimestamp(
    scheduling.value.date,
    scheduling.value.time,
    scheduling.value.timezone,
  );

  reportPayload.start = convertedDateTime.timestamp;

  reportPayload.timezoneOffset = convertedDateTime.offset;

  reportPayload.orgId = store.state.selectedOrganization.identifier;

  reportPayload.destinations = emails.value.split(/[,;]/).map((email) => ({
    email: email.trim(),
  }));

  if (frequency.value.type === "custom") {
    reportPayload.frequency.type = frequency.value.custom.period;
    reportPayload.frequency.interval = Number(frequency.value.custom.interval);
  } else if (frequency.value.type === "cron") {
    reportPayload.frequency.type = frequency.value.type;
    reportPayload.frequency.cron =
      frequency.value.cron.toString().trim() + " *";
  } else {
    reportPayload.frequency.type = frequency.value.type;
    reportPayload.frequency.interval = 1;
  }

  reportPayload.timezone = scheduling.value.timezone;

  if (isEditingReport.value) {
    reportPayload.updatedAt = new Date().toISOString();
    reportPayload.lastEditedBy = store.state.userInfo.email;
  } else {
    reportPayload.createdAt = new Date().toISOString();
    reportPayload.owner = store.state.userInfo.email;
    reportPayload.lastTriggeredAt = null;
    reportPayload.lastEditedBy = store.state.userInfo.email;
    reportPayload.updatedAt = new Date().toISOString();
  }

  if (isCachedReport.value) reportPayload.destinations = [];

  // Check if all report input fields are valid
  try {
    validateReportData();
    await nextTick();
    await nextTick();
    const isValidForm = await addReportFormRef.value.validate();
    if (!isValidForm) return;
  } catch (err) {
    console.log(err);
  }

  // This is unitil we support multiple dashboards and tabs
  if (reportPayload.dashboards[0]?.tabs)
    reportPayload.dashboards[0].tabs = [
      reportPayload.dashboards[0].tabs as string,
    ];

  const reportAction = isEditingReport.value
    ? reports.updateReport
    : reports.createReport;

  const dismiss = q.notify({
    spinner: true,
    message: "Please wait...",
    timeout: 2000,
  });

  reportAction(store.state.selectedOrganization.identifier, reportPayload)
    .then(() => {
      q.notify({
        type: "positive",
        message: `Report ${
          isEditingReport.value ? "updated" : "saved"
        } successfully.`,
        timeout: 3000,
      });
      goToReports();
    })
    .catch((error) => {
      q.notify({
        type: "negative",
        message:
          error?.response?.data?.message ||
          `Error while ${
            isEditingReport.value ? "updating" : "saving"
          } report.`,
        timeout: 4000,
      });
    })
    .finally(() => {
      dismiss();
    });
};

const validateReportData = async () => {
  if (!formData.value.dashboards[0].folder) {
    step.value = 1;
    return;
  }

  if (!formData.value.dashboards[0].dashboard) {
    step.value = 1;
    return;
  }

  if (!formData.value.dashboards[0].tabs) {
    step.value = 1;
    return;
  }

  if (formData.value.dashboards[0].timerange) {
    if (
      formData.value.dashboards[0].timerange.type === "relative" &&
      !formData.value.dashboards[0].timerange.period
    ) {
      step.value = 1;
      return;
    }

    if (
      formData.value.dashboards[0].timerange.type === "absolute" &&
      !(
        formData.value.dashboards[0].timerange.to &&
        formData.value.dashboards[0].timerange.from
      )
    ) {
      step.value = 1;
      return;
    }
  }

  if (formData.value.frequency.type === "cron") {
    try {
      cronParser.parseExpression(frequency.value.cron);
      cronError.value = "";
    } catch (err) {
      cronError.value = "Invalid cron expression!";
      step.value = 2;
      return;
    }
  }

  if (!formData.value.frequency.interval || !formData.value.frequency.type) {
    step.value = 2;
    return;
  }

  if (!formData.value.start || !formData.value.timezone) {
    step.value = 2;
    return;
  }

  if (
    !formData.value.title ||
    !/^([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})(\s*[;,]\s*([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}))*$/.test(
      emails.value,
    )
  ) {
    step.value = 3;
    return;
  }
};

const goToReports = () => {
  router.replace({
    name: "reports",
    query: {
      org_identifier: store.state.selectedOrganization.identifier,
    },
  });
};

const onFilterOptions = (type: string, val: String, update: Function) => {
  if (type === "folders") {
    folderOptions.value = filterOptions(options.value[type] || [], val, update);
  }

  if (type === "dashboards") {
    dashboardOptions.value = filterOptions(
      options.value[type] || [],
      val,
      update,
    );
  }

  if (type === "tabs") {
    dashboardTabOptions.value = filterOptions(
      dashboardTabOptions.value,
      val,
      update,
    );
  }
};

const filterOptions = (options: any[], val: String, update: Function) => {
  let filteredOptions: any[] = [];
  if (val === "") {
    update(() => {
      filteredOptions = [...options];
    });
  }
  update(() => {
    const value = val.toLowerCase();
    filteredOptions = options.filter((option: any) => {
      return option.label.toLowerCase().indexOf(value) > -1;
    });
  });

  return filteredOptions;
};

const setupEditingReport = async (report: any) => {
  formData.value = {
    ...report,
    dashboards: [
      {
        folder: "",
        dashboard: "",
        tabs: "" as string | string[],
        variables: report.dashboards[0].variables,
        timerange: report.dashboards[0].timerange,
      },
    ],
  };

  // set date, time and timezone in scheduling
  const date = new Date(report.start / 1000);

  // Get the day, month, and year from the date object
  const day = String(date.getDate()).padStart(2, "0");
  const month = String(date.getMonth() + 1).padStart(2, "0"); // January is 0!
  const year = date.getFullYear();

  // Combine them in the DD-MM-YYYY format
  scheduling.value.date = `${day}-${month}-${year}`;

  // Get the hours and minutes, ensuring they are formatted with two digits
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");

  // Combine them in the HH:MM format
  scheduling.value.time = `${hours}:${minutes}`;

  scheduling.value.timezone = report.timezone;

  // set selectedTimeTab to scheduleLater
  selectedTimeTab.value = "scheduleLater";

  emails.value = report.destinations
    .map((destination: { email: string }) => destination.email)
    .join(";");

  if (!report.destinations.length) isCachedReport.value = true;

  // set frequency
  if (report.frequency.type === "cron") {
    frequency.value.type = report.frequency.type;
    frequency.value.cron =
      report.frequency.cron.split(" ").length === 7
        ? report.frequency.cron.split(" ").slice(0, 6).join(" ")
        : report.frequency.cron;
  } else if (report.frequency.interval > 1) {
    frequency.value.type = "custom";
    frequency.value.custom.period = report.frequency.type;
    frequency.value.custom.interval = report.frequency.interval;
  } else {
    frequency.value.type = report.frequency.type;
  }

  await setDashboardOptions(report.dashboards[0].folder);

  // Check if folder is present in the options and set the folder
  if (
    folderOptions.value.some(
      (folder) => folder.value === report.dashboards[0].folder,
    )
  ) {
    formData.value.dashboards[0].folder = report.dashboards[0].folder;
  } else {
    formData.value.dashboards[0].folder = "";
    q.notify({
      type: "negative",
      message: "Selected folder has been deleted!",
      timeout: 4000,
    });
  }

  // Check if dashboard is present in the options and set the dashboard
  if (
    dashboardOptions.value.some(
      (dashboard) => dashboard.value === report.dashboards[0].dashboard,
    )
  ) {
    formData.value.dashboards[0].dashboard = report.dashboards[0].dashboard;
  } else {
    formData.value.dashboards[0].dashboard = "";
    q.notify({
      type: "negative",
      message: "Selected dashboard has been deleted!",
      timeout: 4000,
    });
  }

  setDashboardTabOptions(formData.value.dashboards[0].dashboard);

  // Check if tab is present in the options and set the tab
  const tab = dashboardTabOptions.value.filter(
    (tab) => tab.value === report.dashboards[0].tabs[0],
  )[0];

  if (tab) {
    formData.value.dashboards[0].tabs = tab.value;
  } else {
    q.notify({
      type: "negative",
      message: "Selected dashboard tab has been deleted!",
      timeout: 4000,
    });
    formData.value.dashboards[0].tabs = "";
  }
};

const openCancelDialog = () => {
  if (originalReportData.value === JSON.stringify(formData.value)) {
    goToReports();
    return;
  }
  dialog.value.show = true;
  dialog.value.title = "Discard Changes";
  dialog.value.message = "Are you sure you want to cancel report changes?";
  dialog.value.okCallback = goToReports;
};
</script>

<style lang="scss">
.create-report-page {
  .q-expansion-item .q-item {
    padding: 0 8px;
  }
}
</style>
