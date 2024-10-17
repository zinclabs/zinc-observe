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

import type { App } from "vue";
import Logs from "./logs/Index.vue";
import AppMetrics from "./metrics/Index.vue";
import AppTraces from "./traces/Index.vue";

export default {
  install: (app: App, options?: any) => {
    app.component("zinc-logs", Logs);
    app.component("zinc-metrics", AppMetrics);
    app.component("zinc-traces", AppTraces);
  },
};
