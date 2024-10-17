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
      <RenderDashboardCharts
        ref="performanceChartsRef"
        :viewOnly="true"
        :dashboardData="currentDashboardData.data"
        :currentTimeObj="dateTime"
        searchType="RUM"
      >
        <template v-slot:before_panels>
          <div class="flex items-center q-pb q-pt-md text-subtitle1 text-bold">
            <div class="text-center" style="width: 25%">
              {{ t("rum.webVitalsLabel") }}
            </div>
            <div class="text-center" style="width: 25%">
              {{ t("rum.errorLabel") }}
            </div>
            <div class="text-center" style="width: 25%">
              {{ t("rum.sessionLabel") }}
            </div>
          </div>
        </template>
      </RenderDashboardCharts>
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
  onMounted,
  nextTick,
  onActivated,
  type Ref,
} from "vue";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { getDashboard } from "@/utils/commons.ts";
import { parseDuration, generateDurationLabel } from "@/utils/date";
import { reactive } from "vue";
import { useRoute } from "vue-router";
import RenderDashboardCharts from "@/views/Dashboards/RenderDashboardCharts.vue";
import overviewDashboard from "@/utils/rum/overview.json";
import { cloneDeep } from "lodash-es";
import { convertDashboardSchemaVersion } from "../../../utils/dashboard/convertDashboardSchemaVersion";

export default defineComponent({
  name: "PerformanceSummary",
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
    // onMounted(async () => {
    //   await loadDashboard();
    // });

    const performanceChartsRef = ref(null);
    const isLoading: Ref<boolean[]> = ref([]);

    onMounted(async () => {
      await loadDashboard();
    });

    onActivated(() => {
      updateLayout();
    });

    const updateLayout = async () => {
      await nextTick();
      await nextTick();
      await nextTick();
      await nextTick();
      performanceChartsRef.value.layoutUpdate();

      // Dashboards gets overlapped as we have used keep alive
      // Its an internal bug of vue-grid-layout
      // So adding settimeout of 1 sec to fix the issue
      performanceChartsRef.value.layoutUpdate();
      window.dispatchEvent(new Event("resize"));
    };

    const loadDashboard = async () => {
      // schema migration
      currentDashboardData.value.data =
        convertDashboardSchemaVersion(overviewDashboard);

      // if variables data is null, set it to empty list

      if (
        !(
          currentDashboardData.value.data?.variables &&
          currentDashboardData.value.data?.variables?.list.length
        )
      ) {
        variablesData.isVariablesLoading = false;
        variablesData.values = [];
      }
    };

    const { t } = useI18n();
    const route = useRoute();
    const router = useRouter();
    const store = useStore();
    const currentDashboardData = ref({
      data: {},
    });

    // boolean to show/hide settings sidebar
    const showDashboardSettingsDialog = ref(false);

    // variables data
    const variablesData = reactive({});
    const variablesDataUpdated = (data: any) => {
      Object.assign(variablesData, data);
      const variableObj = {};
      data.values.forEach((v) => {
        variableObj[`var-${v.name}`] = v.value;
      });
      router.replace({
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
          dashboard: route.query.dashboard,
          folder: route.query.folder,
          refresh: generateDurationLabel(refreshInterval.value),
          ...getQueryParamsForDuration(selectedDate.value),
          ...variableObj,
        },
      });
    };

    // ======= [START] default variable values

    const initialVariableValues = {};
    Object.keys(route.query).forEach((key) => {
      if (key.startsWith("var-")) {
        const newKey = key.slice(4);
        initialVariableValues[newKey] = route.query[key];
      }
    });
    // ======= [END] default variable values

    const openSettingsDialog = () => {
      showDashboardSettingsDialog.value = true;
    };

    // [START] date picker related variables --------

    /**
     * Retrieves the selected date from the query parameters.
     */
    const getSelectedDateFromQueryParams = (params) => ({
      valueType: params.period
        ? "relative"
        : params.from && params.to
        ? "absolute"
        : "relative",
      startTime: params.from ? params.from : null,
      endTime: params.to ? params.to : null,
      relativeTimePeriod: params.period ? params.period : null,
    });

    const dateTimePicker = ref(null); // holds a reference to the date time picker

    // holds the date picker v-modal
    const selectedDate = ref(getSelectedDateFromQueryParams(route.query));

    // holds the current time for the dashboard
    const currentTimeObj = ref({});

    // refresh interval v-model
    const refreshInterval = ref(0);

    // when the date changes from the picker, update the current time object for the dashboard
    watch(selectedDate, () => {
      currentTimeObj.value = {
        start_time: new Date(selectedDate.value.startTime),
        end_time: new Date(selectedDate.value.endTime),
      };
    });

    const getQueryParamsForDuration = (data: any) => {
      if (data.relativeTimePeriod) {
        return {
          period: data.relativeTimePeriod,
        };
      } else {
        return {
          from: data.startTime,
          to: data.endTime,
        };
      }
    };

    // [END] date picker related variables

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

    const refreshData = () => {
      dateTimePicker.value.refresh();
    };

    // ------- work with query params ----------
    onActivated(async () => {
      const params = route.query;

      if (params.refresh) {
        refreshInterval.value = parseDuration(params.refresh);
      }

      // This is removed due to the bug of the new date time component
      // and is now rendered when the setup method is called
      // instead of onActivated
      // if (params.period || (params.to && params.from)) {
      //   selectedDate.value = getSelectedDateFromQueryParams(params);
      // }

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

    const onDeletePanel = async (panelId: any) => {
      await deletePanel(
        store,
        route.query.dashboard,
        panelId,
        route.query.folder ?? "default"
      );
      await loadDashboard();
    };

    return {
      currentDashboardData,
      goBackToDashboardList,
      addPanelData,
      t,
      getDashboard,
      store,
      // date variables
      dateTimePicker,
      selectedDate,
      currentTimeObj,
      refreshInterval,
      refreshData,
      onDeletePanel,
      variablesData,
      variablesDataUpdated,
      showDashboardSettingsDialog,
      openSettingsDialog,
      loadDashboard,
      getQueryParamsForDuration,
      performanceChartsRef,
      isLoading,
    };
  },
});
</script>

<style lang="scss" scoped>
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

<style lang="scss">
.performance-dashboard {
  min-height: auto !important;
  max-height: calc(100vh - 200px);
  overflow-y: auto;
}
</style>
