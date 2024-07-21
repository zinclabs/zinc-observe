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
  <div style="max-width: 400px; margin-top: 100px" class="q-mx-auto q-pa-md">
    <div
      class="flex justify-center text-center"
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
        class="text-h6 text-bold q-pa-none cursor-pointer q-mr-sm full-width"
        >{{ store.state.zoConfig.custom_logo_text }}</span
      >
      <span class="full-width">
        <img
          v-if="
            store.state.zoConfig.hasOwnProperty('custom_logo_img') &&
            store.state.zoConfig?.custom_logo_img != null
          "
          :src="`data:image; base64, ` + store.state.zoConfig?.custom_logo_img"
          style="max-width: 150px; max-height: 31px"
        />
      </span>
      <img
        class="appLogo"
        style="height: auto"
        :style="
          store.state.zoConfig.custom_logo_text != ''
            ? 'width: 150px;'
            : 'width: 250px;'
        "
        :src="
          store?.state?.theme == 'dark'
            ? getImageURL('images/common/open_observe_logo_2.svg')
            : getImageURL('images/common/open_observe_logo.svg')
        "
      />
    </div>
    <div class="flex justify-center q-mb-lg" v-else>
      <img
        class="appLogo"
        style="height: auto"
        :style="
          store.state.zoConfig.custom_logo_text != ''
            ? 'width: 150px;'
            : 'width: 250px;'
        "
        :src="
          store?.state?.theme == 'dark'
            ? getImageURL('images/common/open_observe_logo_2.svg')
            : getImageURL('images/common/open_observe_logo.svg')
        "
      />
    </div>

    <div style="font-size: 22px" class="full-width text-center q-pb-md">
      Login
    </div>

    <div v-if="showSSO" class="flex justify-center">
      <q-btn
        data-test="sso-login-btn"
        class="text-bold no-border"
        padding="sm lg"
        color="primary"
        no-caps
        style="width: 400px"
        @click="loginWithSSo"
      >
        <div
          class="flex items-center justify-center full-width text-center realtive"
        >
          <img
            class="absolute"
            style="width: 30px; left: 16px"
            :src="getImageURL('images/common/sso.svg')"
          />
          <span class="text-center"> Login with SSO</span>
        </div>
      </q-btn>
    </div>

    <div v-if="showSSO && showInternalLogin" class="q-py-md text-center">
      <a
        class="cursor-pointer login-internal-link q-py-md"
        style="text-decoration: underline"
        @click="loginAsInternalUser = !loginAsInternalUser"
        >Login as internal user</a
      >
    </div>

    <div
      v-if="!showSSO || (showSSO && loginAsInternalUser && showInternalLogin)"
      class="o2-input login-inputs"
    >
      <q-form ref="loginform" class="q-gutter-md" @submit.prevent="">
        <q-input
          v-model="name"
          data-cy="login-user-id"
          data-test="login-user-id"
          outlined
          :label="`${t('login.userEmail')} *`"
          placeholder="Email"
          class="showLabelOnTop no-case"
          type="email"
          dense
          stack-label
          filled
        />

        <q-input
          v-model="password"
          data-cy="login-password"
          data-test="login-password"
          outlined
          :label="`${t('login.password')} *`"
          placeholder="Password"
          class="showLabelOnTop no-case"
          type="password"
          dense
          stack-label
          filled
        />

        <div class="q-mt-lg q-mb-xl">
          <q-btn
            data-cy="login-sign-in"
            unelevated
            class="full-width text-bold no-border"
            color="primary"
            type="submit"
            padding="sm lg"
            :label="t('login.login')"
            :loading="submitting"
            no-caps
            @click="onSignIn()"
          />
        </div>
      </q-form>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, type Ref, onBeforeMount } from "vue";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import { useRouter } from "vue-router";

import { useI18n } from "vue-i18n";
import authService from "@/services/auth";
import organizationsService from "@/services/organizations";
import {
  getBasicAuth,
  b64EncodeStandard,
  useLocalUserInfo,
  useLocalCurrentUser,
  useLocalOrganization,
  getImageURL,
} from "@/utils/zincutils";
import { redirectUser } from "@/utils/common";
import { computed } from "vue";
import config from "@/aws-exports";

export default defineComponent({
  name: "PageLogin",

  setup() {
    const store = useStore();
    const router = useRouter();
    const $q = useQuasar();
    const { t } = useI18n();
    const name = ref("");
    const password = ref("");
    const confirmpassword = ref("");
    const email = ref("");
    const loginform = ref();

    const submitting = ref(false);

    const loginAsInternalUser = ref(false);

    onBeforeMount(() => {
      if (router.currentRoute.value.query.login_as_internal_user === "true") {
        loginAsInternalUser.value = true;
      }
    });

    const showSSO = computed(() => {
      return store.state.zoConfig.sso_enabled && config.isEnterprise === "true";
    });

    const showInternalLogin = computed(() => {
      return store.state.zoConfig.native_login_enabled;
    });

    const loginWithSSo = async () => {
      try {
        authService.get_dex_login().then((res) => {
          if (res) {
            window.location.href = res;
            return;
          }
        });
      } catch (error) {
        console.error("Error during redirection:");
      }
    };

    const onSignIn = () => {
      if (name.value == "" || password.value == "") {
        $q.notify({
          position: "top",
          color: "warning",
          textColor: "white",
          icon: "warning",
          message: "Please input valid username or password.",
        });
      } else {
        submitting.value = true;
        try {
          //authorize user using username and password
          authService
            .sign_in_user({
              email: name.value,
              password: password.value,
            })
            .then(async (res: any) => {
              //if user is authorized, get user info
              if (res.data.status == true) {
                //get user info from backend and extract auth token and set it into localstorage
                const authToken = getBasicAuth(name.value, password.value);
                const userInfo = {
                  given_name: res.data.given_name,
                  auth_time: Math.floor(Date.now() / 1000),
                  username: res.data.username,
                  exp: Math.floor(
                    (new Date().getTime() + 1000 * 60 * 60 * 24 * 30) / 1000
                  ),
                  family_name: res.data.family_name,
                  email: name.value,
                  role: res.data.role,
                };
                const encodedUserInfo: any = b64EncodeStandard(
                  JSON.stringify(userInfo)
                );

                //set user info into localstorage & store
                useLocalUserInfo(encodedUserInfo);
                store.dispatch("setUserInfo", encodedUserInfo);

                useLocalCurrentUser(JSON.stringify(userInfo));
                store.dispatch("setCurrentUser", userInfo);

                //check for redirect URI and redirect user to that page
                const redirectURI =
                  window.sessionStorage.getItem("redirectURI");
                window.sessionStorage.removeItem("redirectURI");

                const selectedOrg: Ref<{
                  label: string;
                  id: string;
                  type: string;
                  identifier: string;
                  user_email: string;
                  ingest_threshold: string;
                  search_threshold: string;
                } | null> = ref(null);

                //check organization information stored in localstorage along with email
                //if email is different, remove organization information from localstorage
                const localOrg: any = useLocalOrganization();
                if (
                  localOrg.value != null &&
                  localOrg.value.user_email !== userInfo.email
                ) {
                  localOrg.value = null;
                  useLocalOrganization("");
                }

                //if organization information is not available in localstorage, get all organizations from backend
                //and set first organization as selected organization
                if (localOrg.value) {
                  selectedOrg.value = localOrg.value;
                } else {
                  await organizationsService
                    .os_list(0, 100000, "id", false, "", "default")
                    .then((res: any) => {
                      if (res.data.data.length) {
                        const firstOrg = res.data.data[0];
                        selectedOrg.value = {
                          label: firstOrg.name,
                          id: firstOrg.id,
                          type: firstOrg.type,
                          identifier: firstOrg.identifier,
                          user_email: firstOrg.user_email,
                          ingest_threshold: firstOrg.ingest_threshold,
                          search_threshold: firstOrg.search_threshold,
                        };
                      }
                    });
                }

                //if selected organization is available, set it into localstorage and store
                if (selectedOrg.value) {
                  useLocalOrganization(selectedOrg.value);
                  store.dispatch("setSelectedOrganization", selectedOrg.value);
                  redirectUser(redirectURI);
                }
              } else {
                //if user is not authorized, show error message and reset form.
                submitting.value = false;
                loginform.value.resetValidation();
                $q.notify({
                  color: "negative",
                  message: res.data.message,
                });
              }
            })
            .catch((e: Error) => {
              //if any error occurs, show error message and reset form.
              submitting.value = false;
              loginform.value.resetValidation();
              $q.notify({
                color: "negative",
                message: "Invalid username or password",
                timeout: 4000,
              });
              console.log(e);
            });
        } catch (e) {
          submitting.value = false;
          loginform.value.resetValidation();
          $q.notify({
            color: "negative",
            message: "Please fill all the fields and try again.",
          });
          console.log(e);
        }
      }
    };

    return {
      t,
      name,
      password,
      confirmpassword,
      email,
      loginform,
      submitting,
      onSignIn,
      tab: ref("signin"),
      innerTab: ref("signup"),
      store,
      getImageURL,
      loginAsInternalUser,
      showSSO,
      showInternalLogin,
      loginWithSSo,
      config,
    };
  },
  methods: {
    selected(item: any) {
      this.$q.notify(`Selected suggestion "${item.label}"`);
    },
  },
});
</script>

<style lang="scss">
.login-internal-link {
  &:hover {
    color: #595959;
  }
}

.my-card {
  width: 400px;
}
</style>

<style lang="scss">
.login-inputs {
  .q-field__label {
    font-weight: normal !important;
    font-size: 12px;
    transform: translate(-0.75rem, -155%);
    color: #3a3a3a;
  }
}
</style>
