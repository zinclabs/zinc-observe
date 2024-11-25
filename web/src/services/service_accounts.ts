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

/* eslint-disable @typescript-eslint/no-explicit-any */
import http from "./http";
import config from "../aws-exports";

const service_accounts = {
  list: (
    org_identifier: string,
  ) => {
    return http().get(
      `/api/${org_identifier}/service_accounts`
    );
  },
  create: (data: any, org_identifier: string) => {
    return http().post(`/api/${org_identifier}/service_accounts`, data);
  },
  update: (data: any, org_identifier: string, user_email: string) => {
    return http().put(`/api/${org_identifier}/service_accounts/${user_email}`, data);
  },
  
};
export default service_accounts;
