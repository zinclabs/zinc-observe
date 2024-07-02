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

import { createStore } from "vuex";
import {
  useLocalOrganization,
  useLocalCurrentUser,
  useLocalTimezone,
} from "../utils/zincutils";
import type { Notification } from "@/ts/interfaces/notification";

const pos = window.location.pathname.indexOf("/web/");

const API_ENDPOINT = import.meta.env.VITE_OPENOBSERVE_ENDPOINT
  ? import.meta.env.VITE_OPENOBSERVE_ENDPOINT.endsWith("/")
    ? import.meta.env.VITE_OPENOBSERVE_ENDPOINT.slice(0, -1)
    : import.meta.env.VITE_OPENOBSERVE_ENDPOINT
  : window.location.origin == "http://localhost:8081"
  ? "/"
  : pos > -1
  ? window.location.origin + window.location.pathname.slice(0, pos)
  : window.location.origin;

const organizationObj = {
  organizationPasscode: "",
  allDashboardList: {},
  rumToken: {
    rum_token: "",
  },
  quotaThresholdMsg: "",
  functions: [],
  streams: {},
  folders: [],
  organizationSettings: {
    scrape_interval: 15,
  },
  isDataIngested: false,
};

export default createStore({
  state: {
    API_ENDPOINT: API_ENDPOINT,
    userInfo: {},
    loggedIn: false,
    loadingState: true,
    errorLoadingState: false,
    selectedOrganization: useLocalOrganization() ? useLocalOrganization() : {},
    organizations: [],
    currentuser: useLocalCurrentUser() ? useLocalCurrentUser() : {},
    searchCollapsibleSection: 20,
    theme: "",
    printMode: false,
    organizationData: JSON.parse(JSON.stringify(organizationObj)),
    zoConfig: {},
    timezone: useLocalTimezone()
      ? useLocalTimezone()
      : Intl.DateTimeFormat().resolvedOptions().timeZone,
    savedViewDialog: false,
    refreshIntervalID: 0,
    savedViewFlag: false,
    savedFunctionDialog: false,
    regionInfo: [],
    notifications: {
      isOpen: false,
      notifications: [
        {
          title: "Energy Consumption Alert",
          message: "Current energy consumption exceeds the set threshold.",
          details: "Consider optimizing equipment usage.",
          time: "Today, 15:00 PM",
        },
        {
          title: "Dr. Laura Cristya",
          message:
            "I'll be holding virtual office hours this week on Tuesday and Thursday from 2:00 PM to 4:00 PM.",
          details: "",
          time: "Today, 14:00 PM",
        },
        {
          title: "Weather Advisory",
          message:
            "Severe weather alert: Expect heavy rain and potential flooding in the area.",
          details: "",
          time: "Yesterday, 08:30 AM",
        },
        {
          title: "Dr. David Lee",
          message:
            "Hello students, I've uploaded the course materials for Week 3. Please review the lecture notes and readings.",
          details: "",
          time: "Yesterday, 08:00 AM",
        },
      ] as Notification[],
    },
    sessionId: "",
    webSocketUrl: "",
  },
  mutations: {
    login(state, payload) {
      if (payload) {
        state.loggedIn = payload.loginState;
        state.userInfo = payload.userInfo;
      }
    },
    logout(state) {
      state.loggedIn = false;
      state.userInfo = {};
    },
    endpoint(state, payload) {
      state.API_ENDPOINT = payload;
    },
    setUserInfo(state, payload) {
      state.userInfo = payload;
    },
    // setIndexData(state, payload) {
    //   state.indexData = payload;
    // },
    setSelectedOrganization(state, payload) {
      state.selectedOrganization = payload;
    },
    setOrganizations(state, payload) {
      state.organizations = payload;
    },
    setCurrentUser(state, payload) {
      state.currentuser = payload;
    },
    setSearchCollapseToggle(state, payload) {
      state.searchCollapsibleSection = payload;
    },
    setOrganizationPasscode(state, payload) {
      state.organizationData.organizationPasscode = payload;
    },
    resetOrganizationData(state, payload) {
      state.organizationData = JSON.parse(JSON.stringify(organizationObj));
    },
    setRUMToken(state, payload) {
      state.organizationData.rumToken = payload;
    },
    // setAllCurrentDashboards(state, payload) {
    //   state.allCurrentDashboards = payload;
    // },
    // setCurrentSelectedDashboard(state, payload) {
    //   state.currentSelectedDashboard = payload;
    // },
    setAllDashboardList(state, payload) {
      state.organizationData.allDashboardList = payload;
    },
    setOrganizationSettings(state, payload) {
      state.organizationData.organizationSettings = payload;
    },
    setFunctions(state, payload) {
      state.organizationData.functions = payload;
    },
    setStreams(state, payload) {
      state.organizationData.streams[payload.name] = payload;
    },
    resetStreams(state, payload) {
      state.organizationData.streams = payload;
    },
    // setSearch(state, payload) {
    //   state.search = payload;
    // },
    // setStreamFields(state, payload) {
    //   state.streamFields = payload;
    // },
    // setCurrentPanelsData(state, payload) {
    //   state.currentPanelsData = payload;
    // },
    setQuotaThresholdMsg(state, payload) {
      state.organizationData.quotaThresholdMsg = payload;
    },
    setConfig(state, payload) {
      state.zoConfig = payload;
    },
    setFolders(state, payload) {
      state.organizationData.folders = payload;
    },
    appTheme(state, payload) {
      state.theme = payload;
    },
    setPrintMode(state, payload) {
      state.printMode = payload;
    },
    setTimezone(state, payload) {
      state.timezone = payload;
    },
    setSavedViewDialog(state, payload) {
      state.savedViewDialog = payload;
    },
    setRefreshIntervalID(state, payload) {
      state.refreshIntervalID = payload;
    },
    setSavedViewFlag(state, payload) {
      state.savedViewFlag = payload;
    },
    setSavedFunctionDialog(state, payload) {
      state.savedFunctionDialog = payload;
    },
    setIsDataIngested(state, payload) {
      state.organizationData.isDataIngested = payload;
    },
    setRegionInfo(state, payload) {
      state.regionInfo = payload;
    },
    addNotification(state, notification: Notification) {
      state.notifications.notifications.push(notification);
      state.notifications.isOpen = true;
    },
    removeNotification(state, notificationId) {
      state.notifications.notifications =
        state.notifications.notifications.filter(
          (notification) => notification.id !== notificationId
        );
    },
    markNotificationAsRead(state, notificationId) {
      const notification = state.notifications.notifications.find(
        (notification) => notification.id === notificationId
      );
      if (notification) {
        notification.read = true;
      }
    },
    markAllNotificationsAsRead(state) {
      state.notifications.notifications.forEach((notification) => {
        notification.read = true;
      });
    },
    expandNotifications(state, notificationId) {
      const notification = state.notifications.notifications.find(
        (notification) => notification.id === notificationId
      );
      if (notification) {
        notification.expanded = !notification.expanded;
      }
    },
    setNotificationDrawer(state, payload) {
      state.notifications.isOpen = payload;
    },
    setSessionId(state, payload) {
      state.sessionId = payload;
    },
    setWebSocketUrl(state, payload) {
      state.webSocketUrl = payload;
    },
  },
  actions: {
    login(context, payload) {
      context.commit("login", payload);
    },
    logout(context) {
      context.commit("logout");
    },
    endpoint(context, payload) {
      context.commit("endpoint", payload);
    },
    // setIndexData(context, payload) {
    //   context.commit("setIndexData", payload);
    // },
    setSelectedOrganization(context, payload) {
      context.commit("setSelectedOrganization", payload);
    },
    setOrganizations(context, payload) {
      context.commit("setOrganizations", payload);
    },
    setCurrentUser(context, payload) {
      context.commit("setCurrentUser", payload);
    },
    setSearchCollapseToggle(context, payload) {
      context.commit("setSearchCollapseToggle", payload);
    },
    setOrganizationPasscode(context, payload) {
      context.commit("setOrganizationPasscode", payload);
    },
    resetOrganizationData(context, payload) {
      context.commit("resetOrganizationData", payload);
    },
    setRUMToken(context, payload) {
      context.commit("setRUMToken", payload);
    },
    // setAllCurrentDashboards(context, payload) {
    //   context.commit('setAllCurrentDashboards', payload);
    // },
    // setCurrentSelectedDashboard(context, payload) {
    //   context.commit('setCurrentSelectedDashboard', payload);
    // },
    setAllDashboardList(context, payload) {
      context.commit("setAllDashboardList", payload);
    },
    setOrganizationSettings(context, payload) {
      context.commit("setOrganizationSettings", payload);
    },
    setFolders(context, payload) {
      context.commit("setFolders", payload);
    },
    setFunctions(context, payload) {
      context.commit("setFunctions", payload);
    },
    setStreams(context, payload) {
      context.commit("setStreams", payload);
    },
    resetStreams(context, payload) {
      context.commit("resetStreams", payload);
    },
    // setSearch(context, payload) {
    //   context.commit("setSearch", payload);
    // },
    // setStreamFields(context, payload) {
    //   context.commit("setStreamFields", payload);
    // },
    // setCurrentPanelsData(context, payload) {
    //   context.commit('setCurrentPanelsData', payload);
    // },
    setQuotaThresholdMsg(context, payload) {
      context.commit("setQuotaThresholdMsg", payload);
    },
    setConfig(context, payload) {
      context.commit("setConfig", payload);
    },
    appTheme(context, payload) {
      context.commit("appTheme", payload);
    },
    setPrintMode(context, payload) {
      context.commit("setPrintMode", payload);
    },
    setTimezone(context, payload) {
      context.commit("setTimezone", payload);
    },
    setSavedViewDialog(context, payload) {
      context.commit("setSavedViewDialog", payload);
    },
    setRefreshIntervalID(context, payload) {
      context.commit("setRefreshIntervalID", payload);
    },
    setSavedViewFlag(context, payload) {
      context.commit("setSavedViewFlag", payload);
    },
    setSavedFunctionDialog(context, payload) {
      context.commit("setSavedFunctionDialog", payload);
    },
    setIsDataIngested(context, payload) {
      context.commit("setIsDataIngested", payload);
    },
    setRegionInfo(context, payload) {
      context.commit("setRegionInfo", payload);
    },
    addNotification({ commit }, notification: Notification) {
      commit("addNotification", notification);
    },
    removeNotification({ commit }, notificationId: number) {
      commit("removeNotification", notificationId);
    },
    markAsRead({ commit }, notificationId: number) {
      commit("markNotificationAsRead", notificationId);
    },
    markAllAsRead({ commit }) {
      commit("markAllNotificationsAsRead");
    },
    expandNotification({ commit }, notificationId: number) {
      commit("expandNotifications", notificationId);
    },
    setNotificationDrawer({ commit }, payload) {
      commit("setNotificationDrawer", payload);
    },
    setSessionId(context, payload) {
      context.commit("setSessionId", payload);
    },
    setWebSocketUrl(context, payload) {
      context.commit("setWebSocketUrl", payload);
    },
  },
  modules: {},
});
