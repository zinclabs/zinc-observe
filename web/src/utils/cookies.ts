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

import Keys from "../constants/key";
// @ts-ignore
import Cookies from "js-cookie";

export const getSidebarStatus = () => Cookies.get(Keys.sidebarStatusKey);
export const setSidebarStatus = (sidebarStatus: string) =>
  Cookies.set(Keys.sidebarStatusKey, sidebarStatus, { path: "/" });

export const getLanguage = () => Cookies.get(Keys.languageKey);
export const setLanguage = (language: string) =>
  Cookies.set(Keys.languageKey, language, { path: "/" });

export const getSize = () => Cookies.get(Keys.sizeKey);
export const setSize = (size: string) =>
  Cookies.set(Keys.sizeKey, size, { path: "/" });

export const getToken = () => Cookies.get(Keys.tokenKey);
export const setToken = (token: string) =>
  Cookies.set(Keys.tokenKey, token, { path: "/" });
export const removeToken = () => Cookies.remove(Keys.tokenKey);
