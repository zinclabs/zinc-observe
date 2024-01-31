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

import { createRouter, createWebHistory } from "vue-router";
import {
  getDecodedUserInfo,
  useLocalToken,
  getPath,
  mergeRoutes,
  b64EncodeUnicode,
  useLocalUserInfo,
  useLocalCurrentUser,
} from "@/utils/zincutils";
import segment from "@/services/segment_analytics";
import config from "@/aws-exports";

import userCloudRoutes from "@/enterprise/composables/router";
import userRoutes from "@/composables/shared/router";
import useOSRoutes from "@/composables/router";
import users from "@/services/users";

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

  router.beforeEach(async (to: any, from: any, next: any) => {
    const isAuthenticated = store.state.loggedIn;

    if (!isAuthenticated) {
      if (to.path === "/cb") {
        next();
      }
      const sessionUserInfo = getDecodedUserInfo();
      const localStorageToken: any = useLocalToken();
      if (localStorageToken.value === "" || sessionUserInfo === null) {
        const res = await users.getProxyAuthUser();
        if (res.status === 200) {
          const user = res.data;
          if (user) {
            const userInfo = {
              given_name: "",
              auth_time: new Date().getTime() / 1000,
              name: user.email,
              exp: Math.floor(
                (new Date().getTime() + 1000 * 60 * 60 * 24 * 30) / 1000
              ),
              email: user.email,
            };
            const encodedUserInfo = b64EncodeUnicode(JSON.stringify(userInfo));

            //set user info into localstorage & store
            useLocalUserInfo(encodedUserInfo);
            store.dispatch("setUserInfo", encodedUserInfo);

            useLocalCurrentUser(JSON.stringify(userInfo));
            store.dispatch("setCurrentUser", userInfo);
          }
        }
      }

      if (
        to.path !== "/login" &&
        to.path !== "/cb" &&
        (localStorageToken.value === "" || sessionUserInfo === null)
      ) {
        if (to.path !== "/logout") {
          window.sessionStorage.setItem("redirectURI", to.fullPath);
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
