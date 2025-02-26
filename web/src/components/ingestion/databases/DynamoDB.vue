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

<script setup lang="ts">
import { useStore } from "vuex";
import CopyContent from "@/components/CopyContent.vue";
import useIngestion from "@/composables/useIngestion";
const docURL = "https://short.openobserve.ai/database/dynamodb"
const name = "DynamoDB";
const store = useStore();
const { endpoint, databaseContent } = useIngestion();
const content = databaseContent.replace("[STREAM_NAME]", name.replace(" ", "_").toLowerCase());
</script>

<template>
  <div class="q-ma-md">
    <div class="tw-text-[16px]">
      <CopyContent class="q-mt-sm" :content="content" />
      <div class="tw-font-bold tw-pt-6 tw-pb-2">
        Check further documentation at:
        <br />
        <a :href="docURL" target="_blank" class="text-blue-500 hover:text-blue-600">{{docURL}}</a>
      </div>
    </div>
  </div>
</template>