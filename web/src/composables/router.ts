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

import Login from "@/views/Login.vue";
import { useLocalUserInfo, useLocalCurrentUser } from "@/utils/zincutils";
import useIngestionRoutes from "./shared/useIngestionRoutes";

const useOSRoutes = () => {
  const parentRoutes = [
    {
      path: "/login",
      component: Login,
    },
    {
      path: "/logout",
      beforeEnter(to: any, from: any, next: any) {
        useLocalCurrentUser("", true);
        useLocalUserInfo("", true);

        window.location.href = "/login";
      },
    },
    {
      path: "/cb",
      name: "callback",
      component: Login,
    },
  ];

  const homeChildRoutes: any[] = [...useIngestionRoutes()];

  return { parentRoutes, homeChildRoutes };
};

export default useOSRoutes;
