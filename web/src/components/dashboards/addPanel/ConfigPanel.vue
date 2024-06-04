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
    <div class="" style="max-width: 300px">
      <div class="q-mb-sm" style="font-weight: 600">
        {{ t("dashboard.description") }}
      </div>
      <q-input
        outlined
        v-model="dashboardPanelData.data.description"
        filled
        autogrow
        class="showLabelOnTop"
        data-test="dashboard-config-description"
      />
    </div>

    <div class="space"></div>

    <q-toggle
      v-if="
        dashboardPanelData.data.type != 'table' &&
        dashboardPanelData.data.type != 'heatmap' &&
        dashboardPanelData.data.type != 'metric' &&
        dashboardPanelData.data.type != 'gauge' &&
        dashboardPanelData.data.type != 'geomap' &&
        dashboardPanelData.data.type != 'sankey'
      "
      v-model="dashboardPanelData.data.config.show_legends"
      :label="t('dashboard.showLegendsLabel')"
      data-test="dashboard-config-show-legend"
    />

    <div class="space"></div>

    <q-toggle
      v-if="dashboardPanelData.data.type == 'table'"
      v-model="dashboardPanelData.data.config.wrap_table_cells"
      :label="t('dashboard.wraptext')"
      data-test="dashboard-config-wrap-table-cells"
    />

    <div class="space"></div>

    <div class="o2-input">
      <q-select
        v-if="
          dashboardPanelData.data.type != 'table' &&
          dashboardPanelData.data.type != 'heatmap' &&
          dashboardPanelData.data.type != 'metric' &&
          dashboardPanelData.data.type != 'gauge' &&
          dashboardPanelData.data.type != 'geomap' &&
          dashboardPanelData.data.config.show_legends &&
          dashboardPanelData.data.type != 'sankey'
        "
        outlined
        v-model="dashboardPanelData.data.config.legends_position"
        :options="legendsPositionOptions"
        dense
        :label="t('dashboard.legendsPositionLabel')"
        class="showLabelOnTop"
        stack-label
        emit-value
        :display-value="`${
          dashboardPanelData.data.config.legends_position ?? 'Auto'
        }`"
        data-test="dashboard-config-legend-position"
      >
      </q-select>

      <div class="space"></div>

      <div class="input-container">
        <q-input
          v-if="
            dashboardPanelData.data.type != 'table' &&
            dashboardPanelData.data.type != 'heatmap' &&
            dashboardPanelData.data.type != 'metric' &&
            dashboardPanelData.data.type != 'gauge' &&
            dashboardPanelData.data.type != 'geomap' &&
            dashboardPanelData.data.config.show_legends &&
            dashboardPanelData.data.config.legends_position == 'right' &&
            dashboardPanelData.data.type != 'sankey'
          "
          v-model.number="legendWidthValue"
          :label="t('common.legendWidth')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop q-mr-sm"
          stack-label
          outlined
          filled
          dense
          label-slot
          :type="'number'"
          placeholder="Auto"
          data-test="dashboard-config-legend-width"
        ></q-input>
        <div
          class="unit-container"
          v-if="
            dashboardPanelData.data.type != 'table' &&
            dashboardPanelData.data.type != 'heatmap' &&
            dashboardPanelData.data.type != 'metric' &&
            dashboardPanelData.data.type != 'gauge' &&
            dashboardPanelData.data.type != 'geomap' &&
            dashboardPanelData.data.config.show_legends &&
            dashboardPanelData.data.config.legends_position == 'right' &&
            dashboardPanelData.data.type != 'sankey'
          "
        >
          <button
            @click="setUnit('px')"
            :class="{
              active:
                dashboardPanelData?.data?.config.legend_width?.unit === null ||
                dashboardPanelData?.data?.config?.legend_width?.unit === 'px',
            }"
            style="height: 100%; width: 100%; font-size: 14px"
            :data-test="`dashboard-config-legend-width-unit-${
              dashboardPanelData?.data?.config?.legend_width?.unit === 'px'
                ? 'active'
                : 'inactive'
            }`"
          >
            px
          </button>
          <button
            @click="setUnit('%')"
            :class="{
              active:
                dashboardPanelData?.data?.config?.legend_width?.unit === '%',
            }"
            style="height: 100%; width: 100%; font-size: 14px"
            :data-test="`dashboard-config-legend-width-unit-${
              dashboardPanelData?.data?.config?.legend_width?.unit === '%'
                ? 'active'
                : 'inactive'
            }`"
          >
            %
          </button>
        </div>
      </div>

      <div class="space"></div>

      <q-select
        outlined
        v-model="dashboardPanelData.data.config.unit"
        :options="unitOptions"
        dense
        :label="t('dashboard.unitLabel')"
        class="showLabelOnTop selectedLabel"
        stack-label
        emit-value
        :display-value="`${
          dashboardPanelData.data.config.unit
            ? unitOptions.find(
                (it) => it.value == dashboardPanelData.data.config.unit
              )?.label
            : 'Default'
        }`"
        data-test="dashboard-config-unit"
      >
      </q-select>
      <!-- :rules="[(val: any) => !!val || 'Field is required!']" -->
      <q-input
        v-if="dashboardPanelData.data.config.unit == 'custom'"
        v-model="dashboardPanelData.data.config.unit_custom"
        :label="t('dashboard.customunitLabel')"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        filled
        dense
        label-slot
        data-test="dashboard-config-custom-unit"
      />
      <div class="space"></div>
      <q-input
        type="number"
        v-model.number="dashboardPanelData.data.config.decimals"
        value="2"
        min="0"
        max="100"
        @update:model-value="
        (value: any) => (dashboardPanelData.data.config.decimals = ( typeof value == 'number' && value >= 0) ? value : 2)
      "
        :rules="[
          (val) =>
            (val >= 0 && val <= 100) || 'Decimals must be between 0 and 100',
        ]"
        label="Decimals"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        filled
        dense
        label-slot
        data-test="dashboard-config-decimals"
      />

      <div class="space"></div>

      <q-select
        v-if="dashboardPanelData.data.type == 'geomap'"
        outlined
        v-model="dashboardPanelData.data.config.base_map.type"
        :options="basemapTypeOptions"
        dense
        :label="t('dashboard.basemapLabel')"
        class="showLabelOnTop"
        stack-label
        emit-value
        :display-value="'OpenStreetMap'"
        data-test="dashboard-config-basemap"
      >
      </q-select>

      <div class="space"></div>

      <div v-if="dashboardPanelData.data.type == 'geomap'">
        <span>Initial View:</span>
        <div class="row">
          <q-input
            v-model.number="dashboardPanelData.data.config.map_view.lat"
            :value="0"
            @blur="
              handleBlur(dashboardPanelData.data.config.map_view, 0, 'lat')
            "
            :label="t('dashboard.latitudeLabel')"
            color="input-border"
            bg-color="input-bg"
            class="col-6 q-py-md showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            label-slot
            :type="'number'"
            data-test="dashboard-config-latitude"
          >
          </q-input>
          <q-input
            v-model.number="dashboardPanelData.data.config.map_view.lng"
            :value="0"
            @blur="
              handleBlur(dashboardPanelData.data.config.map_view, 0, 'lng')
            "
            :label="t('dashboard.longitudeLabel')"
            color="input-border"
            bg-color="input-bg"
            class="col-6 q-py-md showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            label-slot
            :type="'number'"
            data-test="dashboard-config-longitude"
          >
          </q-input>
        </div>
        <q-input
          v-model.number="dashboardPanelData.data.config.map_view.zoom"
          :value="1"
          @blur="handleBlur(dashboardPanelData.data.config.map_view, 1, 'zoom')"
          :label="t('dashboard.zoomLabel')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          label-slot
          :type="'number'"
          data-test="dashboard-config-zoom"
        >
        </q-input>

        <!-- symbol size -->
        <q-select
          v-model="dashboardPanelData.data.config.map_symbol_style.size"
          :label="t('dashboard.symbolsize')"
          outlined
          :options="symbolOptions"
          dense
          class="showLabelOnTop"
          stack-label
          emit-value
          :display-value="`${dashboardPanelData.data.config.map_symbol_style.size}`"
          data-test="dashboard-config-symbol"
        >
        </q-select>

        <div class="space"></div>

        <div class="row">
          <q-input
            v-if="
              dashboardPanelData.data.config.map_symbol_style.size ===
              'by Value'
            "
            v-model.number="
              dashboardPanelData.data.config.map_symbol_style.size_by_value.min
            "
            :value="1"
            @blur="
              handleBlur(
                dashboardPanelData.data.config.map_symbol_style.size_by_value,
                1,
                'min'
              )
            "
            :label="t('dashboard.minimum')"
            color="input-border"
            bg-color="input-bg"
            class="col-6 q-py-md showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            label-slot
            :type="'number'"
            data-test="dashboard-config-map-symbol-min"
            :min="0"
          >
          </q-input>

          <q-input
            v-if="
              dashboardPanelData.data.config.map_symbol_style.size ===
              'by Value'
            "
            v-model.number="
              dashboardPanelData.data.config.map_symbol_style.size_by_value.max
            "
            :value="100"
            @blur="
              handleBlur(
                dashboardPanelData.data.config.map_symbol_style.size_by_value,
                100,
                'max'
              )
            "
            :label="t('dashboard.maximum')"
            color="input-border"
            bg-color="input-bg"
            class="col-6 q-py-md showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            label-slot
            :type="'number'"
            data-test="dashboard-config-map-symbol-max"
            :min="0"
          >
          </q-input>
        </div>
        <q-input
          v-if="
            dashboardPanelData.data.config.map_symbol_style.size === 'fixed'
          "
          v-model.number="
            dashboardPanelData.data.config.map_symbol_style.size_fixed
          "
          :value="2"
          @blur="
            handleBlur(
              dashboardPanelData.data.config.map_symbol_style,
              2,
              'size_fixed'
            )
          "
          :label="t('dashboard.fixedValue')"
          color="input-border"
          bg-color="input-bg"
          class="col-6 q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          label-slot
          :type="'number'"
          data-test="dashboard-config-map-symbol-fixed"
        >
        </q-input>
      </div>

      <div class="space"></div>

      <!-- <q-input v-if="promqlMode" v-model="dashboardPanelData.data.config.promql_legend" label="Legend" color="input-border"
      bg-color="input-bg" class="q-py-md showLabelOnTop" stack-label outlined filled dense label-slot> -->
      <div
        v-if="promqlMode || dashboardPanelData.data.type == 'geomap'"
        class="q-py-md showLabelOnTop"
        style="font-weight: 600"
      >
        Query
        <q-tabs
          v-model="dashboardPanelData.layout.currentQueryIndex"
          narrow-indicator
          dense
          inline-label
          outside-arrows
          mobile-arrows
          data-test="dashboard-config-query-tab"
        >
          <q-tab
            no-caps
            v-for="(tab, index) in dashboardPanelData.data.queries"
            :key="index"
            :name="index"
            :label="'Query ' + (index + 1)"
            :data-test="`dashboard-config-query-tab-${index}`"
          >
          </q-tab>
        </q-tabs>
      </div>
      <!-- </q-input> -->
      <div class="space"></div>

      <!-- for auto sql query limit -->
      <!-- it should not be promql and custom query -->
      <q-input
        v-if="
          !promqlMode &&
          !dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery
        "
        v-model.number="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.limit
        "
        :value="0"
        :min="0"
        @update:model-value="
          (value) =>
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].config.limit = value ? value : 0)
        "
        label="Limit"
        color="input-border"
        bg-color="input-bg"
        class="q-py-sm showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        placeholder="0"
        :type="'number'"
        data-test="dashboard-config-limit"
      >
        <template v-slot:label>
          <div class="row items-center all-pointer-events">
            Query Limit
            <div>
              <q-icon
                class="q-ml-xs"
                size="20px"
                name="info"
                data-test="dashboard-config-limit-info"
              />
              <q-tooltip
                class="bg-grey-8"
                anchor="top middle"
                self="bottom middle"
              >
                Limit for the query result
              </q-tooltip>
            </div>
          </div>
        </template>
      </q-input>

      <CommonAutoComplete
        v-if="promqlMode"
        :label="t('common.legend')"
        v-model="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.promql_legend
        "
        :items="dashboardSelectfieldPromQlList"
        searchRegex="(?:{([^}]*)(?:{.*})*$|([a-zA-Z-_]+)$)"
        color="input-border"
        bg-color="input-bg"
        class="showLabelOnTop"
        stack-label
        outlined
        label-slot
        style="
          top: none !important;
          margin-top: none !important;
          padding-top: 3px !important;
          width: auto !important;
        "
        :value-replace-fn="selectPromQlNameOption"
      >
        <template v-slot:label>
          <div class="row items-center all-pointer-events">
            {{ t("dashboard.legendLabel") }}
            <div>
              <q-icon
                class="q-ml-xs"
                size="20px"
                name="info"
                data-test="dashboard-config-promql-legend-info"
              />
              <q-tooltip
                class="bg-grey-8"
                anchor="top middle"
                self="bottom middle"
              >
                {{ t("dashboard.overrideMessage") }}
              </q-tooltip>
            </div>
          </div>
        </template>
      </CommonAutoComplete>

      <div class="space"></div>

      <q-toggle
        v-if="
          ['area', 'line', 'area-stacked'].includes(
            dashboardPanelData.data.type
          )
        "
        v-model="dashboardPanelData.data.config.connect_nulls"
        label="Connect null values"
        data-test="dashboard-config-connect-null-values"
      />

      <div class="space"></div>

      <q-select
        v-if="dashboardPanelData.data.type == 'geomap'"
        outlined
        v-model="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.layer_type
        "
        :options="layerTypeOptions"
        dense
        :label="t('dashboard.layerType')"
        class="showLabelOnTop"
        stack-label
        emit-value
        :display-value="`${
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.layer_type
        }`"
        data-test="dashboard-config-layer-type"
      >
      </q-select>

      <div class="space"></div>

      <q-input
        v-if="dashboardPanelData.data.type == 'geomap' && !isWeightFieldPresent"
        v-model.number="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.weight_fixed
        "
        :value="1"
        @blur="
          handleBlur(
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].config,
            1,
            'weight_fixed'
          )
        "
        :label="t('common.weight')"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        :type="'number'"
        data-test="dashboard-config-weight"
      >
      </q-input>

      <q-input
        v-if="dashboardPanelData.data.type === 'gauge'"
        v-model.number="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.min
        "
        :value="0"
        @update:model-value="
          (value) =>
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].config.min = value ? value : 0)
        "
        label="Gauge Min Value"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        :type="'number'"
        data-test="dashboard-config-gauge-min"
      >
        <template v-slot:label>
          <div class="row items-center all-pointer-events">Gauge Min Value</div>
        </template>
      </q-input>
      <q-input
        v-if="dashboardPanelData.data.type === 'gauge'"
        v-model.number="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.max
        "
        :value="100"
        @update:model-value="
          (value) =>
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].config.max = value ? value : 100)
        "
        label="Gauge Max Value"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        placeholder="100"
        :type="'number'"
        data-test="dashboard-config-gauge-max"
      >
        <template v-slot:label>
          <div class="row items-center all-pointer-events">Gauge Max Value</div>
        </template>
      </q-input>

      <q-input
        v-if="
          dashboardPanelData.data.type != 'gauge' &&
          dashboardPanelData.data.type != 'metric' &&
          dashboardPanelData.data.type != 'geomap' &&
          dashboardPanelData.data.type != 'table' &&
          dashboardPanelData.data.type != 'pie' &&
          dashboardPanelData.data.type != 'donut' &&
          dashboardPanelData.data.type != 'sankey'
        "
        v-model.number="dashboardPanelData.data.config.axis_width"
        :label="t('common.axisWidth')"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        :type="'number'"
        placeholder="Auto"
        @update:model-value="
          (value) =>
            (dashboardPanelData.data.config.axis_width =
              value !== '' ? value : null)
        "
        data-test="dashboard-config-axis-width"
      >
      </q-input>

      <div class="space"></div>

      <q-toggle
        v-if="
          dashboardPanelData.data.type != 'gauge' &&
          dashboardPanelData.data.type != 'metric' &&
          dashboardPanelData.data.type != 'geomap' &&
          dashboardPanelData.data.type != 'table' &&
          dashboardPanelData.data.type != 'pie' &&
          dashboardPanelData.data.type != 'donut' &&
          dashboardPanelData.data.type != 'sankey'
        "
        v-model="dashboardPanelData.data.config.axis_border_show"
        :label="t('dashboard.showBorder')"
        data-test="dashboard-config-axis-border"
      />

      <div class="space"></div>
      <Drilldown
        v-if="
          !['html', 'markdown', 'geomap', 'maps'].includes(
            dashboardPanelData.data.type
          )
        "
        :variablesData="variablesData"
      />
    </div>
  </div>
</template>

<script lang="ts">
import useDashboardPanelData from "@/composables/useDashboardPanel";
import { computed, defineComponent, onBeforeMount } from "vue";
import { useI18n } from "vue-i18n";
import Drilldown from "./Drilldown.vue";
import CommonAutoComplete from "@/components/dashboards/addPanel/CommonAutoComplete.vue";

export default defineComponent({
  components: { Drilldown, CommonAutoComplete },
  props: ["dashboardPanelData", "variablesData"],
  setup(props) {
    const { dashboardPanelData, promqlMode } = useDashboardPanelData();
    const { t } = useI18n();

    const basemapTypeOptions = [
      {
        label: t("dashboard.openStreetMap"),
        value: "osm",
      },
    ];

    onBeforeMount(() => {
      // Ensure that the nested structure is initialized
      if (!dashboardPanelData.data.config.legend_width) {
        dashboardPanelData.data.config.legend_width = {
          value: null,
          unit: "px",
        };
      }

      if (!dashboardPanelData.data.config.axis_border_show) {
        dashboardPanelData.data.config.axis_border_show = false;
      }

      // Ensure that the nested structure is initialized
      if (!dashboardPanelData.data.config.map_symbol_style) {
        dashboardPanelData.data.config.map_symbol_style = {
          size: "by Value",
          size_by_value: {
            min: 1,
            max: 100,
          },
          size_fixed: 2,
        };
      }

      // by default, use connect_nulls as false
      if (!dashboardPanelData.data.config.connect_nulls) {
        dashboardPanelData.data.config.connect_nulls = false;
      }

      // by default, use wrap_table_cells as false
      if (!dashboardPanelData.data.config.wrap_table_cells) {
        dashboardPanelData.data.config.wrap_table_cells = false;
      }
    });

    const legendWidthValue = computed({
      get() {
        return dashboardPanelData.data.config?.legend_width?.value;
      },
      set(value) {
        // Ensure that the nested structure is initialized
        if (!dashboardPanelData.data.config.legend_width) {
          dashboardPanelData.data.config.legend_width = {
            value: null,
            unit: "px",
          };
        }

        // Set the value
        dashboardPanelData.data.config.legend_width.value =
          value !== "" ? value : null;
      },
    });

    const setUnit = (unit: any) => {
      // Ensure that the nested structure is initialized
      if (!dashboardPanelData.data.config.legend_width) {
        dashboardPanelData.data.config.legend_width = {
          value: null,
          unit: null,
        };
      }

      // Set the unit
      dashboardPanelData.data.config.legend_width.unit = unit;
    };

    const layerTypeOptions = [
      {
        label: t("dashboard.scatter"),
        value: "scatter",
      },
      {
        label: t("dashboard.heatmap"),
        value: "heatmap",
      },
    ];

    //options for symbol
    const symbolOptions = [
      {
        label: "Fixed",
        value: "fixed",
      },
      {
        label: "By Value",
        value: "by Value",
      },
    ];
    // options for legends position
    const legendsPositionOptions = [
      {
        label: t("dashboard.auto"),
        value: null,
      },
      {
        label: t("dashboard.right"),
        value: "right",
      },
      {
        label: t("dashboard.bottom"),
        value: "bottom",
      },
    ];
    const unitOptions = [
      {
        label: t("dashboard.default"),
        value: null,
      },
      {
        label: t("dashboard.bytes"),
        value: "bytes",
      },
      {
        label: t("dashboard.kilobytes"),
        value: "kilobytes",
      },
      {
        label: t("dashboard.megabytes"),
        value: "megabytes",
      },
      {
        label: t("dashboard.bytesPerSecond"),
        value: "bps",
      },
      {
        label: t("dashboard.seconds"),
        value: "seconds",
      },
      {
        label: t("dashboard.microseconds"),
        value: "microseconds",
      },
      {
        label: t("dashboard.milliseconds"),
        value: "milliseconds",
      },
      {
        label: t("dashboard.percent1"),
        value: "percent-1",
      },
      {
        label: t("dashboard.percent"),
        value: "percent",
      },
      {
        label: t("dashboard.custom"),
        value: "custom",
      },
    ];
    const isWeightFieldPresent = computed(() => {
      const layoutFields =
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields;
      return !!layoutFields?.weight;
    });

    const handleBlur = (field: any, key: any, value: any) => {
      if (!field[value]) {
        field[value] = key;
      }
    };

    const selectPromQlNameOption = (option: any) => {
      const inputValue =
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.promql_legend;

      // Find the index of the last opening brace '{'
      const openingBraceIndex = inputValue.lastIndexOf("{");

      //if { is not present add it at the start and than return

      if (openingBraceIndex === -1) {
        const newValue =
          "{" + inputValue.slice(0, openingBraceIndex + 1) + option + "}";
        return newValue;
      } else {
        const newValue =
          inputValue.slice(0, openingBraceIndex + 1) + option + "}";
        return newValue;
      }
    };

    const dashboardSelectfieldPromQlList = computed(() =>
      props.dashboardPanelData.meta.stream.selectedStreamFields.map(
        (it: any) => {
          return {
            label: it.name,
            value: it.name,
          };
        }
      )
    );

    return {
      t,
      dashboardPanelData,
      promqlMode,
      basemapTypeOptions,
      layerTypeOptions,
      symbolOptions,
      legendsPositionOptions,
      unitOptions,
      isWeightFieldPresent,
      setUnit,
      handleBlur,
      legendWidthValue,
      dashboardSelectfieldPromQlList,
      selectPromQlNameOption,
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

.input-container {
  display: flex;
  align-items: center;
}

.input-container button-group {
  border: 1px solid gray !important;
  border-radius: 9px;
}

.input-container button {
  display: block;
  cursor: pointer;
  background-color: #f0eaea;
  border: none;
  font-size: 16px;
  padding: 3px 10px;
}

.input-container button-left {
  border-top-left-radius: 8px;
  border-bottom-left-radius: 8px;
}

.input-container button-right {
  border-top-right-radius: 8px;
  border-bottom-right-radius: 8px;
}

.input-container button.active {
  background-color: var(--q-primary) !important;
  font-weight: bold;
  color: white;
}
.unit-container {
  display: flex;
  height: 36px;
  margin-top: 9px;
  width: 100px;
}
</style>
