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
  <login v-if="user.email == ''" />
</template>

<script lang="ts">
import { defineComponent, onBeforeMount, ref } from "vue";
import { useRouter } from "vue-router";
import Login from "@/components/login/Login.vue";
import config from "@/aws-exports";
import configService from "@/services/config";
import { useStore } from "vuex";
import { getUserInfo, getDecodedUserInfo } from "@/utils/zincutils";
import usersService from "@/services/users";
import organizationsService from "@/services/organizations";
import { useLocalCurrentUser, useLocalOrganization } from "@/utils/zincutils";
import { useQuasar } from "quasar";

export default defineComponent({
  name: "LoginPage",
  components: {
    Login,
  },
  setup() {
    const store = useStore();
    let orgOptions = ref([{ label: Number, value: String }]);
    const selectedOrg = ref({});
    const q = useQuasar();
    const router: any = useRouter();

    onBeforeMount(async () => {
      if (!router?.currentRoute.value.hash) {
        await configService
          .get_config()
          .then(async (res) => {
            store.commit("setConfig", res.data);
          })
          .catch((err) => {
            console.error("Error while fetching config:", err);
          });
      }
    });

    /**
     * Get all organizations for the user
     * check local storage for the default organization if user email is not same as local storage user email reset the local storage
     * there is no organization in localstorage, check for default organization of that user by using user email and set the default organization
     * if there is only one organization set that as default organization
     * redirect user to the page where user was redirected from
     */
    const getDefaultOrganization = () => {
      organizationsService.list(0, 100000, "id", false, "").then((res: any) => {
        const localOrg: any = useLocalOrganization();
        let tempDefaultOrg = {};
        let localOrgFlag = false;
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
            };

            if (
              (Object.keys(selectedOrg.value).length == 0 &&
                (data.type == "default" || data.id == "1") &&
                store.state.userInfo.email == data.UserObj.email) ||
              res.data.data.length == 1
            ) {
              localOrgFlag = true;
              selectedOrg.value = localOrg.value ? localOrg.value : optiondata;
              useLocalOrganization(selectedOrg.value);
              store.dispatch("setSelectedOrganization", selectedOrg.value);
            }

            if (data.type == "default") {
              tempDefaultOrg = optiondata;
            }

            return optiondata;
          }
        );

        if (localOrgFlag == false) {
          selectedOrg.value = tempDefaultOrg;
          useLocalOrganization(tempDefaultOrg);
          store.dispatch("setSelectedOrganization", tempDefaultOrg);
        }
        redirectUser();
      });
    };

    /**
     * Redirect user to the page where user was redirected from
     * @param redirectURI
     */
    const redirectUser = () => {
      const redirectURI = window.sessionStorage.getItem("redirectURI");
      window.sessionStorage.removeItem("redirectURI");
      if (redirectURI != null && redirectURI != "") {
        if (redirectURI.includes("http")) {
          window.location.href = redirectURI;
        } else {
          router.push({ path: redirectURI });
        }
      } else {
        router.push({ path: "/" });
      }
    };

    return {
      q,
      store,
      config,
      router,
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
      userInfo: {
        email: "",
      },
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
    if (this.$route.hash) {
      configService
        .get_config()
        .then(async (res) => {
          this.store.commit("setZoConfig", res.data);
          const token = getUserInfo(this.$route.hash);
          if (token !== null && token.email != null) {
            this.user.email = token.email;
            this.user.cognito_sub = token.sub;
            this.user.first_name = token.given_name ? token.given_name : "";
            this.user.last_name = token.family_name ? token.family_name : "";
          }

          const sessionUserInfo = getDecodedUserInfo();
          const d = new Date();
          this.userInfo =
            sessionUserInfo !== null
              ? JSON.parse(sessionUserInfo as string)
              : null;
          if (
            (this.userInfo !== null &&
              this.userInfo.hasOwnProperty("pgdata")) ||
            config.isEnterprise === "true"
          ) {
            this.store.dispatch("login", {
              loginState: true,
              userInfo: this.userInfo,
            });
            this.getDefaultOrganization();
            // this.redirectUser();
          } else {
            this.VerifyAndCreateUser();
          }
        })
        .catch((err) => {
          console.error("Error while fetching config:", err);
        });
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
      usersService.verifyUser(this.userInfo.email as string).then((res) => {
        useLocalCurrentUser(res.data.data);
        this.store.dispatch("setCurrentUser", res.data.data);

        if (res.data.data.id == 0) {
          const dismiss = this.q.notify({
            spinner: true,
            message: "Please wait while creating new user...",
          });

          usersService.addNewUser(this.user).then((res) => {
            this.store.dispatch("login", {
              loginState: true,
              userInfo: this.userInfo,
            });
            dismiss();

            this.getDefaultOrganization();
            // this.redirectUser();
          });
        } else {
          this.store.dispatch("login", {
            loginState: true,
            userInfo: this.userInfo,
          });

          this.getDefaultOrganization();
          // this.redirectUser();
        }
      });
    },
  },
});
</script>
