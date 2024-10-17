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

const syslogService = {
  // To start/stop syslog route
  toggle: (org_identifier: string, data: any) => {
    return http().post(`/api/${org_identifier}/syslog-server`, data);
  },
  list: (org_identifier: string) => {
    return http().get(`/api/${org_identifier}/syslog-routes`);
  },
  create: (org_identifier: string, data: any) => {
    return http().post(`/api/${org_identifier}/syslog-routes`, data);
  },
  update: (org_identifier: string, routeId: string, data: any) => {
    return http().put(`/api/${org_identifier}/syslog-routes/${routeId}`, data);
  },
  delete: (org_identifier: string, routeId: string) => {
    return http().delete(`/api/${org_identifier}/syslog-routes/${routeId}`);
  },
};

export default syslogService;
