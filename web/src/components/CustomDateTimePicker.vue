<template>
    <div v-if="props.alertPage" class="flex items-center q-mr-sm">
        <div 
          data-test="scheduled-alert-period-title"
          class="text-bold q-py-md flex items-center"
          style="width: 190px"
        >
        Multi Window Selection
          <q-btn
          no-caps
          padding="xs"
          class=""
          size="sm"
          flat
          icon="info_outline"
          data-test="dashboard-addpanel-config-drilldown-info">
           <q-tooltip
              :class="{ 'alert-page-font': alertPage }" anchor="bottom middle" self="top middle" 
              :style="props.alertPage ? 'font-size: 14px': 'font-size: 10px'"
              
              :max-width="props.alertPage ? '300px' : '250px'" >
             <span>Additional timeframe for query execution: <br />
                For example, selecting "past 10 hours" means that each time the query runs, it will retrieve data from 10 hours prior, using the last 10 minutes of that period. <br /> If the query is scheduled from 4:00 PM to 4:10 PM, additionally it will pull data from 6:00 AM to 6:10 AM.
                </span> 
            </q-tooltip>
          </q-btn>
        </div>
      </div>
    <div  v-for="(picker, index) in dateTimePickers" :key="index" class="q-mb-md">
      <q-btn
     style="width: 180px;"
        data-test="date-time-btn"
        :label="getDisplayValue(picker)"
        icon="schedule"
        icon-right="arrow_drop_down"
        class="date-time-button"
        outline
        no-caps
        @click="picker.showMenu = !picker.showMenu"
      />
      <q-menu
        v-if="picker.showMenu"
        class="date-time-dialog"
        anchor="bottom left"
        self="top left"
        no-route-dismiss
        @before-show="onBeforeShow"
        @before-hide="onBeforeHide"
      >
        <q-tab-panels class="tw-flex tw-justify-between" v-model="picker.activeTab">
          <q-tab-panel name="relative" class="q-pa-none">
            <div class="date-time-table relative column">
              <div
                class="relative-row q-px-md q-py-sm"
                v-for="(period, periodIndex) in relativePeriods"
                :key="'date_' + periodIndex"
              >
                <div class="relative-period-name">
                  {{ period.label }}
                </div>
                <div
                  v-for="(item, itemIndex) in relativeDates[period.value]"
                  :key="item"
                >

                  <q-btn
                 
                    :data-test="`date-time-relative-${item}-${period.value}-btn`"
                    :label="item"
                    :class="
                      picker.data.selectedDate.relative.value == item &&
                      picker.data.selectedDate.relative.period == period.value
                        ? 'rp-selector-selected'
                        : `rp-selector ${picker.relativePeriod}`
                    "
                    outline
                    dense
                    flat
                    @click="setRelativeDate(period, item, picker)"
                  />
                </div>
              </div>

              <div class="relative-row q-px-md q-py-sm">
                <div class="relative-period-name">Custom</div>
                <div class="row q-gutter-sm">
                  <div class="col">
                    <q-input
                      v-model="picker.data.selectedDate.relative.value"
                      type="number"
                      dense
                      filled
                      min="1"
                      @update:model-value="onCustomPeriodSelect(picker)"
                    />
                  </div>
                  <div class="col">
                
                    <q-select
                      v-model="picker.data.selectedDate.relative.period"
                      :options="relativePeriodsSelect"
                      dense
                      filled
                      emit-value
                      @update:modelValue="onCustomPeriodSelect(picker)"
                      style="width: 100px"
                    >
                      <template v-slot:selected-item>
                        <div>{{ getPeriodLabel(picker) }}</div>
                      </template>
                    </q-select>
                  </div>
                </div>
              </div>
            </div>
          </q-tab-panel>
        </q-tab-panels>
      </q-menu>

      <q-btn

      v-if="props.alertPage"
                data-test="custom-date-picker-delete-btn"
                :icon="outlinedDelete"
                class=" q-mb-sm q-ml-xs q-mr-sm"
                :class="store.state?.theme === 'dark' ? 'icon-dark' : ''"
                padding="xs"
                unelevated
                size="sm"
                round
                flat
                @click="removeDateTimePicker(index)"
                style="min-width: auto;"
              />
    </div>

    <q-btn
    v-if="props.alertPage"
      data-test="alert-conditions-add-condition-btn"
      icon="add"
      label="Add"
      class="q-mt-sm"
      @click="addDateTimePicker"
    />
</template>


<script setup>
import { ref, reactive, computed, watch, onMounted, onBeforeMount } from 'vue';
import { useStore } from 'vuex';
import { outlinedDelete,outlinedInfo } from '@quasar/extras/material-icons-outlined';

const store = useStore();
const dateTimePickers = ref([createPicker()]);
const relativePeriod = ref("m");
const relativeValue = ref(15);
const selectedType = ref("relative");




function createPicker() {
  return reactive({
    activeTab: 'relative',
    data: {
      selectedDate: {
        relative: {
          value: 15,
          period: "m",
          label: "Minutes"
        }
      },
    },
  });
}
const props = defineProps({
  alertPage: {
    type: Boolean,
    default: false,
  },
});
const tooltipClass = computed(() => ({
  'bg-grey-8': true,
  'alert-page-font': props.alertPage,
}));


let relativePeriods = [
      { label: "Minutes", value: "m" },
      { label: "Hours", value: "h" },
      { label: "Days", value: "d" },
      { label: "Weeks", value: "w" },
      { label: "Months", value: "M" },
    ];
    let relativePeriodsSelect = ref([
      { label: "Minutes", value: "m" },
      { label: "Hours", value: "h" },
      { label: "Days", value: "d" },
      { label: "Weeks", value: "w" },
      { label: "Months", value: "M" },
    ]);

    const relativeDates = {
      m: [1, 5, 10, 15, 30, 45],
      h: [1, 2, 3, 6, 8, 12],
      d: [1, 2, 3, 4, 5, 6],
      w: [1, 2, 3, 4, 5, 6],
      M: [1, 2, 3, 4, 5, 6],
    };

    const relativeDatesInHour = {
      m: [1, 1, 1, 1, 1, 1],
      h: [1, 2, 3, 6, 8, 12],
      d: [24, 48, 72, 96, 120, 144],
      w: [168, 336, 504, 672, 840, 1008],
      M: [744, 1488, 2232, 2976, 3720, 4464],
    };

    let relativePeriodsMaxValue = ref({
      m: 0,
      h: 0,
      d: 0,
      w: 0,
      M: 0,
    });

const emit = defineEmits(['update:dateTime']);

const setRelativeDate = (period, item,picker) => {
  const {label,value} = period;
  picker.data.selectedDate.relative.period = value;
  picker.data.selectedDate.relative.value = item;
  picker.data.selectedDate.relative.label = label
    };
    const onCustomPeriodSelect = (picker) => {
      const {value,period} = picker.data.selectedDate.relative;
      if(value == 0) {

      }
      // const { value, period } = picker.data.selectedDate.relative;
      // picker.data.selectedDate.relative.label = period;
    };

const dateTimeArray = computed(() => {
  return dateTimePickers.value.map(picker => {
    const { value, period } = picker.data.selectedDate.relative;
    return { offSet: value && period ? `${value}${period}` : null };
  });
});

const onBeforeShow = () => {
      // if (props.modelValue) selectedDate.value = cloneDeep(props.modelValue);
    };

    const onBeforeHide = () => {
      if (selectedType.value === "absolute")
        resetTime(selectedTime.value.startTime, selectedTime.value.endTime);
    };
  const getDisplayValue = (picker) => {
        return `${picker.data.selectedDate.relative.value} ${picker.data.selectedDate.relative.label} ago`;
  
    };



function removeDateTimePicker(index) {
    dateTimePickers.value.splice(index, 1);
    emit('update:dateTime', dateTimeArray.value);
}
const getPeriodLabel = (picker) => {
      const periodMapping = {
        m: "Minutes",
        h: "Hours",
        d: "Days",
        w: "Weeks",
        M: "Months",
      };
      picker.data.selectedDate.relative.label = periodMapping[picker.data.selectedDate.relative.period];
      return periodMapping[picker.data.selectedDate.relative.period];
    };

function addDateTimePicker() {
  dateTimePickers.value.push(createPicker());
  emit('update:dateTime', dateTimeArray.value);
}

watch(dateTimeArray, (newVal) => {
  emit('update:dateTime', newVal);
}, { deep: true });

onBeforeMount(()=>{
  emit('update:dateTime',dateTimeArray.value)

})


</script>


  
  <style scoped>
  .relative-row {
    /* Add your styles here */
  }
  
  .date-time-table {
    /* Add your styles here */
  }
  .alerts-condition-action {
  .q-btn {
    &.icon-dark {
      filter: none !important;
    }
  }
}
.alert-page-font {
  background-color: red;
  font-size: 14px; 
}
  </style>
  <style lang="scss" scoped>
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
  