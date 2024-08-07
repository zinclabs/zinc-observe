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

<!-- eslint-disable vue/attribute-hyphenation -->
<!-- eslint-disable vue/v-on-event-hyphenation -->
<template>
  <q-page class="logPage q-my-xs" id="logPage">
    <div id="secondLevel" class="full-height">
      <q-splitter
        class="logs-horizontal-splitter full-height"
        v-model="splitterModel"
        horizontal
      >
        <template v-slot:before>
          <search-bar
            data-test="logs-search-bar"
            ref="searchBarRef"
            :fieldValues="fieldValues"
            :key="searchObj.data.transforms.length || -1"
            @searchdata="searchData"
            @onChangeInterval="onChangeInterval"
            @onChangeTimezone="refreshTimezone"
            @handleQuickModeChange="handleQuickModeChange"
            @handleRunQueryFn="handleRunQueryFn"
            @on-auto-interval-trigger="onAutoIntervalTrigger"
          />
        </template>
        <template v-slot:after>
          <div
            id="thirdLevel"
            class="row scroll relative-position thirdlevel full-height overflow-hidden logsPageMainSection"
            style="width: 100%"
            v-show="searchObj.meta.logsVisualizeToggle == 'logs'"
          >
            <!-- Note: Splitter max-height to be dynamically calculated with JS -->
            <q-splitter
              v-model="searchObj.config.splitterModel"
              :limits="searchObj.config.splitterLimit"
              style="width: 100%"
              class="full-height"
              @update:model-value="onSplitterUpdate"
            >
              <template #before>
                <div class="relative-position full-height">
                  <index-list
                    v-if="searchObj.meta.showFields"
                    data-test="logs-search-index-list"
                    :key="
                      searchObj.data.stream.selectedStream.join(',') ||
                      'default'
                    "
                    class="full-height"
                    @setInterestingFieldInSQLQuery="
                      setInterestingFieldInSQLQuery
                    "
                  />
                  <q-btn
                    data-test="logs-search-field-list-collapse-btn"
                    :icon="
                      searchObj.meta.showFields
                        ? 'chevron_left'
                        : 'chevron_right'
                    "
                    :title="
                      searchObj.meta.showFields
                        ? 'Collapse Fields'
                        : 'Open Fields'
                    "
                    dense
                    size="20px"
                    round
                    class="q-mr-xs field-list-collapse-btn"
                    color="primary"
                    :style="{
                      right: searchObj.meta.showFields ? '-20px' : '-24px',
                    }"
                    @click="collapseFieldList"
                  ></q-btn>
                </div>
              </template>
              <template #after>
                <div
                  v-if="
                    searchObj.data.filterErrMsg !== '' &&
                    searchObj.loading == false
                  "
                  class="q-mt-lg"
                >
                  <h5 class="text-center">
                    <q-icon name="warning"
color="warning" size="10rem" /><br />
                    <div
                      data-test="logs-search-filter-error-message"
                      v-html="searchObj.data.filterErrMsg"
                    ></div>
                  </h5>
                </div>
                <div
                  v-else-if="
                    searchObj.data.errorMsg !== '' && searchObj.loading == false
                  "
                  class="q-ma-lg"
                >
                  <h5 class="text-center q-ma-none">
                    <div
                      data-test="logs-search-result-not-found-text"
                      v-if="
                        searchObj.data.errorCode == 0 &&
                        searchObj.data.errorMsg == ''
                      "
                    >
                      Result not found.
                    </div>
                    <div data-test="logs-search-error-message" v-else>
                      Error occurred while retrieving search events.
                      <q-btn
                        v-if="searchObj.data.errorMsg != ''"
                        @click="toggleErrorDetails"
                        size="sm"
                        data-test="logs-page-result-error-details-btn"
                        >{{ t("search.histogramErrorBtnLabel") }}</q-btn
                      >
                    </div>
                    <div
                      data-test="logs-search-error-20003"
                      v-if="parseInt(searchObj.data.errorCode) == 20003"
                    >
                      <q-btn
                        no-caps
                        unelevated
                        size="sm"
                        bg-secondary
                        class="no-border bg-secondary text-white"
                        :to="
                          '/streams?dialog=' +
                          searchObj.data.stream.selectedStream.label
                        "
                        >Click here</q-btn
                      >
                      to configure a full text search field to the stream.
                    </div>
                    <q-item-label>{{
                      searchObj.data.additionalErrorMsg
                    }}</q-item-label>
                  </h5>
                </div>
                <div
                  v-else-if="
                    searchObj.data.stream.selectedStream.length == 0 &&
                    searchObj.loading == false
                  "
                  class="row q-mt-lg"
                >
                  <h6
                    data-test="logs-search-no-stream-selected-text"
                    class="text-center col-10 q-mx-none"
                  >
                    <q-icon name="info"
color="primary" size="md" /> Select a
                    stream and press 'Run query' to continue. Additionally, you
                    can apply additional filters and adjust the date range to
                    enhance search.
                  </h6>
                </div>
                <div
                  v-else-if="
                    searchObj.data.queryResults.hasOwnProperty('hits') &&
                    searchObj.data.queryResults.hits.length == 0 &&
                    searchObj.loading == false &&
                    searchObj.meta.searchApplied == true
                  "
                  class="row q-mt-lg"
                >
                  <h6
                    data-test="logs-search-error-message"
                    class="text-center q-ma-none col-10"
                  >
                    <q-icon name="info"
color="primary" size="md" />
                    {{ t("search.noRecordFound") }}
                    <q-btn
                      v-if="searchObj.data.errorMsg != ''"
                      @click="toggleErrorDetails"
                      size="sm"
                      data-test="logs-page-result-error-details-btn"
                      >{{ t("search.histogramErrorBtnLabel") }}</q-btn
                    ><br />
                  </h6>
                </div>
                <div
                  v-else-if="
                    searchObj.data.queryResults.hasOwnProperty('hits') &&
                    searchObj.data.queryResults.hits.length == 0 &&
                    searchObj.loading == false &&
                    searchObj.meta.searchApplied == false
                  "
                  class="row q-mt-lg"
                >
                  <h6
                    data-test="logs-search-error-message"
                    class="text-center q-ma-none col-10"
                  >
                    <q-icon name="info"
color="primary" size="md" />
                    {{ t("search.applySearch") }}
                  </h6>
                </div>
                <div
                  v-else
                  data-test="logs-search-search-result"
                  class="full-height search-result-container"
                >
                  <search-result
                    ref="searchResultRef"
                    :expandedLogs="expandedLogs"
                    @update:datetime="setHistogramDate"
                    @update:scroll="getMoreData"
                    @update:recordsPerPage="getMoreDataRecordsPerPage"
                    @expandlog="toggleExpandLog"
                  />
                </div>
                <div class="text-center col-10 q-ma-none">
                  <h5>
                    <span v-if="disableMoreErrorDetails">
                      <SanitizedHtmlRenderer
                        data-test="logs-search-detail-error-message"
                        :htmlContent="searchObj.data.errorMsg"
                      />
                    </span>
                  </h5>
                </div>
              </template>
            </q-splitter>
          </div>
          <div
            v-show="searchObj.meta.logsVisualizeToggle == 'visualize'"
            :style="`height: calc(100vh - ${splitterModel}vh - 40px);`"
          >
            <VisualizeLogsQuery
              :visualizeChartData="visualizeChartData"
              :errorData="visualizeErrorData"
              @update:stream-list="streamListUpdated"
            ></VisualizeLogsQuery>
          </div>
        </template>
      </q-splitter>
    </div>
  </q-page>
</template>

<script lang="ts">
// @ts-nocheck
import {
  defineComponent,
  ref,
  onActivated,
  computed,
  nextTick,
  onBeforeMount,
  watch,
  defineAsyncComponent,
  provide,
} from "vue";
import { useQuasar } from "quasar";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";

import segment from "@/services/segment_analytics";
import config from "@/aws-exports";
import { verifyOrganizationStatus } from "@/utils/zincutils";
import MainLayoutCloudMixin from "@/enterprise/mixins/mainLayout.mixin";
import SanitizedHtmlRenderer from "@/components/SanitizedHtmlRenderer.vue";
import useLogs from "@/composables/useLogs";
import VisualizeLogsQuery from "@/plugins/logs/VisualizeLogsQuery.vue";
import useDashboardPanelData from "@/composables/useDashboardPanel";
import { reactive } from "vue";
import { getConsumableRelativeTime } from "@/utils/date";
import { cloneDeep } from "lodash-es";
import {
  buildSqlQuery,
  getFieldsFromQuery,
  getValidConditionObj,
} from "@/utils/query/sqlUtils";
import useNotifications from "@/composables/useNotifications";

export default defineComponent({
  name: "PageSearch",
  components: {
    SearchBar: defineAsyncComponent(
      () => import("@/plugins/logs/SearchBar.vue")
    ),
    IndexList: defineAsyncComponent(
      () => import("@/plugins/logs/IndexList.vue")
    ),
    SearchResult: defineAsyncComponent(
      () => import("@/plugins/logs/SearchResult.vue")
    ),
    ConfirmDialog: defineAsyncComponent(
      () => import("@/components/ConfirmDialog.vue")
    ),
    SanitizedHtmlRenderer,
    VisualizeLogsQuery,
  },
  mixins: [MainLayoutCloudMixin],
  methods: {
    setHistogramDate(date: any) {
      this.searchBarRef.dateTimeRef.setCustomDate("absolute", date);
    },
    searchData() {
      if (this.searchObj.loading == false) {
        this.searchObj.loading = true;
        this.searchObj.runQuery = true;
      }

      if (config.isCloud == "true") {
        segment.track("Button Click", {
          button: "Search Data",
          user_org: this.store.state.selectedOrganization.identifier,
          user_id: this.store.state.userInfo.email,
          stream_name: this.searchObj.data.stream.selectedStream.join(","),
          show_query: this.searchObj.meta.showQuery,
          show_histogram: this.searchObj.meta.showHistogram,
          sqlMode: this.searchObj.meta.sqlMode,
          showFields: this.searchObj.meta.showFields,
          page: "Search Logs",
        });
      }
    },
    async getMoreDataRecordsPerPage() {
      if (this.searchObj.meta.refreshInterval == 0) {
        // this.searchObj.data.resultGrid.currentPage =
        //   ((this.searchObj.data.queryResults?.hits?.length || 0) +
        //     ((this.searchObj.data.queryResults?.hits?.length || 0) + 150)) /
        //     150 -
        //   1;
        // this.searchObj.data.resultGrid.currentPage =
        //   this.searchObj.data.resultGrid.currentPage + 1;
        this.searchObj.loading = true;
        await this.getQueryData(false);
        this.refreshHistogramChart();

        if (config.isCloud == "true") {
          segment.track("Button Click", {
            button: "Get More Data",
            user_org: this.store.state.selectedOrganization.identifier,
            user_id: this.store.state.userInfo.email,
            stream_name: this.searchObj.data.stream.selectedStream.join(","),
            page: "Search Logs",
          });
        }
      }
    },
    async getMoreData() {
      if (this.searchObj.meta.refreshInterval == 0) {
        // this.searchObj.data.resultGrid.currentPage =
        //   ((this.searchObj.data.queryResults?.hits?.length || 0) +
        //     ((this.searchObj.data.queryResults?.hits?.length || 0) + 150)) /
        //     150 -
        //   1;
        // this.searchObj.data.resultGrid.currentPage =
        //   this.searchObj.data.resultGrid.currentPage + 1;
        this.searchObj.loading = true;
        await this.getQueryData(true);
        this.refreshHistogramChart();

        if (config.isCloud == "true") {
          segment.track("Button Click", {
            button: "Get More Data",
            user_org: this.store.state.selectedOrganization.identifier,
            user_id: this.store.state.userInfo.email,
            stream_name: this.searchObj.data.stream.selectedStream.join(","),
            page: "Search Logs",
          });
        }
      }
    },
    async getLessData() {
      if (
        this.searchObj.meta.sqlMode == false &&
        this.searchObj.meta.refreshInterval == 0 &&
        this.searchObj.data.queryResults.total >
          this.searchObj.data.queryResults.from &&
        this.searchObj.data.queryResults.total >
          this.searchObj.data.queryResults.size &&
        this.searchObj.data.queryResults.total >
          this.searchObj.data.queryResults.size +
            this.searchObj.data.queryResults.from
      ) {
        // this.searchObj.data.resultGrid.currentPage =
        //   ((this.searchObj.data.queryResults?.hits?.length || 0) +
        //     ((this.searchObj.data.queryResults?.hits?.length || 0) + 150)) /
        //     150 -
        //   1;
        this.searchObj.data.resultGrid.currentPage =
          this.searchObj.data.resultGrid.currentPage - 1;

        await this.getQueryData(true);
        this.refreshHistogramChart();

        if (config.isCloud == "true") {
          segment.track("Button Click", {
            button: "Get Less Data",
            user_org: this.store.state.selectedOrganization.identifier,
            user_id: this.store.state.userInfo.email,
            stream_name: this.searchObj.data.stream.selectedStream.join(","),
            page: "Search Logs",
          });
        }
      }
    },
    toggleErrorDetails() {
      this.disableMoreErrorDetails = !this.disableMoreErrorDetails;
    },
  },
  setup() {
    const { t } = useI18n();
    const store = useStore();
    const router = useRouter();
    const $q = useQuasar();
    const disableMoreErrorDetails: boolean = ref(false);
    let {
      searchObj,
      getQueryData,
      fieldValues,
      updateGridColumns,
      refreshData,
      updateUrlQueryParams,
      loadLogsData,
      updateStreams,
      restoreUrlQueryParams,
      handleRunQuery,
      generateHistogramData,
      resetSearchObj,
      resetStreamData,
      getHistogramQueryData,
      fnParsedSQL,
      addOrderByToQuery,
      getRegionInfo,
    } = useLogs();
    const searchResultRef = ref(null);
    const searchBarRef = ref(null);
    let parser: any;
    const expandedLogs = ref({});
    const splitterModel = ref(10);

    // flag to know if it is the first time visualize
    let firstTimeVisualizeFlag = false;

    const { showErrorNotification } = useNotifications();

    provide("dashboardPanelDataPageKey", "logs");
    const visualizeChartData = ref({});
    const {
      dashboardPanelData,
      validatePanel,
      generateLabelFromName,
      resetDashboardPanelData,
    } = useDashboardPanelData("logs");
    const visualizeErrorData: any = reactive({
      errors: [],
    });

    // function restoreUrlQueryParams() {
    //   const queryParams = router.currentRoute.value.query;
    //   if (!queryParams.stream) {
    //     return;
    //   }
    //   const date = {
    //     startTime: queryParams.from,
    //     endTime: queryParams.to,
    //     relativeTimePeriod: queryParams.period || null,
    //     type: queryParams.period ? "relative" : "absolute",
    //   };
    //   if (date) {
    //     searchObj.data.datetime = date;
    //   }
    //   if (queryParams.query) {
    //     searchObj.meta.sqlMode = queryParams.sql_mode == "true" ? true : false;
    //     searchObj.data.editorValue = b64DecodeUnicode(queryParams.query);
    //     searchObj.data.query = b64DecodeUnicode(queryParams.query);
    //   }
    //   if (queryParams.refresh) {
    //     searchObj.meta.refreshInterval = queryParams.refresh;
    //   }
    // }

    // async function loadPageData() {
    //   try {
    //     loadLogsData();
    //   } catch (e) {
    //     searchObj.loading = false;
    //     console.log(e);
    //   }
    // }
    // onUnmounted(() => {
    // resetSearchObj();
    // resetStreamData();
    // });

    onActivated(async () => {
      // if search tab
      if (searchObj.meta.logsVisualizeToggle == "logs") {
        const queryParams: any = router.currentRoute.value.query;
        searchObj.meta.refreshHistogram = true;

        const isStreamChanged =
          queryParams.stream_type !== searchObj.data.stream.streamType ||
          queryParams.stream !== searchObj.data.stream.selectedStream.join(",");

        if (
          isStreamChanged &&
          queryParams.type === "stream_explorer" &&
          searchObj.loading == false
        ) {
          resetSearchObj();
          resetStreamData();
          restoreUrlQueryParams();
          loadLogsData();
          return;
        }

        if (
          searchObj.organizationIdetifier !=
            store.state.selectedOrganization.identifier &&
          searchObj.loading == false
        ) {
          loadLogsData();
        } else if (!searchObj.loading) updateStreams();

        refreshHistogramChart();
      } else {
        // visualize tab
        handleRunQueryFn();
      }
    });

    onBeforeMount(async () => {
      await importSqlParser();
      if (searchObj.meta.logsVisualizeToggle == "logs") {
        // searchObj.loading = true;
        searchObj.meta.pageType = "logs";
        searchObj.meta.refreshHistogram = true;
        if (
          config.isEnterprise == "true" &&
          store.state.zoConfig.super_cluster_enabled
        ) {
          await getRegionInfo();
        }

        searchObj.organizationIdetifier =
          store.state.selectedOrganization.identifier;
        restoreUrlQueryParams();
        if (searchObj.loading == false) {
          resetSearchObj();
          resetStreamData();
          restoreUrlQueryParams();
          loadLogsData();
        }
        if (config.isCloud == "true") {
          MainLayoutCloudMixin.setup().getOrganizationThreshold(store);
        }
        searchObj.meta.quickMode = store.state.zoConfig.quick_mode_enabled;
      }
    });

    /**
     * As we are redirecting stream explorer to logs page, we need to check if the user has changed the stream type from stream explorer to logs.
     * This watcher is used to check if the user has changed the stream type from stream explorer to logs.
     * This gets triggered when stream explorer is active and user clicks on logs icon from left menu sidebar. Then we need to redirect the user to logs page again.
     */
    watch(
      () => router.currentRoute.value.query.type,
      (type, prev) => {
        if (
          searchObj.shouldIgnoreWatcher == false &&
          router.currentRoute.value.name === "logs" &&
          prev === "stream_explorer" &&
          !type
        ) {
          searchObj.meta.pageType = "logs";
          loadLogsData();
        }
      }
    );

    const importSqlParser = async () => {
      const useSqlParser: any = await import("@/composables/useParser");
      const { sqlParser }: any = useSqlParser.default();
      parser = await sqlParser();
    };

    const runQueryFn = async () => {
      // searchObj.data.resultGrid.currentPage = 0;
      // searchObj.runQuery = false;
      // expandedLogs.value = {};
      try {
        await getQueryData();
        refreshHistogramChart();
      } catch (e) {
        console.log(e);
      }
    };

    const refreshTimezone = () => {
      updateGridColumns();
      generateHistogramData();
      refreshHistogramChart();
    };

    const refreshHistogramChart = () => {
      nextTick(() => {
        if (
          searchObj.meta.showHistogram &&
          searchResultRef.value?.reDrawChart
        ) {
          searchResultRef.value.reDrawChart();
        }
      });
    };

    const setQuery = (sqlMode: boolean) => {
      if (!searchBarRef.value) {
        console.error("searchBarRef is null");
        return;
      }

      try {
        if (sqlMode) {
          let selectFields = "";
          let whereClause = "";
          let currentQuery = searchObj.data.query;

          const hasSelect =
            currentQuery != "" &&
            (currentQuery.toLowerCase() === "select" ||
              currentQuery.toLowerCase().indexOf("select ") == 0);
          //check if user try to applied saved views in which sql mode is enabled.
          if (currentQuery.indexOf("SELECT") >= 0) {
            return;
          }

          // Parse the query and check if it is valid
          // It should have one column and one table

          // const hasSelect =
          //   currentQuery.toLowerCase() === "select" ||
          //   currentQuery.toLowerCase().indexOf("select ") == 0;

          if (!hasSelect) {
            if (currentQuery != "") {
              currentQuery = currentQuery.split("|");
              if (currentQuery.length > 1) {
                selectFields = "," + currentQuery[0].trim();
                if (currentQuery[1].trim() != "") {
                  whereClause = "WHERE " + currentQuery[1].trim();
                }
              } else if (currentQuery[0].trim() != "") {
                if (currentQuery[0].trim() != "") {
                  whereClause = "WHERE " + currentQuery[0].trim();
                }
              }
            }

            searchObj.data.query =
              `SELECT [FIELD_LIST]${selectFields} FROM "` +
              searchObj.data.stream.selectedStream.join(",") +
              `" ` +
              whereClause;

            if (searchObj.data.stream.selectedStreamFields.length == 0) {
              const streamData: any = getStream(
                searchObj.data.stream.selectedStream[0],
                searchObj.data.stream.streamType || "logs",
                true
              );
              searchObj.data.stream.selectedStreamFields = streamData.schema;
            }

            const streamFieldNames: any =
              searchObj.data.stream.selectedStreamFields.map(
                (item: any) => item.name
              );

            for (
              let i = searchObj.data.stream.interestingFieldList.length - 1;
              i >= 0;
              i--
            ) {
              const fieldName = searchObj.data.stream.interestingFieldList[i];
              if (!streamFieldNames.includes(fieldName)) {
                searchObj.data.stream.interestingFieldList.splice(i, 1);
              }
            }

            if (
              searchObj.data.stream.interestingFieldList.length > 0 &&
              searchObj.meta.quickMode
            ) {
              searchObj.data.query = searchObj.data.query.replace(
                "[FIELD_LIST]",
                searchObj.data.stream.interestingFieldList.join(",")
              );
            } else {
              searchObj.data.query = searchObj.data.query.replace(
                "[FIELD_LIST]",
                "*"
              );
            }
          }

          searchObj.data.query = addOrderByToQuery(
            searchObj.data.query,
            store.state.zoConfig.timestamp_column,
            "DESC",
            searchObj.data.stream.selectedStream.join(",")
          );

          searchObj.data.editorValue = searchObj.data.query;

          searchBarRef.value.udpateQuery();

          searchObj.data.parsedQuery = parser.astify(searchObj.data.query);
        } else {
          searchObj.data.query = "";
          searchBarRef.value.udpateQuery();
        }
      } catch (e) {
        console.log("Logs : Error in setQuery");
      }
    };

    const collapseFieldList = () => {
      if (searchObj.meta.showFields) searchObj.meta.showFields = false;
      else searchObj.meta.showFields = true;
    };

    const areStreamsPresent = computed(() => {
      return !!searchObj.data.stream.streamLists.length;
    });

    const toggleExpandLog = async (index: number) => {
      if (expandedLogs.value[index.toString()])
        delete expandedLogs.value[index.toString()];
      else expandedLogs.value[index.toString()] = true;
    };

    const onSplitterUpdate = () => {
      window.dispatchEvent(new Event("resize"));
    };

    const onChangeInterval = () => {
      updateUrlQueryParams();
      refreshData();
    };

    const onAutoIntervalTrigger = () => {
      // handle event for visualization page only
      if (searchObj.meta.logsVisualizeToggle == "visualize") {
        handleRunQueryFn();
      }
    };

    function removeFieldByName(data, fieldName) {
      return data.filter((item: any) => {
        if (item.expr) {
          if (item.expr.column === fieldName) {
            return false;
          }
          if (
            item.expr.type === "aggr_func" &&
            item.expr.args.expr.column === fieldName
          ) {
            return false;
          }
        }
        return true;
      });
    }

    const setInterestingFieldInSQLQuery = (
      field: any,
      isFieldExistInSQL: boolean
    ) => {
      //implement setQuery function using node-sql-parser
      //isFieldExistInSQL is used to check if the field is already present in the query or not.
      const parsedSQL = fnParsedSQL();
      if (parsedSQL) {
        if (isFieldExistInSQL) {
          //remove the field from the query
          if (parsedSQL.columns.length > 0) {
            let filteredData = removeFieldByName(parsedSQL.columns, field.name);

            const index = searchObj.data.stream.interestingFieldList.indexOf(
              field.name
            );
            if (index > -1) {
              searchObj.data.stream.interestingFieldList.splice(index, 1);
              field.isInterestingField = false;
            }
            parsedSQL.columns = filteredData;
          }
        } else {
          //add the field in the query
          if (parsedSQL.columns.length > 0) {
            // iterate and remove the * from the query
            parsedSQL.columns = removeFieldByName(parsedSQL.columns, "*");
          }

          parsedSQL.columns.push({
            expr: {
              type: "column_ref",
              column: field.name,
            },
          });
        }

        if (parsedSQL.columns.length == 0) {
          parsedSQL.columns.push({
            expr: {
              type: "column_ref",
              column: "*",
            },
          });
        }

        const newQuery = parser
          .sqlify(parsedSQL)
          .replace(/`/g, "")
          .replace(
            searchObj.data.stream.selectedStream[0],
            `"${searchObj.data.stream.selectedStream[0]}"`
          );
        searchObj.data.query = newQuery;
        searchObj.data.editorValue = newQuery;

        searchBarRef.value.udpateQuery();

        searchObj.data.parsedQuery = parser.astify(searchObj.data.query);
      }
    };

    const handleQuickModeChange = () => {
      if (searchObj.meta.quickMode == true) {
        let field_list: string = "*";
        if (searchObj.data.stream.interestingFieldList.length > 0) {
          field_list = searchObj.data.stream.interestingFieldList.join(",");
        }
        if (searchObj.meta.sqlMode == true) {
          searchObj.data.query = searchObj.data.query.replace(
            /SELECT\s+(.*?)\s+FROM/i,
            (match, fields) => {
              return `SELECT ${field_list} FROM`;
            }
          );
          setQuery(searchObj.meta.quickMode);
          updateUrlQueryParams();
        }
      }
    };

    //validate the data
    const isValid = (onlyChart = false) => {
      const errors = visualizeErrorData.errors;
      errors.splice(0);
      const dashboardData = dashboardPanelData;

      // check if name of panel is there
      if (!onlyChart) {
        if (
          dashboardData.data.title == null ||
          dashboardData.data.title.trim() == ""
        ) {
          errors.push("Name of Panel is required");
        }
      }

      // will push errors in errors array
      validatePanel(errors);

      if (errors.length) {
        showErrorNotification(
          "There are some errors, please fix them and try again"
        );
        return false;
      }
      return true;
    };

    watch(
      () => [
        searchObj.data.tempFunctionContent,
        searchObj.meta.logsVisualizeToggle,
      ],
      () => {
        if (
          searchObj.meta.logsVisualizeToggle == "visualize" &&
          searchObj.meta.toggleFunction &&
          searchObj.data.tempFunctionContent
        ) {
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].vrlFunctionQuery = searchObj.data.tempFunctionContent;
        } else {
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].vrlFunctionQuery = "";
        }
      },
    );

    const setFieldsAndConditions = async () => {
      let logsQuery = searchObj.data.query ?? "";

      // if sql mode is off, then need to make query
      if (searchObj.meta.sqlMode == false) {
        logsQuery = buildSqlQuery(
          searchObj.data.stream.selectedStream[0],
          searchObj.meta.quickMode
            ? searchObj.data.stream.interestingFieldList
            : [],
          logsQuery
        );
      }

      const { fields, conditions, streamName } = await getFieldsFromQuery(
        logsQuery ?? "",
        store.state.zoConfig.timestamp_column ?? "_timestamp"
      );

      // set stream type and stream name
      if (streamName && streamName != "undefined") {
        // set firstTimeVisualizeFlag as true
        firstTimeVisualizeFlag = true;

        dashboardPanelData.data.queries[0].fields.stream_type =
          searchObj.data.stream.streamType ?? "logs";
        dashboardPanelData.data.queries[0].fields.stream = streamName;
      }

      // set fields
      fields.forEach((field) => {
        field.alias = field.alias ?? field.column;
        field.label = generateLabelFromName(field.column);

        // if fields doesnt have aggregation functions, then add it in the x axis fields
        if (
          field.aggregationFunction === null ||
          field.aggregationFunction == "histogram"
        ) {
          dashboardPanelData.data.queries[0].fields.x.push(field);
        } else {
          dashboardPanelData.data.queries[0].fields.y.push(field);
        }
      });

      // if x axis fields length is 2, then add 2nd x axis field to breakdown fields
      if (dashboardPanelData.data.queries[0].fields.x.length == 2) {
        dashboardPanelData.data.queries[0].fields.breakdown.push(
          dashboardPanelData.data.queries[0].fields.x[1]
        );
        // remove 2nd x axis field from x axis fields
        dashboardPanelData.data.queries[0].fields.x.splice(1, 1);
      }
      // if x axis fields length is greater than 2, then select chart type as table
      else if (dashboardPanelData.data.queries[0].fields.x.length > 2) {
        dashboardPanelData.data.type = "table";
      }

      // set conditions
      conditions.forEach((condition) => {
        condition.operator = condition.operator.toLowerCase();

        // get valid condition object
        condition = getValidConditionObj(condition);

        dashboardPanelData.data.queries[0].fields.filter.push(condition);
      });
    };

    // watch for changes in the visualize toggle
    // if it is in visualize mode, then set the query and stream name in the dashboard panel
    watch(
      () => [searchObj.meta.logsVisualizeToggle],
      async () => {
        // emit resize event
        // this will rerender/call resize method of already rendered chart to resize
        window.dispatchEvent(new Event("resize"));

        if (searchObj.meta.logsVisualizeToggle == "visualize") {
          // reset old rendered chart
          visualizeChartData.value = {};

          // set fields and conditions
          await setFieldsAndConditions();
        }
      }
    );

    watch(
      () => dashboardPanelData.data.type,
      async () => {
        // await nextTick();
        visualizeChartData.value = JSON.parse(
          JSON.stringify(dashboardPanelData.data)
        );
      }
    );

    watch(
      () => splitterModel.value,
      () => {
        // rerender chart
        window.dispatchEvent(new Event("resize"));
      }
    );

    watch(
      () => [
        searchObj.data.datetime.type,
        searchObj.data.datetime,
        searchObj.data.datetime.relativeTimePeriod,
      ],
      async () => {
        const dateTime =
          searchObj.data.datetime.type === "relative"
            ? getConsumableRelativeTime(
                searchObj.data.datetime.relativeTimePeriod
              )
            : cloneDeep(searchObj.data.datetime);

        dashboardPanelData.meta.dateTime = {
          start_time: new Date(dateTime.startTime),
          end_time: new Date(dateTime.endTime),
        };
      },
      { deep: true }
    );

    const handleRunQueryFn = () => {
      if (searchObj.meta.logsVisualizeToggle == "visualize") {
        if (!isValid(true)) {
          return;
        }

        // refresh the date time
        searchBarRef.value &&
          searchBarRef.value.dateTimeRef &&
          searchBarRef.value.dateTimeRef.refresh();

        visualizeChartData.value = JSON.parse(
          JSON.stringify(dashboardPanelData.data)
        );
      }
    };

    const handleChartApiError = (errorMessage: any) => {
      const errorList = visualizeErrorData.errors;
      errorList.splice(0);
      errorList.push(errorMessage);
    };

    const streamListUpdated = () => {
      if (
        searchObj.meta.logsVisualizeToggle == "visualize" &&
        firstTimeVisualizeFlag
      ) {
        firstTimeVisualizeFlag = false;
        // run query
        handleRunQueryFn();
      }
    };

    return {
      t,
      store,
      router,
      parser,
      searchObj,
      searchBarRef,
      splitterModel,
      // loadPageData,
      getQueryData,
      searchResultRef,
      runQueryFn,
      refreshData,
      setQuery,
      verifyOrganizationStatus,
      collapseFieldList,
      areStreamsPresent,
      toggleExpandLog,
      expandedLogs,
      fieldValues,
      onSplitterUpdate,
      updateGridColumns,
      updateUrlQueryParams,
      refreshHistogramChart,
      onChangeInterval,
      onAutoIntervalTrigger,
      handleRunQuery,
      refreshTimezone,
      resetSearchObj,
      resetStreamData,
      getHistogramQueryData,
      setInterestingFieldInSQLQuery,
      handleQuickModeChange,
      handleRunQueryFn,
      visualizeChartData,
      handleChartApiError,
      visualizeErrorData,
      streamListUpdated,
      disableMoreErrorDetails,
    };
  },
  computed: {
    showFields() {
      return this.searchObj.meta.showFields;
    },
    showHistogram() {
      return this.searchObj.meta.showHistogram;
    },
    showQuery() {
      return this.searchObj.meta.showQuery;
    },
    moveSplitter() {
      return this.searchObj.config.splitterModel;
    },
    // changeStream() {
    //   return this.searchObj.data.stream.selectedStream;
    // },
    changeRelativeDate() {
      return (
        this.searchObj.data.datetime.relative.value +
        this.searchObj.data.datetime.relative.period.value
      );
    },
    updateSelectedColumns() {
      return this.searchObj.data.stream.selectedFields.length;
    },
    runQuery() {
      return this.searchObj.runQuery;
    },
    changeRefreshInterval() {
      return this.searchObj.meta.refreshInterval;
    },
    fullSQLMode() {
      return this.searchObj.meta.sqlMode;
    },
    refreshHistogram() {
      return this.searchObj.meta.histogramDirtyFlag;
    },
    redrawHistogram() {
      return (
        this.searchObj.data.histogram.hasOwnProperty("xData") &&
        this.searchObj.data.histogram.xData.length
      );
    },
  },
  watch: {
    showFields() {
      if (
        this.searchObj.meta.showHistogram == true &&
        this.searchObj.meta.sqlMode == false
      ) {
        setTimeout(() => {
          if (this.searchResultRef) this.searchResultRef.reDrawChart();
        }, 100);
      }
      if (this.searchObj.config.splitterModel > 0) {
        this.searchObj.config.lastSplitterPosition =
          this.searchObj.config.splitterModel;
      }

      this.searchObj.config.splitterModel = this.searchObj.meta.showFields
        ? this.searchObj.config.lastSplitterPosition
        : 0;
    },
    showHistogram() {
      if (
        this.searchObj.meta.showHistogram &&
        !this.searchObj.shouldIgnoreWatcher
      ) {
        setTimeout(() => {
          if (this.searchResultRef) this.searchResultRef.reDrawChart();
        }, 100);

        if (this.searchObj.meta.histogramDirtyFlag == true) {
          this.searchObj.meta.histogramDirtyFlag = false;
          // this.handleRunQuery();
          this.getHistogramQueryData(this.searchObj.data.histogramQuery).then(
            (res: any) => {
              this.searchResultRef.reDrawChart();
            }
          );
        }
      }
      this.updateUrlQueryParams();
    },
    moveSplitter() {
      if (this.searchObj.meta.showFields == false) {
        this.searchObj.meta.showFields =
          this.searchObj.config.splitterModel > 0;
      }
    },
    // changeStream: {
    //   handler(stream, streamOld) {
    //     if (
    //       this.searchObj.data.stream.selectedStream.hasOwnProperty("value") &&
    //       this.searchObj.data.stream.selectedStream.value != ""
    //     ) {
    //       this.searchObj.data.tempFunctionContent = "";
    //       this.searchBarRef.resetFunctionContent();
    //       if (streamOld.value) this.searchObj.data.query = "";
    //       if (streamOld.value) this.setQuery(this.searchObj.meta.sqlMode);
    //       this.searchObj.loading = true;
    //       // setTimeout(() => {
    //       //   this.runQueryFn();
    //       // }, 500);
    //     }
    //   },
    //   immediate: false,
    // },
    updateSelectedColumns() {
      this.searchObj.meta.resultGrid.manualRemoveFields = true;
      setTimeout(() => {
        this.updateGridColumns();
      }, 50);
    },
    runQuery() {
      if (this.store.state.savedViewFlag == true) return;
      if (this.searchObj.runQuery == true) {
        this.runQueryFn();
      }
    },
    async fullSQLMode(newVal) {
      if (newVal) {
        await nextTick();
        this.setQuery(newVal);
        this.updateUrlQueryParams();
      } else {
        this.searchObj.meta.sqlMode = false;
        this.searchObj.data.query = "";
        this.searchObj.data.editorValue = "";
        if (
          this.searchObj.loading == false &&
          this.searchObj.shouldIgnoreWatcher == false &&
          this.store.state.zoConfig.query_on_stream_selection == false
        ) {
          this.searchObj.loading = true;
          this.getQueryData();
        }
      }
      // this.searchResultRef.reDrawChart();
    },
    refreshHistogram() {
      if (
        this.searchObj.meta.histogramDirtyFlag == true &&
        this.searchObj.meta.showHistogram == true
      ) {
        this.searchObj.meta.histogramDirtyFlag = false;
        this.handleRunQuery();
        this.refreshHistogramChart();
      }
    },
    redrawHistogram() {
      this.refreshHistogramChart();
    },
  },
});
</script>

<style lang="scss">
$navbarHeight: 64px;

.logPage {
  height: calc(100vh - $navbarHeight);
  min-height: calc(100vh - $navbarHeight) !important;

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

  .logsPageMainSection > .q-field__control-container {
    padding-top: 0px !important;
  }

  .logs-horizontal-splitter {
    border: 1px solid var(--q-color-grey-3);
    .q-splitter__panel {
      z-index: auto !important;
    }
    .q-splitter__before {
      overflow: visible !important;
    }
  }

  .thirdlevel {
    .field-list-collapse-btn {
      z-index: 9;
      position: absolute;
      top: 5px;
      font-size: 12px !important;
    }
  }

  .search-result-container {
    position: relative;
    width: 100%;
  }
}
</style>
