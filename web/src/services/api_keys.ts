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

const apiKeys = {
  list: () => {
    return http().get(`/api/usertoken`);
  },
  listRUMTokens: (org_id: string) => {
    return http().get(`/api/${org_id}/rumtoken`);
  },
  createUserAPIKey: (data: object) => {
    return http().post(`/api/usertoken`, data);
  },
  updateUserAPIKey: (data: any) => {
    return http().put(`/api/usertoken/${data.id}`, data);
  },
  createRUMToken: (org_id: string) => {
    return http().post(`/api/${org_id}/rumtoken`);
  },
  updateRUMToken: (org_id: string, id: string) => {
    return http().put(`/api/${org_id}/rumtoken`);
  },
  deleteUserAPIKey: (id: string) => {
    return http().delete(`/api/usertoken/${id}`);
  },
};

export default apiKeys;
