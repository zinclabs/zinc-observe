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
  <div
    class="scheduled-dashboards"
    :class="store.state.theme === 'dark' ? 'dark-mode' : 'bg-white'"
  >
    <q-table
      data-test="scheduled-dashboard-table"
      ref="scheduledDashboardTableRef"
      :rows="formattedReports"
      :columns="columns"
      row-key="id"
      :pagination="pagination"
      :filter="filterQuery"
      :filter-method="filterData"
      style="width: 100%"
      class="q-px-md"
      @row-click="openReport"
    >
      <template #no-data>
        <template v-if="loading">
          <div
            class="text-center full-width full-height q-mt-lg tw-flex tw-justify-center"
          >
            <q-spinner-hourglass color="primary" size="lg" />
          </div>
        </template>
        <template v-else>
          <NoData />
        </template>
      </template>
      <template #top="scope">
        <div class="tw-flex tw-justify-between tw-w-full">
          <div
            class="q-table__title tw-flex tw-items-center"
            data-test="alerts-list-title"
          >
            {{ t("dashboard.scheduledDashboards") }}
          </div>

          <div class="tw-flex tw-items-center">
            <app-tabs
              class="q-mr-md"
              :tabs="reportTypeTabs"
              v-model:active-tab="activeTab"
              @update:active-tab="filterReports"
            />

            <q-input
              data-test="alert-list-search-input"
              v-model="filterQuery"
              borderless
              filled
              dense
              class="q-ml-auto no-border"
              :placeholder="t('reports.search')"
            >
              <template #prepend>
                <q-icon name="search" class="cursor-pointer" />
              </template>
            </q-input>

            <q-btn
              data-test="alert-list-add-alert-btn"
              class="q-ml-md text-bold no-border"
              padding="sm lg"
              color="secondary"
              no-caps
              :label="t(`dashboard.newReport`)"
              @click="createNewReport"
            />

            <div class="q-ml-sm">
              <q-btn
                v-close-popup="true"
                round
                flat
                :icon="'img:' + getImageURL('images/common/close_icon.svg')"
              />
            </div>
          </div>
        </div>

        <QTablePagination
          :scope="scope"
          :position="'top'"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          @update:changeRecordPerPage="changePagination"
        />
      </template>

      <template #bottom="scope">
        <QTablePagination
          :scope="scope"
          :position="'bottom'"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          @update:changeRecordPerPage="changePagination"
        />
      </template>
    </q-table>
  </div>
</template>

<script setup lang="ts">
import { QTable } from "quasar";
import { reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import AppTabs from "@/components/common/AppTabs.vue";
import { ScheduledDashboardReport } from "@/ts/interfaces/dashboard";
import NoData from "@/components/shared/grid/NoData.vue";
import { convertUnixToQuasarFormat } from "@/utils/date";
import { useStore } from "vuex";
import { getImageURL } from "@/utils/zincutils";

const props = defineProps({
  reports: {
    type: Array,
    required: true,
  },
  loading: {
    type: Boolean,
    required: true,
  },
  folderId: {
    type: String,
    required: true,
  },
  dashboardId: {
    type: String,
    required: true,
  },
  tabId: {
    type: String,
    required: true,
  },
  tabs: {
    type: Array,
    required: true,
  },
});

const { t } = useI18n();

const scheduledReports = ref<ScheduledDashboardReport[]>(
  props.reports as ScheduledDashboardReport[]
);

const formattedReports = ref<ScheduledDashboardReport[]>([]);

const scheduledDashboardTableRef = ref<InstanceType<typeof QTable> | null>();

const router = useRouter();

const activeTab = ref("cached");

const store = useStore();

const reportTypeTabs = reactive([
  {
    label: t("reports.cached"),
    value: "cached",
  },
  {
    label: t("reports.scheduled"),
    value: "shared",
  },
]);

watch(
  () => props.reports,
  () => {
    formatReports();
  },
  {
    deep: true,
  }
);

const formatReports = () => {
  resultTotal.value = props.reports.length;

  props.reports.forEach((report: any, index) => {
    scheduledReports.value.push({
      "#": index + 1,
      name: report.name,
      tab: getTabName(report.dashboards?.[0]?.tabs?.[0]),
      time_range: getTimeRangeValue(report.dashboards?.[0]?.timerange),
      frequency: getFrequencyValue(report.frequency),
      last_triggered_at: report.lastTriggeredAt
        ? convertUnixToQuasarFormat(report.lastTriggeredAt)
        : "-",
      created_at: convertUnixToQuasarFormat(
        new Date(report.createdAt).getTime() * 1000
      ),
      orgId: report.orgId,
      isCached: !report.destinations.length,
    });
  });

  filterReports();
};

const getTabName = (tabId: string) => {
  const tab = props.tabs.find((tab: any) => tab.tabId === tabId) as any;
  return tab?.name;
};

const filterReports = () => {
  // filter reports based on the selected tab
  // If reports are cached, show only cached reports
  if (activeTab.value === "cached") {
    formattedReports.value = (
      scheduledReports.value as ScheduledDashboardReport[]
    ).filter((report) => report.isCached);
  } else {
    formattedReports.value = (
      scheduledReports.value as ScheduledDashboardReport[]
    ).filter((report) => !report.isCached);
  }

  formattedReports.value = formattedReports.value.map(
    (report: any, index: number) => {
      return {
        ...report,
        "#": index + 1,
      };
    }
  );

  resultTotal.value = formattedReports.value.length;
};

const columns: any = [
  {
    name: "#",
    label: "#",
    field: "#",
    align: "left",
  },
  {
    name: "name",
    field: "name",
    label: t("reports.name"),
    align: "left",
    sortable: true,
  },
  {
    name: "tab",
    field: "tab",
    label: t("reports.tab"),
    align: "left",
    sortable: true,
  },
  {
    name: "time_range",
    field: "time_range",
    label: t("reports.timeRange"),
    align: "left",
    sortable: true,
  },
  {
    name: "frequency",
    field: "frequency",
    label: t("reports.frequency"),
    align: "left",
    sortable: true,
  },
  {
    name: "last_triggered_at",
    field: "last_triggered_at",
    label: t("reports.lastTriggeredAt"),
    align: "left",
    sortable: false,
  },
  {
    name: "created_at",
    field: "created_at",
    label: t("reports.createdAt"),
    align: "left",
    sortable: false,
  },
];

const pagination: any = ref({
  rowsPerPage: 20,
});

const resultTotal = ref(0);

const perPageOptions: any = [
  { label: "5", value: 5 },
  { label: "10", value: 10 },
  { label: "20", value: 20 },
  { label: "50", value: 50 },
  { label: "100", value: 100 },
  { label: "All", value: 0 },
];

const changePagination = (val: { label: string; value: any }) => {
  pagination.value.rowsPerPage = val.value;
  scheduledDashboardTableRef.value?.setPagination(pagination.value);
};

const filterQuery = ref("");

const filterData = (rows: any, terms: string) => {
  return rows.filter((row: any) => {
    return Object.values(row).some((value) => {
      return String(value).toLowerCase().includes(terms.toLowerCase());
    });
  });
};

const createNewReport = () => {
  router.push({
    name: "createReport",
    query: {
      folderId: props.folderId,
      dashboardId: props.dashboardId,
      tabId: props.tabId,
      type: "cached",
    },
  });
};

const openReport = (event: any, row: any) => {
  router.push({
    name: "createReport",
    query: {
      name: row.name,
      org_identifier: row.orgId,
    },
  });
};

const getFrequencyValue = (frequency: any) => {
  if (frequency.type === "cron") {
    return `Cron ${frequency.cron}`;
  } else {
    switch (frequency.type) {
      case "once":
        return `Once`;
      case "hours":
        return `Every ${frequency.interval > 1 ? frequency.interval : ""} ${
          frequency.interval > 1 ? "Hours" : "Hour"
        }`;
      case "weeks":
        return `Every ${frequency.interval > 1 ? frequency.interval : ""} ${
          frequency.interval > 1 ? "Weeks" : "Week"
        }`;
      case "months":
        return `Every ${frequency.interval > 1 ? frequency.interval : ""} ${
          frequency.interval > 1 ? "Months" : "Month"
        }`;
      case "days":
        return `Every ${frequency.interval > 1 ? frequency.interval : ""} ${
          frequency.interval > 1 ? "Days" : "Day"
        }`;
      default:
        return "";
    }
  }
};

const getTimeRangeValue = (dateTime: any) => {
  if (dateTime.type === "relative") {
    return `Past ${dateTime.period}`;
  } else {
    const startDateTime = convertUnixToQuasarFormat(dateTime.from);
    const endDateTime = convertUnixToQuasarFormat(dateTime.to);
    return `${startDateTime} - ${endDateTime}`;
  }
};
</script>

<style lang="scss" scoped>
.dark-mode {
  background-color: $dark-page;

  &.scheduled-dashboards {
    height: fit-content;

    :deep(.rum-tabs) {
      border: 1px solid #464646;
    }

    :deep(.rum-tab) {
      &:hover {
        background: #464646;
      }

      &.active {
        background: #5960b2;
        color: #ffffff !important;
      }
    }
  }
}

.scheduled-dashboards {
  height: fit-content;

  :deep(.q-table__top) {
    padding-left: 0;
    padding-right: 0;
  }

  :deep(.rum-tabs) {
    border: 1px solid #eaeaea;
    height: fit-content;
    border-radius: 4px;
    overflow: hidden;
  }

  :deep(.rum-tab) {
    width: fit-content !important;
    padding: 4px 12px !important;
    border: none !important;

    &:hover {
      background: #eaeaea;
    }

    &.active {
      background: #5960b2;
      color: #ffffff !important;
    }
  }
}
</style>
