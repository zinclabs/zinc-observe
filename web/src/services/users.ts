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

/* eslint-disable @typescript-eslint/no-explicit-any */
import http from "./http";
import config from "../aws-exports";

const users = {
  list: (
    page_num: number,
    page_size: number,
    sort_by: string,
    desc: boolean,
    name: string
  ) => {
    return http().get(
      `/api/users?page_num=${page_num}&page_size=${page_size}&sort_by=${sort_by}&desc=${desc}&name=${name}`
    );
  },
  create: (data: any, org_identifier: string) => {
    return http().post(`/api/${org_identifier}/users`, data);
  },
  update: (data: any, org_identifier: string, username: string) => {
    return http().put(`/api/${org_identifier}/users/${username}`, data);
  },
  updateexistinguser: (
    data: any,
    org_identifier: string,
    username: string
  ) => {
    return http().post(`/api/${org_identifier}/users/${username}`, data);
  },
  delete: (org_identifier: string, username: string) => {
    return http().delete(`/api/${org_identifier}/users/${username}`);
  },
  verifyUser: (email: string) => {
    return http().get(`/api/users/verifyuser/${email}`);
  },
  addNewUser: (data: any) => {
    return http().post(`/api/users/new_user`, data);
  },
  orgUsers: (
    page_num: number,
    page_size: number,
    sort_by: string,
    desc: boolean,
    name: string,
    org_identifier: string
  ) => {
    if (config.isCloud === "true") {
      return http().get(
        `/api/${org_identifier}/org_users?page_num=${page_num}&page_size=${page_size}&sort_by=${sort_by}&desc=${desc}&name=${name}`
      );
    } else {
      return http().get(`/api/${org_identifier}/users`);
    }
  },
  getRefreshToken: () => {
    return http().get(`/api/auth/refresh_token`);
  },
  getRoles: (org_identifier: string) => {
    return http().get(`/api/${org_identifier}/users/roles`);
  },
 logout: () => {
    return http().get(`/config/logout`);
  },
};

export default users;
