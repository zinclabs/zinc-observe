import { ref } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import config from "../../aws-exports";

import { useLocalToken, getUserInfo, getImageURL } from "../../utils/zincutils";
import organizationService from "../../services/organizations";
import billingService from "../../services/billings";
import configService from "../../services/config";
import userService from "../../services/users";

import Tracker from "@openreplay/tracker";

const MainLayoutCloudMixin = {
  setup() {
    const store: any = useStore();
    const router = useRouter();
    const { t } = useI18n();

    const customOrganization = router.currentRoute.value.query.org_identifier;
    const selectedOrg = ref(store.state.selectedOrganization);
    const orgOptions = ref([{ label: Number, value: String }]);

    if (config.enableAnalytics == "true") {
      const tracker = new Tracker({
        projectKey: config.openReplayKey,
      });
      tracker.start();
      tracker.setUserID(store.state.userInfo.email);
    }

    const leftNavigationLinks = (linksList: any) => {
      linksList.value.splice(5, 0, {
        title: t("menu.function"),
        icon: "transform",
        link: "/functions",
      });
      linksList.value.splice(7, 0, {
        title: t("menu.organizations"),
        icon: "img:" + getImageURL("images/left_nav/organization_icon.svg"),
        link: "/organizations",
      });

      return linksList.value;
    };

    const getConfig = async () => {
      await configService
        .get_config()
        .then((res: any) => {
          store.dispatch("setConfig", res.data);
        })
        .catch((error) => console.log(error));
    };

    const getDefaultOrganization = async () => {
      await organizationService
        .list(0, 1000, "id", false, "")
        .then((res: any) => {
          store.dispatch("setOrganizations", res.data.data);
        })
        .catch((error) => console.log(error));
    };

    const getOrganizationThreshold = async () => {
      const organization: {
        identifier: "";
        subscription_type: "Free-Plan-USD-Monthly";
      } = store.state.selectedOrganization;
      if (organization.subscription_type == "Free-Plan-USD-Monthly") {
        await billingService
          .get_quota_threshold(organization.identifier)
          .then((res: any) => {
            const searchData: number = res.data.data.search;
            const ingestData: number = res.data.data.ingest;
            // res.data.data.forEach((element: any) => {
            //   if (element.event == "search") {
            //     searchData += element.size;
            //   } else if (element.event == "multi" || element.event == "bulk") {
            //     ingestData += element.size;
            //   }
            // });
            const searchNearThreshold = Math.floor(
              (store.state.selectedOrganization.search_threshold *
                parseInt(config.zincQuotaThreshold)) /
                100
            );

            const ingestNearThreshold = Math.floor(
              (store.state.selectedOrganization.ingest_threshold *
                parseInt(config.zincQuotaThreshold)) /
                100
            );
            let usageMessage = "";
            if (
              searchData > searchNearThreshold ||
              ingestData > ingestNearThreshold
            ) {
              if (searchNearThreshold >= 100 || ingestNearThreshold >= 100) {
                usageMessage =
                  "You’ve exceeded monthly free limit. Search: [SEARCH_USAGE]%, Ingestion: [INGEST_USAGE]%";
              } else {
                usageMessage =
                  "You’re approaching monthly free limit. Search: [SEARCH_USAGE]%, Ingestion: [INGEST_USAGE]%";
              }

              const percentageSearchQuota: any =
                store.state.selectedOrganization.search_threshold > 0
                  ? (
                      (searchData /
                        store.state.selectedOrganization.search_threshold) *
                      100
                    ).toFixed(2)
                  : 0;

              const percentageIngestQuota: any =
                store.state.selectedOrganization.ingest_threshold > 0
                  ? (
                      (ingestData /
                        store.state.selectedOrganization.ingest_threshold) *
                      100
                    ).toFixed(2)
                  : 0;

              usageMessage = usageMessage.replace(
                "[SEARCH_USAGE]",
                percentageSearchQuota <= 100 ? percentageSearchQuota : 100
              );
              usageMessage = usageMessage.replace(
                "[INGEST_USAGE]",
                percentageIngestQuota <= 100 ? percentageIngestQuota : 100
              );
            }
            // quotaThresholdMsg.value = usageMessage;
            store.dispatch("setQuotaThresholdMsg", usageMessage);
          })
          .catch((error) => console.log(error));
      }
    };

    const getRefreshToken = () => {
      userService
        .getRefreshToken()
        .then((res) => {
          useLocalToken(res.data.data.id_token);
          const sessionUserInfo: any = getUserInfo(
            "#id_token=" + res.data.data.id_token
          );

          const userInfo = sessionUserInfo !== null ? sessionUserInfo : null;
          if (userInfo !== null) {
            store.dispatch("login", {
              loginState: true,
              userInfo: userInfo,
            });
          }
          const d = new Date();
          const timeoutinterval = Math.floor(d.getTime() / 1000);
          const timeout =
            (store.state.userInfo.exp - timeoutinterval - 30) * 1000;
          setTimeout(() => {
            getRefreshToken();
          }, timeout);
        })
        .catch((e) => {
          console.log("Error while fetching refresh token:", e);
        });
    };

    getConfig();
    getDefaultOrganization();
    getOrganizationThreshold();

    return {
      t,
      orgOptions,
      selectedOrg,
      customOrganization,
      getImageURL,
      leftNavigationLinks,
      getRefreshToken,
    };
  },
};

export default MainLayoutCloudMixin;
