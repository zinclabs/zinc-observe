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
  <div>
    <div>
      <div class="row items-center no-wrap">
        <span :class="`fi fi-${column.country_iso_code} q-mr-sm`" />
        <div style="font-size: 12px">{{ column.country }}</div>
      </div>
      <div class="row q-mt-xs items-center">
        <div class="text-grey-8">
          {{ column.city || "Unknown" }}
        </div>
        <q-icon name="circle" size="4px" class="q-mx-md text-grey-6" />
        <div class="text-grey-8">{{ column.browser }}</div>
        <q-icon name="circle" size="4px" class="q-mx-md text-grey-6" />
        <div class="text-grey-8">{{ column.os }}</div>
      </div>
    </div>
  </div>
</template>
<script lang="ts" setup>
import { defineProps } from "vue";
import "flag-icons/css/flag-icons.min.css";

defineProps({
  column: {
    type: Object,
    required: true,
  },
});
</script>
<style lang="scss">
.error_type {
  font-size: 16px;
  color: $info;
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
</style>
