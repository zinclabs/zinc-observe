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
      <div class="q-pt-sm q-pb-xs flex justify-start">
        <div
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
        <span class="text-bold">Event ID:</span>
        <span :title="error.error_id" class="q-pl-xs cursor-pointer"
          >{{ error.error_id }}
          <q-icon
            size="12px"
            name="content_copy"
            @click="copyErrorId(error.error_id)"
        /></span>
        <span class="q-ml-lg">{{ error.timestamp }}</span>
      </div>
      <div class="row items-center no-wrap q-my-xs">
        <div class="error_type text-bold">{{ error.type }}</div>
      </div>
      <div class="error_message q-pt-xs row items-center">
        <div
          v-if="error.error_handling === 'unhandled'"
          :class="
            error.error_handling === 'unhandled'
              ? 'unhandled_error text-red-6 q-px-xs q-mr-sm'
              : ''
          "
        >
          {{ error.error_handling }}
        </div>
        {{ error.error_message }}
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { defineProps } from "vue";
import { useRouter } from "vue-router";
import { copyToClipboard, useQuasar } from "quasar";

const router = useRouter();
const q = useQuasar();
const props = defineProps({
  error: {
    type: Object,
    required: true,
  },
});

const copyErrorId = (id: string) => {
  q.notify({
    type: "positive",
    message: "Copied to clipboard",
    timeout: 1500,
  });
  copyToClipboard(id);
};
</script>

<style lang="scss">
.error_type {
  font-size: 22px;
}

.error_description {
  font-size: 18px;
}

.error_message {
  font-size: 16px;
}
.error_symbol {
  width: 12px;
  height: 12px;
  background-color: rgb(251, 119, 119);
  border-radius: 50%;
}
.error_time {
  font-size: 16px;
}

.unhandled_error {
  border: 1px solid rgb(246, 68, 68);
  border-radius: 4px;
  font-size: 14px;
}
</style>
