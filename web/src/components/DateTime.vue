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
  <div icon="info" class="justify-between date-time-container">
    <q-btn
      :data-test="dataTestName"
      id="date-time-button"
      ref="datetimeBtn"
      data-cy="date-time-button"
      outline
      no-caps
      :label="getDisplayValue"
      icon="schedule"
      icon-right="arrow_drop_down"
      class="date-time-button"
      :class="selectedType + 'type'"
      :disable="disable"
      @click="showOnlyAbsolute"
    >
      <q-menu
        id="date-time-menu"
        class="date-time-dialog"
        anchor="bottom left"
        self="top left"
        no-route-dismiss
        @before-show="onBeforeShow"
        @before-hide="onBeforeHide"
      >
        <div v-if="showRelative" class="flex justify-evenly q-py-sm">
          <q-btn
            data-test="date-time-relative-tab"
            size="md"
            class="tab-button no-border"
            color="primary"
            no-caps
            :flat="selectedType !== 'relative'"
            @click="setDateType('relative')"
          >
            Relative
          </q-btn>
          <q-separator vertical inset />
          <q-btn
            data-test="date-time-absolute-tab"
            size="md"
            class="tab-button no-border"
            color="primary"
            no-caps
            :flat="selectedType !== 'absolute'"
            @click="setDateType('absolute')"
          >
            Absolute
          </q-btn>
        </div>
        <q-separator />
        <q-tab-panels v-model="selectedType" animated>
          <q-tab-panel v-if="showRelative" name="relative" class="q-pa-none">
            <div class="date-time-table relative column">
              <div
                class="relative-row q-px-md q-py-sm"
                v-for="(period, index) in relativePeriods"
                :key="'date_' + index"
              >
                <div class="relative-period-name">
                  {{ period.label }}
                </div>
                <div
                  v-for="(item, item_index) in relativeDates[period.value]"
                  :key="item"
                >
                  <q-btn
                    :disable="
                      relativeDatesInHour[period.value][item_index] >
                        queryRangeRestrictionInHour &&
                      queryRangeRestrictionInHour > 0
                    "
                    :data-test="`date-time-relative-${item}-${period.value}-btn`"
                    :class="
                      selectedType == 'relative' &&
                      relativePeriod == period.value &&
                      relativeValue == item
                        ? 'rp-selector-selected'
                        : `rp-selector ${relativePeriod}`
                    "
                    :label="item"
                    outline
                    dense
                    flat
                    @click="setRelativeDate(period.value, item)"
                    :key="'period_' + item_index"
                  >
                    <q-tooltip
                      style="z-index: 10001; font-size: 14px"
                      anchor="center right"
                      self="center left"
                      max-width="300px"
                      v-if="
                        relativeDatesInHour[period.value][item_index] >
                          queryRangeRestrictionInHour &&
                        queryRangeRestrictionInHour > 0
                      "
                    >
                      {{ queryRangeRestrictionMsg }}
                    </q-tooltip>
                  </q-btn>
                </div>
              </div>

              <div class="relative-row q-px-md q-py-sm">
                <div class="relative-period-name">Custom</div>
                <q-tooltip
                  style="z-index: 10001; font-size: 14px"
                  anchor="center right"
                  self="center left"
                  max-width="300px"
                  v-if="queryRangeRestrictionInHour > 0"
                >
                  {{ queryRangeRestrictionMsg }}
                </q-tooltip>

                <div class="row q-gutter-sm">
                  <div class="col">
                    <q-input
                      v-model="relativeValue"
                      type="number"
                      dense
                      filled
                      min="1"
                      :max="
                        relativePeriodsMaxValue[relativePeriod] > 0
                          ? relativePeriodsMaxValue[relativePeriod]
                          : ''
                      "
                      @update:model-value="onCustomPeriodSelect"
                    />
                  </div>
                  <div class="col">
                    <q-select
                      v-model="relativePeriod"
                      :options="relativePeriodsSelect"
                      dense
                      filled
                      emit-value
                      @update:modelValue="onCustomPeriodSelect"
                      popup-content-style="z-index: 10002"
                      style="width: 100px"
                    >
                      <template v-slot:selected-item>
                        <div>{{ getPeriodLabel }}</div>
                      </template>
                    </q-select>
                  </div>
                </div>
              </div>
            </div>
          </q-tab-panel>
          <q-tab-panel name="absolute" class="q-pa-none">
            <div class="date-time-table">
              <q-tooltip
                anchor="center right"
                self="center left"
                max-width="300px"
                v-if="queryRangeRestrictionInHour > 0"
              >
                <span style="font-size: 14px">
                  {{ queryRangeRestrictionMsg }}</span
                ></q-tooltip
              >
              <div class="flex justify-center q-pa-none">
                <!-- here add -->
                <q-date
                  size="sm"
                  v-model="selectedDate"
                  class="absolute-calendar"
                  range
                  :locale="dateLocale"
                  :options="optionsFn"
                  
                />
              </div>
              <div  class="notePara">* You can choose multiple date</div>
              <q-separator v-if="showRelative" class="q-my-sm" />

              <table v-if="showRelative" class="q-px-md startEndTime">
                <tr>
                  <td class="label tw-px-2">Start time</td>
                  <td class="label tw-px-2">End time</td>
                </tr>
                <tr>
                  <td>
                    <q-input
                      v-model="selectedTime.startTime"
                      dense
                      filled
                      mask="fulltime"
                      :rules="['fulltime']"
                      @blur="
                        resetTime(selectedTime.startTime, selectedTime.endTime)
                      "
                    >
                      <template #append>
                        <q-icon name="access_time" class="cursor-pointer">
                          <q-popup-proxy
                            transition-show="scale"
                            transition-hide="scale"
                            style="z-index: 10002"
                          >
                            <q-time
                              v-model="selectedTime.startTime"
                              with-seconds
                            >
                              <div class="row items-center justify-end">
                                <q-btn
                                  v-close-popup
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
                  </td>
                  <td>
                    <q-input
                      v-model="selectedTime.endTime"
                      dense
                      filled
                      mask="fulltime"
                      :rules="['fulltime']"
                      @blur="
                        resetTime(selectedTime.startTime, selectedTime.endTime)
                      "
                    >
                      <template #append>
                        <q-icon name="access_time" class="cursor-pointer">
                          <q-popup-proxy
                            transition-show="scale"
                            transition-hide="scale"
                            style="z-index: 10002"
                          >
                            <q-time
                              v-model="selectedTime.endTime"
                              :with-seconds="true"
                            >
                              <div class="row items-center justify-end">
                                <q-btn
                                  v-close-popup
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
                  </td>
                </tr>
              </table>
            </div>
          </q-tab-panel>
        </q-tab-panels>
        <q-select
          data-test="datetime-timezone-select"
          v-model="timezone"
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
          :label="t('logStream.timezone')"
          @update:modelValue="onTimezoneChange"
          :display-value="`Timezone: ${timezone}`"
          class="timezone-select"
          popup-content-style="z-index: 10002"
        >
        </q-select>
        <div v-if="!autoApply " class="flex justify-end q-py-sm q-px-md">
          <q-separator class="q-my-sm" />
          <q-btn
            data-test="date-time-apply-btn"
            class="no-border q-py-xs"
            color="secondary"
            no-caps
            size="sm"
            @click="saveDate(null)"
            v-close-popup
          >
            Apply
          </q-btn>
        </div>
      </q-menu>
    </q-btn>
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import {
  ref,
  defineComponent,
  computed,
  onMounted,
  watch,
  nextTick,
  onActivated,
  onBeforeUnmount,
  onBeforeMount,
} from "vue";
import {
  getImageURL,
  useLocalTimezone,
  convertToUtcTimestamp,
  timestampToTimezoneDate,
} from "../utils/zincutils";
import { date, useQuasar } from "quasar";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { toZonedTime } from "date-fns-tz";
import { max } from "moment";

export default defineComponent({
  props: {
    defaultType: {
      type: String,
      default: "relative",
    },
    defaultAbsoluteTime: {
      type: Object,
      default: null,
    },
    defaultRelativeTime: {
      type: String,
      default: "15m",
    },
    autoApply: {
      type: Boolean,
      default: false,
    },
    initialTimezone: {
      required: false,
      default: null,
    },
    disable: {
      type: Boolean,
      default: false,
    },
    queryRangeRestrictionMsg: {
      type: String,
      default: "",
    },
    queryRangeRestrictionInHour: {
      type: Number,
      default: 0,
    },
    dataTestName: {
      type: String,
      default: "date-time-btn",
    },
    showRelative: {
      type: Boolean,
      default: true,
    },
    minDate: {
      type: String,
      default: null,
    }
  },

  emits: ["on:date-change", "on:timezone-change"],

  setup(props, { emit }) {
    const store = useStore();
    const { t } = useI18n();
    const $q = useQuasar();
    const selectedType = ref("relative");
    const selectedTime = ref({
      startTime: "00:00:00",
      endTime: "23:59:59",
    });
    const selectedDate = ref({
      from: "",
      to: "",
    });
    const relativePeriod = ref("m");
    const relativeValue = ref(15);
    const currentTimezone =
      useLocalTimezone() || Intl.DateTimeFormat().resolvedOptions().timeZone;
    const timezone = ref(currentTimezone);
    let timezoneOptions = Intl.supportedValuesOf("timeZone").map((tz) => {
      return tz;
    });
    const browserTime =
      "Browser Time (" + Intl.DateTimeFormat().resolvedOptions().timeZone + ")";
    const router = useRouter();

    // Add the UTC option
    timezoneOptions.unshift("UTC");
    timezoneOptions.unshift(browserTime);

    const onTimezoneChange = async () => {
      let selectedTimezone = timezone.value;
      if (selectedTimezone.toLowerCase() == browserTime.toLowerCase()) {
        selectedTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
      }
      //Intl.DateTimeFormat().resolvedOptions().timeZone
      useLocalTimezone(selectedTimezone);
      store.dispatch("setTimezone", selectedTimezone);
      await nextTick();
      if (selectedType.value == "absolute") saveDate("absolute");
      else saveDate("relative");
      emit("on:timezone-change");
    };

    // if initial timezone is provided
    if (props.initialTimezone) {
      // check if it is a valid timezone
      // ignore case
      timezone.value =
        timezoneOptions.find(
          (tz) => tz.toLowerCase() === props.initialTimezone?.toLowerCase(),
        ) || currentTimezone;

      // call onTimezoneChange to set the timezone in the store
      onTimezoneChange();
    }

    const filteredTimezone: any = ref([]);

    let relativePeriods = [
      { label: "Seconds", value: "s" },
      { label: "Minutes", value: "m" },
      { label: "Hours", value: "h" },
      { label: "Days", value: "d" },
      { label: "Weeks", value: "w" },
      { label: "Months", value: "M" },
    ];

    let relativePeriodsSelect = ref([
      { label: "Seconds", value: "s" },
      { label: "Minutes", value: "m" },
      { label: "Hours", value: "h" },
      { label: "Days", value: "d" },
      { label: "Weeks", value: "w" },
      { label: "Months", value: "M" },
    ]);

    const relativeDates = {
      s: [1, 5, 10, 15, 30, 45],
      m: [1, 5, 10, 15, 30, 45],
      h: [1, 2, 3, 6, 8, 12],
      d: [1, 2, 3, 4, 5, 6],
      w: [1, 2, 3, 4, 5, 6],
      M: [1, 2, 3, 4, 5, 6],
    };

    const relativeDatesInHour = {
      s: [1, 1, 1, 1, 1, 1],
      m: [1, 1, 1, 1, 1, 1],
      h: [1, 2, 3, 6, 8, 12],
      d: [24, 48, 72, 96, 120, 144],
      w: [168, 336, 504, 672, 840, 1008],
      M: [744, 1488, 2232, 2976, 3720, 4464],
    };

    let relativePeriodsMaxValue: object = ref({
      s: 0,
      m: 0,
      h: 0,
      d: 0,
      w: 0,
      M: 0,
    });

    const periodUnits = ["s", "m", "h", "d", "w", "M"];

    const datetimeBtn = ref(null);

    const displayValue = ref("");

    const datePayload = ref({});

    const dateLocale = {
      daysShort: ["S", "M", "T", "W", "T", "F", "S"],
    };
    onBeforeMount(()=>{
      if(!props.showRelative) setDateType("absolute");
    })

    onMounted(() => {
      // updateDisplayValue();
      if(!props.showRelative) setDateType("absolute");
      try {
        resetTime("", "");

        let startTime = (new Date().getTime() - 900000) * 1000;
        let endTime = new Date().getTime() * 1000;

        if (props.defaultAbsoluteTime?.startTime) {
          startTime =
            props.defaultAbsoluteTime?.startTime.toString().length > 13
              ? props.defaultAbsoluteTime?.startTime
              : props.defaultAbsoluteTime?.startTime * 1000;
        }

        if (props.defaultAbsoluteTime?.endTime) {
          endTime =
            props.defaultAbsoluteTime?.endTime.toString().length > 13
              ? props.defaultAbsoluteTime?.endTime
              : props.defaultAbsoluteTime?.endTime * 1000;
        }

        selectedType.value = props.defaultType;

        setAbsoluteTime(startTime, endTime);

        setRelativeTime(props.defaultRelativeTime);

        if (props.queryRangeRestrictionInHour) computeRelativePeriod();
        // displayValue.value = getDisplayValue();

        saveDate(props.defaultType);
      } catch (e) {
        console.log(e);
      }
    });

    watch(
      () => {
        selectedTime.value.startTime;
        selectedTime.value.endTime;
        selectedDate.value?.from;
        selectedDate.value?.to;
      },
      () => {
        if (
          props.autoApply &&
          selectedType.value === "absolute" &&
          store.state.savedViewFlag == false
        ) {
          saveDate("absolute");
        }
      },
      { deep: true },
    );

    watch(
      () => props.defaultType,
      () => {
        if (props.defaultType !== selectedType.value) {
          selectedType.value = props.defaultType;
        }
      },
    );

    const setRelativeDate = (period, value) => {
      selectedType.value = "relative";
      relativePeriod.value = period;
      relativeValue.value = value;

      if (props.autoApply) saveDate("relative");
    };

    const onCustomPeriodSelect = () => {
      if (
        selectedType.value == "relative" &&
        props.queryRangeRestrictionInHour > 0 &&
        relativeValue.value >
          relativePeriodsMaxValue.value[relativePeriod.value]
      ) {
        relativeValue.value =
          relativePeriodsMaxValue.value[relativePeriod.value] > -1
            ? relativePeriodsMaxValue.value[relativePeriod.value]
            : 15;
      }
      if (props.autoApply) saveDate("relative-custom");
    };

    const setRelativeTime = (period) => {
      const periodString = period?.match(/(\d+)([smhdwM])/);

      if (periodString) {
        const periodValue = periodString[1];
        const periodUnit = periodString[2];

        if (periodUnits.includes(periodUnit)) {
          relativePeriod.value = periodUnit;
        }

        if (periodValue) {
          relativeValue.value = parseInt(periodValue);
        }
      }
    };

    const resetTime = (startTime, endTime) => {
      if (!startTime || !endTime) {
        var dateString = new Date().toLocaleDateString("en-ZA");

        if (!selectedDate.value.from) selectedDate.value.from = dateString;

        if (!selectedDate.value.to) selectedDate.value.to = dateString;

        if (!startTime) selectedTime.value.startTime = "00:00:00";

        if (!endTime) {
          const endDateTime = convertUnixTime(new Date().getTime() * 1000);

          // If the selected date is today, set the end time to the current time
          if (selectedDate.value.to === endDateTime.date) {
            selectedTime.value.endTime = endDateTime.time;
          } else selectedTime.value.endTime = "23:59:59";
        }

        return;
      }
      return;
    };

    const setAbsoluteTime = (startTime, endTime) => {
      if (!startTime || !endTime) {
        var dateString = new Date().toLocaleDateString("en-ZA");
        selectedDate.value.from = dateString;
        selectedDate.value.to = dateString;
        return;
      }

      const startDateTime = convertUnixTime(startTime);
      const endDateTime = convertUnixTime(endTime);

      selectedDate.value.from = startDateTime.date;
      selectedDate.value.to = endDateTime.date;

      selectedTime.value.startTime = startDateTime.time;
      selectedTime.value.endTime = endDateTime.time;
    };

    function convertUnixTime(unixTimeMicros) {
      // Convert microseconds to milliseconds and create a new Date object
      var date = toZonedTime(unixTimeMicros / 1000, store.state.timezone);

      // Extract hour, minute, day, month, and year
      const hours = ("0" + date.getHours()).slice(-2); // pad with leading zero if needed
      const minutes = ("0" + date.getMinutes()).slice(-2); // pad with leading zero if needed
      const seconds = ("0" + date.getSeconds()).slice(-2); // pad with leading zero if needed

      // Build formatted strings
      var dateString = new Date(date).toLocaleDateString("en-ZA");
      var timeString = hours + ":" + minutes + ":" + seconds;

      // Return both strings
      return { date: dateString, time: timeString };
    }

    const refresh = () => {
      saveDate();
    };

    const saveDate = (dateType) => {
      // displayValue.value = getDisplayValue();
      const date = getConsumableDateTime();
      if (isNaN(date.endTime) || isNaN(date.startTime)) {
        return false;
      }
      datePayload.value = date;
      date["valueType"] = dateType || selectedType.value;
      // date["relativeTimePeriod"] = "";
      if (store.state.savedViewFlag == false) {
        emit("on:date-change", date);
      }
    };

    function formatDate(d) {
      var year = d.getFullYear();
      var month = ("0" + (d.getMonth() + 1)).slice(-2); // Months are zero-based
      var day = ("0" + d.getDate()).slice(-2);
      var hours = ("0" + d.getHours()).slice(-2);
      var minutes = ("0" + d.getMinutes()).slice(-2);
      var seconds = ("0" + d.getSeconds()).slice(-2);

      return {
        date: year + "/" + month + "/" + day,
        time: hours + ":" + minutes + ":" + seconds,
      };
    }

    const setCustomDate = (dateType, dateobj) => {
      var start_date = new Date(Math.floor(dateobj.start));
      const startObj = formatDate(start_date);

      var end_date = new Date(Math.floor(dateobj.end));
      const endObj = formatDate(end_date);

      selectedDate.value.from = startObj.date;
      selectedDate.value.to = endObj.date;
      selectedTime.value.startTime = startObj.time;
      selectedTime.value.endTime = endObj.time;

      selectedType.value = dateType;
    };

    const onBeforeShow = () => {
      // if (props.modelValue) selectedDate.value = cloneDeep(props.modelValue);
    };

    const onBeforeHide = () => {
      if (selectedType.value === "absolute")
        resetTime(selectedTime.value.startTime, selectedTime.value.endTime);
    };

    const getPeriodLabel = computed(() => {
      const periodMapping = {
        s: "Seconds",
        m: "Minutes",
        h: "Hours",
        d: "Days",
        w: "Weeks",
        M: "Months",
      };
      return periodMapping[relativePeriod.value];
    });

    function isValidDateTimeString(dateStr: string, timeStr: string): boolean {
      return !isNaN(Date.parse(`${dateStr} ${timeStr}`));
    }

    const getConsumableDateTime = () => {
      if (selectedType.value == "relative") {
        let period = getPeriodLabel.value.toLowerCase();
        let periodValue = relativeValue.value;

        // quasar does not support arithmetic on weeks. convert to days.
        if (relativePeriod.value === "w") {
          period = "days";
          periodValue = periodValue * 7;
        }

        const subtractObject = {};

        if (period && periodValue) subtractObject[period] = periodValue;
        else {
          console.error("getConsumableDateTime: Invalid relative period");
          subtractObject["Minutes"] = 15;
        }

        const endTimeStamp = new Date();

        const startTimeStamp = date.subtractFromDate(
          endTimeStamp,
          subtractObject,
        );

        return {
          startTime: new Date(startTimeStamp).getTime() * 1000,
          endTime: new Date(endTimeStamp).getTime() * 1000,
          relativeTimePeriod: relativeValue.value + relativePeriod.value,
        };
      } else {
        if (typeof selectedDate.value === "string") {
          selectedDate.value = {
            from: selectedDate.value,
            to: selectedDate.value,
          };
        }

        const startDateStr =
          selectedDate.value.from + " " + selectedTime.value.startTime;
        if (
          !isValidDateTimeString(
            selectedDate.value.from,
            selectedTime.value.startTime,
          )
        ) {
          console.error(`Invalid start date/time: ${startDateStr}`);
          return new Date();
        }

        const endDateStr =
          selectedDate.value.to + " " + selectedTime.value.endTime;
        if (
          !isValidDateTimeString(
            selectedDate.value.to,
            selectedTime.value.endTime,
          )
        ) {
          console.error(`Invalid end date/time: ${endDateStr}`);
          return new Date();
        }

        let start, end;
        if (!selectedDate.value?.from && !selectedTime.value?.startTime) {
          start = new Date();
        } else {
          start = new Date(startDateStr);
        }

        if (selectedDate.value?.to == "" && selectedTime.value?.endTime == "") {
          end = new Date();
        } else {
          end = new Date(endDateStr);
        }

        if (
          start.toString() === "Invalid Date" &&
          end.toString() === "Invalid Date"
        ) {
          start = selectedDate.value + " " + selectedTime.value.startTime;
          end = selectedDate.value + " " + selectedTime.value.endTime;
        }
        const absoluteUTCTimestamp = getUTCTimeStamp();
        const rVal = {
          startTime: absoluteUTCTimestamp.startUTC,
          endTime: absoluteUTCTimestamp.endUTC,
          relativeTimePeriod: null,
          selectedDate: JSON.parse(JSON.stringify(selectedDate.value)),
          selectedTime: JSON.parse(JSON.stringify(selectedTime.value)),
        };
        // console.log(rVal)
        return rVal;
      }
    };

    const getUTCTimeStamp = () => {
      let startTime =
        selectedDate.value.from + " " + selectedTime.value.startTime;
      let endTime = selectedDate.value.to + " " + selectedTime.value.endTime;
      const startUTC = convertToUtcTimestamp(startTime, store.state.timezone);
      const endUTC = convertToUtcTimestamp(endTime, store.state.timezone);
      return { startUTC, endUTC };
    };

    const setSavedDate = (dateobj: any) => {
      timezone.value = store.state.timezone;
      selectedType.value = dateobj.type;

      if (dateobj.type === "relative") {
        setRelativeTime(dateobj.relativeTimePeriod);
        selectedType.value = "relative";
      } else {
        if (
          dateobj.hasOwnProperty("selectedDate") &&
          dateobj.hasOwnProperty("selectedTime") &&
          dateobj.selectedDate.hasOwnProperty("from") &&
          dateobj.selectedDate.hasOwnProperty("to") &&
          dateobj.selectedTime.hasOwnProperty("startTime") &&
          dateobj.selectedTime.hasOwnProperty("endTime")
        ) {
          selectedDate.value = dateobj.selectedDate;
          selectedTime.value = dateobj.selectedTime;
        } else {
          setCustomDate(dateobj.type, {
            start: dateobj.startTime / 1000,
            end: dateobj.endTime / 1000,
          });
        }
      }

      // displayValue.value = getDisplayValue();
    };

    const getDisplayValue = computed(() => {
      if(!props.showRelative){
        selectedType.value = "absolute";
      }
      if (selectedType.value === "relative") {
        return `Past ${relativeValue.value} ${getPeriodLabel.value}`;
      } else {
        if (selectedDate.value != null) {
          // Here as if multiple dates is selected we get object with from and to keys
          // If single date is selected we get string with date value
          // So have added check for from and to
          if (selectedDate.value?.from && selectedDate.value?.to) {
            return `${selectedDate.value.from} ${selectedTime.value.startTime} - ${selectedDate.value.to} ${selectedTime.value.endTime}`;
          } else {
            return `${selectedDate.value} ${selectedTime.value.startTime} - ${selectedDate.value} ${selectedTime.value.endTime}`;
          }
        } else {
          const todayDate = new Date().toLocaleDateString("en-ZA");
          return `${todayDate} ${selectedTime.value.startTime} - ${todayDate} ${selectedTime.value.endTime}`;
        }
      }
    });

    const timezoneFilterFn = (val, update) => {
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
        filteredOptions = options.filter((column: any) =>
          column.toLowerCase().includes(value),
        );
      });
      return filteredOptions;
    };

    const optionsFn = (date) => {

      const formattedDate = timestampToTimezoneDate(
        new Date().getTime(),
        store.state.timezone,
        "yyyy/MM/dd",
      );
      if(!props.showRelative){
        return date >= props.minDate;
      }
      return date >= "1999/01/01" && date <= formattedDate;
    };

    const setDateType = (type) => {
      selectedType.value = type;
      // displayValue.value = getDisplayValue();

      if (props.autoApply)
        saveDate(type === "absolute" ? "absolute" : "relative-custom");
    };

    const computeRelativePeriod = () => {
      if (selectedType.value === "relative") {
        if (props.queryRangeRestrictionInHour > 0) {
          for (let period of relativePeriods) {
            if (period.value == "s") {
              relativePeriodsMaxValue.value[period.value] = 60;
            }

            if (period.value == "m") {
              relativePeriodsMaxValue.value[period.value] = 60;
            }

            if (period.value == "h" && props.queryRangeRestrictionInHour > 0) {
              relativePeriodsMaxValue.value[period.value] =
                props.queryRangeRestrictionInHour;
            } else if (period.value == "h") {
              relativePeriodsMaxValue.value[period.value] = -1;
            }

            if (period.value == "d" && props.queryRangeRestrictionInHour > 24) {
              relativePeriodsMaxValue.value[period.value] =
                Math.round(props.queryRangeRestrictionInHour / 24) || 31;
            } else if (period.value == "d") {
              relativePeriodsMaxValue.value[period.value] = -1;
            }

            if (
              period.value == "w" &&
              props.queryRangeRestrictionInHour > 24 * 7
            ) {
              relativePeriodsMaxValue.value[period.value] =
                Math.round(props.queryRangeRestrictionInHour / (24 * 7)) || 100;
            } else if (period.value == "w") {
              relativePeriodsMaxValue.value[period.value] = -1;
            }

            if (
              period.value == "M" &&
              props.queryRangeRestrictionInHour > 24 * 30
            ) {
              relativePeriodsMaxValue.value[period.value] =
                Math.round(props.queryRangeRestrictionInHour / (24 * 30)) ||
                100;
            } else if (period.value == "M") {
              relativePeriodsMaxValue.value[period.value] = -1;
            }
          }
        } else {
          relativePeriodsMaxValue.value = {
            s: 0,
            m: 0,
            h: 0,
            d: 0,
            w: 0,
            M: 0,
          };
        }

        relativePeriodsSelect.value = relativePeriods.filter((period) => {
          if (relativePeriodsMaxValue.value[period.value] > -1) {
            return period;
          }
        });

        if (props.queryRangeRestrictionInHour > 0) {
          const maxRelativeValue =
            relativePeriodsMaxValue.value[relativePeriod.value];

          try {
            if (
              maxRelativeValue !== -1 &&
              relativeValue.value > maxRelativeValue
            ) {
              setRelativeDate(relativePeriod.value, maxRelativeValue);
            } else if (maxRelativeValue === -1) {
              const periodIndex = periodUnits.indexOf(relativePeriod.value);
              for (let i = periodIndex; i >= 0; i--) {
                if (relativePeriodsMaxValue.value[periodUnits[i]] > -1) {
                  setRelativeDate(
                    periodUnits[i],
                    relativeDates[periodUnits[i]]?.[0] ?? relativeDates["m"][0], // fallback to 15 minutes,
                  );
                  break;
                }
              }
            }
          } catch (e) {
            console.log("Error while setting relative date", e);
          }
        }
      }
    };

    const showOnlyAbsolute = () =>{
      if(!props.showRelative){
        setDateType("absolute");
      }
    }

    return {
      t,
      datetimeBtn,
      getImageURL,
      onCustomPeriodSelect,
      setRelativeDate,
      relativePeriods,
      relativeDates,
      saveDate,
      onBeforeShow,
      selectedType,
      selectedTime,
      selectedDate,
      relativePeriod,
      relativeValue,
      getPeriodLabel,
      displayValue,
      refresh,
      dateLocale,
      resetTime,
      onTimezoneChange,
      timezone,
      filteredTimezone,
      timezoneFilterFn,
      setCustomDate,
      setSavedDate,
      optionsFn,
      setDateType,
      getConsumableDateTime,
      relativeDatesInHour,
      setAbsoluteTime,
      setRelativeTime,
      getDisplayValue,
      relativePeriodsMaxValue,
      relativePeriodsSelect,
      computeRelativePeriod,
      onBeforeHide,
      showOnlyAbsolute,
    };
  },
  computed: {
    relativePeriodWatch() {
      return this.queryRangeRestrictionInHour;
    },
  },
  watch: {
    relativePeriodWatch() {
      this.computeRelativePeriod();
    },
  },
});
</script>

<style lang="scss" scoped>
.date-time-container {
  .date-time-button {
    &.relativetype {
    }
    &.absolutetype {
      min-width: 286px;
    }
  }
}
</style>
<style lang="scss">
.q-btn--rectangle {
  border-radius: 3px;
}
.date-time-button {
  height: 100%;
  border-radius: 3px;
  padding: 0px 5px;
  font-size: 12px;
  min-width: auto;
  background: rgba(89, 96, 178, 0.2) !important;

  .q-icon.on-right {
    transition: transform 0.25s ease;
  }
  &.isOpen .q-icon.on-right {
    transform: rotate(180deg);
  }

  .q-btn__content {
    justify-content: flex-start;

    .block {
      font-weight: 600;
    }
  }
}

.date-time-dialog {
  width: 341px;
  z-index: 10001;
  max-height: 600px;

  .tab-button {
    &.q-btn {
      padding-bottom: 0.1rem;
      padding-top: 0.1rem;
      font-size: 0.75rem;
      font-weight: 700;

      &.text-primary {
        .q-btn__content {
        }
      }
    }
  }
}

.date-time-table.relative {
  display: flex;

  .relative-row {
    display: flex;
    flex: 1;
    align-items: center;
    border-bottom: 1px solid $border-color;

    .block {
      font-weight: 700;
    }
    .q-field {
      &__control {
        height: 40px;
      }
      &__native {
        font-size: 0.875rem;
        font-weight: 600;
      }
      .q-select__dropdown-icon {
      }
    }

    > * {
      margin-right: 6px;
    }
  }
}

.absolute-calendar {
  box-shadow: none;
  .q-date__header {
    display: none;
  }
  .q-date__view {
    padding: 0;
  }
}

.relative-period-name {
  font-size: 0.875rem;
  font-weight: 600;
  min-width: 75px;
}

.rp-selector,
.rp-selector-selected {
  height: 32px;
  width: 32px;
  // border: $secondary;
  background: rgba(0, 0, 0, 0.07);
}

.rp-selector-selected {
  color: #ffffff;
  background: $primary;
}

.tab-button {
  width: 154px;
}

.notePara {
  padding-right: 1.5rem;
  padding-left: 1.5rem;
  font-size: 0.625rem;
}
.q-date {
  &__navigation {
    justify-content: center;
    padding: 0 0.5rem;

    .q-date__arrow {
      & + .q-date__arrow {
        margin-left: auto;
      }
      & + .col {
        flex: initial;
      }
    }

    .q-btn .block {
      font-size: 0.75rem;
      font-weight: 700;
    }
  }
  &__calendar {
    &-item .block {
      font-weight: 700;
    }
    &-weekdays > div {
      font-size: 0.875rem;
      font-weight: 700;
      opacity: 1;
    }
  }
  &__range {
    &,
    &-from,
    &-to {
      .block {
        color: white;
      }
      &:before {
        bottom: 3px;
        top: 3px;
      }
    }
    .block {
      color: $dark-page;
    }
  }
}
.startEndTime {
  .q-field {
    padding-bottom: 0.125rem;
  }
  .label {
    font-size: 0.75rem;
    // color: $dark-page;
    font-weight: 600;
  }
  .timeInput {
    .q-field__control {
      padding-right: 0.375rem;
    }

    .q-btn-group {
      & > .q-btn-item {
        border-radius: 2px;
      }

      .q-btn {
        padding: 0 0.3125rem;

        .block {
          font-size: 0.625rem;
          font-weight: 700;
        }
      }
    }
  }
}
.drawer-footer {
  .q-btn {
    font-size: 0.75rem;
    font-weight: 700;

    &.clearBtn {
      margin-right: 1rem;
      color: $dark-page;
    }
  }
}
.timezone-select {
  .q-item:nth-child(2) {
    border-bottom: 1px solid #dcdcdc;
  }
}
</style>
