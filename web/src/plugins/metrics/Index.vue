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

<!-- eslint-disable vue/attribute-hyphenation -->
<!-- eslint-disable vue/v-on-event-hyphenation -->
<template>
  <q-page class="metrics-page" id="metricsPage">
    <div class="row scroll" style="width: 100%">
      <!-- Note: Splitter max-height to be dynamically calculated with JS -->
      <q-splitter
        v-model="searchObj.config.splitterModel"
        :limits="searchObj.config.splitterLimit"
        style="width: 100%"
        @update:model-value="onSplitterUpdate"
      >
        <template #before>
          <metric-list
            data-test="logs-search-index-list"
            v-model="searchObj.data.metrics.selectedMetric"
            :metrics-list="searchObj.data.metrics.metricList"
            @select-label="addLabelToEditor"
            @update:change-metric="onMetricChange"
          />
        </template>
        <template #separator>
          <q-avatar
            color="primary"
            text-color="white"
            size="20px"
            icon="drag_indicator"
            style="top: 10px"
          />
        </template>
        <template #after>
          <div class="row">
            <div
              class="text-left col-2 auto q-px-sm q-py-xs flex justify-start metrics-date-time"
            >
              <syntax-guide-metrics class="q-mr-sm" />
            </div>
            <div
              class="text-right col q-px-sm q-py-xs flex justify-end metrics-date-time"
            >
              <date-time
                auto-apply
                :default-type="searchObj.data.datetime.type"
                :default-absolute-time="{
                  startTime: searchObj.data.datetime.startTime,
                  endTime: searchObj.data.datetime.endTime,
                }"
                :default-relative-time="
                  searchObj.data.datetime.relativeTimePeriod
                "
                data-test="logs-search-bar-date-time-dropdown"
                @on:date-change="updateDateTime"
              />
              <auto-refresh-interval
                class="q-pr-sm"
                v-model="searchObj.meta.refreshInterval"
                @update:model-value="onChangeRefreshInterval"
              />
              <q-btn
                data-test="metrics-explorer-run-query-button"
                data-cy="metrics-explorer-run-query-button"
                dense
                flat
                :title="t('metrics.runQuery')"
                class="q-pa-none bg-secondary search-button"
                @click="searchData"
                size="sm"
                :disable="
                  searchObj.loading || searchObj.data.streamResults.length == 0
                "
              >
                {{ t("metrics.runQuery") }}
              </q-btn>
              <q-btn
                data-test="logs-search-bar-share-link-btn"
                class="q-mx-sm download-logs-btn q-px-sm q-py-xs"
                size="sm"
                icon="share"
                style="height: 30px"
                :title="t('search.shareLink')"
                @click="shareLink"
              />
            </div>
          </div>
          <div>
            <div class="row metrics-query-editor-container">
              <div
                class="col q-pa-sm"
                style="border-top: 1px solid #dbdbdb; height: 100%"
              >
                <div class="q-pb-xs text-bold">
                  {{ t("metrics.promqlLabel") }}:
                </div>
                <div
                  v-if="searchObj.data.metrics.selectedMetric?.help?.length"
                  class="q-pb-sm"
                  style="display: inline"
                >
                  <q-icon
                    name="info"
                    style="font-size: 16px; display: inline-block"
                    title="Info"
                  />
                  <span
                    class="q-pl-xs info-message"
                    :style="{
                      color:
                        store.state.theme === 'light' ? '#049cbc' : '#3fd5f4',
                    }"
                  >
                    <span
                      v-show="searchObj.data.metrics.selectedMetric.type"
                      class="text-capitalize"
                      >{{ searchObj.data.metrics.selectedMetric.type }}</span
                    >
                    <span
                      v-show="searchObj.data.metrics.selectedMetric.type"
                      class="q-mx-xs"
                      :class="
                        store.state.theme === 'dark'
                          ? 'text-grey-4'
                          : 'text-grey-7'
                      "
                      >|</span
                    >{{ searchObj.data.metrics.selectedMetric.help }}
                  </span>
                </div>

                <query-editor
                  editor-id="metrics-query-editor"
                  ref="metricsQueryEditorRef"
                  class="monaco-editor"
                  v-model:query="searchObj.data.query"
                  :keywords="autoCompletePromqlKeywords"
                  @update-query="updateQueryValue"
                  @run-query="searchData"
                />
              </div>
            </div>
          </div>
          <div
            v-if="searchObj.loading"
            class="flex justify-center items-center"
            style="height: calc(100% - 300px)"
          >
            <div class="q-pb-lg">
              <q-spinner-hourglass
                color="primary"
                size="40px"
                style="margin: 0 auto; display: block"
              />
              <span class="text-center">
                {{ t("metrics.holdMessage") }}
              </span>
            </div>
          </div>
          <div
            v-else-if="
              searchObj.data.errorMsg !== '' && searchObj.loading == false
            "
          >
            <h5 class="text-center">
              <SanitizedHtmlRenderer
                data-test="logs-search-error-message"
                :htmlContent="
                  searchObj.data.errorMsg +
                  '<h6 style=\'font-size: 14px; margin: 0;\'>' +
                  searchObj.data.errorDetail +
                  '</h6>'
                "
              />
              <br />
              <q-item-label>{{
                searchObj.data.additionalErrorMsg
              }}</q-item-label>
            </h5>
          </div>
          <div v-else-if="!!!searchObj.data.metrics.selectedMetric?.value">
            <h5
              data-test="logs-search-no-stream-selected-text"
              class="text-center"
            >
              {{ t("metrics.noStreamSelected") }}
            </h5>
          </div>
          <div
            v-else-if="
              searchObj.data.queryResults.hasOwnProperty('total') &&
              !!!searchObj.data.queryResults?.hits?.length &&
              !searchObj.loading
            "
          >
            <h5 class="text-center">No result found.</h5>
          </div>
          <template v-if="searchObj.data.metrics.metricList?.length">
            <div class="flex justify-end q-pr-lg q-mb-md q-pt-xs">
              <q-btn
                size="md"
                class="q-px-sm no-border"
                no-caps
                dense
                color="primary"
                @click="addToDashboard"
                :title="t('metrics.addToDashboard')"
                >{{ t("metrics.addToDashboard") }}</q-btn
              >
            </div>
            <div style="height: 500px">
              <PanelSchemaRenderer
                v-if="chartData"
                :height="6"
                :width="6"
                :panelSchema="chartData"
                :selectedTimeObj="dashboardPanelData.meta.dateTime"
                :variablesData="{}"
                searchType="UI"
              />
            </div>
          </template>
        </template>
      </q-splitter>
    </div>
    <q-dialog
      v-model="showAddToDashboardDialog"
      position="right"
      full-height
      maximized
    >
      <add-to-dashboard
        @save="addPanelToDashboard"
        :dashboardPanelData="dashboardPanelData"
      />
    </q-dialog>
  </q-page>
</template>

<script lang="ts">
// @ts-nocheck
import {
  defineComponent,
  onMounted,
  ref,
  onDeactivated,
  onActivated,
  onBeforeMount,
  watch,
  nextTick,
  defineAsyncComponent,
} from "vue";
import { useQuasar, date, copyToClipboard } from "quasar";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";

import useMetrics from "@/composables/useMetrics";

import streamService from "@/services/stream";
import { b64DecodeUnicode, b64EncodeUnicode } from "@/utils/zincutils";
import segment from "@/services/segment_analytics";
import config from "@/aws-exports";
import { verifyOrganizationStatus } from "@/utils/zincutils";
import useMetricsExplorer from "@/composables/useMetricsExplorer";
import { cloneDeep } from "lodash-es";
import { addPanel, getPanelId } from "@/utils/commons";
import usePromqlSuggestions from "@/composables/usePromqlSuggestions";
import useNotifications from "@/composables/useNotifications";
import SyntaxGuideMetrics from "./SyntaxGuideMetrics.vue";
import { getConsumableRelativeTime } from "@/utils/date";
import useStreams from "@/composables/useStreams";
import SanitizedHtmlRenderer from "@/components/SanitizedHtmlRenderer.vue";
import useDashboardPanelData from "@/composables/useDashboardPanel";
import { provide } from "vue";

export default defineComponent({
  name: "AppMetrics",
  components: {
    MetricList: defineAsyncComponent(() => import("./MetricList.vue")),
    DateTime: defineAsyncComponent(() => import("@/components/DateTime.vue")),
    AutoRefreshInterval: defineAsyncComponent(
      () => import("@/components/AutoRefreshInterval.vue"),
    ),
    QueryEditor: defineAsyncComponent(
      () => import("@/components/QueryEditor.vue"),
    ),
    AddToDashboard: defineAsyncComponent(() => import("./AddToDashboard.vue")),
    SyntaxGuideMetrics,
    PanelSchemaRenderer: defineAsyncComponent(
      () => import("@/components/dashboards/PanelSchemaRenderer.vue"),
    ),
    SanitizedHtmlRenderer,
  },
  methods: {
    searchData() {
      if (!this.searchObj.loading) {
        this.runQuery();
      }

      if (config.isCloud == "true") {
        segment.track("Button Click", {
          button: "Refresh Metrics",
          user_org: this.store.state.selectedOrganization.identifier,
          user_id: this.store.state.userInfo.email,
          stream_name: this.searchObj.data.metrics.selectedMetric?.value,
          page: "Metrics explorer",
        });
      }
    },
  },
  setup() {
    const store = useStore();
    const router = useRouter();
    const $q = useQuasar();
    const { t } = useI18n();
    const { searchObj, resetSearchObj } = useMetrics();
    let dismiss = null;
    let refreshIntervalID = 0;
    const searchResultRef = ref(null);
    const searchBarRef = ref(null);
    let parser: any;
    const metricsQueryEditorRef = ref(null);
    provide("dashboardPanelDataPageKey", "metrics");
    const { dashboardPanelData } = useDashboardPanelData("metrics");
    const {
      autoCompleteData,
      autoCompletePromqlKeywords,
      getSuggestions,
      updateMetricKeywords,
      parsePromQlQuery,
    } = usePromqlSuggestions();
    const promqlKeywords = ref([]);
    const isMounted = ref(false);
    const logStreams = ref([]);
    const { getStreams } = useStreams();

    const metricTypeMapping: any = {
      Summary: "summary",
      Gauge: "gauge",
      Histogram: "histogram",
      Counter: "counter",
    };

    const chartData = ref({});
    const { showErrorNotification } = useNotifications();

    searchObj.organizationIdentifier =
      store.state.selectedOrganization.identifier;

    const importSqlParser = async () => {
      const useSqlParser: any = await import("@/composables/useParser");
      const { sqlParser }: any = useSqlParser.default();
      parser = await sqlParser();
    };

    const updateStreams = () => {
      if (searchObj.data.streamResults?.list?.length) {
        const streamType = "metrics";
        getStreams(streamType, true).then((response: any) => {
          searchObj.data.streamResults = response;
          searchObj.data.metrics.metricList = [];
          response.list.map((item: any) => {
            let itemObj = {
              label: item.name,
              value: item.name,
              type: metricTypeMapping[item.metrics_meta.metric_type] || "",
              help: item.metrics_meta.help || "",
            };
            searchObj.data.metrics.metricList.push(itemObj);
          });
        });
      } else {
        loadPageData();
      }
    };
    const showAddToDashboardDialog = ref(false);

    onBeforeMount(async () => {
      await importSqlParser();
      restoreUrlQueryParams();
      verifyOrganizationStatus(store.state.organizations, router);
      await getLogStreams();
      if (searchObj.loading == false) {
        loadPageData(true);
        refreshData();
      }
      dashboardPanelData.data.type = "line";
      dashboardPanelData.data.queryType = "promql";
      dashboardPanelData.data.queries[0].fields.stream_type = "metrics";
      dashboardPanelData.data.queries[0].customQuery = true;
    });

    onMounted(() => {
      setTimeout(() => {
        isMounted.value = true;
      }, 0);
    });

    onDeactivated(() => {
      clearInterval(refreshIntervalID);
    });

    onActivated(() => {
      // As onActivated hook is getting called after mounted hook on rendering component for first time.
      // we fetch streams in before mount and also in activated (to refresh streams list if streams updated)
      // So added this flag to avoid calling updateStreams() on first time rendering.
      // This is just an workaround, need to find better solution while refactoring this component.
      if (isMounted.value) updateStreams();

      if (
        searchObj.organizationIdentifier !=
        store.state.selectedOrganization.identifier
      ) {
        loadPageData();
        refreshData();
      }

      setTimeout(() => {
        if (searchResultRef.value) searchResultRef.value.reDrawChart();
      }, 1500);
    });

    const getLogStreams = () => {
      getStreams("logs", false)
        .then(
          (res) =>
            (logStreams.value = res.list.map((stream) => ({
              name: stream.name,
            }))),
        )
        .finally(() => {
          return Promise.resolve();
        });
    };

    watch(
      () => searchObj.data.metrics.metricList,
      (metrics) => {
        updateMetricKeywords(metrics);
      },
      { deep: true },
    );

    function getStreamList(isFirstLoad = false) {
      try {
        searchObj.data.errorMsg = "";
        searchObj.loading = true;
        getStreams("metrics", false)
          .then((res) => {
            searchObj.data.streamResults = res;

            if (res.list.length > 0) {
              //extract stream data from response
              loadStreamLists(isFirstLoad);
            } else {
              searchObj.data.errorMsg =
                "No stream found in selected organization!";
              searchObj.data.metrics.metricList = [];
              searchObj.data.queryResults = {};
              searchObj.data.histogram = {
                xData: [],
                yData: [],
                chartParams: {},
              };
            }
          })
          .catch((e) => {
            $q.notify({
              type: "negative",
              message:
                "Error while pulling index for selected organization" +
                e.message,
              timeout: 2000,
            });
          })
          .finally(() => {
            searchObj.loading = false;
          });
      } catch (e) {
        searchObj.loading = false;
        showErrorNotification("Error while getting streams");
      }
    }

    function loadStreamLists(isFirstLoad = false) {
      try {
        searchObj.data.metrics.metricList = [];
        searchObj.data.metrics.selectedMetric = null;
        if (searchObj.data.streamResults.list.length) {
          let lastUpdatedStreamTime = 0;
          let selectedStreamItemObj = {};
          searchObj.data.streamResults.list.forEach((item: any) => {
            let itemObj = {
              label: item.name,
              value: item.name,
              type: metricTypeMapping[item.metrics_meta.metric_type] || "",
              help: item.metrics_meta.help || "",
            };
            searchObj.data.metrics.metricList.push(itemObj);
            // If isFirstLoad is true, then select the stream from query params
            if (
              isFirstLoad &&
              router.currentRoute.value?.query?.stream == item.name
            ) {
              lastUpdatedStreamTime = item.stats.doc_time_max;
              selectedStreamItemObj = itemObj;
            }

            // If stream from query params doesn't match the selected stream, then select the stream from last updated time
            if (
              item.stats.doc_time_max >= lastUpdatedStreamTime &&
              !(
                router.currentRoute.value.query?.stream &&
                selectedStreamItemObj.value &&
                router.currentRoute.value.query.stream ===
                  selectedStreamItemObj.value
              )
            ) {
              lastUpdatedStreamTime = item.stats.doc_time_max;
              selectedStreamItemObj = itemObj;
            }
          });
          if (selectedStreamItemObj.label != undefined) {
            searchObj.data.metrics.selectedMetric = {
              ...selectedStreamItemObj,
            };
          } else {
            searchObj.data.queryResults = {};
            searchObj.data.metrics.selectedMetric = null;
            searchObj.data.histogram = {
              xData: [],
              yData: [],
              chartParams: {},
            };
          }
        }
      } catch (e) {
        console.log("Error while loading streams");
      }
    }

    function getConsumableDateTime() {
      try {
        if (searchObj.data.datetime.tab == "relative") {
          let period = "";
          let periodValue = 0;
          // quasar does not support arithmetic on weeks. convert to days.

          if (
            searchObj.data.datetime.relative.period.label.toLowerCase() ==
            "weeks"
          ) {
            period = "days";
            periodValue = searchObj.data.datetime.relative.value * 7;
          } else {
            period =
              searchObj.data.datetime.relative.period.label.toLowerCase();
            periodValue = searchObj.data.datetime.relative.value;
          }
          const subtractObject = '{"' + period + '":' + periodValue + "}";

          let endTimeStamp = new Date();

          const startTimeStamp = date.subtractFromDate(
            endTimeStamp,
            JSON.parse(subtractObject),
          );

          return {
            start_time: startTimeStamp,
            end_time: endTimeStamp,
          };
        } else {
          let start, end;
          if (
            searchObj.data.datetime.absolute.date.from == "" &&
            searchObj.data.datetime.absolute.startTime == ""
          ) {
            start = new Date();
          } else {
            start = new Date(
              searchObj.data.datetime.absolute.date.from +
                " " +
                searchObj.data.datetime.absolute.startTime,
            );
          }
          if (
            searchObj.data.datetime.absolute.date.to == "" &&
            searchObj.data.datetime.absolute.endTime == ""
          ) {
            end = new Date();
          } else {
            end = new Date(
              searchObj.data.datetime.absolute.date.to +
                " " +
                searchObj.data.datetime.absolute.endTime,
            );
          }
          const rVal = {
            start_time: start,
            end_time: end,
          };
          return rVal;
        }
      } catch (e) {
        console.log("Error while getting consumable date time");
      }
    }

    function runQuery() {
      try {
        if (
          !searchObj.data.metrics.selectedMetric?.value ||
          !searchObj.data.query
        ) {
          return false;
        }

        searchObj.data.errorMsg = "";
        const timestamps: any =
          searchObj.data.datetime.type === "relative"
            ? getConsumableRelativeTime(
                searchObj.data.datetime.relativeTimePeriod,
              )
            : cloneDeep(searchObj.data.datetime);
        dashboardPanelData.meta.dateTime = {
          start_time: new Date(timestamps.startTime / 1000),
          end_time: new Date(timestamps.endTime / 1000),
        };
        chartData.value = cloneDeep(dashboardPanelData.data);

        updateUrlQueryParams();
      } catch (e) {
        showErrorNotification("Request failed.");
      }
    }

    function loadPageData(isFirstLoad = false) {
      resetSearchObj();
      searchObj.organizationIdentifier =
        store.state.selectedOrganization.identifier;

      //get stream list
      getStreamList(isFirstLoad);
    }

    const refreshData = () => {
      if (
        searchObj.meta.refreshInterval > 0 &&
        router.currentRoute.value.name == "metrics"
      ) {
        clearInterval(refreshIntervalID);

        refreshIntervalID = setInterval(
          () => {
            if (!searchObj.loading) runQuery();
          },
          parseInt(searchObj.meta.refreshInterval) * 1000,
        );

        $q.notify({
          message: `Live mode is enabled`,
          color: "positive",
          position: "top",
          timeout: 1000,
        });
      } else {
        clearInterval(refreshIntervalID);
      }
    };

    const setQuery = () => {};

    const updateDateTime = (value: any) => {
      searchObj.data.datetime = {
        startTime: value.startTime,
        endTime: value.endTime,
        relativeTimePeriod: value.relativeTimePeriod
          ? value.relativeTimePeriod
          : searchObj.data.datetime.relativeTimePeriod,
        type: value.relativeTimePeriod ? "relative" : "absolute",
      };

      if (config.isCloud == "true" && value.userChangedValue) {
        segment.track("Button Click", {
          button: "Date Change",
          tab: value.tab,
          value: value,
          //user_org: this.store.state.selectedOrganization.identifier,
          //user_id: this.store.state.userInfo.email,
          stream_name: searchObj.data.stream.selectedStream.value,
          page: "Search Logs",
        });
      }

      if (value.valueType === "relative") runQuery();
    };

    const updateQueryValue = async (event, value) => {
      dashboardPanelData.data.queries[0].query = value;
      autoCompleteData.value.query = value;
      autoCompleteData.value.text = event.changes[0].text;
      autoCompleteData.value.dateTime = searchObj.data.datetime;
      autoCompleteData.value.position.cursorIndex =
        metricsQueryEditorRef.value.getCursorIndex();
      autoCompleteData.value.popup.open =
        metricsQueryEditorRef.value.triggerAutoComplete;
      autoCompleteData.value.popup.close =
        metricsQueryEditorRef.value.disableSuggestionPopup;
      getSuggestions();
    };

    const addToDashboard = () => {
      showAddToDashboardDialog.value = true;
    };

    const addPanelToDashboard = () => {
      showAddToDashboardDialog.value = false;
    };

    const onMetricChange = async (metric) => {
      if (!searchObj.data.metrics.selectedMetric?.value) return;

      const query = metric?.value + "{}";
      nextTick(() => {
        metricsQueryEditorRef?.value?.setValue(query);
      });
    };

    function restoreUrlQueryParams() {
      const queryParams = router.currentRoute.value.query;
      if (!queryParams.stream) {
        return;
      }

      const date = {
        startTime: queryParams.from,
        endTime: queryParams.to,
        relativeTimePeriod: queryParams.period || null,
        type: queryParams.period ? "relative" : "absolute",
      };
      if (date) {
        searchObj.data.datetime = date;
      }
      if (queryParams.query) {
        searchObj.data.query = b64DecodeUnicode(queryParams.query);
        dashboardPanelData.data.queries[0].query = searchObj.data.query;
      }
      if (queryParams.refresh) {
        searchObj.meta.refreshInterval = queryParams.refresh;
      }
    }

    function generateURLQuery(isShareLink = false) {
      try {
        const date = searchObj.data.datetime;
        const query = {
          stream: searchObj.data.metrics.selectedMetric?.value,
        };

        if (date.type == "relative" && !isShareLink) {
          query["period"] = date.relativeTimePeriod;
        } else {
          query["from"] = date.startTime;
          query["to"] = date.endTime;
        }
        query["refresh"] = searchObj.meta.refreshInterval;

        if (searchObj.data.query) {
          query["query"] = b64EncodeUnicode(searchObj.data.query);
        }
        query["org_identifier"] = store.state.selectedOrganization.identifier;

        return query;
      } catch (err) {
        console.log(err);
      }
    }

    function updateUrlQueryParams() {
      const query = generateURLQuery();
      router.push({ query });
    }

    const addLabelToEditor = (label) => {
      try {
        const parsedQuery = parsePromQlQuery(searchObj.data.query);
        let query = "";
        if (!parsedQuery.label.hasLabels) {
          query = dashboardPanelData.data.queries[0].query + `{${label}}`;
        } else {
          query =
            dashboardPanelData.data.queries[0].query.slice(
              0,
              parsedQuery.label.position.end,
            ) +
            (dashboardPanelData.data.queries[0].query[
              parsedQuery.label.position.end - 1
            ] !== "," &&
            parsedQuery.label.position.end - parsedQuery.label.position.start >
              1
              ? ","
              : "") +
            label +
            dashboardPanelData.data.queries[0].query.slice(
              parsedQuery.label.position.end,
              dashboardPanelData.data.queries[0].query.length,
            );
        }
        metricsQueryEditorRef.value.setValue(query);
      } catch (e) {
        console.log(e);
      }
    };

    const onSplitterUpdate = () => {
      window.dispatchEvent(new Event("resize"));
    };

    const onChangeRefreshInterval = () => {
      updateUrlQueryParams();
      refreshData();
    };

    const shareLink = () => {
      const queryObj = generateURLQuery(true);
      const queryString = Object.entries(queryObj)
        .map(
          ([key, value]) =>
            `${encodeURIComponent(key)}=${encodeURIComponent(value)}`,
        )
        .join("&");

      let shareURL = window.location.origin + window.location.pathname;

      if (queryString != "") {
        shareURL += "?" + queryString;
      }

      copyToClipboard(shareURL)
        .then(() => {
          $q.notify({
            type: "positive",
            message: "Link Copied Successfully!",
            timeout: 5000,
          });
        })
        .catch(() => {
          $q.notify({
            type: "negative",
            message: "Error while copy link.",
            timeout: 5000,
          });
        });
    };

    return {
      t,
      store,
      router,
      parser,
      searchObj,
      searchBarRef,
      loadPageData,
      runQuery,
      searchResultRef,
      getConsumableDateTime,
      refreshData,
      setQuery,
      updateDateTime,
      verifyOrganizationStatus,
      metricsQueryEditorRef,
      updateQueryValue,
      dashboardPanelData,
      chartData,
      addToDashboard,
      showAddToDashboardDialog,
      addPanelToDashboard,
      promqlKeywords,
      autoCompletePromqlKeywords,
      onMetricChange,
      updateUrlQueryParams,
      addLabelToEditor,
      onSplitterUpdate,
      resetSearchObj,
      onChangeRefreshInterval,
      shareLink,
    };
  },
  computed: {
    showQuery() {
      return this.searchObj.meta.showQuery;
    },
    selectedMetric() {
      return this.searchObj.data.metrics.selectedMetric;
    },
    changeRelativeDate() {
      return (
        this.searchObj.data.datetime.relative.value +
        this.searchObj.data.datetime.relative.period.value
      );
    },
  },
  watch: {
    selectedMetric: {
      deep: true,
      handler: function (metric, oldMetric) {
        // This is temporary fix
        // This is to handle the stream selection for first time
        // Avoid calling onMetricChange() when there is data query i.e when user loads shared link or url has query param
        if (
          this.searchObj.data.metrics.selectedMetric?.value &&
          !oldMetric &&
          !this.searchObj.data.query
        ) {
          this.onMetricChange(metric);
        }
      },
    },
  },
});
</script>

<style lang="scss">
.metrics-query-editor-container {
  .monaco-editor {
    height: 80px !important;
  }
}
.metrics-page {
  .index-menu .field_list .field_overlay .field_label,
  .q-field__native,
  .q-field__input,
  .q-table tbody td {
    font-size: 12px !important;
  }

  .q-splitter__after {
    overflow: hidden;
  }

  .q-item__label span {
    /* text-transform: capitalize; */
  }

  .index-table :hover::-webkit-scrollbar,
  #searchGridComponent:hover::-webkit-scrollbar {
    height: 13px;
    width: 13px;
  }

  .index-table ::-webkit-scrollbar-track,
  #searchGridComponent::-webkit-scrollbar-track {
    -webkit-box-shadow: inset 0 0 6px rgba(0, 0, 0, 0.3);
    border-radius: 10px;
  }

  .index-table ::-webkit-scrollbar-thumb,
  #searchGridComponent::-webkit-scrollbar-thumb {
    border-radius: 10px;
    -webkit-box-shadow: inset 0 0 6px rgba(0, 0, 0, 0.5);
  }

  .q-table__top {
    padding: 0px !important;
  }

  .q-table__control {
    width: 100%;
  }

  .q-field__control-container {
    padding-top: 0px !important;
  }

  .refresh-button {
    height: 30px;
    line-height: 29px;
    .q-icon {
      font-size: 18px;
      padding-right: 2px;
    }
  }

  .metrics-date-time {
    height: 40px;
    .date-time-button {
      height: 30px;
      padding: 0 8px;
    }
  }

  .search-button {
    min-width: 96px;
    height: 30px;
    line-height: 29px;
    font-weight: bold;
    text-transform: initial;
    font-size: 11px;
    color: white;

    .q-btn__content {
      background: $secondary;
      border-radius: 3px 3px 3px 3px;
      padding: 0px 5px;

      .q-icon {
        font-size: 15px;
        color: #ffffff;
      }
    }
  }
  div.plotly-notifier {
    visibility: hidden;
  }
}
</style>
