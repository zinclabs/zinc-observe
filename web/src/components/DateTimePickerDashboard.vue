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
  <date-time
    ref="dateTimePicker"
    :auto-apply="autoApplyDashboard"
    :default-type="modelValue.valueType"
    :default-absolute-time="{
      startTime: modelValue.startTime,
      endTime: modelValue.endTime,
    }"
    :default-relative-time="modelValue.relativeTimePeriod"
    @on:date-change="updateDateTime"
    :initialTimezone="initialTimezone"
    :disable="disable"
  >
  </date-time>
</template>

<script lang="ts">
import { ref, defineComponent, reactive, watch, computed } from "vue";
import DateTime from "@/components/DateTime.vue";

export default defineComponent({
  name: "DateTimePickerDashboard",
  components: {
    DateTime,
  },
  props: {
    modelValue: {
      type: Object,
      default: () => ({
        startTime: 0,
        endTime: 0,
        relativeTimePeriod: "15m",
        valueType: "relative",
      }),
    },
    initialTimezone: {
      required: false,
      default: null,
    },
    disable: {
      required: false,
      default: false,
    },
    autoApplyDashboard: {
      required: false,
      default: false,
    }
  },
  emits: ["update:modelValue"],

  setup(props, { emit }) {
    const dateTimePicker: any = ref(null);

    const updateDateTime = (date: any) => {
      const newValue = {
        startTime: date.startTime,
        endTime: date.endTime,
        relativeTimePeriod: date.relativeTimePeriod,
        valueType: ["relative", "relative-custom"].includes(date.valueType)
          ? "relative"
          : "absolute",
      };

      emit("update:modelValue", newValue);
    };

    const refresh = () => {
      dateTimePicker.value.refresh();
    };

    // setCustomDate with type of date(absolute) and dateObj(start and end)
    const setCustomDate = (type: string, dateObj: any) => {
      dateTimePicker.value.setCustomDate(type, dateObj);
    };

    const getConsumableDateTime = () => {
      return dateTimePicker.value.getConsumableDateTime();
    };

    return {
      updateDateTime,
      refresh,
      dateTimePicker,
      setCustomDate,
      getConsumableDateTime,
    };
  },
});
</script>

<style lang="scss" scoped></style>
