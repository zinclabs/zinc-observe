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
  <div style="height: 100%">
    <div class="q-pa-none" style="width: 100px">
      <q-list separator style="display: flex; flex-wrap: wrap">
        <q-item
          :class="[
            'q-pa-none',
            selectedChartType === item.id
              ? store.state.theme === 'dark'
                ? 'bg-grey-5'
                : 'bg-grey-3'
              : '',
            store.state.theme === 'dark' ? 'darkModeBorder' : 'whiteModeBorder',
          ]"
          v-for="(item, index) in ChartsArray"
          :disable="
            promqlMode &&
            item.id != 'line' &&
            item.id != 'area' &&
            item.id != 'bar' &&
            item.id != 'scatter' &&
            item.id != 'area-stacked' &&
            item.id != 'metric' &&
            item.id != 'gauge' &&
            item.id != 'html' &&
            item.id != 'markdown'
          "
          :key="index"
          clickable
          v-ripple="true"
          @click="$emit('update:selectedChartType', item.id)"
          style="width: 50px"
          data-test="dashboard-addpanel-chart-selection-item"
        >
          <q-item-section
            :data-test="`selected-chart-${item.id}-item`"
            class=""
          >
            <q-icon
              size="sm"
              color="primary"
              :name="item.image"
              class="q-mx-auto q-mb-sm"
              data-test="dashboard-addpanel-chart-selection-icon"
            />
            <!-- <q-item-label
              class="q-pa-none q-mx-auto"
              style="text-align: center; font-size: 8px;"
              caption
              >{{ item.title }}</q-item-label
            > -->
            <q-tooltip
              style="text-align: center"
              caption
              data-test="dashboard-addpanel-chart-selection-tooltip"
              >{{ item.title }}</q-tooltip
            >
          </q-item-section>
        </q-item>
      </q-list>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, inject, ref } from "vue";
import { getImageURL } from "../../../utils/zincutils";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";

export default defineComponent({
  name: "ChartSelection",
  props: ["selectedChartType"],
  emits: ["update:selectedChartType"],

  setup() {
    const store = useStore();
    const { t } = useI18n();
    // array of charts
    const chartsArray = ref([
      {
        image: "img:" + getImageURL("images/dashboard/charts/area.png"),
        title: t("dashboard.areaLabel"),
        id: "area",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/area-graph.png"),
        title: t("dashboard.areaStackedLabel"),
        id: "area-stacked",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/bar-chart.png"),
        title: t("dashboard.barLabel"),
        id: "bar",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/bar-graph.png"),
        title: t("dashboard.horizontalLabel"),
        id: "h-bar",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/line-chart.png"),
        title: t("dashboard.lineLabel"),
        id: "line",
      },
      {
        image:
          "img:" + getImageURL("images/dashboard/charts/scatter-graph.png"),
        title: t("dashboard.scatterLabel"),
        id: "scatter",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/stacked.png"),
        title: t("dashboard.stackedLabel"),
        id: "stacked",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/h-stacked.png"),
        title: t("dashboard.hstackedLabel"),
        id: "h-stacked",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/geomap.png"),
        title: t("dashboard.geomapLabel"),
        id: "geomap",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/world-map.png"),
        title: t("dashboard.worldmapLabel"),
        id: "maps",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/pie-chart.png"),
        title: t("dashboard.pieLabel"),
        id: "pie",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/donut-chart.png"),
        title: t("dashboard.donutLabel"),
        id: "donut",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/heatmap.png"),
        title: t("dashboard.heatmapLabel"),
        id: "heatmap",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/table.png"),
        title: t("dashboard.tableLabel"),
        id: "table",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/123.png"),
        title: t("dashboard.metricTextLabel"),
        id: "metric",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/Gauge.png"),
        title: "Gauge",
        id: "gauge",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/HTML.png"),
        title: "HTML",
        id: "html",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/Markdown.svg"),
        title: "Markdown",
        id: "markdown",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/sankey.svg"),
        title: "Sankey",
        id: "sankey",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/chart.png"),
        title: "Custom Chart",
        id: "custom_chart",
      },
    ]);

    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "dashboard",
    );
    const { promqlMode, dashboardPanelData } = useDashboardPanelData(
      dashboardPanelDataPageKey,
    );
    return {
      t,
      ChartsArray: chartsArray,
      promqlMode,
      dashboardPanelData,
      store,
    };
  },
  components: {},
});
</script>

<style scoped>
.darkModeBorder {
  border: 0.5px solid rgba(255, 255, 255, 0.28);
}

.whiteModeBorder {
  border: 0.5px solid rgba(0, 0, 0, 0.12);
}
</style>
