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

import http from "./http";

// POST /{org_id}/reports  -- Create Report
// GET /{org_id}/reports/{name} -- Get One Report
// GET /{org_id}/reports -- List of Reports
// PUT /{org_id}/reports/{name} -- Update Report
// DELETE /{org_id}/reports/{name} -- Delete Report
// PUT /{org_id}/reports/{name}/enable?value=<true|false> -- Enable/disable
// PUT /{org_id}/reports/{name}/trigger -- trigger report immediately

const reports = {
  list: (
    org_identifier: string = "",
    folder_id: string = "",
    dashboard_id: string = "",
    cache: boolean = false,
  ) => {
    let query = "";

    const params = [];
    if (folder_id) params.push(`folder_id=${folder_id}`);
    if (dashboard_id) params.push(`dashboard_id=${dashboard_id}`);
    if (cache) params.push(`cache=${cache}`);
    query = params.join("&");

    return http().get(`/api/${org_identifier}/reports?${query}`);
  },
  getReport: (org_identifier: string, reportName: string) => {
    return http().get(
      `/api/${org_identifier}/reports/${encodeURIComponent(reportName)}`,
    );
  },
  createReport: (org_identifier: string, payload: any) => {
    return http().post(`/api/${org_identifier}/reports`, payload);
  },
  updateReport: (org_identifier: string, payload: any) => {
    return http().put(
      `/api/${org_identifier}/reports/${encodeURIComponent(payload.name)}`,
      payload,
    );
  },
  deleteReport: (org_identifier: string, reportName: string) => {
    return http().delete(
      `/api/${org_identifier}/reports/${encodeURIComponent(reportName)}`,
    );
  },
  triggerReport: (org_identifier: string, reportName: string) => {
    return http().put(
      `/api/${org_identifier}/reports/${encodeURIComponent(reportName)}/trigger`,
    );
  },
  toggleReportState: (
    org_identifier: string,
    reportName: string,
    state: boolean,
  ) => {
    return http().put(
      `/api/${org_identifier}/reports/${encodeURIComponent(reportName)}/enable?value=${state}`,
    );
  },
};

export default reports;
