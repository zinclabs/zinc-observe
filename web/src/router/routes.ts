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

import { createRouter, createWebHistory } from "vue-router";
import { getPath, mergeRoutes } from "@/utils/zincutils";

import userRoutes from "@/composables/shared/router";
import useOSRoutes from "@/composables/router";
let { parentRoutes, homeChildRoutes } = userRoutes();

const envRoutes: any = useOSRoutes();

parentRoutes = mergeRoutes(parentRoutes, envRoutes.parentRoutes);
homeChildRoutes = mergeRoutes(homeChildRoutes, envRoutes.homeChildRoutes);

const routes = [
  ...parentRoutes,
  {
    path: "/",
    component: () => import("@/layouts/MainLayout.vue"),
    children: [...homeChildRoutes],
  },
];
interface RouterMap {
  history: any;
  routes: any;
}
const routerMap: RouterMap = {
  history: createWebHistory(getPath()),
  // history: createWebHistory(window.location.pathname),
  routes: routes,
};

export default createRouter(routerMap);
