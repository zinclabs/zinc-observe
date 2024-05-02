<template>
  <div class="q-my-sm q-px-sm">
    <div style="font-size: 14px" class="text-bold text-grey-8 q-mb-sm">
      Frequency
    </div>
    <div
      style="border: 1px solid #d7d7d7; width: fit-content; border-radius: 2px"
    >
      <template v-for="visual in frequencyTabs" :key="visual.value">
        <q-btn
          :data-test="`add-report-schedule-frequency-${visual.value}-btn`"
          :color="visual.value === schedule.type ? 'primary' : ''"
          :flat="visual.value === schedule.type ? false : true"
          dense
          no-caps
          size="12px"
          class="q-px-lg visual-selection-btn"
          style="padding-top: 4px; padding-bottom: 4px"
          @click="schedule.type = visual.value"
        >
          {{ visual.label }}</q-btn
        >
      </template>
    </div>

    <template v-if="schedule.type === 'cron'">
      <div class="flex items-center justify-start q-mt-md">
        <div
          data-test="add-report-schedule-custom-interval-input"
          class="o2-input q-mr-sm"
          style="padding-top: 0; width: 320px"
        >
          <div class="q-mb-xs text-bold text-grey-8">
            {{ t("reports.cronExpression") }}
            <q-icon
              :name="outlinedInfo"
              size="17px"
              class="q-ml-xs cursor-pointer"
              :class="
                store.state.theme === 'dark' ? 'text-grey-5' : 'text-grey-7'
              "
            >
              <q-tooltip anchor="center right" self="center left">
                <span style="font-size: 14px">
                  Pattern: * * * * * * means every second.
                  <br />
                  Format: [Second (optional) 0-59] [Minute 0-59] [Hour 0-23]
                  [Day of Month 1-31, 'L'] [Month 1-12] [Day of Week 0-7 or
                  '1L-7L', 0 and 7 for Sunday].
                  <br />
                  Use '*' to represent any value, 'L' for the last day/weekday.
                  <br />
                  Example: 0 0 12 * * ? - Triggers at 12:00 PM daily. It
                  specifies second, minute, hour, day of month, month, and day
                  of week, respectively.</span
                >
              </q-tooltip>
            </q-icon>
          </div>
          <q-input
            filled
            v-model="schedule.cron"
            color="input-border"
            bg-color="input-bg"
            type="text"
            outlined
            :rules="[() => (cronError.length ? cronError : true)]"
            dense
            style="width: 100%"
          />
        </div>
      </div>
    </template>
    <template v-else>
      <div
        class="q-mt-md"
        style="
          border: 1px solid #d7d7d7;
          width: fit-content;
          border-radius: 2px;
        "
      >
        <template v-for="visual in timeTabs" :key="visual.value">
          <q-btn
            :data-test="`add-report-schedule-${visual.value}-btn`"
            :color="visual.value === selectedTimeTab ? 'primary' : ''"
            :flat="visual.value === selectedTimeTab ? false : true"
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

      <div
        v-if="schedule.type === 'custom'"
        class="flex items-start justify-start q-mt-md"
      >
        <div
          data-test="add-report-schedule-custom-interval-input"
          class="o2-input q-mr-sm"
          style="padding-top: 0; width: 160px"
        >
          <q-input
            filled
            v-model="schedule.custom.interval"
            label="Repeat every *"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            type="number"
            outlined
            dense
            :rules="[(val) => !!val || 'Field is required!']"
            style="width: 100%"
          />
        </div>

        <div
          data-test="add-report-schedule-custom-frequency-select"
          class="o2-input"
          style="padding-top: 0; width: 160px"
        >
          <q-select
            v-model="schedule.custom.period"
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
        v-if="selectedTimeTab === 'sendLater'"
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
              (val) =>
                /^(0[1-9]|[12]\d|3[01])-(0[1-9]|1[0-2])-\d{4}$/.test(val) ||
                'Date format is incorrect!',
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
                  <q-date v-model="scheduling.date" mask="DD-MM-YYYY">
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
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { outlinedInfo } from "@quasar/extras/material-icons-outlined";
import { useLocalTimezone } from "@/utils/zincutils";

const { t } = useI18n();

const store = useStore();

const cronError = ref("");

const selectedTimeTab = ref("sendLater");

const scheduling = ref({
  date: "",
  time: "",
  timezone: "",
});

const filteredTimezone: any = ref([]);

const currentTimezone =
  useLocalTimezone() || Intl.DateTimeFormat().resolvedOptions().timeZone;
const timezone = ref(currentTimezone);

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

const timeTabs = [
  {
    label: "Send now",
    value: "sendNow",
  },
  {
    label: "Send later",
    value: "sendLater",
  },
];

const schedule = ref({
  interval: 1,
  type: "once",
  cron: "",
  custom: {
    interval: 1,
    frequency: "hours",
    period: "hours",
  },
});

// @ts-ignore
let timezoneOptions = Intl.supportedValuesOf("timeZone").map((tz: any) => {
  return tz;
});

const browserTime =
  "Browser Time (" + Intl.DateTimeFormat().resolvedOptions().timeZone + ")";

// Add the UTC option
timezoneOptions.unshift("UTC");
timezoneOptions.unshift(browserTime);

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
      (column: any) => column.toLowerCase().indexOf(value) > -1
    );
  });
  return filteredOptions;
};
</script>

<style scoped></style>
