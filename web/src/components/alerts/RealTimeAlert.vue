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
    <fields-input
      class="q-mt-md"
      :stream-fields="columns"
      :fields="conditions"
      @add="addField"
      @remove="removeField"
      @input:update="
        (name: string, field: any) => emits('input:update', name, field)
      "
    />
  </div>
</template>

<script lang="ts" setup>
import FieldsInput from "./FieldsInput.vue";

defineProps(["columns", "conditions"]);

const emits = defineEmits(["field:add", "field:remove", "input:update"]);

const addField = (field: any) => {
  emits("field:add", field);
  emits("input:update", "conditions", field);
};

const removeField = (field: any) => {
  emits("field:remove", field);
  emits("input:update", "conditions", field);
};
</script>

<style lang="scss" scoped></style>
