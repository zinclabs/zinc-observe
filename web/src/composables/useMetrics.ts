// Copyright 2023 OpenObserve Inc.
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

import { cloneDeep } from "lodash-es";
import { reactive, ref } from "vue";

const defaultObject = {
  organizationIdentifier: "",
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
    totalMetricValues: 1000,
  },
  data: {
    errorMsg: "",
    errorCode: 0,
    additionalErrorMsg: "",
    metrics: {
      metricList: <any>[],
      selectedMetric: null as null | {
        type: string;
        name: string;
        value: string;
        help: string;
      },
      selectedMetricType: "",
    },
    queryResults: <any>[],
    streamResults: <any>[],
    histogram: <any>{},
    datetime: {
      startTime: "",
      endTime: "",
      relativeTimePeriod: "15m",
      type: "relative",
    },
    query: "",
  },
};

const searchObj = ref(cloneDeep(defaultObject));

const useMetrics = () => {
  const resetSearchObj = () => {
    // delete searchObj.data;
    searchObj.value = cloneDeep(defaultObject);
  };

  return { searchObj: searchObj.value, resetSearchObj };
};

export default useMetrics;
