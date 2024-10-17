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

import { reactive } from "vue";

const defaultObject = {
  loading: [],
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
    resultGrid: {
      wrapCells: false,
      rowsPerPage: 150,
      chartInterval: "1 second",
      chartKeyFormat: "HH:mm:ss",
      navigation: {
        currentRowIndex: 0,
      },
    },
  },
  data: {
    parsedQuery: {},
    errorMsg: "",
    errorCode: 0,
    additionalErrorMsg: "",
    stream: {
      logStream: "",
      sessionStream: "",
      sessionDataStream: "",
      selectedStreamFields: [],
    },
    resultGrid: {
      currentDateTime: new Date(),
      currentPage: 0,
      columns: <any>[],
      size: 150,
    },
    sessions: <any>[],
    streamResults: <any>[],
    histogram: <any>{},
    editorValue: "",
    datetime: {
      startTime: 0,
      endTime: 0,
      relativeTimePeriod: "15m",
      valueType: "relative",
    },
    selectedSession: {
      start_time: 0,
      end_time: 0,
      browser: "",
      os: "",
      ip: "",
      user_email: "",
      city: "",
      country: "",
      session_id: "",
      timestamp: 0,
      type: "",
      time_spent: 0,
    },
  },
};

let sessionState = reactive(Object.assign({}, defaultObject));

const useSession = () => {
  const resetSessionState = () => {
    // delete searchObj.data;
    sessionState = reactive(Object.assign({}, defaultObject));
  };

  return { sessionState, resetSessionState };
};

export default useSession;
