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

import http from "./http";

var index = {
  nameList: (org_identifier: string, type: string, schema: boolean) => {
    let url = `/api/${org_identifier}/streams`;

    if (type != "") {
      url += "?type=" + type;
    }

    if (schema) {
      url +=
        url.indexOf("?") > 0
          ? "&fetchSchema=" + schema
          : "?fetchSchema=" + schema;
    }
    return http().get(url);
  },
  schema: (org_identifier: string, stream_name: string, type: string) => {
    let url = `/api/${org_identifier}/${stream_name}/schema`;

    if (type != "") {
      url += "?type=" + type;
    }
    return http().get(url);
  },
  updateSettings: (
    org_identifier: string,
    stream_name: string,
    type: string,
    data: any
  ) => {
    let url = `/api/${org_identifier}/streams/${stream_name}/settings`;

    if (type != "") {
      url += "?type=" + type;
    }
    return http().put(url, data);
  },
};

export default index;
