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

const alerts = {

  list: (
    page_num: number,
    page_size: number,
    sort_by: string,
    desc: boolean,
    name: string,
    org_identifier: string
  ) => {
    console.log("this is working")
    return http().get(
      `/api/${org_identifier}/alerts?page_num=${page_num}&page_size=${page_size}&sort_by=${sort_by}&desc=${desc}&name=${name}`
    );
  },
  create: (
    org_identifier: string,
    stream_name: string,
    stream_type: string,
    data: any
  ) => {
    return http().post(
      `/api/${org_identifier}/${stream_name}/alerts?type=${stream_type}`,
      data
    );
  },
  update: (
    org_identifier: string,
    stream_name: string,
    stream_type: string,
    data: any
  ) => {
    console.log('is this gettging invoed')
    return http().put(
      `/api/${org_identifier}/${stream_name}/alerts/${encodeURIComponent(
        data.name
      )}?type=${stream_type}`,
      data
    );
  },
  get_with_name: (
    org_identifier: string,
    stream_name: string,
    alert_name: string
  ) => {
    return http().get(
      `/api/${org_identifier}/${stream_name}/alerts/${encodeURIComponent(
        alert_name
      )}`
    );
  },
  delete: (
    org_identifier: string,
    stream_name: string,
    alert_name: string,
    type: string
  ) => {
    let url = `/api/${org_identifier}/${stream_name}/alerts/${encodeURIComponent(
      alert_name
    )}`;
    if (type != "") {
      url += "?type=" + type;
    }
    return http().delete(url);
  },
  toggleState: (
    org_identifier: string,
    stream_name: string,
    alert_name: string,
    enable: boolean,
    stream_type: string
  ) => {
    const url = `/api/${org_identifier}/${stream_name}/alerts/${encodeURIComponent(
      alert_name
    )}/enable?value=${enable}&type=${stream_type}`;
    return http().put(url);
  },

  preview: (
    org_identifier: string,
    stream_name: string,
    alert_name: string,
    stream_type: string
  ) => {
    const url = `/api/${org_identifier}/${stream_name}/alerts/${encodeURIComponent(
      alert_name
    )}/preview?type=${stream_type}`;
    return http().get(url);
  },
};

export default alerts;
