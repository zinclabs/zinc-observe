<!-- Copyright 2023 Zinc Labs Inc.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
-->

<template>
  <q-layout
    view="hHh lpR fFf"
    :class="[
      miniMode ? 'miniMode' : '',
      store.state.printMode === true ? 'printMode' : '',
    ]"
  >
    <q-header
      :class="[store?.state?.theme == 'dark' ? 'dark-mode' : 'bg-white']"
    >
      <q-toolbar>
        <div
          class="flex relative-position q-mr-sm"
          v-if="
            (config.isEnterprise == 'true' &&
              store.state.zoConfig.hasOwnProperty('custom_logo_text') &&
              store.state.zoConfig.custom_logo_text != '') ||
            (config.isEnterprise == 'true' &&
              store.state.zoConfig.hasOwnProperty('custom_logo_img') &&
              store.state.zoConfig.custom_logo_img != null)
          "
        >
          <span
            v-if="
              store.state.zoConfig.hasOwnProperty('custom_logo_text') &&
              store.state.zoConfig?.custom_logo_text != ''
            "
            class="text-h6 text-bold q-pa-none cursor-pointer q-mr-sm"
            @click="goToHome"
            >{{ store.state.zoConfig.custom_logo_text }}</span
          >
          <img
            v-if="
              store.state.zoConfig.hasOwnProperty('custom_logo_img') &&
              store.state.zoConfig?.custom_logo_img != null
            "
            :src="
              `data:image; base64, ` + store.state.zoConfig?.custom_logo_img
            "
            style="max-width: 150px; max-height: 31px"
          />
          <img
            class="appLogo"
            loading="lazy"
            :src="
              store?.state?.theme == 'dark'
                ? getImageURL('images/common/open_observe_logo_2.svg')
                : getImageURL('images/common/open_observe_logo.svg')
            "
            @click="goToHome"
          />
        </div>
        <div v-else class="flex relative-position q-mr-sm">
          <img
            class="appLogo"
            loading="lazy"
            :src="getImageURL('images/common/open_observe_logo.svg')"
            @click="goToHome"
          />
          <span v-if="config.isCloud == 'true'" class="absolute beta-text"
            >Beta</span
          >
        </div>

        <q-toolbar-title></q-toolbar-title>
        <div
          class="headerMenu float-left"
          v-if="store.state.organizationData.quotaThresholdMsg"
        >
          <div
            type="warning"
            icon="cloud"
            class="warning-msg"
            style="display: inline"
          >
            <q-icon name="warning" size="xs" class="warning" />{{
              store.state.organizationData.quotaThresholdMsg
            }}
          </div>
          <q-btn
            color="secondary"
            size="sm"
            style="display: inline; padding: 5px 10px"
            rounded
            borderless
            dense
            class="q-ma-xs"
            @click="router.replace('/billings/plans')"
            >Upgrade to PRO Plan</q-btn
          >
        </div>
        <ThemeSwitcher></ThemeSwitcher>
        <template
          v-if="
            config.isCloud !== 'true' &&
            !store.state.zoConfig?.custom_hide_menus
              ?.split(',')
              ?.includes('openapi')
          "
        >
          <q-btn
            class="q-ml-xs no-border"
            size="13px"
            no-caps
            :label="t(`menu.openapi`)"
            @click="navigateToOpenAPI(zoBackendUrl)"
          />
        </template>
        <q-btn
          class="q-ml-xs no-border"
          size="13px"
          no-caps
          :label="t(`menu.docs`)"
          @click="navigateToDocs()"
        />
        <div class="languageWrapper">
          <q-btn-dropdown
            data-test="language-dropdown"
            unelevated
            no-caps
            flat
            class="languageDdl"
            :icon="selectedLanguage.icon"
          >
            <template #label>
              <div class="row no-wrap">
                {{ selectedLanguage.label }}
              </div>
            </template>
            <q-list class="languagelist">
              <q-item
                data-test="language-dropdown-item"
                v-for="lang in langList"
                :key="lang.code"
                v-ripple="true"
                v-close-popup="true"
                clickable
                v-bind="lang"
                active-class="activeLang"
                @click="changeLanguage(lang)"
              >
                <q-item-section avatar>
                  <q-icon :name="lang.icon" class="flagIcon" />
                </q-item-section>

                <q-item-section
                  :data-test="`language-dropdown-item-${lang.code}`"
                >
                  <q-item-label>{{ lang.label }}</q-item-label>
                </q-item-section>
              </q-item>
            </q-list>
          </q-btn-dropdown>
        </div>

        <div
          data-test="navbar-organizations-select"
          class="q-mx-sm current-organization"
        >
          <q-select
            v-model="selectedOrg"
            borderless
            :options="orgOptions"
            class="q-px-none q-py-none q-mx-none q-my-none organizationlist"
            @update:model-value="updateOrganization()"
          />
        </div>

        <div data-test="navbar-notifications" class="q-mx-sm tw-relative">
          <q-btn
            flat
            dense
            unelevated
            no-caps
            size="12px"
            padding="xs sm"
            :icon="outlinedNotifications"
            @click.stop="toggleNotifications"
          >
          </q-btn>

          <div
            v-if="unreadNotifications.length"
            class="tw-absolute tw-top-[6px] tw-right-[8px] tw-text-[11px] tw-w-[10px] tw-h-[10px] tw-bg-white tw-rounded-full tw-flex tw-justify-center tw-items-center"
          >
            <div
              class="tw-top-[6px] tw-right-[10px] tw-text-[11px] tw-w-[7px] tw-h-[7px] tw-bg-primary tw-rounded-full"
            />
          </div>
        </div>

        <div class="q-mr-xs">
          <q-btn-dropdown flat unelevated no-caps padding="xs sm">
            <template #label>
              <div class="row items-center no-wrap">
                <q-avatar size="md" color="grey" text-color="white">
                  <img
                    :src="
                      user.picture
                        ? user.picture
                        : getImageURL('images/common/profile.svg')
                    "
                  />
                </q-avatar>
                <div class="userInfo">
                  <div class="userName">
                    {{
                      user.given_name
                        ? user.given_name + " " + user.family_name
                        : user.email
                    }}
                  </div>
                </div>
              </div>
            </template>
            <q-list>
              <q-item-label header>{{ t("menu.account") }}</q-item-label>

              <q-item
                v-if="config.isCloud == 'true'"
                v-ripple="true"
                v-close-popup="true"
                clickable
                :to="{ path: '/settings' }"
              >
                <q-item-section avatar>
                  <q-avatar
                    size="md"
                    icon="settings"
                    color="red"
                    text-color="white"
                  />
                </q-item-section>
                <q-item-section>
                  <q-item-label>{{ t("menu.settings") }}</q-item-label>
                </q-item-section>
              </q-item>
              <q-item
                v-ripple="true"
                v-close-popup="true"
                clickable
                @click="signout"
              >
                <q-item-section avatar>
                  <q-avatar
                    size="md"
                    icon="exit_to_app"
                    color="red"
                    text-color="white"
                  />
                </q-item-section>
                <q-item-section>
                  <q-item-label>{{ t("menu.signOut") }}</q-item-label>
                </q-item-section>
              </q-item>
            </q-list>
          </q-btn-dropdown>
        </div>
      </q-toolbar>
    </q-header>

    <q-drawer
      :mini="miniMode"
      bordered
      show-if-above
      @mouseover="expandMenu"
      @mouseout="miniMode = true"
      mini-to-overlay
    >
      <q-list class="leftNavList">
        <menu-link
          v-for="nav in linksList"
          :key="nav.title"
          :link-name="nav.name"
          v-bind="{ ...nav, mini: miniMode }"
        />
      </q-list>
    </q-drawer>
    <q-page-container
      :key="store.state.selectedOrganization?.identifier"
      v-if="isLoading"
    >
      <router-view v-slot="{ Component }">
        <template v-if="$route.meta.keepAlive">
          <keep-alive>
            <component :is="Component" />
          </keep-alive>
        </template>
        <template v-else>
          <component :is="Component" />
        </template>
      </router-view>
    </q-page-container>
  </q-layout>
</template>

<script lang="ts">
import {
  QPage,
  QPageContainer,
  QLayout,
  QDrawer,
  QList,
  QItem,
  QItemLabel,
  QItemSection,
  QBtn,
  QBtnDropdown,
  QToolbarTitle,
  QHeader,
  QToolbar,
  QAvatar,
  QIcon,
  QSelect,
  useQuasar,
} from "quasar";
import MenuLink from "../components/MenuLink.vue";
import { useI18n } from "vue-i18n";
import {
  useLocalCurrentUser,
  useLocalOrganization,
  useLocalUserInfo,
  getImageURL,
  invlidateLoginData,
  getLogoutURL,
  getUUID,
} from "../utils/zincutils";

import {
  ref,
  defineComponent,
  KeepAlive,
  computed,
  onMounted,
  watch,
  markRaw,
  nextTick,
  onUnmounted,
} from "vue";
import { useStore } from "vuex";
import { useRouter, RouterView } from "vue-router";
import config from "../aws-exports";

import { setLanguage } from "../utils/cookies";
import { getLocale } from "../locales";

import MainLayoutOpenSourceMixin from "@/mixins/mainLayout.mixin";
import MainLayoutCloudMixin from "@/enterprise/mixins/mainLayout.mixin";

import configService from "@/services/config";
import streamService from "@/services/stream";
import billings from "@/services/billings";
import ThemeSwitcher from "../components/ThemeSwitcher.vue";
import {
  outlinedHome,
  outlinedSearch,
  outlinedBarChart,
  outlinedAccountTree,
  outlinedDashboard,
  outlinedWindow,
  outlinedReportProblem,
  outlinedFilterAlt,
  outlinedPerson,
  outlinedFormatListBulleted,
  outlinedSettings,
  outlinedManageAccounts,
  outlinedDescription,
  outlinedNotifications,
} from "@quasar/extras/material-icons-outlined";
import SlackIcon from "@/components/icons/SlackIcon.vue";
import ManagementIcon from "@/components/icons/ManagementIcon.vue";
import organizations from "@/services/organizations";
import useStreams from "@/composables/useStreams";
import { openobserveRum } from "@openobserve/browser-rum";
import useWebSocket from "@/composables/useWebSocket";
import useNotifications from "@/composables/useNotifications";

let mainLayoutMixin: any = null;
if (config.isCloud == "true") {
  mainLayoutMixin = MainLayoutCloudMixin;
} else {
  mainLayoutMixin = MainLayoutOpenSourceMixin;
}

export default defineComponent({
  name: "MainLayout",
  mixins: [mainLayoutMixin],
  components: {
    "menu-link": MenuLink,
    "keep-alive": KeepAlive,
    "q-page": QPage,
    "q-page-container": QPageContainer,
    "q-layout": QLayout,
    "q-drawer": QDrawer,
    "q-list": QList,
    "q-item": QItem,
    "q-item-label": QItemLabel,
    "q-item-section": QItemSection,
    "q-btn": QBtn,
    "q-btn-dropdown": QBtnDropdown,
    "q-toolbar-title": QToolbarTitle,
    "q-header": QHeader,
    "q-toolbar": QToolbar,
    "router-view": RouterView,
    "q-avatar": QAvatar,
    "q-icon": QIcon,
    "q-select": QSelect,
    SlackIcon,
    ManagementIcon,
    ThemeSwitcher,
  },
  methods: {
    navigateToDocs() {
      let docURL = "https://openobserve.ai/docs";
      if (
        this.config.isEnterprise == "true" &&
        this.store.state.zoConfig.custom_docs_url != ""
      ) {
        docURL = this.store.state.zoConfig.custom_docs_url;
      }
      window.open(docURL, "_blank");
    },
    navigateToOpenAPI(zoBackendUrl: string) {
      window.open(zoBackendUrl + "/swagger/index.html", "_blank");
    },
    signout() {
      if (config.isEnterprise == "true") {
        invlidateLoginData();
      }

      const logoutURL = getLogoutURL();
      this.store.dispatch("logout");

      useLocalCurrentUser("", true);
      useLocalUserInfo("", true);

      if (config.isCloud == "true") {
        window.location.href = logoutURL;
      }

      this.closeWebSocket();
      this.$router.push("/logout");
    },
    goToHome() {
      this.$router.push("/");
    },
    changeLanguage(item: { code: string; label: string; icon: string }) {
      setLanguage(item.code);
      window.location.reload();
    },
  },
  setup() {
    const store: any = useStore();
    const router: any = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const miniMode = ref(true);
    const zoBackendUrl = store.state.API_ENDPOINT;
    const isLoading = ref(false);
    const { getStreams, resetStreams } = useStreams();

    const { openWebSocket, closeWebSocket } = useWebSocket();
    const { unreadNotifications } = useNotifications();

    const isMonacoEditorLoaded = ref(false);

    let customOrganization = router.currentRoute.value.query.hasOwnProperty(
      "org_identifier"
    )
      ? router.currentRoute.value.query.org_identifier
      : undefined;
    const selectedOrg = ref(store.state.selectedOrganization);
    const excludeParentRedirect = [
      "pipeline",
      "functionList",
      "streamFunctions",
      "enrichmentTables",
      "alerts",
      "alertList",
      "alertDestinations",
      "alertTemplates",
      "/ingestion/",
    ];

    const orgOptions = ref([{ label: Number, value: String }]);
    let slackURL = "https://short.openobserve.ai/community";
    if (
      config.isEnterprise == "true" &&
      store.state.zoConfig.custom_slack_url != ""
    ) {
      slackURL = store.state.zoConfig.custom_slack_url;
    }

    let user = store.state.userInfo;

    var linksList = ref([
      {
        title: t("menu.home"),
        icon: outlinedHome,
        link: "/",
        exact: true,
        name: "home",
      },
      {
        title: t("menu.search"),
        icon: outlinedSearch,
        link: "/logs",
        name: "logs",
      },
      {
        title: t("menu.metrics"),
        icon: outlinedBarChart,
        link: "/metrics",
        name: "metrics",
      },
      {
        title: t("menu.traces"),
        icon: outlinedAccountTree,
        link: "/traces",
        name: "traces",
      },
      {
        title: t("menu.rum"),
        icon: "devices",
        link: "/rum/performance/overview",
        name: "rum",
      },
      {
        title: t("menu.dashboard"),
        icon: outlinedDashboard,
        link: "/dashboards",
        name: "dashboards",
      },
      {
        title: t("menu.index"),
        icon: outlinedWindow,
        link: "/streams",
        name: "streams",
      },
      {
        title: t("menu.alerts"),
        icon: outlinedReportProblem,
        link: "/alerts",
        name: "alerts",
      },
      {
        title: t("menu.ingestion"),
        icon: outlinedFilterAlt,
        link: "/ingestion",
        name: "ingestion",
      },
      {
        title: t("menu.iam"),
        icon: outlinedManageAccounts,
        link: "/iam",
        display: store.state?.currentuser?.role == "admin" ? true : false,
        name: "iam",
      },
      {
        title: t("menu.settings"),
        iconComponent: markRaw(ManagementIcon),
        link: "/settings/",
        name: "settings",
      },
      {
        title: t("menu.slack"),
        iconComponent: markRaw(SlackIcon),
        link: slackURL,
        target: "_blank",
        external: true,
        name: "slack",
      },
      {
        title: t("menu.about"),
        icon: outlinedFormatListBulleted,
        link: "/about",
        name: "about",
      },
    ]);

    const langList = [
      {
        label: "English",
        code: "en-gb",
        icon: "img:" + getImageURL("images/language_flags/en-gb.svg"),
      },
      {
        label: "Türkçe",
        code: "tr-turk",
        icon: "img:" + getImageURL("images/language_flags/tr-turk.svg"),
      },
      {
        label: "简体中文",
        code: "zh-cn",
        icon: "img:" + getImageURL("images/language_flags/zh-cn.svg"),
      },
      {
        label: "Français",
        code: "fr",
        icon: "img:" + getImageURL("images/language_flags/fr.svg"),
      },
      {
        label: "Español",
        code: "es",
        icon: "img:" + getImageURL("images/language_flags/es.svg"),
      },
      {
        label: "Deutsch",
        code: "de",
        icon: "img:" + getImageURL("images/language_flags/de.svg"),
      },
      {
        label: "Italiano",
        code: "it",
        icon: "img:" + getImageURL("images/language_flags/it.svg"),
      },
      {
        label: "日本語",
        code: "ja",
        icon: "img:" + getImageURL("images/language_flags/ja.svg"),
      },
      {
        label: "한국어",
        code: "ko",
        icon: "img:" + getImageURL("images/language_flags/ko.svg"),
      },
      {
        label: "Nederlands",
        code: "nl",
        icon: "img:" + getImageURL("images/language_flags/nl.svg"),
      },
      {
        label: "Português",
        code: "pt",
        icon: "img:" + getImageURL("images/language_flags/pt.svg"),
      },
    ];

    onMounted(async () => {
      generateSessionId();
      setWebSocket();

      miniMode.value = true;
      filterMenus();

      // TODO OK : Clean get config functions which sets rum user and functions menu. Move it to common method.
      if (
        !store.state.zoConfig.hasOwnProperty("version") ||
        store.state.zoConfig.version == ""
      ) {
        getConfig();
      } else {
        if (config.isCloud == "false") {
          linksList.value = mainLayoutMixin
            .setup()
            .leftNavigationLinks(linksList, t);
          filterMenus();
        }
        await nextTick();
        // if rum enabled then setUser to capture session details.
        if (store.state.zoConfig.rum?.enabled) {
          setRumUser();
        }
      }
    });

    onUnmounted(() => {
      closeWebSocket();
    });

    const selectedLanguage: any =
      langList.find((l) => l.code == getLocale()) || langList[0];

    const generateSessionId = () => {
      store.dispatch(
        "setSessionId",
        `session_${getUUID().replace(/-/g, "").slice(0, 16)}`
      );
    };

    const setWebSocket = () => {
      let url = `ws://${store.state.API_ENDPOINT.split("//")[1]}/api/ws/${
        store.state.userInfo.email
      }?request_id=${store.state.sessionId}`;

      store.dispatch("setWebSocketUrl", url);

      openWebSocket(store.state.webSocketUrl);
    };

    const filterMenus = () => {
      const disableMenus = new Set(
        store.state.zoConfig?.custom_hide_menus
          ?.split(",")
          ?.map((val: string) => val?.trim()) || []
      );
      linksList.value = linksList.value.filter(
        (link: { name: string }) => !disableMenus.has(link.name)
      );
    };

    // additional links based on environment and conditions
    if (config.isCloud == "true") {
      linksList.value = mainLayoutMixin
        .setup()
        .leftNavigationLinks(linksList, t);
      filterMenus();
    } else {
      linksList.value.splice(7, 0, {
        title: t("menu.report"),
        icon: outlinedDescription,
        link: "/reports",
        name: "reports",
      });
    }

    //orgIdentifier query param exists then clear the localstorage and store.
    if (store.state.selectedOrganization != null) {
      if (
        mainLayoutMixin.setup().customOrganization != undefined &&
        mainLayoutMixin.setup().customOrganization !=
          store.state.selectedOrganization?.identifier
      ) {
        useLocalOrganization("");
        store.dispatch("setSelectedOrganization", {});
      }
    }

    const triggerRefreshToken = () => {
      const expirationTimeUnix = store.state.userInfo.exp;

      // Convert the expiration time to milliseconds
      const expirationTimeMilliseconds = expirationTimeUnix * 1000;

      // Get the current time in milliseconds
      const currentTimeMilliseconds = Date.now();

      // Calculate the time difference
      const timeUntilNextAPICall =
        expirationTimeMilliseconds - currentTimeMilliseconds - 100;

      // Convert the time difference from milliseconds to seconds
      const timeUntilNextAPICallInSeconds = timeUntilNextAPICall / 1000;

      setTimeout(() => {
        mainLayoutMixin.setup().getRefreshToken();
      }, timeUntilNextAPICallInSeconds);
    };

    //get refresh token for cloud environment
    if (store.state.hasOwnProperty("userInfo") && store.state.userInfo.email) {
      if (config.isCloud == "true") {
        triggerRefreshToken();
      }
    }

    const updateOrganization = async () => {
      resetStreams();
      store.dispatch("setIsDataIngested", false);
      const orgIdentifier = selectedOrg.value.identifier;
      const queryParams =
        router.currentRoute.value.path.indexOf(".logs") > -1
          ? router.currentRoute.value.query
          : {};
      router.push({
        path: router.currentRoute.value.path,
        query: {
          ...queryParams,
          org_identifier: orgIdentifier,
        },
      });
      useLocalOrganization(selectedOrg.value);
      store.dispatch("setSelectedOrganization", { ...selectedOrg.value });

      // if (
      //   config.isCloud &&
      //   selectedOrg.value.subscription_type == config.freePlan
      // ) {
      //   await billings
      //     .list_subscription(selectedOrg.value.identifier)
      //     .then(async (res: any) => {
      //       if (res.data.data.length == 0) {
      //         router.push({ name: "plans" });
      //       } else if (
      //         res.data.data.CustomerBillingObj.customer_id == null ||
      //         res.data.data.CustomerBillingObj.customer_id == ""
      //       ) {
      //         router.push({ name: "plans" });
      //       } else {
      //         await verifyStreamExist(selectedOrg.value);
      //       }
      //     });
      // } else {
      if (
        store.state.zoConfig.hasOwnProperty(
          "restricted_routes_on_empty_data"
        ) &&
        store.state.zoConfig.restricted_routes_on_empty_data == true &&
        store.state.organizationData.isDataIngested == false
      ) {
        await verifyStreamExist(selectedOrg.value);
      }
      // }
    };

    const verifyStreamExist = async (selectedOrgData: any) => {
      await getStreams("", false).then((response: any) => {
        store.dispatch("setSelectedOrganization", {
          ...selectedOrgData,
        });
        if (response.list.length == 0) {
          store.dispatch("setIsDataIngested", false);
          $q.notify({
            type: "warning",
            message:
              "You haven't initiated the data ingestion process yet. To explore other pages, please start the data ingestion.",
            timeout: 5000,
          });
          router.push({ name: "ingestion" });
        } else {
          store.dispatch("setIsDataIngested", true);
        }
      });
    };

    const setSelectedOrganization = async () => {
      customOrganization = router.currentRoute.value.query.hasOwnProperty(
        "org_identifier"
      )
        ? router.currentRoute.value.query.org_identifier
        : "";
      let tempDefaultOrg = {};
      let localOrgFlag = false;
      if (store.state.organizations?.length > 0) {
        const localOrg: any = useLocalOrganization();
        orgOptions.value = store.state.organizations.map(
          (data: {
            id: any;
            name: any;
            type: any;
            identifier: any;
            UserObj: any;
            ingest_threshold: number;
            search_threshold: number;
            CustomerBillingObj: { subscription_type: string; note: string };
            status: string;
          }) => {
            const optiondata: any = {
              label: data.name,
              id: data.id,
              identifier: data.identifier,
              user_email: store.state.userInfo.email,
              ingest_threshold: data.ingest_threshold,
              search_threshold: data.search_threshold,
              subscription_type: data.hasOwnProperty("CustomerBillingObj")
                ? data.CustomerBillingObj.subscription_type
                : "",
              status: data.status,
              note: data.hasOwnProperty("CustomerBillingObj")
                ? data.CustomerBillingObj.note
                : "",
            };

            if (
              config.isCloud == "true" &&
              localOrg.value?.identifier == data?.identifier &&
              (customOrganization == "" || customOrganization == undefined)
            ) {
              // localOrg.value.subscription_type =
              //   data.CustomerBillingObj.subscription_type;
              // useLocalOrganization(localOrg.value);
              useLocalOrganization(optiondata);
            }

            if (localOrg.value.identifier == data.identifier) {
              localOrgFlag = true;
            }

            if (
              ((selectedOrg.value == "" || selectedOrg.value == undefined) &&
                data.type == "default" &&
                store.state.userInfo.email == data.UserObj.email &&
                (customOrganization == "" ||
                  customOrganization == undefined)) ||
              (store.state.organizations?.length == 1 &&
                (customOrganization == "" || customOrganization == undefined))
            ) {
              selectedOrg.value = localOrg.value ? localOrg.value : optiondata;
              useLocalOrganization(optiondata);
              store.dispatch("setSelectedOrganization", optiondata);
            } else if (data.identifier == customOrganization) {
              selectedOrg.value = optiondata;
              useLocalOrganization(optiondata);
              store.dispatch("setSelectedOrganization", optiondata);
            }

            if (data.type == "default") {
              tempDefaultOrg = optiondata;
            }

            return optiondata;
          }
        );
      }

      if (localOrgFlag == false) {
        selectedOrg.value = tempDefaultOrg;
        useLocalOrganization(tempDefaultOrg);
        store.dispatch("setSelectedOrganization", tempDefaultOrg);
      }

      if (router.currentRoute.value.query.action == "subscribe") {
        router.push({
          name: "plans",
        });
      }

      if (selectedOrg.value.identifier != "" && config.isCloud == "true") {
        mainLayoutMixin.setup().getOrganizationThreshold(store);
      }

      await getOrganizationSettings();
      isLoading.value = true;
    };

    // get organizations settings on first load and identifier change
    const getOrganizationSettings = async () => {
      try {
        //get organizations settings
        const orgSettings: any = await organizations.get_organization_settings(
          store.state?.selectedOrganization?.identifier
        );

        //set settings in store
        //scrape interval will be in number
        store.dispatch("setOrganizationSettings", {
          scrape_interval: orgSettings?.data?.data?.scrape_interval ?? 15,
        });
      } catch (error) {}
      return;
    };

    /**
     * Get configuration from the backend.
     * @return {"version":"","instance":"","commit_hash":"","build_date":"","default_fts_keys":["field1","field2"],"telemetry_enabled":true,"default_functions":[{"name":"function name","text":"match_all('v')"}}
     * @throws {Error} If the request fails.
     */
    const getConfig = async () => {
      await configService
        .get_config()
        .then(async (res: any) => {
          if (config.isCloud == "false") {
            linksList.value = mainLayoutMixin
              .setup()
              .leftNavigationLinks(linksList, t);
            filterMenus();
          }
          store.dispatch("setConfig", res.data);
          await nextTick();
          // if rum enabled then setUser to capture session details.
          if (res.data.rum.enabled) {
            setRumUser();
          }
        })
        .catch((error) => console.log(error));
    };

    if (config.isCloud == "true") {
      mainLayoutMixin.setup().getDefaultOrganization(store);
    }

    const redirectToParentRoute = (machedRoutes: any) => {
      if (router.currentRoute.value.path.indexOf("/dashboards/") > -1) {
        router.push({
          name: "dashboards",
        });
      } else if (
        machedRoutes?.length > 2 &&
        !excludeParentRedirect.includes(router.currentRoute.value.name) &&
        router.currentRoute.value.path.indexOf("/ingestion/") == -1 &&
        router.currentRoute.value.path.indexOf("/billings/") == -1
      ) {
        if (machedRoutes[machedRoutes.length - 2]?.children?.length > 0) {
          machedRoutes[machedRoutes.length - 2].children.forEach(
            (route: any) => {
              if (route.name == machedRoutes[machedRoutes.length - 1].name) {
                router.push({
                  path: machedRoutes[machedRoutes.length - 2].path,
                });
              }
            }
          );
        }
      } else {
        router.push({
          query: { org_identifier: selectedOrg.value.identifier },
        });
      }
    };

    const setRumUser = () => {
      if (store.state.zoConfig?.rum?.enabled == true) {
        const userInfo = store.state.userInfo;
        openobserveRum.setUser({
          name: userInfo.given_name + " " + userInfo.family_name,
          email: userInfo.email,
        });
      }
    };

    const prefetch = () => {
      const href = "/web/assets/editor.api.v1.js";
      const existingLink = document.querySelector(
        `link[rel="prefetch"][href="${href}"]`
      );

      if (!existingLink) {
        // Create a new link element
        isMonacoEditorLoaded.value = true;
        const link = document.createElement("link");
        link.rel = "prefetch";
        link.href = href;
        document.head.appendChild(link);
      }
    };

    const expandMenu = () => {
      miniMode.value = false;
      if (!isMonacoEditorLoaded.value) prefetch();
    };

    const toggleNotifications = () => {
      store.dispatch(
        "setNotificationDrawer",
        !store.state.notifications.isOpen
      );
    };

    return {
      t,
      router,
      store,
      config,
      langList,
      selectedLanguage,
      linksList,
      selectedOrg,
      orgOptions,
      leftDrawerOpen: false,
      miniMode,
      user,
      zoBackendUrl,
      isLoading,
      getImageURL,
      updateOrganization,
      setSelectedOrganization,
      redirectToParentRoute,
      getOrganizationSettings,
      resetStreams,
      triggerRefreshToken,
      prefetch,
      expandMenu,
      toggleNotifications,
      outlinedNotifications,
      unreadNotifications,
      closeWebSocket,
    };
  },
  computed: {
    changeOrganization() {
      return this.store?.state?.organizations;
    },
    changeOrganizationIdentifier() {
      return this.store?.state?.selectedOrganization?.identifier;
    },
    forceFetchOrganization() {
      return this.router?.currentRoute?.value?.query?.update_org;
    },
    changeUserInfo() {
      return this.store?.state?.userInfo;
    },
  },
  watch: {
    forceFetchOrganization() {
      mainLayoutMixin.setup().getDefaultOrganization(this.store);
    },
    changeOrganization() {
      setTimeout(() => {
        this.setSelectedOrganization();
      }, 500);
    },
    async changeOrganizationIdentifier() {
      this.isLoading = false;
      this.resetStreams();
      this.store.dispatch("setOrganizationPasscode", "");
      this.store.dispatch("resetOrganizationData", {});

      await this.getOrganizationSettings();

      this.isLoading = true;
      setTimeout(() => {
        this.redirectToParentRoute(this.$route.matched);
        // this.setSelectedOrganization();
      }, 500);
    },
    changeUserInfo(newVal) {
      if (JSON.stringify(newVal) != "{}") {
        this.triggerRefreshToken();
      }
    },
  },
});
</script>

<style lang="scss">
@import "../styles/app.scss";

.printMode {
  .q-header {
    display: none;
  }

  .q-drawer {
    display: none;
  }

  .q-page-container {
    padding-left: 0px !important;
  }
}

.warning-msg {
  background-color: var(--q-warning);
  padding: 5px;
  border-radius: 5px;
}

.alert-msg {
  background-color: var(--q-alert);
  padding: 5px;
  border-radius: 5px;
}

.q-header .q-btn-dropdown__arrow {
  margin-left: -4px;
}

.q-header {
  color: unset;
  @extend .border-bottom;

  .beta-text {
    font-size: 11px;
    right: 1px;
    bottom: -9px;
  }

  .appLogo {
    margin-left: 0.5rem;
    margin-right: 0;
    width: 150px;
    cursor: pointer;

    &__mini {
      margin-right: 0.25rem;
      // margin-left: 0.25rem;
      height: 30px;
      width: 30px;
    }
  }
}

.q-page-container {
  padding-left: 57px;
}

.q-drawer {
  @extend .border-right;
  min-width: 50px;
  max-width: 210px;
  color: unset;

  &--mini {
    .leftNavList {
      padding: 5px 8px;
    }
  }

  &--standard {
    z-index: 99999;
  }
}

.headerMenu {
  margin-right: 1rem;

  .block {
    font-weight: 700;
    color: #404040;
  }
}

.q-item {
  min-height: 30px;
  padding: 8px 8px;
}

.q-list {
  &.leftNavList {
    padding-bottom: 0px;
    padding-top: 5px;

    .q-item {
      margin-bottom: 5px;

      .q-icon {
        height: 1.5rem;
        width: 1.5rem;
      }

      &.q-router-link--active {
        .q-icon img {
          filter: brightness(100);
        }
      }
    }
  }

  .flagIcon img {
    border-radius: 3px;
    object-fit: cover;
    display: block;
    height: 16px;
    width: 24px;
  }

  .q-item {
    &__section {
      &--avatar {
        padding-right: 0.875rem;
        min-width: 1.5rem;
      }
    }

    &__label {
      font-weight: 400;
    }

    &.activeLang {
      &__label {
        font-weight: 600;
        color: $primary;
      }
    }
  }
}

.userInfo {
  align-items: flex-start;
  flex-direction: column;
  margin-left: 0.875rem;
  margin-right: 1rem;
  display: flex;

  .userName {
    line-height: 1.25rem;
    font-weight: 700;
  }

  .userRole {
    font-size: 0.75rem;
    line-height: 1rem;
    color: #565656;
    font-weight: 600;
  }
}

.headerMenu {
  margin-right: 1rem;

  .block {
    font-weight: 700;
    color: #404040;
  }
}

.languageWrapper {
  margin-right: 0.75rem;
  margin-left: 1rem;
}

.languageDdl {
  padding-right: 0.75rem;
  padding-left: 0.75rem;

  &.q-btn {
    .q-icon {
      & + .row {
        margin-left: 0.875rem;
        margin-right: 0.5rem;
      }
    }
  }
}

.q-list {
  &.leftNavList {
    .q-item {
      .q-icon {
        height: 1.5rem;
        width: 1.5rem;
      }

      &.q-router-link--active {
        .q-icon img {
          filter: brightness(100);
        }
      }
    }
  }

  .flagIcon img {
    border-radius: 3px;
    object-fit: cover;
    display: block;
    height: 16px;
    width: 24px;
  }

  .q-item {
    &__section {
      &--avatar {
        padding-right: 0.875rem;
        min-width: 1.5rem;
      }
    }

    &__label {
      font-weight: 400;
    }

    &.activeLang {
      &__label {
        font-weight: 600;
        color: $primary;
      }
    }
  }
}

.userInfo {
  align-items: flex-start;
  flex-direction: column;
  margin-left: 0.875rem;
  margin-right: 1rem;
  display: flex;

  .userName {
    line-height: 1.25rem;
    font-weight: 700;
  }

  .userRole {
    font-size: 0.75rem;
    line-height: 1rem;
    color: #565656;
    font-weight: 600;
  }
}

.dark-mode {
  background-color: $dark-page;
}

.languagelist {
  .q-item {
    padding: 4px 8px;
  }
}

.text-powered-by {
  float: left;
  display: inline-block;
  position: absolute;
  margin-top: 16px;
  margin-left: 0px;
}

.custom-text-logo {
  display: inline-block;
  float: left;
  position: absolute;
  margin-left: 72px !important;
  margin-top: 16px !important;
  width: 80px !important;
}
</style>
