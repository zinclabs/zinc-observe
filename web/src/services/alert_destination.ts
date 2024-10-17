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

const destination = {
  create: ({ org_identifier, destination_name, data }: any) => {
    return http().post(`/api/${org_identifier}/alerts/destinations`, data);
  },
  update: ({ org_identifier, destination_name, data }: any) => {
    return http().put(
      `/api/${org_identifier}/alerts/destinations/${encodeURIComponent(
        destination_name
      )}`,
      data
    );
  },
  list: ({ org_identifier, page_num, page_size, desc, sort_by }: any) => {
    return http().get(
      `/api/${org_identifier}/alerts/destinations?page_num=${page_num}&page_size=${page_size}&sort_by=${sort_by}&desc=${desc}`
    );
  },
  get_by_name: ({ org_identifier, destination_name }: any) => {
    return http().get(
      `/api/${org_identifier}/alerts/destinations/${encodeURIComponent(
        destination_name
      )}`
    );
  },
  delete: ({ org_identifier, destination_name }: any) => {
    return http().delete(
      `/api/${org_identifier}/alerts/destinations/${encodeURIComponent(
        destination_name
      )}`
    );
  },
};

export default destination;
