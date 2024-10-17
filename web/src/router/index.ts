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
import { getDecodedUserInfo, getPath, mergeRoutes } from "@/utils/zincutils";
import segment from "@/services/segment_analytics";
import config from "@/aws-exports";

import userCloudRoutes from "@/enterprise/composables/router";
import userRoutes from "@/composables/shared/router";
import useOSRoutes from "@/composables/router";

export default function (store: any) {
  let { parentRoutes, homeChildRoutes } = userRoutes();

  let envRoutes: any;
  if (config.isCloud == "true") {
    envRoutes = userCloudRoutes();
  } else {
    envRoutes = useOSRoutes();
  }

  // parentRoutes = parentRoutes.concat(envRoutes.parentRoutes);
  // homeChildRoutes = homeChildRoutes.concat(envRoutes.homeChildRoutes);
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

  const router = createRouter(routerMap);

  router.beforeEach((to: any, from: any, next: any) => {
    const isAuthenticated = store.state.loggedIn;

    if (!isAuthenticated && (to.path == "/cb" || to.path == "/web/cb")) {
      next();
    } else if (!isAuthenticated) {
      const sessionUserInfo = getDecodedUserInfo();

      if (
        to.path !== "/login" &&
        to.path !== "/cb" &&
        to.path != "/web/cb" &&
        sessionUserInfo === null
      ) {
        if (
          to.path !== "/logout" &&
          to.path !== "/cb" &&
          to.path != "/web/cb"
        ) {
          // if query params contains redirect_url, store that URL in session storage
          // else store the current URL in session storage
          // this conditions added specifically for short URL feature where user will be redirected to backend API endpoint
          // if user is not logged in, then user will be redirected to login page and after successful login, user will be redirected to the short URL
          if (Object.hasOwn(to.query, "short_url")) {
            window.sessionStorage.setItem("redirectURI", to.query.short_url);
          } else {
            window.sessionStorage.setItem("redirectURI", window.location.href);
          }
        }
        next({ path: "/login" });
      } else {
        if (sessionUserInfo !== null) {
          const userInfo = JSON.parse(String(sessionUserInfo));
          store.dispatch("login", {
            loginState: true,
            userInfo: userInfo,
          });
        }
        next();
      }
    } else {
      const sessionUserInfo = getDecodedUserInfo();
      const userID = JSON.parse(String(sessionUserInfo)).email;

      segment.track("page view", {
        path: to.path,
        referrer: from.path,
      });
      next();
    }
  });
  return router;
}
