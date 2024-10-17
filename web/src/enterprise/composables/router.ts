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
import LoginCallback from "@/enterprise/components/login/Login.vue";
import {
  useLocalUserInfo,
  useLocalCurrentUser,
  getLoginURL,
  getLogoutURL,
  invlidateLoginData,
} from "@/utils/zincutils";

import authService from "@/services/auth";
import config from "@/aws-exports";
import Organizations from "@/enterprise/components/organizations/Organization.vue";
import Billing from "@/enterprise/components/billings/Billing.vue";
import Plans from "@/enterprise/components/billings/plans.vue";
import InvoiceHistory from "@/enterprise/components/billings/invoiceHistory.vue";
import Usage from "@/enterprise/components/billings/usage.vue";
import { routeGuard } from "@/utils/zincutils";

const useEnvRoutes = () => {
  const parentRoutes = [
    {
      path: "/login",
      component: Login,
      beforeEnter: async (to: any, from: any, next: any) => {
        if (config.isEnterprise == "true") {
          try {
            const url = await authService.get_dex_login();
            if (url) {
              window.location.href = url;
            } else {
              next();
            }
          } catch (error) {
            console.error("Error during redirection:", error);
            next(false);
          }
        } else {
          window.location.href = getLoginURL();
        }
      },
    },
    {
      path: "/logout",
      beforeEnter: async (to: any, from: any, next: any) => {
        useLocalCurrentUser("", true);
        useLocalUserInfo("", true);
        if (config.isEnterprise == "true") {
          invlidateLoginData();
        }
        //  else {
        //   window.location.href = getLogoutURL();
        // }
      },
    },
    {
      path: "/cb",
      name: "callback",
      component: LoginCallback,
    },
  ];

  const homeChildRoutes = [
    {
      path: "billings",
      name: "billings",
      component: Billing,
      meta: {
        keepAlive: false,
      },
      children: [
        {
          path: "usage",
          name: "usage",
          component: Usage,
        },
        {
          path: "plans",
          name: "plans",
          component: Plans,
        },
        {
          path: "invoice_history",
          name: "invoice_history",
          component: InvoiceHistory,
        },
      ],
    },
  ];

  return { parentRoutes, homeChildRoutes };
};

export default useEnvRoutes;
