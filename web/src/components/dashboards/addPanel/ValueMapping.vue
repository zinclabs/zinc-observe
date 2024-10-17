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
  <div>
    <div class="q-mb-sm" style="font-weight: 600">
      <span>Value Mappings</span>
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
          Enhance table readability by mapping values to custom text and
          backgorund colors for clearer data visualization.
        </q-tooltip>
      </q-btn>
    </div>
    <q-btn
      @click="openValueMappingPopUp"
      style="cursor: pointer; padding: 0px 5px"
      :label="
        dashboardPanelData.data.config.mappings.length
          ? ' Edit Value Mapping'
          : ' Add Value Mapping'
      "
      no-caps
      data-test="dashboard-addpanel-config-drilldown-add-btn"
    />
    <q-dialog v-model="showValueMappingPopUp">
      <ValueMappingPopUp
        :value-mapping="
          JSON.parse(JSON.stringify(dashboardPanelData.data.config.mappings))
        "
        @close="showValueMappingPopUp = false"
        @save="saveValueMappingConfig"
        :class="store.state.theme == 'dark' ? 'dark-mode' : 'bg-white'"
      />
    </q-dialog>
  </div>
</template>

<script lang="ts">
import { defineComponent, inject, ref } from "vue";
import { useStore } from "vuex";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import ValueMappingPopUp from "./ValueMappingPopUp.vue";
import { onBeforeMount } from "vue";

export default defineComponent({
  name: "ValueMapping",
  components: { ValueMappingPopUp },
  props: [],
  setup() {
    const store = useStore();
    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "dashboard"
    );
    const { dashboardPanelData } = useDashboardPanelData(
      dashboardPanelDataPageKey
    );

    const showValueMappingPopUp = ref(false);

    const openValueMappingPopUp = () => {
      showValueMappingPopUp.value = true;
    };

    onBeforeMount(() => {
      // Ensure that the drilldown object is initialized in config
      if (!dashboardPanelData.data.config.mappings) {
        dashboardPanelData.data.config.mappings = [];
      }
    });

    const saveValueMappingConfig = (valueMapping: any) => {
      dashboardPanelData.data.config.mappings = valueMapping;
      showValueMappingPopUp.value = false;
    };

    return {
      store,
      dashboardPanelData,
      showValueMappingPopUp,
      openValueMappingPopUp,
      saveValueMappingConfig,
    };
  },
});
</script>

<style lang="scss" scoped></style>
