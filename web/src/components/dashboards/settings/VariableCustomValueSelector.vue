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
  <div>
    <q-select
      style="min-width: 150px"
      filled
      outlined
      dense
      v-model="selectedValue"
      :display-value="displayValue"
      :label="variableItem?.label || variableItem?.name"
      :options="fieldsFilteredOptions"
      input-debounce="0"
      emit-value
      option-value="value"
      option-label="label"
      behavior="menu"
      use-input
      stack-label
      @filter="fieldsFilterFn"
      class="textbox col no-case"
      :loading="variableItem.isLoading"
      data-test="dashboard-variable-query-value-selector"
      :multiple="variableItem.multiSelect"
      popup-no-route-dismiss
      popup-content-style="z-index: 10001"
    >
      <template v-slot:no-option>
        <q-item>
          <q-item-section class="text-italic text-grey">
            No Data Found
          </q-item-section>
        </q-item>
      </template>
      <template
        v-if="variableItem.multiSelect && fieldsFilteredOptions.length > 0"
        v-slot:before-options
      >
        <q-item>
          <q-item-section side>
            <q-checkbox
              v-model="isAllSelected"
              @update:model-value="toggleSelectAll"
              dense
              class="q-ma-none"
            />
          </q-item-section>
          <q-item-section @click.stop="toggleSelectAll" style="cursor: pointer">
            <q-item-label>Select All</q-item-label>
          </q-item-section>
        </q-item>
        <q-separator />
      </template>
      <template v-slot:option="{ itemProps, opt, selected, toggleOption }">
        <q-item v-bind="itemProps">
          <q-item-section side v-if="variableItem.multiSelect">
            <q-checkbox
              :model-value="selected"
              @update:model-value="toggleOption(opt)"
              class="q-ma-none"
              dense
            />
          </q-item-section>
          <q-item-section>
            <q-item-label v-html="opt.label" />
          </q-item-section>
        </q-item>
      </template>
    </q-select>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, toRef, watch, computed } from "vue";
import { useSelectAutoComplete } from "../../../composables/useSelectAutocomplete";

export default defineComponent({
  name: "VariableCustomValueSelector",
  props: ["modelValue", "variableItem"],
  emits: ["update:modelValue"],
  setup(props: any, { emit }) {
    //get v-model value for selected value  using props
    const selectedValue = ref(props.variableItem?.value);

    const options = toRef(props.variableItem, "options");

    // get filtered options
    const { filterFn: fieldsFilterFn, filteredOptions: fieldsFilteredOptions } =
      useSelectAutoComplete(options, "value");

    // set watcher on variable item changes at that time change the option value
    watch(
      () => props.variableItem,
      () => {
        options.value = props.variableItem?.options;
      }
    );

    // isAllSelected should be true if all options are selected and false otherwise
    const isAllSelected = computed(() => {
      return (
        fieldsFilteredOptions.value.length > 0 &&
        selectedValue.value.length === fieldsFilteredOptions.value.length
      );
    });

    // Function to toggle select/deselect all options
    const toggleSelectAll = () => {
      if (!isAllSelected.value) {
        selectedValue.value = fieldsFilteredOptions.value.map(
          (option: any) => option.value
        );
      } else {
        selectedValue.value = [];
      }
    };

    // update selected value
    watch(selectedValue, () => {
      emit("update:modelValue", selectedValue.value);
    });

    // Display the selected value
    const displayValue = computed(() => {
      if (selectedValue.value) {
        if (Array.isArray(selectedValue.value)) {
          if (selectedValue.value.length > 2) {
            const firstTwoValues = selectedValue.value.slice(0, 2).join(", ");
            const remainingCount = selectedValue.value.length - 2;
            return `${firstTwoValues} ...+${remainingCount} more`;
          } else {
            return selectedValue.value.join(", ");
          }
        } else {
          return selectedValue.value;
        }
      } else if (!props.variableItem.isLoading) {
        return "(No Options Available)";
      } else {
        return "";
      }
    });

    return {
      selectedValue,
      fieldsFilterFn,
      fieldsFilteredOptions,
      isAllSelected,
      toggleSelectAll,
      displayValue,
    };
  },
});
</script>

<style lang="scss" scoped></style>
