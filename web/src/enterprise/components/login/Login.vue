<!-- Copyright 2023 OpenObserve Inc.

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
  <div class="justify align-center">
    <h5>Wait while redirecting...</h5>
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, onBeforeMount, ref } from "vue";
import { useQuasar } from "quasar";
import { useRouter } from "vue-router";
import { useStore } from "vuex";
import { getUserInfo, getDecodedUserInfo, getPath } from "@/utils/zincutils";
import config from "../../../aws-exports";
import usersService from "@/services/users";
import organizationsService from "@/services/organizations";
import { useLocalCurrentUser, useLocalOrganization } from "@/utils/zincutils";
import segment from "@/services/segment_analytics";
import { openobserveRum } from "@openobserve/browser-rum";

export default defineComponent({
  name: "PageLoginCallback",
  setup() {
    const store = useStore();
    const q = useQuasar();
    const router = useRouter();
    const selectedOrg = ref("");
    let orgOptions = ref([{ label: Number, value: String }]);
    let moment: any;

    const importMoment = async () => {
      const momentModule: any = await import("moment");
      moment = momentModule.default;
    };

    onBeforeMount(async () => {
      await importMoment();
    });

    /**
     * Get all organizations for the user
     * check local storage for the default organization if user email is not same as local storage user email reset the local storage
     * there is no organization in localstorage, check for default organization of that user by using user email and set the default organization
     * if there is only one organization set that as default organization
     * redirect user to the page where user was redirected from
     */
    const getDefaultOrganization = () => {
      organizationsService
        .os_list(0, 100000, "id", false, "")
        .then((res: any) => {
          const localOrg: any = useLocalOrganization();
          if (
            localOrg.value != null &&
            localOrg.value.user_email !== store.state.userInfo.email
          ) {
            localOrg.value = null;
            useLocalOrganization("");
          }

          store.dispatch("setOrganizations", res.data.data);
          orgOptions.value = res.data.data.map(
            (data: {
              id: any;
              name: any;
              type: any;
              identifier: any;
              UserObj: any;
            }) => {
              let optiondata: any = {
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
                (selectedOrg.value == "" &&
                  (data.type == "default" || data.id == "1") &&
                  store.state.userInfo.email == data.UserObj.email) ||
                res.data.data.length == 1
              ) {
                selectedOrg.value = localOrg.value
                  ? localOrg.value
                  : optiondata;
                useLocalOrganization(selectedOrg.value);
                store.dispatch("setSelectedOrganization", selectedOrg.value);
              }
              return optiondata;
            }
          );

          const redirectURI = window.sessionStorage.getItem("redirectURI");
          window.sessionStorage.removeItem("redirectURI");
          redirectUser(redirectURI);
        });
    };

    /**
     * Redirect user to the page where user was redirected from
     * @param redirectURI
     */
    const redirectUser = (redirectURI) => {
      const path = getPath();
      if (redirectURI != null && redirectURI != "") {
        if (redirectURI.includes("http")) {
          window.location = redirectURI;
        } else {
          window.location.replace(redirectURI);
        }
      } else {
        // router.push({ path: "/" });
        window.location.replace(path);
      }
    };

    return {
      router,
      store,
      redirectUser,
      getDefaultOrganization,
    };
  },
  data() {
    return {
      user: {
        email: "",
        cognito_sub: "",
        first_name: "",
        last_name: "",
      },
      userInfo: {},
    };
  },
  created() {
    /**
     * Get the user info from the url hash
     * if user info is not null and user email is not null
     * set the user info to the user object
     * check the local storage for the user info
     * if user info is not null and user info has property pgdata
     * set the user info to the vuex store
     * else call the verify and create user method as user is not registered
     */
    const token = getUserInfo(this.$route.hash);
    if (token !== null && token.email != null) {
      this.user.email = token.email;
      this.user.cognito_sub = token.sub;
      this.user.first_name = token.given_name ? token.given_name : "";
      this.user.last_name = token.family_name ? token.family_name : "";
    } else {
      //redirect to error page
    }

    const sessionUserInfo = getDecodedUserInfo();
    const d = new Date();
    this.userInfo =
      sessionUserInfo !== null ? JSON.parse(sessionUserInfo) : null;
    if (
      (this.userInfo !== null && this.userInfo.hasOwnProperty("pgdata")) ||
      config.isEnterprise === "true"
    ) {
      this.store.dispatch("login", {
        loginState: true,
        userInfo: this.userInfo,
      });
      this.getDefaultOrganization();
    } else {
      this.store.dispatch("login", {
        loginState: true,
        userInfo: this.userInfo,
      });

      //analytics
      const userId = this.userInfo.email;
      segment.identify(
        "Log In",
        {
          email: userId,
          name: this.userInfo.given_name + " " + this.userInfo.family_name,
          firstName: this.userInfo.given_name,
          lastName: this.userInfo.family_name,
        },
        { originalTimestamp: moment.utc().format() }
      );

      openobserveRum.setUser({
        name: this.userInfo.given_name + " " + this.userInfo.family_name,
        email: this.userInfo.email,
      });
      this.getDefaultOrganization();
    }
  },
  methods: {
    /**
     * Verify the user by using user email
     * if user id is 0 then call the add new user method
     * else set the user info to the vuex store
     * get the default organization for the user
     */
    VerifyAndCreateUser() {
      usersService.verifyUser(this.userInfo.email).then((res) => {
        useLocalCurrentUser(res.data.data);
        this.store.dispatch("setCurrentUser", res.data.data);

        if (res.data.data.id == 0) {
          const dismiss = this.q.notify({
            spinner: true,
            message: "Please wait while creating new user...",
          });

          usersService.addNewUser(this.user).then((res) => {
            this.userInfo.pgdata = res.data.data;
            this.store.dispatch("login", {
              loginState: true,
              userInfo: this.userInfo,
            });
            dismiss();

            this.getDefaultOrganization();

            //analytics
            segment.identify(
              "Log In",
              {
                email: this.userInfo.email,
                name:
                  this.userInfo.given_name + " " + this.userInfo.family_name,
                firstName: this.userInfo.given_name,
                lastName: this.userInfo.family_name,
              },
              { originalTimestamp: moment.utc().format() }
            );

            openobserveRum.setUser({
              name: this.userInfo.given_name + " " + this.userInfo.family_name,
              email: this.userInfo.email,
            });
            //analytics
          });
        } else {
          this.userInfo.pgdata = res.data.data;
          this.store.dispatch("login", {
            loginState: true,
            userInfo: this.userInfo,
          });

          //analytics
          const userId = this.userInfo.email;
          segment.identify(
            "Log In",
            {
              email: userId,
              name: this.userInfo.given_name + " " + this.userInfo.family_name,
              firstName: this.userInfo.given_name,
              lastName: this.userInfo.family_name,
            },
            { originalTimestamp: moment.utc().format() }
          );

          openobserveRum.setUser({
            name: this.userInfo.given_name + " " + this.userInfo.family_name,
            email: this.userInfo.email,
          });
          //analytics

          this.getDefaultOrganization();
        }
      });
    },
  },
});
</script>
