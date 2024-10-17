<!-- Copyright 2023 OpenObserve Inc.

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

     http:www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License. 
-->

<!-- eslint-disable vue/no-unused-components -->
<template>
  <div
    class="scroll"
    data-test="dashboard-value-mapping-popup"
    style="padding: 0px 10px; min-width: min(1200px, 90vw)"
  >
    <div
      class="flex justify-between items-center q-pa-md header"
      style="border-bottom: 2px solid gray; margin-bottom: 5px"
    >
      <div class="flex items-center q-table__title q-mr-md">
        <span>Value Mappings</span>
      </div>
      <q-btn
        icon="close"
        class="q-ml-xs"
        unelevated
        size="sm"
        round
        outline
        :title="t('dashboard.cancel')"
        @click.stop="cancelEdit"
        data-test="dashboard-tab-settings-tab-name-edit-cancel"
      ></q-btn>
    </div>
    <div class="tw-mb-4">
      <draggable
        v-model="editedValueMapping"
        :options="dragOptions"
        @mousedown.stop="() => {}"
        data-test="dashboard-addpanel-config-value-mapping-drag"
      >
        <div
          v-for="(mapping, index) in editedValueMapping"
          :key="index"
          class="draggable-row"
        >
          <div class="draggable-handle tw-self-center">
            <q-icon
              name="drag_indicator"
              color="grey-13"
              class="q-mr-xs"
              :data-test="`dashboard-addpanel-config-value-mapping-drag-handle-${index}`"
            />
          </div>
          <div class="draggable-content tw-flex tw-gap-x-6">
            <q-select
              v-model="mapping.type"
              label="Type"
              :options="mappingTypes"
              :data-test="`dashboard-addpanel-config-value-mapping-type-select-${index}`"
              emit-value
              map-options
              input-debounce="0"
              behavior="menu"
              filled
              borderless
              dense
              class="q-mb-xs tw-flex-1"
            ></q-select>
            <div
              v-if="mapping.type === 'value'"
              class="input-container tw-flex-1"
            >
              <q-input
                v-model="mapping.value"
                label="Value"
                class="input-spacing"
                dense
                :data-test="`dashboard-addpanel-config-value-mapping-value-input-${index}`"
              />
            </div>
            <div
              v-if="mapping.type === 'regex'"
              class="input-container tw-flex-1"
            >
              <q-input
                v-model="mapping.pattern"
                label="Regex"
                class="input-spacing"
                dense
                :data-test="`dashboard-addpanel-config-value-mapping-pattern-input-${index}`"
              />
            </div>
            <div
              v-if="mapping.type === 'range'"
              class="input-container tw-flex-1"
            >
              <q-input
                v-model="mapping.from"
                label="From"
                class="input-spacing"
                dense
                :data-test="`dashboard-addpanel-config-value-mapping-from-input-${index}`"
              />
              <q-input
                v-model="mapping.to"
                label="To"
                class="input-spacing tw-flex-1"
                dense
                :data-test="`dashboard-addpanel-config-value-mapping-to-input-${index}`"
              />
            </div>
            <q-input
              v-model="mapping.text"
              label="Display Value"
              class="input-spacing tw-flex-1"
              dense
              :data-test="`dashboard-addpanel-config-value-mapping-text-input-${index}`"
            />
            <div class="color-section tw-flex-1">
              <div
                v-if="mapping.color !== null"
                class="tw-items-center tw-flex"
              >
                <q-input
                  v-model="mapping.color"
                  filled
                  style="width: 90%"
                  class="input-spacing"
                  dense
                >
                  <template v-slot:append>
                    <q-icon name="colorize" class="cursor-pointer">
                      <q-popup-proxy cover transition-show="scale">
                        <q-color v-model="mapping.color" />
                      </q-popup-proxy>
                    </q-icon>
                  </template>
                </q-input>
                <q-icon
                  :name="outlinedCancel"
                  style="width: 10%"
                  class="cursor-pointer tw-align-middle"
                  size="xs"
                  title="Remove color"
                  @click="removeColorByIndex(index)"
                />
              </div>
              <div v-else class="tw-w-full">
                <q-btn
                  label="Set color"
                  no-caps
                  flat
                  dense
                  class="tw-text-blue-700 tw-font-semibold tw-w-full"
                  @click="setColorByIndex(index)"
                />
              </div>
            </div>
            <q-btn
              icon="close"
              class="delete-btn"
              dense
              flat
              round
              @click="removeValueMappingByIndex(index)"
              :data-test="`dashboard-addpanel-config-value-mapping-delete-btn-${index}`"
            />
          </div>
        </div>
      </draggable>
      <div class="flex justify-between">
        <q-btn
          @click="addValueMapping"
          label="+ Add a new mapping"
          no-caps
          outline
          dense
          data-test="dashboard-addpanel-config-value-mapping-add-btn"
        />
        <q-btn
          @click="applyValueMapping"
          color="primary"
          label="Apply"
          style="margin-right: 10px"
          padding="5px 14px"
          no-caps
          dense
          data-test="dashboard-addpanel-config-value-mapping-apply-btn"
        />
      </div>
    </div>
  </div>
</template>
<script lang="ts">
import { ref } from "vue";
import { defineComponent } from "vue";
import { useI18n } from "vue-i18n";
import { onMounted } from "vue";
import { VueDraggableNext } from "vue-draggable-next";
import { outlinedCancel } from "@quasar/extras/material-icons-outlined";

export default defineComponent({
  name: "ValueMappingPopUp",
  components: { draggable: VueDraggableNext },
  props: {
    valueMapping: {
      type: Array,
      default: () => [],
    },
  },
  emits: ["close", "save"],
  setup(props: any, { emit }) {
    const { t } = useI18n();

    const editedValueMapping = ref(props.valueMapping);

    const dragOptions = ref({
      animation: 200,
    });

    const mappingTypes = [
      {
        label: "Value",
        value: "value",
      },
      {
        label: "Range",
        value: "range",
      },
      {
        label: "Regex",
        value: "regex",
      },
    ];

    const addValueMapping = () => {
      editedValueMapping.value.push({
        type: "value",
        value: "",
        text: "",
        color: null,
      });
    };

    const removeValueMappingByIndex = (index: number) => {
      editedValueMapping.value.splice(index, 1);
    };

    onMounted(() => {
      // if mappings is empty, add default value mapping
      if (editedValueMapping.value.length == 0) {
        addValueMapping();
      }
    });

    const setColorByIndex = (index: number) => {
      editedValueMapping.value[index].color = "#000000";
    };

    const removeColorByIndex = (index: number) => {
      editedValueMapping.value[index].color = null;
    };

    const applyValueMapping = () => {
      emit("save", editedValueMapping.value);
    };

    const cancelEdit = () => {
      emit("close");
    };

    return {
      t,
      addValueMapping,
      removeValueMappingByIndex,
      mappingTypes,
      dragOptions,
      setColorByIndex,
      removeColorByIndex,
      applyValueMapping,
      cancelEdit,
      editedValueMapping,
      outlinedCancel,
    };
  },
});
</script>

<style lang="scss" scoped>
.draggable-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid #cccccc70;
  margin-bottom: 8px;
}

.draggable-handle {
  cursor: move;
  padding: 8px;
}

.draggable-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex: 1;
}

.input-spacing {
  margin-right: 10px;
}

.color-section {
  display: flex;
  align-items: center;
}

.delete-btn {
  margin-left: 10px;
}
</style>
