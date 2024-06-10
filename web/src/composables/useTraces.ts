// Copyright 2023 Zinc Labs Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

import { reactive } from "vue";
import { b64EncodeUnicode, useLocalTraceFilterField } from "@/utils/zincutils";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { copyToClipboard, useQuasar } from "quasar";

const defaultObject = {
  organizationIdetifier: "",
  runQuery: false,
  loading: false,

  config: {
    splitterModel: 20,
    lastSplitterPosition: 0,
    splitterLimit: [0, 40],
    refreshTimes: [
      [
        { label: "5 sec", value: 5 },
        { label: "1 min", value: 60 },
        { label: "1 hr", value: 3600 },
      ],
      [
        { label: "10 sec", value: 10 },
        { label: "5 min", value: 300 },
        { label: "2 hr", value: 7200 },
      ],
      [
        { label: "15 sec", value: 15 },
        { label: "15 min", value: 900 },
        { label: "1 day", value: 86400 },
      ],
      [
        { label: "30 sec", value: 30 },
        { label: "30 min", value: 1800 },
      ],
    ],
  },
  meta: {
    refreshInterval: 0,
    refreshIntervalLabel: "Off",
    showFields: true,
    showQuery: true,
    showHistogram: true,
    showDetailTab: false,
    showTraceDetails: false,
    sqlMode: false,
    filterType: "basic",
    resultGrid: {
      wrapCells: false,
      manualRemoveFields: false,
      rowsPerPage: 25,
      chartInterval: "1 second",
      chartKeyFormat: "HH:mm:ss",
      navigation: {
        currentRowIndex: 0,
      },
    },
    scrollInfo: {},
    serviceColors: {} as any,
  },
  data: {
    query: "",
    advanceFiltersQuery: "",
    parsedQuery: {},
    errorMsg: "",
    errorCode: 0,
    additionalErrorMsg: "",
    stream: {
      streamLists: [],
      selectedStream: { label: "", value: "" },
      selectedStreamFields: [],
      selectedFields: <string[]>[],
      filterField: "",
      addToFilter: "",
      functions: [],
      filters: [] as any[],
      fieldValues: {} as {
        [key: string | number]: {
          isLoading: boolean;
          values: { key: string; count: string }[];
          selectedValues: string[];
          size: number;
          isOpen: boolean;
          searchKeyword: string;
        };
      },
    },
    resultGrid: {
      currentDateTime: new Date(),
      currentPage: 0,
      columns: <any>[],
    },
    queryPayload: <any>{},
    transforms: <any>[],
    queryResults: <any>[],
    sortedQueryResults: <any>[],
    streamResults: <any>[],
    histogram: <any>{},
    editorValue: "",
    datetime: {
      startTime: 0,
      endTime: 0,
      relativeTimePeriod: "15m",
      type: "relative",
    },
    searchAround: {
      indexTimestamp: 0,
      size: <number>10,
      histogramHide: false,
    },
    traceDetails: {
      selectedTrace: null,
      traceId: "",
      spanList: [],
      loading: false,
      selectedSpanId: "" as String | null,
      expandedSpans: [] as String[],
      showSpanDetails: false,
      selectedLogStreams: [] as String[],
    },
  },
};

const searchObj = reactive(Object.assign({}, defaultObject));

const useTraces = () => {
  const store = useStore();
  const router = useRouter();
  const $q = useQuasar();

  const resetSearchObj = () => {
    // delete searchObj.data;
    searchObj.data.errorMsg = "No stream found in selected organization!";
    searchObj.data.stream.streamLists = [];
    searchObj.data.stream.selectedStream = { label: "", value: "" };
    searchObj.data.stream.selectedStreamFields = [];
    searchObj.data.queryResults = {};
    searchObj.data.sortedQueryResults = [];
    searchObj.data.histogram = {
      xData: [],
      yData: [],
      chartParams: {
        title: "",
        unparsed_x_data: [],
        timezone: "",
      },
    };
    searchObj.data.query = "";
    searchObj.data.editorValue = "";
    searchObj.meta.sqlMode = false;
    searchObj.runQuery = false;
  };
  const updatedLocalLogFilterField = (): void => {
    const identifier: string = searchObj.organizationIdetifier || "default";
    const selectedFields: any =
      useLocalTraceFilterField()?.value != null
        ? useLocalTraceFilterField()?.value
        : {};
    selectedFields[
      `${identifier}_${searchObj.data.stream.selectedStream.value}`
    ] = searchObj.data.stream.selectedFields;
    useLocalTraceFilterField(selectedFields);
  };

  function getUrlQueryParams(getShareLink: boolean = false) {
    const date = searchObj.data.datetime;
    const query: any = {};

    query["stream"] = searchObj.data.stream.selectedStream.value;

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

    if (router.currentRoute.value.query.span_id)
      query["span_id"] = router.currentRoute.value.query.span_id;

    return query;
  }

  const copyTracesUrl = (
    customTimeRange: { from: string; to: string } | null = null
  ) => {
    const queryParams = getUrlQueryParams(true);

    if (customTimeRange) {
      queryParams.from = customTimeRange.from;
      queryParams.to = customTimeRange.to;
    }

    const queryString = Object.entries(queryParams)
      .map(
        ([key, value]: any) =>
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

  return {
    searchObj,
    resetSearchObj,
    updatedLocalLogFilterField,
    getUrlQueryParams,
    copyTracesUrl,
  };
};

export default useTraces;
