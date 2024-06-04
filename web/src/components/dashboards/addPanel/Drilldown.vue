<!-- Copyright 2023 Zinc Labs Inc.

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
  <div>
    <div class="q-mb-sm" style="font-weight: 600">
      <span>Drilldown</span>
      <q-btn
        no-caps
        padding="xs"
        class=""
        size="sm"
        flat
        icon="info_outline"
        data-test="dashboard-addpanel-config-drilldown-info"
      >
        <q-tooltip
          class="bg-grey-8"
          anchor="bottom middle"
          self="top middle"
          max-width="250px"
        >
          Navigate to a another dashboard or specified URL, carrying over all
          current variables for seamless data exploration.
        </q-tooltip>
      </q-btn>
    </div>
    <div
      v-for="(data, index) in dashboardPanelData.data.config.drilldown"
      :key="JSON.stringify(data) + index"
    >
      <div
        style="
          display: flex;
          justify-content: space-between;
          margin-bottom: 5px;
        "
      >
        <div
          @click="onDrilldownClick(index)"
          style="
            cursor: pointer;
            padding-left: 10px;
            width: 250px;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
          "
          :data-test="`dashboard-addpanel-config-drilldown-name-${index}`"
        >
          {{ index + 1 }}. {{ data.name }}
        </div>
        <q-icon
          class="q-mr-xs"
          size="15px"
          name="close"
          style="cursor: pointer"
          @click="removeDrilldownByIndex(index)"
          :data-test="`dashboard-addpanel-config-drilldown-remove-${index}`"
        />
      </div>
    </div>
    <q-btn
      @click="addNewDrilldown"
      style="cursor: pointer; padding: 0px 5px"
      label="+ Add"
      no-caps
      data-test="dashboard-addpanel-config-drilldown-add-btn"
    />
    <q-dialog v-model="showDrilldownPopUp">
      <drilldown-pop-up
        :drilldown-data-index="selectedDrilldownIndexToEdit"
        :is-edit-mode="isDrilldownEditMode"
        :variables-data="variablesData"
        @close="saveDrilldownData"
        :class="store.state.theme == 'dark' ? 'dark-mode' : 'bg-white'"
      />
    </q-dialog>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import DrilldownPopUp from "./DrilldownPopUp.vue";
import { useStore } from "vuex";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import { onBeforeMount } from "vue";

export default defineComponent({
  name: "Drilldown",
  components: { DrilldownPopUp },
  props: ["variablesData"],
  setup() {
    const store = useStore();
    const showDrilldownPopUp = ref(false);
    const isDrilldownEditMode = ref(false);
    const selectedDrilldownIndexToEdit: any = ref(null);
    const { dashboardPanelData } = useDashboardPanelData();

    onBeforeMount(() => {
      // Ensure that the drilldown object is initialized in config
      if (!dashboardPanelData.data.config.drilldown) {
        dashboardPanelData.data.config.drilldown = [];
      }
    });

    const onDrilldownClick = (index: any) => {
      selectedDrilldownIndexToEdit.value = index;
      isDrilldownEditMode.value = true;
      showDrilldownPopUp.value = true;
    };

    const addNewDrilldown = () => {
      isDrilldownEditMode.value = false;
      selectedDrilldownIndexToEdit.value = null;
      showDrilldownPopUp.value = true;
    };

    const removeDrilldownByIndex = (index: any) => {
      dashboardPanelData.data.config.drilldown.splice(index, 1);
    };

    const saveDrilldownData = () => {
      showDrilldownPopUp.value = false;
      selectedDrilldownIndexToEdit.value = null;
      isDrilldownEditMode.value = false;
    };

    return {
      store,
      dashboardPanelData,
      onDrilldownClick,
      showDrilldownPopUp,
      removeDrilldownByIndex,
      selectedDrilldownIndexToEdit,
      saveDrilldownData,
      addNewDrilldown,
      isDrilldownEditMode,
    };
  },
});
</script>

<style lang="scss" scoped></style>
