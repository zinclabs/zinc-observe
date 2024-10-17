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
  <div class="q-mt-lg">
    <div class="tags-title text-bold q-mb-sm q-ml-xs">Session Replay</div>
    <div class="row">
      <template v-for="(value, tag) in getSessionTags" :key="tag.tag">
        <ErrorTag :tag="{ key: tag, value }" />
      </template>
    </div>
    <div
      class="play-button bg-primary cursor-pointer q-mt-md text-white row items-center"
      style="width: fit-content; border-radius: 3px; padding: 6px 8px"
      @click="playSessionReplay"
    >
      <q-icon name="play_circle" size="18px" class="q-mr-xs" /> Play Session
      Replay
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import ErrorTag from "./ErrorTag.vue";
import { useRouter } from "vue-router";

const props = defineProps({
  error: {
    type: Object,
    required: true,
  },
});

const router = useRouter();

const getSessionTags = computed(() => {
  return {
    session_id: props.error.session_id,
    view_id: props.error.view_id,
  };
});

const playSessionReplay = () => {
  router.push({
    name: "SessionViewer",
    params: {
      id: props.error.session_id,
    },
    query: {
      start_time: props.error._timestamp,
      end_time: props.error._timestamp,
    },
  });
};
</script>

<style scoped>
.tags-title {
  font-size: 16px;
}
</style>
