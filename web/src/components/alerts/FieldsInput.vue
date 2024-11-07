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
    <div data-test="alert-conditions-text" class="text-bold">
      Conditions * (AND operator is used by default to evaluate multiple
      conditions)
    </div>
    <template v-if="!fields.length">
      <q-btn
        data-test="alert-conditions-add-btn"
        color="primary"
        class="q-mt-sm text-bold add-field"
        label="Add Condition"
        size="sm"
        icon="add"
        style="
          border-radius: 4px;
          text-transform: capitalize;
          background: #f2f2f2 !important;
          color: #000 !important;
        "
        @click="addApiHeader"
      />
    </template>
    <template v-else>
      <div
        v-for="(field, index) in fields as any"
        :key="field.uuid"
        class="flex justify-start items-end q-col-gutter-sm q-pb-sm"
        :data-test="`alert-conditions-${index + 1}`"
      >
        <div
          data-test="alert-conditions-select-column"
          class="q-ml-none o2-input"
        >
          <q-select
            v-model="field.column"
            :options="filteredFields"
            :popup-content-style="{ textTransform: 'lowercase' }"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm"
            filled
            emit-value
            dense
            use-input
            hide-selected
            fill-input
            :input-debounce="400"
            :placeholder="t('alerts.column')"
            @filter="filterColumns"
            behavior="menu"
            :rules="[(val: any) => !!val || 'Field is required!']"
            style="min-width: 220px"
            v-bind="newValueMode"
          />
        </div>
        <div
          data-test="alert-conditions-operator-select"
          class="q-ml-none o2-input"
        >
          <q-select
            v-model="field.operator"
            :options="triggerOperators"
            :popup-content-style="{ textTransform: 'capitalize' }"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm"
            stack-label
            outlined
            filled
            dense
            :rules="[(val: any) => !!val || 'Field is required!']"
            style="min-width: 120px"
            @update:model-value="emits('input:update', 'conditions', field)"
          />
        </div>
        <div
          data-test="alert-conditions-value-input"
          class="q-ml-none flex items-end o2-input"
        >
          <q-input
            v-model="field.value"
            :options="streamFields"
            :popup-content-style="{ textTransform: 'capitalize' }"
            :placeholder="t('common.value')"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm"
            stack-label
            outlined
            filled
            dense
            :rules="[(val: any) => !!val || 'Field is required!']"
            style="width: 140px"
            @update:model-value="emits('input:update', 'conditions', field)"
          />
        </div>
        <div
          class="q-ml-none alerts-condition-action"
          style="margin-bottom: 12px"
        >
          <q-btn
            data-test="alert-conditions-delete-condition-btn"
            :icon="outlinedDelete"
            class="q-ml-xs iconHoverBtn"
            :class="store.state?.theme === 'dark' ? 'icon-dark' : ''"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            :title="t('alert_templates.edit')"
            @click="deleteApiHeader(field)"
            style="min-width: auto"
          />
          <q-btn
            data-test="alert-conditions-add-condition-btn"
            v-if="index === fields.length - 1"
            icon="add"
            class="q-ml-xs iconHoverBtn"
            :class="store.state?.theme === 'dark' ? 'icon-dark' : ''"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            :title="t('alert_templates.edit')"
            @click="addApiHeader()"
            style="min-width: auto"
          />
        </div>
      </div>
    </template>
  </div>
</template>

<script lang="ts" setup>
import { defineProps, ref,computed } from "vue";
import { useI18n } from "vue-i18n";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";
import { useStore } from "vuex";

const props = defineProps({
  fields: {
    type: Array,
    default: () => [],
    required: true,
  },
  streamFields: {
    type: Array,
    default: () => [],
    required: true,
  },
  enableNewValueMode: {
      type: Boolean,
      default: false,
    }
  
});
var triggerOperators: any = ref([
  "=",
  "!=",
  ">=",
  "<=",
  ">",
  "<",
  "Contains",
  "NotContains",
]);
const emits = defineEmits(["add", "remove", "input:update"]);

const filteredFields = ref(props.streamFields);

const store = useStore();

const { t } = useI18n();

const deleteApiHeader = (field: any) => {
  emits("remove", field);
  emits("input:update", "conditions", field);
};

const addApiHeader = () => {
  emits("add");
};

const filterColumns = (val: string, update: Function) => {
  if (val === "") {
    update(() => {
      filteredFields.value = [...props.streamFields];
    });
  }
  update(() => {
    const value = val.toLowerCase();
    filteredFields.value = props.streamFields.filter(
      (column: any) => column.value.toLowerCase().indexOf(value) > -1
    );
  });
};
const newValueMode = computed(() => {
      return props.enableNewValueMode ? { 'new-value-mode': 'unique' } : {};
    });

</script>

<style lang="scss">
.add-field {
  .q-icon {
    margin-right: 4px !important;
    font-size: 15px !important;
  }
}

.alerts-condition-action {
  .q-btn {
    &.icon-dark {
      filter: none !important;
    }
  }
}
</style>
