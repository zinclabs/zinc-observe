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
  <q-btn
    id="date-time-button"
    ref="datetimeBtn"
    data-cy="date-time-button"
    outline
    no-caps
    :label="displayValue"
    icon="schedule"
    icon-right="arrow_drop_down"
    class="date-time-button"
    color=""
  >
    <q-menu
      no-route-dismiss
      id="date-time-menu"
      class="date-time-dialog"
      anchor="bottom left"
      self="top left"
    >
      <div class="flex justify-evenly q-py-sm">
        <q-btn
          class="tab-button no-border"
          color="primary"
          :flat="data.selectedDate.tab !== 'relative'"
          @click="data.selectedDate.tab = 'relative'"
        >
          {{ t("common.datetimeRelative") }}
        </q-btn>
        <q-separator vertical inset />
        <q-btn
          class="tab-button no-border"
          color="primary"
          :flat="data.selectedDate.tab !== 'absolute'"
          @click="data.selectedDate.tab = 'absolute'"
        >
          {{ t("common.datetimeAbsolute") }}
        </q-btn>
      </div>
      <q-separator />
      <q-tab-panels v-model="data.selectedDate.tab" animated>
        <q-tab-panel name="relative" class="q-pa-none">
          <div class="date-time-table relative column">
            <div
              class="relative-row q-px-md q-py-sm"
              v-for="(period, index) in relativePeriods"
              :key="'date_' + index"
            >
              <div class="relative-period-name">
                {{ period.value }}
              </div>
              <div
                v-for="(item, item_index) in (relativeDates as any)[period.value]"
                :key="item"
              >
                <q-btn
                  :class="
                    data.selectedDate.tab == 'relative' &&
                    data.selectedDate.relative.period.value == period.value &&
                    data.selectedDate.relative.value == item
                      ? 'rp-selector-selected'
                      : `rp-selector ${data.selectedDate.relative.period.value}`
                  "
                  :label="item"
                  outline
                  dense
                  flat
                  @click="setRelativeDate(period, item)"
                  :key="'period_' + item_index"
                />
              </div>
            </div>

            <div class="relative-row q-px-md q-py-sm">
              <div class="relative-period-name">Custom</div>

              <div class="row q-gutter-sm">
                <div class="col">
                  <q-input
                    v-model="data.selectedDate.relative.value"
                    type="number"
                    dense
                    filled
                    min="1"
                    @change="calculateMaxValue"
                  ></q-input>
                </div>
                <div class="col">
                  <q-select
                    v-model="data.selectedDate.relative.period"
                    :options="relativePeriods"
                    dense
                    filled
                    @update:modelValue="onCustomPeriodSelect"
                  ></q-select>
                </div>
              </div>
            </div>
          </div>
        </q-tab-panel>
        <q-tab-panel name="absolute" class="q-pa-none">
          <div class="date-time-table">
            <div class="flex justify-center q-pa-none">
              <q-date
                v-model="data.selectedDate.absolute.date"
                class="absolute-calendar"
                range
                :locale="{
                  daysShort: ['S', 'M', 'T', 'W', 'T', 'F', 'S'],
                }"
              />
            </div>
            <div class="notePara">{{ t("common.datetimeMessage") }}</div>
            <q-separator class="q-my-sm" />

            <table class="q-px-md startEndTime">
              <tr>
                <td class="label">{{ t("common.startTime") }}</td>
                <td class="label">{{ t("common.endTime") }}</td>
              </tr>
              <tr>
                <td>
                  <q-input
                    v-model="data.selectedDate.absolute.startTime"
                    dense
                    filled
                    mask="time"
                    :rules="['time']"
                  >
                    <template #append>
                      <q-icon name="access_time" class="cursor-pointer">
                        <q-popup-proxy
                          transition-show="scale"
                          transition-hide="scale"
                        >
                          <q-time
                            v-model="data.selectedDate.absolute.startTime"
                          >
                            <div class="row items-center justify-end">
                              <q-btn
                                v-close-popup="true"
                                :label="t('common.close')"
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
                    v-model="data.selectedDate.absolute.endTime"
                    dense
                    filled
                    mask="time"
                    :rules="['time']"
                  >
                    <template #append>
                      <q-icon name="access_time" class="cursor-pointer">
                        <q-popup-proxy
                          transition-show="scale"
                          transition-hide="scale"
                        >
                          <q-time v-model="data.selectedDate.absolute.endTime">
                            <div class="row items-center justify-end">
                              <q-btn
                                v-close-popup="true"
                                :label="t('common.close')"
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
    </q-menu>
  </q-btn>
</template>

<script lang="ts">
import { ref, defineComponent, reactive, watch, computed } from "vue";
import { getImageURL } from "../utils/zincutils";
import { isEqual } from "lodash-es";
import { useI18n } from "vue-i18n";

export default defineComponent({
  name: "DateTimePicker",
  props: {
    modelValue: {
      type: Object,
      default: () => ({
        tab: "relative",
        relative: {
          period: { label: "Minutes", value: "Minutes" },
          value: 15,
        },
        absolute: {
          date: {
            from: new Date().toLocaleDateString("en-ZA"),
            to: new Date().toLocaleDateString("en-ZA"),
          },
          startTime: "00:00",
          endTime: "23:59",
        },
      }),
    },
  },
  emits: ["update:modelValue"],

  setup(props, { emit }) {
    const datetimeBtn = ref();
    const { t } = useI18n();

    // v-model computed value which is used for the getvalue and setvalue for the props
    const selectedDateEmitValue = computed({
      get() {
        return props.modelValue;
      },
      set(value) {
        emit("update:modelValue", value);
      },
    });

    // when the props value is changed copy in to the data.selectedDate
    // because data.selectedDate has actual value
    watch(selectedDateEmitValue, () => {
      Object.assign(
        data.selectedDate,
        JSON.parse(JSON.stringify(props.modelValue))
      );
    });

    const relativePeriods = [
      { label: "Minutes", value: "Minutes" },
      { label: "Hours", value: "Hours" },
      { label: "Days", value: "Days" },
      { label: "Weeks", value: "Weeks" },
      { label: "Months", value: "Months" },
    ];

    const relativeDates = {
      Minutes: [1, 5, 10, 15, 30, 45],
      Hours: [1, 2, 3, 6, 8, 12],
      Days: [1, 2, 3, 4, 5, 6],
      Weeks: [1, 2, 3, 4, 5, 6],
      Months: [1, 2, 3, 4, 5, 6],
    };

    // Internal date object, who is containing actual value
    const data = reactive({
      selectedDate: JSON.parse(JSON.stringify(props.modelValue)),
    });

    // this method set the value of date when the relative tab is selected
    const setRelativeDate = (period: any, value: any) => {
      data.selectedDate.tab = "relative";
      data.selectedDate.relative.period = period;
      data.selectedDate.relative.value = value;
      datetimeBtn.value.click();
    };

    const onCustomPeriodSelect = () => {
      data.selectedDate.relative.value = 1;
    };

    // change the actual date object based on selected tab.
    const displayValue = computed(() => {
      if (data.selectedDate.tab === "relative") {
        return `${data.selectedDate.relative.value} ${data.selectedDate.relative.period.label}`;
      } else {
        if (data.selectedDate.absolute.date != null) {
          if (typeof data.selectedDate.absolute.date != "object") {
            if (data.selectedDate.absolute.date != "") {
              data.selectedDate.absolute.date = {
                from: data.selectedDate.absolute.date,
                to: data.selectedDate.absolute.date,
              };
            } else {
              const todayDate = new Date().toLocaleDateString("en-ZA");
              data.selectedDate.absolute.date = {
                from: todayDate,
                to: todayDate,
              };
            }
          }

          return `${data.selectedDate.absolute.date.from} ${data.selectedDate.absolute.startTime} - ${data.selectedDate.absolute.date.to} ${data.selectedDate.absolute.endTime}`;
        } else {
          const todayDate = new Date().toLocaleDateString("en-ZA");
          data.selectedDate.absolute.date = {
            from: todayDate,
            to: todayDate,
          };
          return `${todayDate} ${data.selectedDate.absolute.startTime} - ${todayDate} ${data.selectedDate.absolute.endTime}`;
        }
      }
    });

    const calculateMaxValue = () => {
      if (
        data.selectedDate.relative.period.value == "Minutes" &&
        data.selectedDate.relative.value > 60
      ) {
        data.selectedDate.relative.value = 60;
        return false;
      } else if (
        data.selectedDate.relative.period.value == "Hours" &&
        data.selectedDate.relative.value > 24
      ) {
        data.selectedDate.relative.value = 24;
        return false;
      } else if (
        data.selectedDate.relative.period.value == "Days" &&
        data.selectedDate.relative.value > 31
      ) {
        data.selectedDate.relative.value = 31;
        return false;
      } else if (
        data.selectedDate.relative.period.value == "Weeks" &&
        data.selectedDate.relative.value > 6
      ) {
        data.selectedDate.relative.value = 6;
        return false;
      } else if (
        data.selectedDate.relative.period.value == "Months" &&
        data.selectedDate.relative.value > 6
      ) {
        data.selectedDate.relative.value = 6;
        return false;
      }
    };

    // set watcher on date selected variable.
    // when, the old value and new value is not same at that time copy the new value in selectedDateEmit
    watch(
      data.selectedDate,
      () => {
        if (!isEqual(selectedDateEmitValue.value, data.selectedDate)) {
          updateEmitValue();
        }
      },
      { deep: true }
    );

    const updateEmitValue = () => {
      selectedDateEmitValue.value = JSON.parse(
        JSON.stringify(data.selectedDate)
      );
    };

    // on the initial call emit the value once
    updateEmitValue();

    return {
      t,
      relativePeriods,
      relativeDates,
      data,
      setRelativeDate,
      onCustomPeriodSelect,
      datetimeBtn,
      displayValue,
      calculateMaxValue,
      getImageURL,
    };
  },
});
</script>

<style lang="scss" scoped>
.q-btn--rectangle {
  border-radius: 3px;
}

.date-time-button {
  border-radius: 3px;
  padding: 0px 5px;
  // font-size: 12px;
  min-width: auto;

  .q-icon.on-right {
    transition: transform 0.25s ease;
    color: $light-text2;
  }

  &.isOpen .q-icon.on-right {
    transform: rotate(180deg);
  }

  .q-btn__content {
    justify-content: flex-start;

    .block {
      color: $dark-page;
      font-weight: 600;

      &::before {
        content: "Past ";
      }
    }
  }
}

.date-time-dialog {
  width: 341px;

  .tab-button {
    &.q-btn {
      padding-bottom: 0.6rem;
      padding-top: 0.6rem;
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
        color: $light-text2;
        font-size: 0.875rem;
        font-weight: 600;
      }

      .q-select__dropdown-icon {
        color: $light-text2;
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
  color: $light-text;
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
      color: $dark-page;
      font-weight: 700;
    }
  }

  &__calendar {
    &-item .block {
      color: $light-text;
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
</style>
