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

<!-- eslint-disable vue/v-on-event-hyphenation -->
<!-- eslint-disable vue/attribute-hyphenation -->
<template>
  <q-page class="relative-position">
    <div
      class="q-mx-sm performance-dashboard"
      :style="{ visibility: isLoading.length ? 'hidden' : 'visible' }"
    >
      <div
        class="text-bold q-ml-sm q-px-sm rounded q-mt-md q-py-xs learn-web-vitals-link flex items-center"
        :class="store.state.theme === 'dark' ? 'bg-indigo-7' : 'bg-indigo-2'"
      >
        <q-icon
          name="info"
          size="16px"
          class="material-symbols-outlined q-mr-xs"
        />
        {{ t("rum.learnWebVitalsLabel") }}
        <a
          href="https://web.dev/articles/vitals"
          title="https://web.dev/articles/vitals"
          class="q-ml-xs"
          target="_blank"
          :class="store.state.theme === 'dark' ? 'text-white' : 'text-dark'"
        >
          {{ t("rum.clickHereLabel") }}
        </a>
      </div>
      <RenderDashboardCharts
        ref="webVitalsChartsRef"
        :viewOnly="true"
        :dashboardData="currentDashboardData.data"
        :currentTimeObj="dateTime"
        searchType="RUM"
      />
    </div>
    <div
      v-show="isLoading.length"
      class="q-pb-lg flex items-center justify-center text-center absolute full-width"
      style="height: calc(100vh - 250px); top: 0"
    >
      <div>
        <q-spinner-hourglass
          color="primary"
          size="40px"
          style="margin: 0 auto; display: block"
        />
        <div class="text-center full-width">Loading Dashboard</div>
      </div>
    </div>
  </q-page>
</template>

<script lang="ts">
// @ts-nocheck
import {
  defineComponent,
  ref,
  watch,
  onActivated,
  nextTick,
  onMounted,
} from "vue";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { getConsumableDateTime, getDashboard } from "@/utils/commons.ts";
import {
  parseDuration,
  generateDurationLabel,
  getDurationObjectFromParams,
  getQueryParamsForDuration,
} from "@/utils/date";
import { toRaw, unref, reactive } from "vue";
import { useRoute } from "vue-router";
import RenderDashboardCharts from "@/views/Dashboards/RenderDashboardCharts.vue";
import overviewDashboard from "@/utils/rum/web_vitals.json";
import { convertDashboardSchemaVersion } from "../../../utils/dashboard/convertDashboardSchemaVersion";

export default defineComponent({
  name: "WebVitalsDashboard",
  components: {
    RenderDashboardCharts,
  },
  props: {
    dateTime: {
      type: Object,
      default: () => ({}),
    },
  },
  setup() {
    const { t } = useI18n();
    const route = useRoute();
    const router = useRouter();
    const store = useStore();
    const currentDashboardData = reactive({
      data: {},
    });
    const showDashboardSettingsDialog = ref(false);
    const viewOnly = ref(true);
    const eventLog = ref([]);

    const refDateTime: any = ref(null);
    const currentDurationSelectionObj = ref({});
    const refreshInterval = ref(0);
    const selectedDate = ref();
    const webVitalsChartsRef = ref(null);
    const isLoading: Ref<boolean[]> = ref([]);

    // variables data
    const variablesData = reactive({});
    const variablesDataUpdated = (data: any) => {
      Object.assign(variablesData, data);
    };

    onMounted(async () => {
      await loadDashboard();
      updateLayout();
    });

    onActivated(() => {
      updateLayout();
    });

    const updateLayout = async () => {
      await nextTick();
      await nextTick();
      await nextTick();
      await nextTick();
      // emit window resize event to trigger the layout
      webVitalsChartsRef.value.layoutUpdate();

      // Dashboards gets overlapped as we have used keep alive
      // Its an internal bug of vue-grid-layout
      // So adding settimeout of 1 sec to fix the issue

      webVitalsChartsRef.value.layoutUpdate();
      window.dispatchEvent(new Event("resize"));
    };

    const loadDashboard = async () => {
      // schema migration
      currentDashboardData.data =
        convertDashboardSchemaVersion(overviewDashboard);

      // if variables data is null, set it to empty list

      if (
        !(
          currentDashboardData.data?.variables &&
          currentDashboardData.data?.variables?.list.length
        )
      ) {
        variablesData.isVariablesLoading = false;
        variablesData.values = [];
      }
    };

    const addSettingsData = () => {
      showDashboardSettingsDialog.value = true;
    };

    // back button to render dashboard List page
    const goBackToDashboardList = () => {
      return router.push({
        path: "/dashboards",
        query: {
          dashboard: route.query.dashboard,
          folder: route.query.folder ?? "default",
        },
      });
    };

    //add panel
    const addPanelData = () => {
      return router.push({
        path: "/dashboards/add_panel",
        query: {
          dashboard: route.query.dashboard,
          folder: route.query.folder ?? "default",
        },
      });
    };

    // ------- work with query params ----------
    onActivated(async () => {
      const params = route.query;

      if (params.refresh) {
        refreshInterval.value = parseDuration(params.refresh);
      }

      if (params.period || (params.to && params.from)) {
        selectedDate.value = getDurationObjectFromParams(params);
      }

      // resize charts if needed
      await nextTick();
      window.dispatchEvent(new Event("resize"));
    });

    // whenever the refreshInterval is changed, update the query params
    watch([refreshInterval, selectedDate], () => {
      router.replace({
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
          dashboard: route.query.dashboard,
          folder: route.query.folder,
          refresh: generateDurationLabel(refreshInterval.value),
          ...getQueryParamsForDuration(selectedDate.value),
        },
      });
    });

    return {
      currentDashboardData,
      goBackToDashboardList,
      addPanelData,
      t,
      getDashboard,
      store,
      refDateTime,
      filterQuery: ref(""),
      filterData(rows: string | any[], terms: string) {
        const filtered = [];
        terms = terms.toLowerCase();
        for (let i = 0; i < rows.length; i++) {
          if (rows[i]["name"].toLowerCase().includes(terms)) {
            filtered.push(rows[i]);
          }
        }
        return filtered;
      },
      refreshInterval,
      selectedDate,
      viewOnly,
      eventLog,
      variablesData,
      variablesDataUpdated,
      addSettingsData,
      showDashboardSettingsDialog,
      loadDashboard,
      webVitalsChartsRef,
      isLoading,
    };
  },
});
</script>

<style lang="scss" scoped>
.learn-web-vitals-link {
  font-size: 14px;
  width: fit-content;
  border-radius: 4px;
}
.performance_title {
  font-size: 24px;
}
.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}
</style>

<style lang="scss"></style>
