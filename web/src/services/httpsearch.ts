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

import store from "../stores";
import router from "../router";
import axios from "axios";
import { Notify } from "quasar";

const http = () => {
  const instance = axios.create({
    // timeout: 10000,
    baseURL: store.state.API_ENDPOINT,
    headers: {},
  });

  instance.interceptors.response.use(
    function (response) {
      return response;
    },
    function (error) {
      if (error && error.response && error.response.status) {
        switch (error.response.status) {
          case 400:
            Notify.create({
              position: "bottom-right",
              progress: true,
              multiLine: true,
              color: "red-5",
              textColor: "white",
              icon: "warning",
              message: JSON.stringify(
                error.response.data["error"] || "Bad Request"
              ),
            });
            break;
          case 401:
            Notify.create({
              position: "bottom-right",
              progress: true,
              multiLine: true,
              color: "red-5",
              textColor: "white",
              icon: "warning",
              message: error.response.data["error"] || "Invalid credentials",
            });
            store.dispatch("logout");
            localStorage.clear();
            sessionStorage.clear();
            window.location.reload();
            break;
          case 404:
            Notify.create({
              position: "bottom-right",
              progress: true,
              multiLine: true,
              color: "red-5",
              textColor: "white",
              icon: "warning",
              message: error.response.data["error"] || "Not Found",
            });
            break;
          case 500:
            Notify.create({
              position: "bottom-right",
              progress: true,
              multiLine: true,
              color: "red-5",
              textColor: "white",
              icon: "warning",
              message: JSON.stringify(
                error.response.data["error"] || "Internal ServerError"
              ),
            });
            break;
          default:
          // noop
        }
      }
      return Promise.reject(error);
    }
  );

  return instance;
};

export default http;
