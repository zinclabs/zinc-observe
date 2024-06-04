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
<template>
  <q-page class="tracePage" id="tracePage" style="min-height: auto">
    <div id="tracesSecondLevel">
      <search-bar
        data-test="logs-search-bar"
        ref="searchBarRef"
        :fieldValues="fieldValues"
        :isLoading="searchObj.loading"
        @searchdata="searchData"
        @onChangeTimezone="refreshTimezone"
        @shareLink="copyTracesUrl"
      />
      <div
        id="tracesThirdLevel"
        class="row scroll traces-search-result-container"
        style="width: 100%"
      >
        <!-- Note: Splitter max-height to be dynamically calculated with JS -->
        <q-splitter
          v-model="searchObj.config.splitterModel"
          :limits="searchObj.config.splitterLimit"
          style="width: 100%"
          @update:model-value="onSplitterUpdate"
        >
          <template #before v-if="searchObj.meta.showFields">
            <index-list
              ref="indexListRef"
              :field-list="searchObj.data.stream.selectedStreamFields"
              data-test="logs-search-index-list"
              :key="searchObj.data.stream.streamLists"
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
            <div
              class="full-height flex justify-center items-center"
              v-if="searchObj.loading == true"
            >
              <div class="q-pb-lg">
                <q-spinner-hourglass
                  color="primary"
                  size="40px"
                  style="margin: 0 auto; display: block"
                />
                <span class="text-center">
                  Hold on tight, we're fetching your traces.
                </span>
              </div>
            </div>
            <div
              v-if="
                searchObj.data.errorMsg !== '' && searchObj.loading == false
              "
            >
              <h5 class="text-center">
                <div
                  data-test="logs-search-result-not-found-text"
                  v-if="
                    searchObj.data.stream.streamLists.length &&
                    searchObj.data.errorCode == 0
                  "
                >
                  Result not found.
                </div>
                <SanitizedHtmlRenderer
                  data-test="logs-search-error-message"
                  :html-content="searchObj.data.errorMsg"
                />

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
                <br />
                <q-item-label>{{
                  searchObj.data.additionalErrorMsg
                }}</q-item-label>
              </h5>
            </div>
            <div v-else-if="searchObj.data.stream.selectedStream.label == ''">
              <h5
                data-test="logs-search-no-stream-selected-text"
                class="text-center"
              >
                No stream selected.
              </h5>
            </div>
            <div
              v-else-if="
                searchObj.data.queryResults.hasOwnProperty('total') &&
                searchObj.data.queryResults.hits.length == 0 &&
                searchObj.loading == false
              "
            >
              <h5 class="text-center">No result found.</h5>
            </div>
            <div
              data-test="logs-search-search-result"
              v-show="
                searchObj.data.queryResults.hasOwnProperty('total') &&
                !!searchObj.data.queryResults.hits.length
              "
            >
              <search-result
                ref="searchResultRef"
                @update:datetime="setHistogramDate"
                @update:scroll="getMoreData"
                @get:traceDetails="getTraceDetails"
                @shareLink="copyTracesUrl"
              />
            </div>
          </template>
        </q-splitter>
      </div>
    </div>
  </q-page>
</template>

<script lang="ts">
// @ts-nocheck
import {
  defineComponent,
  ref,
  onDeactivated,
  onActivated,
  onBeforeMount,
  nextTick,
  defineAsyncComponent,
} from "vue";
import { useQuasar, date, copyToClipboard } from "quasar";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";

import useTraces from "@/composables/useTraces";

import streamService from "@/services/stream";
import searchService from "@/services/search";
import TransformService from "@/services/jstransform";
import {
  b64EncodeUnicode,
  verifyOrganizationStatus,
  b64DecodeUnicode,
  formatTimeWithSuffix,
  timestampToTimezoneDate,
} from "@/utils/zincutils";
import segment from "@/services/segment_analytics";
import config from "@/aws-exports";
import { logsErrorMessage } from "@/utils/common";
import useNotifications from "@/composables/useNotifications";
import { getConsumableRelativeTime } from "@/utils/date";
import { cloneDeep } from "lodash-es";
import { computed } from "vue";
import useStreams from "@/composables/useStreams";
import SanitizedHtmlRenderer from "@/components/SanitizedHtmlRenderer.vue";

export default defineComponent({
  name: "PageSearch",
  components: {
    SearchBar: defineAsyncComponent(() => import("./SearchBar.vue")),
    IndexList: defineAsyncComponent(() => import("./IndexList.vue")),
    SearchResult: defineAsyncComponent(() => import("./SearchResult.vue")),
    SanitizedHtmlRenderer,
  },
  methods: {
    async setHistogramDate(date: any) {
      this.searchBarRef.dateTimeRef.setCustomDate("absolute", date);
      await nextTick();
      await nextTick();
      await nextTick();

      this.searchData();
    },
    searchData() {
      if (
        !(
          this.searchObj.data.stream.streamLists.length &&
          this.searchObj.data.stream.selectedStream?.label
        )
      ) {
        return;
      }
      if (this.searchObj.loading == false) {
        this.searchObj.loading = true;
        this.searchObj.runQuery = true;
        this.indexListRef.filterExpandedFieldValues();
      }

      if (config.isCloud == "true") {
        segment.track("Button Click", {
          button: "Search Data",
          user_org: this.store.state.selectedOrganization.identifier,
          user_id: this.store.state.userInfo.email,
          stream_name: this.searchObj.data.stream.selectedStream.value,
          show_query: this.searchObj.meta.showQuery,
          show_histogram: this.searchObj.meta.showHistogram,
          sqlMode: this.searchObj.meta.sqlMode,
          showFields: this.searchObj.meta.showFields,
          page: "Search Logs",
        });
      }
    },
    getMoreData() {
      if (this.searchObj.meta.refreshInterval == 0) {
        this.getQueryData();

        if (config.isCloud == "true") {
          segment.track("Button Click", {
            button: "Get More Data",
            user_org: this.store.state.selectedOrganization.identifier,
            user_id: this.store.state.userInfo.email,
            stream_name: this.searchObj.data.stream.selectedStream.value,
            page: "Search Logs",
          });
        }
      }
    },
  },
  setup() {
    const store = useStore();
    const router = useRouter();
    const $q = useQuasar();
    const { t } = useI18n();
    const { searchObj, resetSearchObj } = useTraces();
    let refreshIntervalID = 0;
    const searchResultRef = ref(null);
    const searchBarRef = ref(null);
    let parser: any;
    const fieldValues = ref({});
    const { showErrorNotification } = useNotifications();
    const serviceColorIndex = ref(0);
    const colors = ref(["#b7885e", "#1ab8be", "#ffcb99", "#f89570", "#839ae2"]);
    const indexListRef = ref(null);
    const { getStreams, getStream } = useStreams();

    searchObj.organizationIdetifier =
      store.state.selectedOrganization.identifier;

    const selectedStreamName = computed(
      () => searchObj.data.stream.selectedStream.value
    );

    const importSqlParser = async () => {
      const useSqlParser: any = await import("@/composables/useParser");
      const { sqlParser }: any = useSqlParser.default();
      parser = await sqlParser();
    };

    function getQueryTransform() {
      try {
        searchObj.data.stream.functions = [];
        TransformService.list(
          1,
          100000,
          "name",
          false,
          "",
          store.state.selectedOrganization.identifier
        )
          .then((res) => {
            res.data.list.map((data: any) => {
              let args: any = [];
              for (let i = 0; i < parseInt(data.num_args); i++) {
                args.push("'${1:value}'");
              }

              let itemObj = {
                name: data.name,
                args: "(" + args.join(",") + ")",
              };
              if (!data.stream_name) {
                searchObj.data.stream.functions.push(itemObj);
              }
            });
          })
          .catch((err) => console.log(err));

        return;
      } catch (e) {
        searchObj.loading = false;
        showErrorNotification("Error while getting functions");
      }
    }

    async function getStreamList() {
      try {
        getStreams("traces", false)
          .then(async (res) => {
            searchObj.data.streamResults = res;

            if (res.list.length > 0) {
              if (config.isCloud == "true") {
                getQueryTransform();
              }

              //extract stream data from response
              loadStreamLists();
            } else {
              searchObj.loading = false;
              searchObj.data.errorMsg =
                "No stream found in selected organization!";
              searchObj.data.stream.streamLists = [];
              searchObj.data.stream.selectedStream = { label: "", value: "" };
              searchObj.data.stream.selectedStreamFields = [];
              searchObj.data.queryResults = {};
              searchObj.data.sortedQueryResults = [];
              searchObj.data.histogram = {
                layout: {},
                data: [],
              };
            }

            await extractFields();

            if (
              searchObj.data.editorValue &&
              searchObj.data.stream.selectedStreamFields.length
            )
              nextTick(() => {
                restoreFilters(searchObj.data.editorValue);
              });
          })
          .catch((e) => {
            searchObj.loading = false;
            $q.notify({
              type: "negative",
              message:
                "Error while pulling index for selected organization" +
                e.message,
              timeout: 2000,
            });
          });
      } catch (e) {
        searchObj.loading = false;
        console.log(e);
        showErrorNotification("Error while getting streams");
      }
    }

    function loadStreamLists() {
      try {
        const queryParams = router.currentRoute.value.query;
        searchObj.data.stream.streamLists = [];
        if (searchObj.data.streamResults.list.length > 0) {
          let lastUpdatedStreamTime = 0;
          let selectedStreamItemObj = {};
          searchObj.data.streamResults.list.map((item: any) => {
            let itemObj = {
              label: item.name,
              value: item.name,
            };
            searchObj.data.stream.streamLists.push(itemObj);

            if (queryParams.stream === item.name) {
              selectedStreamItemObj = itemObj;
            } else if (
              !queryParams.stream &&
              item.stats.doc_time_max >= lastUpdatedStreamTime
            ) {
              lastUpdatedStreamTime = item.stats.doc_time_max;
              selectedStreamItemObj = itemObj;
            }
          });

          if (selectedStreamItemObj.label != undefined) {
            searchObj.data.stream.selectedStream = selectedStreamItemObj;
          } else {
            searchObj.data.stream.selectedStream = {};
            searchObj.loading = false;
            searchObj.data.queryResults = {};
            searchObj.data.sortedQueryResults = [];
            searchObj.data.stream.selectedStreamFields = [];
            searchObj.data.histogram = {
              layout: {},
              data: [],
            };
          }
        } else {
          searchObj.loading = false;
        }
      } catch (e) {
        searchObj.loading = false;
        showErrorNotification("Error while loading streams");
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
          if (searchObj.data.resultGrid.currentPage > 0) {
            endTimeStamp = searchObj.data.resultGrid.currentDateTime;
          } else {
            searchObj.data.resultGrid.currentDateTime = endTimeStamp;
          }

          const startTimeStamp = date.subtractFromDate(
            endTimeStamp,
            JSON.parse(subtractObject)
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
                searchObj.data.datetime.absolute.startTime
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
                searchObj.data.datetime.absolute.endTime
            );
          }
          const rVal = {
            start_time: start,
            end_time: end,
          };
          return rVal;
        }
      } catch (e) {
        searchObj.loading = false;
        console.log("Error while getting consumable date time");
      }
    }

    const getDefaultRequest = () => {
      return {
        query: {
          sql: `select min(${store.state.zoConfig.timestamp_column}) as zo_sql_timestamp, min(start_time/1000) as trace_start_time, max(end_time/1000) as trace_end_time, min(service_name) as service_name, min(operation_name) as operation_name, count(trace_id) as spans, SUM(CASE WHEN span_status='ERROR' THEN 1 ELSE 0 END) as errors, max(duration) as duration, trace_id [QUERY_FUNCTIONS] from "[INDEX_NAME]" [WHERE_CLAUSE] group by trace_id order by zo_sql_timestamp DESC`,
          start_time: (new Date().getTime() - 900000) * 1000,
          end_time: new Date().getTime() * 1000,
          from: 0,
          size: 0,
        },
        encoding: "base64",
      };
    };

    function buildSearch() {
      try {
        let query = searchObj.data.editorValue;
        var req = getDefaultRequest();
        req.query.from =
          searchObj.data.resultGrid.currentPage *
          searchObj.meta.resultGrid.rowsPerPage;
        req.query.size = parseInt(searchObj.meta.resultGrid.rowsPerPage, 10);

        let timestamps: any =
          searchObj.data.datetime.type === "relative"
            ? getConsumableRelativeTime(
                searchObj.data.datetime.relativeTimePeriod
              )
            : cloneDeep(searchObj.data.datetime);

        req.query.start_time = timestamps.startTime;
        req.query.end_time = timestamps.endTime;

        req.query["sql_mode"] = "full";

        let parseQuery = query.split("|");
        let queryFunctions = "";
        let whereClause = "";

        if (parseQuery.length > 1) {
          queryFunctions = "," + parseQuery[0].trim();
          whereClause = parseQuery[1].trim();
        } else {
          whereClause = parseQuery[0].trim();
        }

        if (whereClause.trim() != "") {
          whereClause = whereClause
            .replace(/=(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " =")
            .replace(/>(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " >")
            .replace(/<(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " <");

          whereClause = whereClause
            .replace(/!=(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " !=")
            .replace(/! =(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " !=")
            .replace(/< =(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " <=")
            .replace(/> =(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " >=");

          req.query.sql = req.query.sql.replace(
            "[WHERE_CLAUSE]",
            " WHERE " + whereClause
          );
        } else {
          req.query.sql = req.query.sql.replace("[WHERE_CLAUSE]", "");
        }

        req.query.sql = req.query.sql.replace(
          "[QUERY_FUNCTIONS]",
          queryFunctions
        );

        req.query.sql = req.query.sql.replace(
          "[INDEX_NAME]",
          searchObj.data.stream.selectedStream.value
        );
        // const parsedSQL = parser.astify(req.query.sql);
        // const unparsedSQL = parser.sqlify(parsedSQL);
        // console.log(unparsedSQL);

        req.query.sql = b64EncodeUnicode(req.query.sql);

        const queryParams = getUrlQueryParams();
        router.push({ query: queryParams });
        return req;
      } catch (e) {
        searchObj.loading = false;
        showErrorNotification("Invalid SQL Syntax");
      }
    }

    const openTraceDetails = () => {
      searchObj.loading = true;
      const queryReq = buildSearch();

      let filter = searchObj.data.editorValue;

      if (filter?.length)
        filter += ` and trace_id='${router.currentRoute.value.query.trace_id}'`;
      else filter += `trace_id='${router.currentRoute.value.query.trace_id}'`;

      searchService
        .get_traces({
          org_identifier: searchObj.organizationIdetifier,
          start_time: queryReq.query.start_time,
          end_time: queryReq.query.end_time,
          filter: filter || "",
          size: 1,
          from: 0,
          stream_name: selectedStreamName.value,
        })
        .then(async (res) => {
          const trace = getTracesMetaData(res.data.hits)[0];
          if (!trace) {
            showTraceDetailsError();
            return;
          }
          searchObj.data.traceDetails.selectedTrace = trace;
          getTraceDetails();
        })
        .catch(() => {
          showTraceDetailsError();
        })
        .finally(() => {
          searchObj.loading = false;
        });
    };

    const showTraceDetailsError = () => {
      showErrorNotification(
        `Trace ${router.currentRoute.value.query.trace_id} not found`
      );
      const query = cloneDeep(router.currentRoute.value.query);
      delete query.trace_id;
      router.push({
        name: "traces",
        query: {
          ...query,
        },
      });
      return;
    };

    const buildTraceSearchQuery = (trace: string) => {
      const req = getDefaultRequest();
      req.query.from = 0;
      req.query.size = 1000;
      req.query.start_time = trace.trace_start_time - 30000000;
      req.query.end_time = trace.trace_end_time + 30000000;

      req.query.sql = b64EncodeUnicode(
        `SELECT * FROM ${selectedStreamName.value} WHERE trace_id = '${trace.trace_id}' ORDER BY start_time`
      );

      return req;
    };

    const getTraceDetails = () => {
      searchObj.meta.showTraceDetails = true;
      searchObj.data.traceDetails.loading = true;
      searchObj.data.traceDetails.spanList = [];
      const req = buildTraceSearchQuery(
        searchObj.data.traceDetails.selectedTrace
      );

      delete req.aggs;

      searchService
        .search({
          org_identifier: searchObj.organizationIdetifier,
          query: req,
          page_type: "traces",
        })
        .then((res) => {
          searchObj.data.traceDetails.spanList = res.data?.hits || [];
        })
        .finally(() => {
          searchObj.data.traceDetails.loading = false;
        });
    };

    const updateFieldValues = (data) => {
      const excludedFields = [store.state.zoConfig.timestamp_column];
      data.forEach((item) => {
        // Create set for each field values and add values to corresponding set
        Object.keys(item).forEach((key) => {
          if (excludedFields.includes(key)) {
            return;
          }

          if (fieldValues.value[key] == undefined) {
            fieldValues.value[key] = new Set();
          }

          if (!fieldValues.value[key].has(item[key])) {
            fieldValues.value[key].add(item[key]);
          }
        });
      });
    };

    async function getQueryData() {
      try {
        if (searchObj.data.stream.selectedStream.value == "") {
          return false;
        }
        searchObj.data.errorMsg = "";
        if (searchObj.data.resultGrid.currentPage == 0) {
          searchObj.loading = true;
          // searchObj.data.stream.selectedFields = [];
          // searchObj.data.stream.addToFilter = "";
          searchObj.data.queryResults = {};
          // searchObj.data.resultGrid.columns = [];
          searchObj.data.sortedQueryResults = [];
          // searchObj.data.streamResults = [];
          searchObj.data.histogram = {
            layout: {},
            data: [],
          };
          // searchObj.data.editorValue = "";
        }
        // dismiss = Notify();
        let queryReq;

        if (!searchObj.data.resultGrid.currentPage) {
          queryReq = buildSearch();
          searchObj.data.queryPayload = queryReq;
        } else {
          queryReq = searchObj.data.queryPayload;
        }

        if (queryReq == null) {
          // dismiss();
          return false;
        }

        searchObj.data.errorCode = 0;
        queryReq.query.from =
          searchObj.data.resultGrid.currentPage *
          searchObj.meta.resultGrid.rowsPerPage;

        let dismiss = null;
        if (searchObj.data.resultGrid.currentPage) {
          dismiss = $q.notify({
            type: "positive",
            message: "Fetching more traces...",
            actions: [
              {
                icon: "cancel",
                color: "white",
                handler: () => {
                  /* ... */
                },
              },
            ],
          });
        }

        const durationFilter = indexListRef.value.duration.input;

        let filter = searchObj.data.editorValue.trim();

        let duration = "";
        if (durationFilter.max) {
          duration += ` duration >= ${
            durationFilter.min * 1000
          } AND duration <= ${durationFilter.max * 1000}`;

          filter = filter
            ? searchObj.data.editorValue + " AND" + duration
            : duration;
        }

        searchService
          .get_traces({
            org_identifier: searchObj.organizationIdetifier,
            start_time: queryReq.query.start_time,
            end_time: queryReq.query.end_time,
            filter: filter || "",
            size: queryReq.query.size,
            from: queryReq.query.from,
            stream_name: selectedStreamName.value,
          })
          .then(async (res) => {
            searchObj.loading = false;
            const formattedHits = getTracesMetaData(res.data.hits);
            if (res.data.from > 0) {
              searchObj.data.queryResults.from = res.data.from;
              searchObj.data.queryResults.hits.push(...formattedHits);
            } else {
              searchObj.data.queryResults = {
                ...res.data,
                hits: formattedHits,
              };
            }

            updateFieldValues(res.data.hits);

            generateHistogramData();

            //update grid columns
            updateGridColumns();

            if (router.currentRoute.value.query.trace_id) openTraceDetails();

            // dismiss();
          })
          .catch((err) => {
            searchObj.loading = false;
            // dismiss();
            if (err.response != undefined) {
              searchObj.data.errorMsg = err.response.data.error;
            } else {
              searchObj.data.errorMsg = err.message;
            }

            const customMessage = logsErrorMessage(err.response.data.code);
            searchObj.data.errorCode = err.response.data.code;
            if (customMessage != "") {
              searchObj.data.errorMsg = t(customMessage);
            }

            // $q.notify({
            //   message: searchObj.data.errorMsg,
            //   color: "negative",
            // });
          })
          .finally(() => {
            if (dismiss) dismiss();
          });
      } catch (e) {
        console.log(e?.message);
        searchObj.loading = false;
        showErrorNotification("Search request failed");
      }
    }

    const getTracesMetaData = (traces) => {
      if (!traces.length) return [];

      return traces.map((trace) => {
        const _trace = {
          trace_id: trace.trace_id,
          trace_start_time: Math.round(trace.start_time / 1000),
          trace_end_time: Math.round(trace.end_time / 1000),
          service_name: trace.first_event.service_name,
          operation_name: trace.first_event.operation_name,
          spans: trace.spans[0],
          errors: trace.spans[1],
          duration: trace.duration,
          services: {},
          zo_sql_timestamp: new Date(trace.start_time / 1000).getTime(),
        };
        trace.service_name.forEach((service) => {
          if (!searchObj.meta.serviceColors[service.service_name]) {
            if (serviceColorIndex.value >= colors.value.length)
              generateNewColor();

            searchObj.meta.serviceColors[service.service_name] =
              colors.value[serviceColorIndex.value];

            serviceColorIndex.value++;
          }
          _trace.services[service.service_name] = service.count;
        });
        return _trace;
      });
    };

    function generateNewColor() {
      // Generate a color in HSL format
      const hue = colors.value.length * (360 / 50);
      const lightness = 50 + (colors.value.length % 2) * 15;
      colors.value.push(`hsl(${hue}, 100%, ${lightness}%)`);
      return colors;
    }

    async function extractFields() {
      try {
        searchObj.data.stream.selectedStreamFields = [];
        if (searchObj.data.streamResults.list.length > 0) {
          const schema = [];
          const ignoreFields = [store.state.zoConfig.timestamp_column];
          let ftsKeys;

          const stream = await getStream(
            searchObj.data.stream.selectedStream.value,
            "traces",
            true
          );

          schema.push(...stream.schema);
          ftsKeys = new Set([...stream.settings.full_text_search_keys]);

          const idFields = {
            trace_id: 1,
            span_id: 1,
            reference_parent_span_id: 1,
            reference_parent_trace_id: 1,
            start_time: 1,
            end_time: 1,
          };

          const importantFields = {
            duration: 1,
            service_name: 1,
            operation_name: 1,
            span_status: 1,
            trace_id: 1,
            span_id: 1,
            reference_parent_span_id: 1,
            reference_parent_trace_id: 1,
            start_time: 1,
          };

          // Ignoring timestamp as start time is present
          let fields: any = {};
          Object.keys(importantFields).forEach((rowName) => {
            if (fields[rowName] == undefined) {
              fields[rowName] = {};
              searchObj.data.stream.selectedStreamFields.push({
                name: rowName,
                ftsKey: ftsKeys.has(rowName),
                showValues: !idFields[rowName],
              });
            }
          });

          schema.forEach((row: any) => {
            // let keys = deepKeys(row);
            // for (let i in row) {
            if (
              !importantFields[row.name] &&
              !ignoreFields.includes(row.name)
            ) {
              if (fields[row.name] == undefined) {
                fields[row.name] = {};
                searchObj.data.stream.selectedStreamFields.push({
                  name: row.name,
                  ftsKey: ftsKeys.has(row.name),
                  showValues: !idFields[row.name],
                });
              }
            }
          });
        }
      } catch (e) {
        searchObj.loading = false;
        console.log("Error while extracting fields", e);
      }
    }

    function updateGridColumns() {
      try {
        searchObj.data.resultGrid.columns = [];

        searchObj.data.stream.selectedFields = [];

        searchObj.meta.resultGrid.manualRemoveFields = false;

        searchObj.data.resultGrid.columns.push({
          name: "@timestamp",
          field: (row: any) =>
            timestampToTimezoneDate(
              row["trace_start_time"],
              store.state.timezone,
              "yyyy-MM-dd HH:mm:ss.SSS"
            ),
          prop: (row: any) =>
            timestampToTimezoneDate(
              row["trace_start_time"],
              store.state.timezone,
              "yyyy-MM-dd HH:mm:ss.SSS"
            ),
          label: "Start Time",
          align: "left",
          sortable: true,
        });

        searchObj.data.resultGrid.columns.push({
          name: "operation_name",
          field: (row: any) => row.operation_name,
          prop: (row: any) => row.operation_name,
          label: "Operation",
          align: "left",
          sortable: true,
        });

        searchObj.data.resultGrid.columns.push({
          name: "service_name",
          field: (row: any) => row.service_name,
          prop: (row: any) => row.service_name,
          label: "Service",
          align: "left",
          sortable: true,
        });

        searchObj.data.resultGrid.columns.push({
          name: "duration",
          field: (row: any) => row.duration,
          prop: (row: any) => row.duration,
          label: "Duration",
          align: "left",
          sortable: true,
          format: (val) => formatTimeWithSuffix(val),
        });

        searchObj.loading = false;
      } catch (e) {
        searchObj.loading = false;
        console.log("Error while updating grid columns");
      }
    }

    function generateHistogramData() {
      const unparsed_x_data: any[] = [];
      const xData: string[] = [];
      const yData: number[] = [];

      var trace1 = {
        x: xData,
        y: yData,
        name: "Trace",
        type: "scatter",
        mode: "markers",
        hovertemplate: "%{x} <br> %{y}", // hovertemplate for custom tooltip
      };

      var data = [trace1];

      var layout = {
        title: {
          text: "",
          font: {
            size: 12,
            color: store.state.theme === "dark" ? "#fff" : "#181a1b",
          },
        },
        margin: {
          l: 50,
          r: 50,
          t: 22,
          b: 50,
        },
        font: {
          size: 12,
          color: store.state.theme === "dark" ? "#fff" : "#181a1b",
        },
        xaxis: { type: "date" },
        yaxis: { ticksuffix: "ms" },
        scattergap: 0.7,
        height: 150,
        paper_bgcolor: store.state.theme === "dark" ? "#181a1b" : "#fff",
        plot_bgcolor: store.state.theme === "dark" ? "#181a1b" : "#fff",
        autosize: true,
      };

      if (searchObj.data.queryResults.hits) {
        searchObj.data.queryResults.hits.forEach(
          (bucket: {
            zo_sql_timestamp: string | number | Date;
            duration: number | Date;
          }) => {
            unparsed_x_data.push(bucket.zo_sql_timestamp);
            let histDate = new Date(Math.floor(bucket.zo_sql_timestamp / 1000));
            xData.push(Math.floor(histDate.getTime()));
            yData.push(Number((bucket.duration / 1000).toFixed(2)));
          }
        );
      }

      // const totalRecords =
      //   (searchObj.data.resultGrid.currentPage + 1) *
      //     searchObj.meta.resultGrid.rowsPerPage <
      //   searchObj.data.queryResults.hits.length
      //     ? (searchObj.data.resultGrid.currentPage + 1) *
      //       searchObj.meta.resultGrid.rowsPerPage
      //     : searchObj.data.queryResults.hits.length;

      // layout.title.text =
      //   "Showing " +
      //   (searchObj.data.queryResults.from == 0
      //     ? searchObj.data.queryResults.size
      //     : totalRecords) +
      //   " out of " +
      //   searchObj.data.queryResults.total.toLocaleString() +
      //   " hits in " +
      //   searchObj.data.queryResults.took +
      //   " ms. (Scan Size: " +
      //   searchObj.data.queryResults.scan_size +
      //   "MB)";

      searchObj.data.histogram = {
        data: data,
        layout: layout,
      };

      if (
        searchObj.meta.showHistogram == true &&
        searchResultRef.value?.reDrawChart
      ) {
        searchResultRef.value.reDrawChart();
      }
    }

    async function loadPageData() {
      searchObj.loading = true;

      searchObj.data.resultGrid.currentPage = 0;

      resetSearchObj();
      searchObj.organizationIdetifier =
        store.state.selectedOrganization.identifier;

      //get stream list
      await getStreamList();
    }

    function refreshStreamData() {
      // searchObj.loading = true;
      // this.searchObj.data.resultGrid.currentPage = 0;
      // resetSearchObj();
      // searchObj.organizationIdetifier =
      //   store.state.selectedOrganization.identifier;
      // //get stream list
      // getStreamList();
    }

    onBeforeMount(async () => {
      await importSqlParser();
      if (searchObj.loading == false) {
        // eslint-disable-next-line no-prototype-builtins
        await loadPageData();
        restoreUrlQueryParams();
      }
    });

    onDeactivated(() => {
      clearInterval(refreshIntervalID);
    });

    onActivated(() => {
      if (
        searchObj.organizationIdetifier !=
        store.state.selectedOrganization.identifier
      ) {
        loadPageData();
      }

      if (
        searchObj.meta.showHistogram == true &&
        router.currentRoute.value.path.indexOf("/traces") > -1
      ) {
        setTimeout(() => {
          if (searchResultRef.value) searchResultRef.value.reDrawChart();
        }, 1500);
      }
    });

    const runQueryFn = () => {
      searchObj.data.resultGrid.currentPage = 0;
      searchObj.runQuery = false;
      getQueryData();
    };

    function restoreUrlQueryParams() {
      const queryParams = router.currentRoute.value.query;

      const date = {
        startTime: queryParams.from,
        endTime: queryParams.to,
        relativeTimePeriod: queryParams.period || null,
        type: queryParams.period ? "relative" : "absolute",
      };

      if (
        date &&
        ((date.startTime && date.endTime) || date.relativeTimePeriod)
      ) {
        searchObj.data.datetime = date;
      }

      if (queryParams.query) {
        searchObj.data.editorValue = b64DecodeUnicode(queryParams.query);
      }

      if (queryParams.filter_type) {
        searchObj.meta.filterType = queryParams.filter_type;
      }

      if (
        queryParams.stream &&
        searchObj.data.stream.selectedStream.value !== queryParams.stream
      ) {
        searchObj.data.stream.selectedStream = {
          label: queryParams.stream,
          value: queryParams.stream,
        };
      }
    }

    const copyTracesUrl = (customTimeRange = null) => {
      const queryParams = getUrlQueryParams(true);

      if (customTimeRange) {
        queryParams.from = customTimeRange.from;
        queryParams.to = customTimeRange.to;
      }

      const queryString = Object.entries(queryParams)
        .map(
          ([key, value]) =>
            `${encodeURIComponent(key)}=${encodeURIComponent(value)}`
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

    function getUrlQueryParams(getShareLink: false) {
      const date = searchObj.data.datetime;
      const query = {};

      query["stream"] = selectedStreamName.value;

      if (date.type == "relative" && !getShareLink) {
        query["period"] = date.relativeTimePeriod;
      } else {
        query["from"] = date.startTime;
        query["to"] = date.endTime;
      }

      query["query"] = b64EncodeUnicode(searchObj.data.editorValue);

      query["filter_type"] = searchObj.meta.filterType;

      query["org_identifier"] = store.state.selectedOrganization.identifier;

      query["trace_id"] = router.currentRoute.value.query.trace_id;

      return query;
    }

    const onSplitterUpdate = () => {
      window.dispatchEvent(new Event("resize"));
    };

    const refreshTimezone = () => {
      updateGridColumns();
      generateHistogramData();
      searchResultRef.value.reDrawChart();
    };

    const restoreFiltersFromQuery = (node: any) => {
      if (!node) return;
      if (node.type === "binary_expr") {
        if (node.left.column) {
          let values = [];
          if (node.operator === "IN") {
            values = node.right.value.map(
              (_value: { value: string }) => _value.value
            );
          }
          searchObj.data.stream.fieldValues[node.left.column].selectedValues =
            values;
        }
      }

      // Recurse through AND/OR expressions
      if (
        node.type === "binary_expr" &&
        (node.operator === "AND" || node.operator === "OR")
      ) {
        restoreFiltersFromQuery(node.left);
        restoreFiltersFromQuery(node.right);
      }
    };

    const restoreFilters = (query: string) => {
      // const filters = searchObj.data.stream.filters;

      const defaultQuery = `SELECT * FROM '${selectedStreamName.value}' WHERE `;

      const parsedQuery = parser.astify(defaultQuery + query);

      restoreFiltersFromQuery(parsedQuery.where);
    };

    return {
      store,
      router,
      parser,
      searchObj,
      searchBarRef,
      loadPageData,
      getQueryData,
      searchResultRef,
      refreshStreamData,
      updateGridColumns,
      getConsumableDateTime,
      runQueryFn,
      getTraceDetails,
      verifyOrganizationStatus,
      fieldValues,
      onSplitterUpdate,
      refreshTimezone,
      indexListRef,
      copyTracesUrl,
      extractFields,
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
    changeStream() {
      return this.searchObj.data.stream.selectedStream;
    },
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
        this.searchObj.meta.showHistogram == true &&
        this.searchObj.meta.sqlMode == false
      ) {
        setTimeout(() => {
          if (this.searchResultRef) this.searchResultRef.reDrawChart();
        }, 100);
      }
    },
    moveSplitter() {
      if (this.searchObj.meta.showFields == false) {
        this.searchObj.meta.showFields =
          this.searchObj.config.splitterModel > 0;
      }
    },
    changeStream: {
      handler(stream, oldStream) {
        if (stream.value === oldStream.value) return;
        if (this.searchObj.data.stream.selectedStream.hasOwnProperty("value")) {
          if (oldStream.value) {
            this.searchObj.data.query = "";
            this.searchObj.data.advanceFiltersQuery = "";
          }
          setTimeout(() => {
            this.runQueryFn();
            this.extractFields();
          }, 500);
        }
      },
      immediate: false,
    },
    updateSelectedColumns() {
      this.searchObj.meta.resultGrid.manualRemoveFields = true;
      setTimeout(() => {
        this.updateGridColumns();
      }, 300);
    },
    runQuery() {
      if (this.searchObj.runQuery == true) {
        this.runQueryFn();
      }
    },
  },
});
</script>

<style lang="scss" scoped>
.traces-search-result-container {
  height: calc(100vh - 165px) !important;
}
</style>
<style lang="scss">
.tracePage {
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
  #tracesSearchGridComponent:hover::-webkit-scrollbar {
    height: 13px;
    width: 13px;
  }

  .index-table ::-webkit-scrollbar-track,
  #tracesSearchGridComponent::-webkit-scrollbar-track {
    -webkit-box-shadow: inset 0 0 6px rgba(0, 0, 0, 0.3);
    border-radius: 10px;
  }

  .index-table ::-webkit-scrollbar-thumb,
  #tracesSearchGridComponent::-webkit-scrollbar-thumb {
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
}
</style>
