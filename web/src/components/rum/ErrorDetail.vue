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
  <div style="width: 40vw" class="error_details">
    <div @click="handleErrorTypeClick" class="error_type cursor-pointer">
      {{ column.error_type || "Error" }}
    </div>
    <div
      class="error_message cursor-pointer ellipsis q-mt-xs"
      :title="column.error_message"
    >
      {{ column.error_message }}
    </div>
    <div class="error_time row items-center q-mt-xs">
      <span class="q-mr-md text-grey-8"> {{ column.service }}</span>
      <div
        class="q-mr-md"
        :class="
          column.error_handling === 'unhandled'
            ? 'unhandled_error text-red-6 q-px-xs'
            : 'handled_error text-grey-8'
        "
      >
        {{ column.error_handling }}
      </div>
      <q-icon name="schedule" size="14px" class="text-grey-8" />
      <span class="q-pl-xs text-grey-8">{{
        getFormattedDate(column.zo_sql_timestamp / 1000)
      }}</span>
    </div>
  </div>
</template>
<script lang="ts" setup>
import { date } from "quasar";
import { defineProps } from "vue";
const props = defineProps({
  column: {
    type: Object,
    required: true,
  },
});

const emit = defineEmits(["event-emitted"]);

const getFormattedDate = (timestamp: number) =>
  date.formatDate(Math.floor(timestamp), "MMM DD, YYYY HH:mm:ss.SSS Z");

const handleErrorTypeClick = () => {
  emit("event-emitted", {
    type: "error_type_click",
    data: {
      error_type: props,
    },
  });
};
</script>

<style lang="scss" scoped>
.error_type {
  font-size: 16px;
  color: $info;
  text-transform: capitalize;
}

.error_description {
  font-size: 14px;
}

.error_message {
  font-size: 14px;
}

.error_time {
  font-size: 12px;
}

.unhandled_error {
  border: 1px solid rgb(246, 68, 68);
  border-radius: 4px;
}
</style>
