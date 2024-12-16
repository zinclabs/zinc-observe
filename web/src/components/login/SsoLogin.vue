<template>
  <div style="max-width: 400px; margin-top: 100px" class="q-mx-auto q-pa-md">
    <div class="flex justify-center" style="height: 150px">
      <img
        class="appLogo"
        style="width: 250px; height: auto"
        :src="
          store?.state?.theme == 'dark'
            ? getImageURL('images/common/open_observe_logo_2.svg')
            : getImageURL('images/common/open_observe_logo.svg')
        "
      />
    </div>

    <div class="flex justify-center q-mt-lg">
      <q-btn
        data-test="sso-login-btn"
        class="text-bold no-border"
        padding="sm lg"
        color="primary"
        no-caps
        style="width: 400px"
      >
        <div
          data-test="sso-login-btn"
          class="flex items-center justify-center full-width text-center relative"
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

    <div class="q-mb-md q-mt-lg text-center">
      <a
        class="cursor-pointer login-internal-link"
        style="text-decoration: underline"
        @click="showLoginInput = !showLoginInput"
        >Sign in with an internal user</a
      >

      <div v-show="showLoginInput" class="o2-input login-inputs q-pt-lg">
        <q-form ref="loginform" class="q-gutter-md" @submit.prevent="">
          <q-input
            v-model="name"
            data-cy="login-user-id"
            data-test="login-user-id"
            outlined
            :label="`${t('login.userEmail')} *`"
            placeholder="Email"
            class="showLabelOnTop no-case"
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
              :label="t('login.signIn')"
              :loading="isSubmitting"
              no-caps
              @click="onSignIn()"
            />
          </div>
        </q-form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { getImageURL } from "@/utils/zincutils";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";

const store = useStore();

const { t } = useI18n();

const name = ref("");
const password = ref("");

const isSubmitting = ref(false);

const showLoginInput = ref(false);

const onSignIn = () => {
  console.log("onSignIn");
};
</script>

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

<style scoped lang="scss">
.login-internal-link {
  &:hover {
    color: #595959;
  }
}
</style>
