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
  <q-page :key="store.state.selectedOrganization.identifier">
    <div
      ref="fullscreenDiv"
      :class="`${isFullscreen ? 'fullscreen' : ''}  ${
        store.state.theme === 'light' ? 'bg-white' : 'dark-mode'
      }`"
    >
      <div
        :class="`${
          store.state.theme === 'light' ? 'bg-white' : 'dark-mode'
        } stickyHeader ${
          isFullscreen || store.state.printMode === true
            ? 'fullscreenHeader'
            : ''
        }`"
      >
        <div class="tw-flex justify-between items-center q-pa-xs tw-w-full tw-min-w-0">
          <div class="tw-flex tw-flex-1 tw-overflow-hidden">
            <q-btn
              v-if="!isFullscreen"
              no-caps
              @click="goBackToDashboardList"
              padding="xs"
              outline
              icon="arrow_back_ios_new"
              data-test="dashboard-back-btn"
              class="hideOnPrintMode"
            />
            <span
              class="q-table__title folder-name tw-px-2 tw-cursor-pointer tw-transition-all tw-rounded-sm tw-ml-2"
              @click="goBackToDashboardList"
              >{{ folderNameFromFolderId }}
            </span>
            <q-spinner-dots v-if="!store.state.organizationData.folders.length" color="primary" size="2em" />
            <q-icon
              class="q-table__title tw-text-gray-400 tw-mt-1"
              name="chevron_right"
            ></q-icon>
            {{ currentDashboardData.data }}
            <span
              class="q-table__title q-mx-sm tw-truncate tw-flex-1"
              :title="currentDashboardData.data?.dash?.title"
            >
              {{ currentDashboardData.data?.dash?.title }}
            </span>
          </div>
          <div class="tw-flex">
            <q-btn
              v-if="!isFullscreen"
              outline
              class="dashboard-icons q-px-sm hideOnPrintMode"
              size="sm"
              no-caps
              icon="add"
              @click="addPanelData"
              data-test="dashboard-panel-add"
            >
              <q-tooltip>{{ t("panel.add") }}</q-tooltip>
            </q-btn>
            <!-- <DateTimePicker 
            class="q-ml-sm"
            ref="refDateTime"
            v-model="selectedDate"
          /> -->
            <!-- for Print Mode -->
            <!-- if time is relative, show start and end time -->
            <!-- format: YYYY/MM/DD HH:mm - YYYY/MM/DD HH:mm (TIMEZONE) -->
            <div
              v-if="
                store.state.printMode === true &&
                currentTimeObj.start_time &&
                currentTimeObj.end_time
              "
              style="padding-top: 5px"
            >
              {{ timeString }} ({{ store.state.timezone }})
            </div>
            <!-- do not show date time picker for print mode -->
            <DateTimePickerDashboard
              v-if="selectedDate"
              v-show="store.state.printMode === false"
              ref="dateTimePicker"
              class="dashboard-icons q-ml-sm"
              size="sm"
              v-model="selectedDate"
              :initialTimezone="initialTimezone"
              :disable="arePanelsLoading"
            />
            <AutoRefreshInterval
              v-model="refreshInterval"
              trigger
              @trigger="refreshData"
              class="dashboard-icons hideOnPrintMode"
              size="sm"
            />
            <q-btn
              v-if="config.isEnterprise == 'true' && arePanelsLoading"
              outline
              class="dashboard-icons q-px-sm q-ml-sm hideOnPrintMode"
              size="sm"
              no-caps
              icon="cancel"
              @click="cancelQuery"
              data-test="dashboard-cancel-btn"
              color="negative"
            >
              <q-tooltip>{{ t("panel.cancel") }}</q-tooltip>
            </q-btn>
            <q-btn
              v-else
              outline
              class="dashboard-icons q-px-sm q-ml-sm hideOnPrintMode"
              size="sm"
              no-caps
              icon="refresh"
              @click="refreshData"
              :disable="arePanelsLoading"
              data-test="dashboard-refresh-btn"
            >
              <q-tooltip>{{ t("dashboard.refresh") }}</q-tooltip>
            </q-btn>

            <ExportDashboard
              v-if="!isFullscreen"
              class="hideOnPrintMode"
              :dashboardId="currentDashboardData.data?.dashboardId"
            />
            <q-btn
              v-if="!isFullscreen"
              outline
              class="dashboard-icons q-px-sm q-ml-sm hideOnPrintMode"
              size="sm"
              no-caps
              icon="share"
              @click="shareLink.execute()"
              :loading="shareLink.isLoading.value"
              data-test="dashboard-share-btn"
              ><q-tooltip>{{ t("dashboard.share") }}</q-tooltip></q-btn
            >
            <q-btn
              v-if="!isFullscreen"
              outline
              class="dashboard-icons q-px-sm q-ml-sm hideOnPrintMode"
              size="sm"
              no-caps
              icon="settings"
              data-test="dashboard-setting-btn"
              @click="openSettingsDialog"
            >
              <q-tooltip>{{ t("dashboard.setting") }}</q-tooltip>
            </q-btn>
            <q-btn
              outline
              class="dashboard-icons q-px-sm q-ml-sm"
              size="sm"
              no-caps
              :icon="store.state.printMode === true ? 'close' : 'print'"
              @click="printDashboard"
              data-test="dashboard-print-btn"
              ><q-tooltip>{{
                store.state.printMode === true
                  ? t("common.close")
                  : t("dashboard.print")
              }}</q-tooltip></q-btn
            >
            <q-btn
              outline
              class="dashboard-icons q-px-sm q-ml-sm hideOnPrintMode"
              size="sm"
              no-caps
              :icon="
                quasar.fullscreen.isActive ? 'fullscreen_exit' : 'fullscreen'
              "
              @click="toggleFullscreen"
              data-test="dashboard-fullscreen-btn"
              ><q-tooltip>{{
                quasar.fullscreen.isActive
                  ? t("dashboard.exitFullscreen")
                  : t("dashboard.fullscreen")
              }}</q-tooltip></q-btn
            >
            <q-btn
              v-if="!isFullscreen"
              outline
              class="dashboard-icons q-px-sm q-ml-sm hideOnPrintMode"
              size="sm"
              no-caps
              :icon="outlinedDescription"
              @click="openScheduledReports"
              data-test="view-dashboard-scheduled-reports"
              ><q-tooltip>
                {{ t("dashboard.scheduledDashboards") }}
              </q-tooltip></q-btn
            >
          </div>
        </div>
        <q-separator></q-separator>
      </div>

      <RenderDashboardCharts
        v-if="selectedDate"
        ref="renderDashboardChartsRef"
        @variablesData="variablesDataUpdated"
        :initialVariableValues="initialVariableValues"
        :viewOnly="store.state.printMode"
        :dashboardData="currentDashboardData.data"
        :folderId="route.query.folder"
        :reportId="reportId"
        :currentTimeObj="currentTimeObjPerPanel"
        :selectedDateForViewPanel="selectedDate"
        @onDeletePanel="onDeletePanel"
        @onMovePanel="onMovePanel"
        @updated:data-zoom="onDataZoom"
        @refresh="loadDashboard"
        @refreshPanelRequest="refreshPanelRequest"
        @openEditLayout="openLayoutConfig"
        :showTabs="true"
        :forceLoad="store.state.printMode"
        :searchType="searchType"
        @panelsValues="handleEmittedData"
        @searchRequestTraceIds="searchRequestTraceIds"
      />

      <q-dialog
        v-model="showDashboardSettingsDialog"
        position="right"
        full-height
        maximized
      >
        <DashboardSettings @refresh="loadDashboard" />
      </q-dialog>

      <q-dialog
        v-model="selectedPanelConfig.show"
        position="right"
        full-height
        maximized
      >
        <PanelLayoutSettings
          :layout="selectedPanelConfig.data.layout"
          @save:layout="savePanelLayout"
        />
      </q-dialog>

      <q-dialog
        v-model="showScheduledReportsDialog"
        position="right"
        full-height
        maximized
      >
        <ScheduledDashboards
          :reports="scheduledReports"
          :loading="isLoadingReports"
          :folderId="folderId"
          :dashboardId="dashboardId"
          :tabId="tabId"
          :tabs="currentDashboardData?.data?.tabs || []"
        />
      </q-dialog>
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
  provide,
  defineAsyncComponent,
  reactive,
  onMounted,
  onUnmounted,
  onBeforeMount,
  computed,
} from "vue";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import DateTimePickerDashboard from "@/components/DateTimePickerDashboard.vue";
import { useRouter } from "vue-router";
import {
  getDashboard,
  movePanelToAnotherTab,
  getFoldersList,
} from "../../utils/commons.ts";
import { parseDuration, generateDurationLabel } from "../../utils/date";
import { useRoute } from "vue-router";
import { deletePanel } from "../../utils/commons";
import AutoRefreshInterval from "@/components/AutoRefreshInterval.vue";
import ExportDashboard from "@/components/dashboards/ExportDashboard.vue";
import RenderDashboardCharts from "./RenderDashboardCharts.vue";
import { copyToClipboard, useQuasar } from "quasar";
import useNotifications from "@/composables/useNotifications";
import ScheduledDashboards from "./ScheduledDashboards.vue";
import reports from "@/services/reports";
import destination from "@/services/alert_destination.js";
import { outlinedDescription } from "@quasar/extras/material-icons-outlined";
import config from "@/aws-exports";
import queryService from "../../services/search";
import useCancelQuery from "@/composables/dashboard/useCancelQuery";
import PanelLayoutSettings from "./PanelLayoutSettings.vue";
import { useLoading } from "@/composables/useLoading";
import shortURLService from "@/services/short_url";

const DashboardSettings = defineAsyncComponent(() => {
  return import("./DashboardSettings.vue");
});

export default defineComponent({
  name: "ViewDashboard",
  emits: ["onDeletePanel"],
  components: {
    DateTimePickerDashboard,
    AutoRefreshInterval,
    ExportDashboard,
    DashboardSettings,
    RenderDashboardCharts,
    ScheduledDashboards,
    PanelLayoutSettings,
  },
  setup() {
    const { t } = useI18n();
    const route = useRoute();
    const router = useRouter();
    const store = useStore();
    const quasar = useQuasar();
    const currentDashboardData = reactive({
      data: {},
    });
    const showScheduledReportsDialog = ref(false);
    const {
      showPositiveNotification,
      showErrorNotification,
      showConfictErrorNotificationWithRefreshBtn,
    } = useNotifications();

    let moment: any = () => {};

    const folderNameFromFolderId = computed(() => {
      if (store.state.organizationData.folders.length === 0) {
        return "";
      }
      return (
        store.state.organizationData.folders.find(
          (item: any) => item.folderId === route.query.folder ?? "default",
        )?.name ?? "default"
      );
    });

    const importMoment = async () => {
      const momentModule: any = await import("moment-timezone");
      moment = momentModule.default;
    };

    const scheduledReports = ref([]);
    const isLoadingReports = ref(false);

    const dashboardId = computed(() => route.query.dashboard);

    const folderId = computed(() => route.query.folder);

    const tabId = computed(() => route.query.tab);

    const reportId = computed(() => route.query.tab);

    const renderDashboardChartsRef = ref(null);

    onBeforeMount(async () => {
      await importMoment();
      setTimeString();
    });

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
      relativeTimePeriod: params.period ? params.period : "15m",
    });

    const dateTimePicker = ref(null); // holds a reference to the date time picker

    // holds the date picker v-modal
    const selectedDate = ref(null);

    // holds the current time for the dashboard
    const currentTimeObj = ref({});

    // refresh interval v-model
    const refreshInterval = ref(0);

    // initial timezone, which will come from the route query
    const initialTimezone = ref(route.query.timezone ?? null);

    // search_type for search query
    const searchType = ref(route.query.searchtype ?? null);

    // dispatch setPrintMode, to set print mode
    const setPrint = (printMode: any) => {
      store.dispatch("setPrintMode", printMode);
    };

    const timeString = ref("");

    const printDashboard = () => {
      // set print mode as true
      setPrint(store.state.printMode != true);

      const query = {
        ...route.query,
        print: store.state.printMode,
      };

      // replace query params with print=true
      router.replace({ query });
    };

    // boolean to show/hide settings sidebar
    const showDashboardSettingsDialog = ref(false);

    const selectedPanelConfig = ref({
      data: null,
      show: false,
    });

    // selected tab
    const selectedTabId: any = ref(route.query.tab ?? null);
    // provide it to child components
    provide("selectedTabId", selectedTabId);

    // variables data
    const variablesData = reactive({});
    const variablesDataUpdated = (data: any) => {
      Object.assign(variablesData, data);
      const variableObj = {};
      data.values.forEach((variable) => {
        if (variable.type === "dynamic_filters") {
          const filters = (variable.value || []).filter(
            (item: any) => item.name && item.operator && item.value,
          );
          const encodedFilters = filters.map((item: any) => ({
            name: item.name,
            operator: item.operator,
            value: item.value,
          }));
          variableObj[`var-${variable.name}`] = encodeURIComponent(
            JSON.stringify(encodedFilters),
          );
        } else {
          variableObj[`var-${variable.name}`] = variable.value;
        }
      });
      router.replace({
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
          dashboard: route.query.dashboard,
          folder: route.query.folder,
          tab: selectedTabId.value,
          refresh: generateDurationLabel(refreshInterval.value),
          ...getQueryParamsForDuration(selectedDate.value),
          ...variableObj,
          print: store.state.printMode,
          searchtype: route.query.searchtype,
        },
      });
    };

    // ======= [START] default variable values

    const initialVariableValues = { value: {} };
    Object.keys(route.query).forEach((key) => {
      if (key.startsWith("var-")) {
        const newKey = key.slice(4);
        initialVariableValues.value[newKey] = route.query[key];
      }
    });
    // ======= [END] default variable values

    onMounted(async () => {
      await loadDashboard();
      if (!store.state.organizationData.folders.length) {
        await getFoldersList(store);
      }
    });

    const setTimeString = () => {
      if (!moment()) return;
      timeString.value = ` ${moment(
        currentTimeObj.value?.start_time?.getTime() / 1000,
      )
        .tz(store.state.timezone)
        .format("YYYY/MM/DD HH:mm")}
              -
               ${moment(currentTimeObj.value?.end_time?.getTime() / 1000)
                 .tz(store.state.timezone)
                 .format("YYYY/MM/DD HH:mm")}

                  `;
    };

    const loadDashboard = async () => {
      currentDashboardData.data = await getDashboard(
        store,
        route.query.dashboard,
        route.query.folder ?? "default",
      );

      if (
        !currentDashboardData?.data ||
        typeof currentDashboardData.data !== "object" ||
        !Object.keys(currentDashboardData.data).length
      ) {
        goBackToDashboardList();
        return;
      }

      // set selected tab from query params
      const selectedTab = currentDashboardData?.data?.tabs?.find(
        (tab: any) => tab.tabId === route.query.tab,
      );

      selectedTabId.value = selectedTab
        ? selectedTab.tabId
        : currentDashboardData?.data?.tabs?.[0]?.tabId;

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

      // check if route has time related query params
      // if not, take dashboard default time settings
      if (!((route.query.from && route.query.to) || route.query.period)) {
        // if dashboard has relative time settings
        if (
          (currentDashboardData.data?.defaultDatetimeDuration?.type ??
            "relative") === "relative"
        ) {
          selectedDate.value = {
            valueType: "relative",
            relativeTimePeriod:
              currentDashboardData.data?.defaultDatetimeDuration
                ?.relativeTimePeriod ?? "15m",
          };
        } else {
          // else, dashboard will have absolute time settings
          selectedDate.value = {
            valueType: "absolute",
            startTime:
              currentDashboardData.data?.defaultDatetimeDuration?.startTime,
            endTime:
              currentDashboardData.data?.defaultDatetimeDuration?.endTime,
          };
        }
      } else {
        // take route time related query params
        selectedDate.value = getSelectedDateFromQueryParams(route.query);
      }
    };

    // [START] cancel running queries

    const arePanelsLoading = ref(false);

    const handleEmittedData = (allPanelsLoaded) => {
      arePanelsLoading.value = !allPanelsLoaded;
    };

    const { traceIdRef, searchRequestTraceIds, cancelQuery } = useCancelQuery();

    // [END] cancel running queries

    const openSettingsDialog = () => {
      showDashboardSettingsDialog.value = true;
    };

    const openLayoutConfig = (id: string) => {
      selectedPanelConfig.value.show = true;

      const panelData = getPanelFromTab(selectedTabId.value, id);

      if (!panelData) {
        console.log("Panel not found");
        return;
      }

      selectedPanelConfig.value.data = JSON.parse(JSON.stringify(panelData));
    };

    const savePanelLayout = async (layout) => {
      const panel = getPanelFromTab(
        selectedTabId.value,
        selectedPanelConfig.value.data.id,
      );
      if (panel) panel.layout = layout;

      selectedPanelConfig.value.show = false;
      selectedPanelConfig.value.data = null;

      await nextTick();

      window.dispatchEvent(new Event("resize"));

      await renderDashboardChartsRef.value?.saveDashboard();
    };

    // when the date changes from the picker, update the current time object for the dashboard
    watch(selectedDate, () => {
      if (selectedDate.value && dateTimePicker.value) {
        const date = dateTimePicker.value?.getConsumableDateTime();

        currentTimeObj.value = {
          start_time: new Date(date.startTime),
          end_time: new Date(date.endTime),
        };

        currentTimeObjPerPanel.value = {
          __global: {
            start_time: new Date(date.startTime),
            end_time: new Date(date.endTime),
          },
        };

        setTimeString();
      }
    });

    const getPanelFromTab = (tabId: string, panelId: string) => {
      const tab = currentDashboardData.data.tabs.find(
        (tab) => tab.tabId === tabId,
      );

      if (!tab || !tab.panels) {
        return null;
      }

      return tab.panels.find((panel) => panel.id === panelId);
    };

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
          folder: route.query.folder ?? "default",
        },
      });
    };

    //add panel
    const addPanelData = () => {
      return router.push({
        path: "/dashboards/add_panel",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
          dashboard: route.query.dashboard,
          folder: route.query.folder ?? "default",
          tab: route.query.tab ?? currentDashboardData.data.tabs[0].tabId,
        },
      });
    };

    const refreshData = () => {
      if (!arePanelsLoading.value) {
        dateTimePicker.value.refresh();
      }
    };

    const onDataZoom = (event: any) => {
      const selectedDateObj = {
        start: new Date(event.start),
        end: new Date(event.end),
      };
      // Truncate seconds and milliseconds from the dates
      selectedDateObj.start.setMilliseconds(0);
      selectedDateObj.end.setMilliseconds(0);

      // Compare the truncated dates
      if (selectedDateObj.start.getTime() === selectedDateObj.end.getTime()) {
        // Increment the end date by 1 minute
        selectedDateObj.end.setMinutes(selectedDateObj.end.getMinutes() + 1);
      }

      // set it as a absolute time
      dateTimePicker?.value?.setCustomDate("absolute", selectedDateObj);

      // refresh dashboard
      dateTimePicker.value.refresh();
    };

    // ------- work with query params ----------
    onMounted(async () => {
      const params = route.query;

      if (params.refresh) {
        refreshInterval.value = parseDuration(params.refresh);
      }

      // check if timezone query params exist
      // will be used for initial time zone only, so remove it from query
      if (params.timezone) {
        // get the query params
        const query = {
          ...route.query,
        };

        // remove timezone from query
        delete query.timezone;

        // replace route with query params
        router.replace({
          query,
        });
      }

      // check if print query params exist
      if (params.print !== undefined) {
        // set print mode
        setPrint(params.print == "true" ? true : false);
      } else {
        // set print mode as false which is default in store
        router.replace({
          query: {
            ...route.query,
            print: store.state.printMode,
          },
        });
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
    watch([refreshInterval, selectedDate, selectedTabId], () => {
      router.replace({
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
          dashboard: route.query.dashboard,
          folder: route.query.folder,
          tab: selectedTabId.value,
          refresh: generateDurationLabel(refreshInterval.value),
          ...getQueryParamsForDuration(selectedDate.value),
          print: store.state.printMode,
          searchtype: route.query.searchtype,
        },
      });
    });

    const onDeletePanel = async (panelId: any) => {
      try {
        await deletePanel(
          store,
          route.query.dashboard,
          panelId,
          route.query.folder ?? "default",
          route.query.tab ?? currentDashboardData.data.tabs[0].tabId,
        );
        await loadDashboard();

        showPositiveNotification("Panel deleted successfully", {
          timeout: 2000,
        });
      } catch (error: any) {
        if (error?.response?.status === 409) {
          showConfictErrorNotificationWithRefreshBtn(
            error?.response?.data?.message ??
              error?.message ??
              "Panel deletion failed",
          );
        } else {
          showErrorNotification(error?.message ?? "Panel deletion failed", {
            timeout: 2000,
          });
        }
      }
    };

    // move single panel to another tab
    const onMovePanel = async (panelId: any, newTabId: any) => {
      try {
        await movePanelToAnotherTab(
          store,
          route.query.dashboard,
          panelId,
          route.query.folder ?? "default",
          route.query.tab ?? currentDashboardData.data.tabs[0].tabId,
          newTabId,
        );
        await loadDashboard();

        showPositiveNotification("Panel moved successfully!", {
          timeout: 2000,
        });
      } catch (error: any) {
        if (error?.response?.status === 409) {
          showConfictErrorNotificationWithRefreshBtn(
            error?.response?.data?.message ??
              error?.message ??
              "Panel move failed",
          );
        } else {
          showErrorNotification(error?.message ?? "Panel move failed", {
            timeout: 2000,
          });
        }
      }
    };

    const shareLink = useLoading(async () => {
      const urlObj = new URL(window.location.href);
      const urlSearchParams = urlObj?.searchParams;

      // if relative time period, convert to absolute time
      if (urlSearchParams?.has("period")) {
        urlSearchParams.delete("period");
        urlSearchParams.set(
          "from",
          currentTimeObj?.value?.start_time?.getTime(),
        );
        urlSearchParams.set("to", currentTimeObj?.value?.end_time?.getTime());
      }

      try {
        const res = await shortURLService.create(
          store.state.selectedOrganization.identifier,
          urlObj?.href,
        );
        const shortURL = res?.data?.short_url;
        copyToClipboard(shortURL)
          .then(() => {
            showPositiveNotification("Link copied successfully");
          })
          .catch(() => {
            showErrorNotification("Error while copying link");
          });
      } catch (error) {
        showErrorNotification("Error while sharing link");
      }
    });

    // Fullscreen
    const fullscreenDiv = ref(null);
    const isFullscreen = ref(false);

    const toggleFullscreen = () => {
      if (!quasar.fullscreen.isActive) {
        quasar.fullscreen
          .request()
          .then(() => {
            isFullscreen.value = true;
          })
          .catch(() => {
            isFullscreen.value = false;
          });
      } else {
        quasar.fullscreen
          .exit()
          .then(() => {
            isFullscreen.value = false;
          })
          .catch(() => {
            isFullscreen.value = true;
          });
      }
    };

    const onFullscreenChange = () => {
      if (!document.fullscreenElement) {
        isFullscreen.value = false;
      }
    };

    const openScheduledReports = () => {
      if (isLoadingReports.value) return;

      showScheduledReportsDialog.value = true;
      scheduledReports.value = [];
      isLoadingReports.value = true;

      reports
        .list(
          store.state.selectedOrganization.identifier,
          folderId.value,
          dashboardId.value,
        )
        .then((response) => {
          scheduledReports.value = response.data;
        })
        .catch((error) => {
          showErrorNotification(error?.message || "Failed to fetch reports");
          isLoadingReports.value = false;
        })
        .finally(() => {
          isLoadingReports.value = false;
        });
    };

    onMounted(() => {
      document.addEventListener("fullscreenchange", onFullscreenChange);
    });

    onUnmounted(() => {
      document.removeEventListener("fullscreenchange", onFullscreenChange);
    });

    onMounted(() => {
      isFullscreen.value = false;
    });

    const currentTimeObjPerPanel = ref({});

    const refreshPanelRequest = (panelId) => {
      // when the date changes from the picker, update the current time object for the dashboard
      if (selectedDate.value && dateTimePicker.value) {
        const date = dateTimePicker.value?.getConsumableDateTime();

        currentTimeObjPerPanel.value = {
          ...currentTimeObjPerPanel.value,
          [panelId]: {
            start_time: new Date(date.startTime),
            end_time: new Date(date.endTime),
          },
        };

        setTimeString();
      }
    };

    return {
      currentDashboardData,
      toggleFullscreen,
      fullscreenDiv,
      isFullscreen,
      goBackToDashboardList,
      addPanelData,
      t,
      getDashboard,
      store,
      route,
      // date variables
      dateTimePicker,
      selectedDate,
      currentTimeObj,
      currentTimeObjPerPanel,
      refreshInterval,
      // ----------------
      refreshData,
      onDeletePanel,
      variablesData,
      variablesDataUpdated,
      showDashboardSettingsDialog,
      openSettingsDialog,
      loadDashboard,
      refreshPanelRequest,
      initialVariableValues,
      getQueryParamsForDuration,
      onDataZoom,
      shareLink,
      selectedTabId,
      onMovePanel,
      printDashboard,
      initialTimezone,
      timeString,
      searchType,
      quasar,
      openScheduledReports,
      showScheduledReportsDialog,
      isLoadingReports,
      scheduledReports,
      dashboardId,
      folderId,
      reportId,
      tabId,
      outlinedDescription,
      searchRequestTraceIds,
      arePanelsLoading,
      cancelQuery,
      traceIdRef,
      handleEmittedData,
      config,
      openLayoutConfig,
      selectedPanelConfig,
      savePanelLayout,
      renderDashboardChartsRef,
      folderNameFromFolderId,
    };
  },
});
</script>

<style lang="scss" scoped>
.printMode {
  .hideOnPrintMode {
    display: none;
  }
}

.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}

.dark-mode {
  background-color: $dark-page;
}

.bg-white {
  background-color: $white;
}

.stickyHeader {
  position: sticky;
  top: 40px;
  z-index: 1001;
}
.stickyHeader.fullscreenHeader {
  top: 0px;
}

.fullscreen {
  width: 100vw;
  height: 100vh;
  overflow-y: auto; /* Enables scrolling within the div */
  position: fixed;
  top: 0;
  left: 0;
  z-index: 10000; /* Ensure it's on top */
  /* Additional styling as needed */
}

.dashboard-icons {
  height: 30px;
}

.folder-name {
  color: $primary !important;
}

.folder-name:hover {
  background-color: $accent !important;
}
</style>
