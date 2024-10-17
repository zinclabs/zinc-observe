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
  <div class="row q-mt-lg q-ml-xs">
    <div class="col-12">
      <div class="tags-title text-bold q-mb-xs">Error Stack</div>
      <div class="q-mb-sm">{{ error_stack[0] }}</div>
      <div class="error-stacks">
        <template v-for="(stack, index) in error_stack" :key="stack">
          <div
            v-if="index"
            class="error_stack q-px-sm"
            :style="{
              'border-top': Number(index) === 1 ? '1px solid #e0e0e0' : '',
              'border-radius':
                Number(index) === error_stack.length - 1
                  ? '0 0 4px 4px'
                  : Number(index) === 1
                  ? '4px 4px 0 0'
                  : '',
            }"
          >
            {{ stack }}
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps({
  error_stack: {
    type: Array,
    required: true,
  },
  error: {
    type: Object,
    required: true,
  },
});
</script>

<style lang="scss">
.tags-title {
  font-size: 16px;
}

.error_stack {
  border-bottom: 1px solid #e0e0e0;
  border-left: 1px solid #e0e0e0;
  border-right: 1px solid #e0e0e0;
  font-size: 13px;
  padding: 6px 8px;
}

.error_stacks:first-child .error_stack {
  border-top: 1px solid #e0e0e0;
}
</style>
