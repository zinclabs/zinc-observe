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
  <div class="tag-block row items-center no-wrap q-mr-sm q-mt-sm">
    <div class="q-px-md">{{ tag.key }}</div>
    <q-separator vertical />
    <div class="q-px-md tag-bg" style="word-break: break-all">
      {{ tag.value }}
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps({
  tag: {
    type: Object,
    required: true,
  },
});
</script>

<style lang="scss" scoped>
.tag-block {
  border: 1px solid #e0e0e0;
  border-radius: 4px;
}

.tag-bg {
  background-color: lightgray;
  color: black;
}
</style>
