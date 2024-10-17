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
  <q-page
    class="relative-position"
    :key="store.state.selectedOrganization.identifier"
  >
    <div
      class="api-performance-dashboards"
      :style="{ visibility: isLoading.length ? 'hidden' : 'visible' }"
    >
      <div class="q-px-sm performance-dashboard">
        <RenderDashboardCharts
          ref="apiDashboardChartsRef"
          :viewOnly="true"
          :dashboardData="currentDashboardData.data"
          :currentTimeObj="dateTime"
          @variablesData="variablesDataUpdated"
          searchType="RUM"
        />
      </div>
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
  onActivated,
  nextTick,
  type Ref,
} from "vue";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { reactive } from "vue";
import { useRoute } from "vue-router";
import searchService from "@/services/search";
import apiDashboard from "@/utils/rum/api.json";
import RenderDashboardCharts from "@/views/Dashboards/RenderDashboardCharts.vue";
import { convertDashboardSchemaVersion } from "@/utils/dashboard/convertDashboardSchemaVersion";

export default defineComponent({
  name: "ApiDashboard",
  components: {
    RenderDashboardCharts,
  },
  props: {
    dateTime: {
      type: Object,
      default: () => ({}),
    },
    selectedDate: {
      type: Object,
      default: () => ({}),
    },
  },
  setup(props) {
    const { t } = useI18n();
    const route = useRoute();
    const router = useRouter();
    const store = useStore();
    const currentDashboardData = reactive({
      data: {},
    });
    const showDashboardSettingsDialog = ref(false);
    const apiDashboardChartsRef: Ref<any> = ref(null);
    const viewOnly = ref(true);
    const eventLog = ref([]);
    const topSlowResources = ref([]);
    const topHeavyResources = ref([]);
    const topErrorResources = ref([]);
    const variablesData = ref(null);

    const refDateTime: any = ref(null);
    const refreshInterval = ref(0);
    const topCount = 10;
    const isLoading: Ref<boolean[]> = ref([]);

    // variables data
    const variablesDataUpdated = (data: any) => {
      if (JSON.stringify(variablesData.value) === JSON.stringify(data)) return;

      variablesData.value = data;
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
      apiDashboardChartsRef.value.layoutUpdate();

      // Dashboards gets overlapped as we have used keep alive
      // Its an internal bug of vue-grid-layout
      // So adding settimeout of 1 sec to fix the issue
      apiDashboardChartsRef.value.layoutUpdate();
      window.dispatchEvent(new Event("resize"));
    };

    const getTopSlowResources = () => {
      topSlowResources.value = [];

      let whereClause = `where resource_duration>=0 and resource_method is not null`;
      const variablesString = getVariablesString();
      if (variablesString && variablesString.length) {
        whereClause += ` ${variablesString}`;
      }

      const req = {
        query: {
          sql: `SELECT avg(resource_duration/1000000) as max_duration, SPLIT_PART(resource_url, '?', 1) AS url FROM "_rumdata" ${whereClause} group by url order by max_duration desc`,
          start_time: props.selectedDate.startTime,
          end_time: props.selectedDate.endTime,
          from: 0,
          size: 10,
          sql_mode: "full",
        },
      };

      searchService
        .search(
          {
            org_identifier: store.state.selectedOrganization.identifier,
            query: req,
            page_type: "logs",
          },
          "RUM"
        )
        .then((res) => {
          res.data.hits.slice(0, topCount).forEach((element: any) => {
            topSlowResources.value.push({
              url: new URL(element.url).pathname,
              max_duration: element.max_duration.toFixed(2),
            });
          });
        });
    };

    const getTopHeavyResources = () => {
      topHeavyResources.value = [];

      let whereClause = `WHERE resource_size>=0`;
      const variablesString = getVariablesString();
      if (variablesString && variablesString.length) {
        whereClause += ` ${variablesString}`;
      }

      const req = {
        query: {
          sql: `SELECT avg(resource_size/1024) as max_resource_size, SPLIT_PART(resource_url, '?', 1) AS url FROM "_rumdata" ${whereClause} group by url order by max_resource_size desc`,
          start_time: props.selectedDate.startTime,
          end_time: props.selectedDate.endTime,
          from: 0,
          size: topCount,
          sql_mode: "full",
        },
      };

      searchService
        .search(
          {
            org_identifier: store.state.selectedOrganization.identifier,
            query: req,
            page_type: "logs",
          },
          "RUM"
        )
        .then((res) => {
          res.data.hits.forEach((element: any) => {
            topHeavyResources.value.push({
              ...element,
              url: new URL(element.url).pathname,
            });
          });
        });
    };

    const getTopErrorResources = () => {
      topErrorResources.value = [];

      let whereClause = `WHERE resource_status_code>400`;
      const variablesString = getVariablesString();
      if (variablesString && variablesString.length) {
        whereClause += ` ${variablesString}`;
      }

      const req = {
        query: {
          sql: `SELECT SPLIT_PART(resource_url, '?', 1) AS url, count(*) as error_count FROM "_rumdata" ${whereClause}  group by url order by error_count desc`,
          start_time: props.selectedDate.startTime,
          end_time: props.selectedDate.endTime,
          from: 0,
          size: topCount,
          sql_mode: "full",
        },
      };

      searchService
        .search(
          {
            org_identifier: store.state.selectedOrganization.identifier,
            query: req,
            page_type: "logs",
          },
          "RUM"
        )
        .then((res) => {
          res.data.hits.forEach((element: any) => {
            topErrorResources.value.push({
              ...element,
              url: new URL(element.url).pathname,
            });
          });
        });
    };

    const getVariablesString = () => {
      let variablesString = "";
      variablesData.value?.values?.length &&
        variablesData.value.values.forEach((element: any) => {
          if (element.type === "query_values" && !!element.value) {
            variablesString += ` and ${element.name}='${element.value}'`;
          }
        });

      return variablesString;
    };

    const loadDashboard = async () => {
      currentDashboardData.data = convertDashboardSchemaVersion(apiDashboard);

      // if variables data is null, set it to empty list
      if (
        !(
          currentDashboardData.data?.variables &&
          currentDashboardData.data?.variables?.list.length
        )
      ) {
        variablesData.value.isVariablesLoading = false;
        variablesData.value.values = [];
      }
    };

    const addSettingsData = () => {
      showDashboardSettingsDialog.value = true;
    };

    watch(
      () => props.selectedDate,
      (newVal, oldValue) => {
        if (JSON.stringify(newVal) !== JSON.stringify(oldValue)) {
          getTopHeavyResources();
          getTopSlowResources();
        }
      }
    );

    return {
      currentDashboardData,
      t,
      store,
      refDateTime,
      refreshInterval,
      viewOnly,
      eventLog,
      variablesData,
      variablesDataUpdated,
      addSettingsData,
      showDashboardSettingsDialog,
      loadDashboard,
      apiDashboard,
      isLoading,
      apiDashboardChartsRef,
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

.view-error-table {
  margin-top: 4px;
  border: 1px solid rgba(194, 194, 194, 0.4784313725) !important;
  border-radius: 4px;
  min-height: 200px;
}

.api-performance-dashboards {
  min-height: auto !important;
  max-height: calc(100vh - 196px);
  overflow-y: auto;
}
</style>

<style lang="scss">
.view-error-table {
  .q-table {
    td {
      padding: 6px 10px !important;
      height: auto !important;
    }
  }

  .q-table thead tr {
    th {
      padding: 6px 8px !important;
      height: auto !important;
    }
    padding: 6px 0px !important;
    height: auto !important;
  }
}
</style>
