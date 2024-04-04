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
    <div style="display: flex; align-items: center">
      <q-select
        v-model="dashboardPanelData.data.config.color.mode"
        :options="colorOptions"
        outlined
        dense
        label="Color palette"
        class="showLabelOnTop"
        stack-label
        :display-value="selectedOptionLabel"
        @update:model-value="onColorModeChange"
        style="width: 100%"
      >
        <template v-slot:option="props">
          <q-item v-bind="props.itemProps">
            <q-item-section style="padding: 2px">
              <q-item-label>{{ props.opt.label }}</q-item-label>
              <q-item-label caption>
                <div v-if="props.opt.subLabel">
                  <div style="font-weight: 200">
                    {{ props.opt.subLabel }}
                  </div>
                </div>
                <div
                  v-if="Array.isArray(props.opt.colorPalette)"
                  class="color-container"
                >
                  <div
                    :style="{
                      background: `linear-gradient(to right, ${props.opt.colorPalette.join(
                        ', '
                      )})`,
                      width: '100%',
                      height: '8px',
                      borderRadius: '3px',
                    }"
                  ></div>
                </div>
              </q-item-label>
            </q-item-section>
          </q-item>
        </template>
      </q-select>
      <div
        class="color-input-wrapper"
        v-if="
          ['fixed', 'shades'].includes(
            dashboardPanelData.data.config.color.mode
          )
        "
        style="margin-top: 30px; margin-left: 5px"
      >
        <input
          type="color"
          v-model="dashboardPanelData.data.config.color.fixedColor[0]"
          format-model="rgb"
        />
      </div>
    </div>
    <div
      class="q-pt-md"
      v-if="
        ['continuous-green-yellow-red', 'continuous-red-yellow-green'].includes(
          dashboardPanelData.data.config.color.mode
        )
      "
    >
      Color series by:
      <div>
        <q-btn-toggle
          v-model="dashboardPanelData.data.config.color.seriesBy"
          push
          toggle-color="primary"
          size="md"
          :options="[
            { label: 'Last', value: 'last' },
            { label: 'Min', value: 'min' },
            { label: 'Max', value: 'max' },
          ]"
        />
      </div>
    </div>
  </div>
</template>
<script lang="ts">
import useDashboardPanelData from "@/composables/useDashboardPanel";
import { classicColorPalette } from "@/utils/dashboard/colorPalette";
import { computed } from "vue";
import { onBeforeMount } from "vue";
import { defineComponent } from "vue";
export default defineComponent({
  name: "ColorPaletteDropdown",
  setup() {
    const { dashboardPanelData, promqlMode } = useDashboardPanelData();
    // on before mount need to check whether color object is there or not else use palette-classic as a default
    onBeforeMount(() => {
      if (!dashboardPanelData?.data?.config?.color) {
        dashboardPanelData.data.config.color = {
          mode: "palette-classic",
          fixedColor: ["#53ca53"],
          seriesBy: "last",
        };
      }
    });

    const colorOptions = [
      {
        label: "Fixed",
        subLabel: "Set a specific color to all series",
        value: "fixed",
      },
      {
        label: "Shades",
        subLabel: "Different shades of specific color",
        value: "shades",
      },
      {
        label: "Palette-Classic",
        subLabel: "Random color for each series",
        colorPalette: [
          "#5470c6",
          "#91cc75",
          "#fac858",
          "#ee6666",
          "#73c0de",
          "#3ba272",
          "#fc8452",
          "#9a60b4",
          "#ea7ccc",
          "#59c4e6",
          "#edafda",
          "#93b7e3",
          "#a5e7f0",
        ],
        value: "palette-classic",
      },
      {
        label: "Palette-Classic (By Series)",
        subLabel: "Same color for same series name",
        colorPalette: classicColorPalette,
        value: "palette-classic-by-series",
      },
      {
        label: "Green-Yellow-Red",
        colorPalette: ["#00FF00", "#FFFF00", "#FF0000"],
        value: "continuous-green-yellow-red",
      },
      {
        label: "Red-Yellow-Green",
        colorPalette: ["#FF0000", "#FFFF00", "#00FF00"],
        value: "continuous-red-yellow-green",
      },
    ];

    const selectedOptionLabel = computed(() => {
      const selectedOption = colorOptions.find(
        (option) =>
          option.value === dashboardPanelData?.data?.config?.color?.mode ??
          "palette-classic"
      );
      return selectedOption ? selectedOption.label : "palette-classic";
    });

    const onColorModeChange = (value: any) => {
      // if value.value is fixed or shades, assign ["#53ca53"] to fixedcolor
      if (["fixed", "shades"].includes(value.value)) {
        dashboardPanelData.data.config.color.fixedColor = ["#53ca53"];
        dashboardPanelData.data.config.color.seriesBy = "last";
      } else {
        // else assign sublabel to fixedcolor
        dashboardPanelData.data.config.color.fixedColor = value.colorPalette;
      }
      dashboardPanelData.data.config.color.mode = value.value;
    };
    return {
      dashboardPanelData,
      promqlMode,
      colorOptions,
      onColorModeChange,
      selectedOptionLabel,
    };
  },
});
</script>
<style lang="scss" scoped>
:deep(.selectedLabel span) {
  text-transform: none !important;
}
.space {
  margin-top: 10px;
  margin-bottom: 10px;
}
.color-input-wrapper {
  height: 25px;
  width: 25px;
  overflow: hidden;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  position: relative;
}
.color-input-wrapper input[type="color"] {
  position: absolute;
  height: 4em;
  width: 4em;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  overflow: hidden;
  border: none;
  margin: 0;
  padding: 0;
}
.color-container {
  display: flex;
  height: 8px;
}
</style>
