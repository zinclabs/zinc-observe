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
  <q-page class="q-pa-md">
    <div class="row justify-between items-center q-pl-xl q-pr-xl">
      <div class="text-body1 text-weight-medium">
        {{ t("billing.plans") }}
      </div>
      <div>
        <q-btn
          v-if="listSubscriptionResponse.hasOwnProperty('card')"
          class="q-ml-md q-mb-xs text-bold"
          outline
          padding="sm lg"
          color="white"
          text-color="black"
          no-caps
          :label="t('billing.manageCards')"
          @click="
            onChangePaymentDetail(
              listSubscriptionResponse.card.gateway_account_id
            )
          "
        />
      </div>
    </div>
    <div
      class="row justify-start text-h6 text-weight-bold q-pl-xl q-pb-xs subtitle"
    >
      {{ t("billing.subtitle") }}
    </div>
    <div
      v-if="
        store.state.selectedOrganization.hasOwnProperty('note') &&
        store.state.selectedOrganization.note
      "
      class="row justify-start warning-message text-negative text-h6 q-pl-xl q-pb-lg"
    >
      <q-icon name="warning" class="q-pt-sm"></q-icon
      >{{ store.state.selectedOrganization.note }}
    </div>
    <div v-if="loading">
      <q-spinner-dots
        color="primary"
        size="40px"
        style="margin: 0 auto; display: block"
      />
    </div>
    <div v-else class="row q-gutter-md justify-center">
      <plan-card
        v-for="plan in Plans"
        :key="plan.id"
        :plan="plan"
        :isPaidPlan="planType"
        :freeLoading="freeLoading"
        :proLoading="proLoading"
        @update:freeSubscription="subscribeFreePlan"
        @update:proSubscription="onLoadSubscription('pro')"
        @update:businessSubscription="onLoadSubscription('business')"
      ></plan-card>
    </div>

    <!-- <div v-if="listSubscriptionResponse.card" style="min-height: 80%"> -->
    <q-dialog v-model="changePayment">
      <q-card style="width: 500px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-body1 text-weight-medium">
            {{ t("billing.manageCards") }}
          </div>
          <q-space />
          <q-btn icon="close" flat round dense v-close-popup="true" />
        </q-card-section>
        <q-card-section>
          <iframe
            ref="updatesubscriptionref"
            title="Update Subscription checkout"
            v-if="updatePaymentResponse"
            :src="updatePaymentResponse.url"
            allowfullscreen
            frameborder="0"
            style="min-height: 70vh; min-width: 100%"
          ></iframe>
          <div v-else>Loading...</div>
        </q-card-section>
      </q-card>
    </q-dialog>
    <!-- </div> -->
    <!-- <div v-if="!isActiveSubscription && !isProPlan && hostedResponse.urlƒ"> -->
    <q-dialog v-model="subScribePlan">
      <q-card style="width: 500px">
        <q-card-section class="row items-center q-pb-none">
          <div class="text-body1 text-weight-medium">
            {{ t("billing.subscriptionCheckout") }}
          </div>
          <q-space />
          <q-btn icon="close" flat round dense v-close-popup="true" />
        </q-card-section>

        <q-card-section>
          <iframe
            ref="subscriptionref"
            title="Subscription checkout"
            v-if="!isActiveSubscription && hostedResponse.url"
            :src="hostedResponse.url"
            allowfullscreen
            frameborder="0"
            style="min-height: 70vh; min-width: 100%"
          ></iframe>
          <div v-else>Loading...</div>
        </q-card-section>
      </q-card>
    </q-dialog>

    <q-dialog v-model="confirm_downgrade_subscription" persistent>
      <q-card>
        <q-card-section class="row items-center">
          <span class="q-ml-sm"
            ><q-avatar
              icon="warning"
              size="sm"
              color="primary"
              text-color="white"
              class="q-mr-sm"
            />{{ t("billing.downgradeMessage") }}</span
          >
        </q-card-section>

        <q-card-actions align="right">
          <q-btn
            :label="t('common.cancel')"
            color="secondary"
            v-close-popup="true"
          />
          <q-btn
            :label="t('common.confirm')"
            color="primary"
            v-close-popup="true"
            @click="onUnsubscribe"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <!-- </div> -->
  </q-page>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import PlanCard from "./planCard.vue";
import Plan from "@/constants/plans";
import BillingService from "@/services/billings";
import { useStore } from "vuex";
import { useQuasar, date } from "quasar";
import { useLocalOrganization, convertToTitleCase } from "@/utils/zincutils";
import config from "@/aws-exports";

export default defineComponent({
  name: "plans",
  components: {
    PlanCard,
  },
  emits: ["update:freeSubscription", "update:proSubscription"],
  mounted() {
    this.loading = true;
    this.loadSubscription();
  },
  methods: {
    subscribeFreePlan() {
      if (
        this.currentPlanDetail.CustomerBillingObj.subscription_type ==
        config.freePlan
      ) {
        this.onLoadSubscription("Developer");
      } else {
        this.confirm_downgrade_subscription = true;
      }
    },
    onLoadSubscription(planType: string) {
      this.proLoading = true;
      if (this.listSubscriptionResponse.card != undefined) {
        BillingService.resume_subscription(
          this.store.state.selectedOrganization.identifier
        )
          .then((res) => {
            this.loadSubscription(true);
          })
          .catch((e) => {
            this.proLoading = false;
            this.$q.notify({
              type: "negative",
              message: e.message,
              timeout: 5000,
            });
          });
      } else {
        BillingService.get_hosted_url(
          this.store.state.selectedOrganization.identifier,
          convertToTitleCase(planType)
        )
          .then((res) => {
            console.log(res);
            // alert(res.data.data.url)
            window.location.href = res.data.data.url;
            // this.isActiveSubscription = false;
            // this.subScribePlan = true;
            // this.hostedResponse = res.data.data.url;
            // setInterval(this.retrieveHostedPage, 5000);
            // this.loadSubscription(true);
          })
          .catch((e) => {
            this.$q.notify({
              type: "negative",
              message: e.message,
              timeout: 5000,
            });
          });
      }
    },
    onUnsubscribe() {
      this.freeLoading = true;
      BillingService.unsubscribe(
        this.store.state.selectedOrganization.identifier
      )
        .then((res) => {
          this.loadSubscription();
          this.$router.go(0);
        })
        .catch((e) => {
          this.freeLoading = false;
          this.$q.notify({
            type: "negative",
            message: e.message,
            timeout: 5000,
          });
        });
    },
    onChangePaymentDetail(gatewayId: string) {
      this.changePayment = true;
      BillingService.change_payment_detail(
        this.store.state.selectedOrganization.identifier,
        gatewayId
      )
        .then((res) => {
          this.updatePaymentResponse = res.data.data.hosted_page;
          setInterval(this.retrieveHostedPage, 5000);
        })
        .catch((e) => {
          this.$q.notify({
            type: "negative",
            message: e.message,
            timeout: 5000,
          });
        });
    },
    loadSubscription(fromPro = false) {
      BillingService.list_subscription(
        this.store.state.selectedOrganization.identifier
      )
        .then((res) => {
          this.currentPlanDetail = res.data.data;

          if (
            res.data.data.CustomerBillingObj.customer_id !== "" &&
            res.data.data.CustomerBillingObj.customer_id !== null
          ) {
            if (
              res.data.data.CustomerBillingObj.subscription_type ==
              "professional-USD-Monthly"
            ) {
              this.planType = "pro";
              const localOrg: any = useLocalOrganization();
              localOrg.value.subscription_type = "professional-USD-Monthly";
              useLocalOrganization(localOrg.value);
              this.store.dispatch("setSelectedOrganization", localOrg.value);
              this.store.dispatch("setQuotaThresholdMsg", "");
            } else if (
              res.data.data.CustomerBillingObj.subscription_type ==
              config.freePlan
            ) {
              this.planType = "basic";
              const localOrg: any = useLocalOrganization();
              localOrg.value.subscription_type = config.freePlan;
              useLocalOrganization(localOrg.value);
              this.store.dispatch("setSelectedOrganization", localOrg.value);
            } else if (
              res.data.data.CustomerBillingObj.subscription_type ==
              "business-USD-Monthly"
            ) {
              this.planType = "business";
              const localOrg: any = useLocalOrganization();
              localOrg.value.subscription_type = "professional-USD-Monthly";
              useLocalOrganization(localOrg.value);
              this.store.dispatch("setSelectedOrganization", localOrg.value);
              this.store.dispatch("setQuotaThresholdMsg", "");
            }
          } else {
            this.$q.notify({
              type: "warning",
              message: "Please subscribe to one of the plan.",
              timeout: 5000,
            });
          }
          // this.listSubscriptionResponse = res.data.data;
          // this.listSubscriptionResponse.subscription.current_term_end =
          //   date.formatDate(
          //     Math.floor(
          //       this.listSubscriptionResponse.subscription.current_term_end *
          //         1000
          //     ),
          //     "MMM DD, YYYY"
          //   );
          // this.listSubscriptionResponse.subscription.current_term_start =
          //   date.formatDate(
          //     Math.floor(
          //       this.listSubscriptionResponse.subscription.current_term_start *
          //         1000
          //     ),
          //     "MMM DD, YYYY"
          //   );
          // res.data.data.subscription.subscription_items.forEach(
          //   (element: any) => {
          //     if (element.item_price_id == "professional-USD-Monthly") {
          //       this.isProPlan = true;
          //       const localOrg: any = useLocalOrganization();
          //       localOrg.value.subscription_type = "professional-USD-Monthly";
          //       useLocalOrganization(localOrg.value);
          //       this.store.dispatch("setSelectedOrganization", localOrg.value);
          //       this.store.dispatch("setQuotaThresholdMsg", "");
          //     } else if (element.item_price_id == "Free-Plan-USD-Monthly") {
          //       this.isProPlan = false;
          //       const localOrg: any = useLocalOrganization();
          //       localOrg.value.subscription_type = "Free-Plan-USD-Monthly";
          //       useLocalOrganization(localOrg.value);
          //       this.store.dispatch("setSelectedOrganization", localOrg.value);
          //     }
          //   }
          // );
          // if (
          //   res.data.data.card &&
          //   res.data.data.card.payment_source_id != ""
          // ) {
          //   this.isActiveSubscription = true;
          // } else {
          //   BillingService.get_hosted_url(
          //     this.store.state.selectedOrganization.identifier
          //   )
          //     .then((res) => {
          //       this.hostedResponse = res.data.data.hosted_page;
          //       setInterval(this.retrieveHostedPage, 5000);
          //     })
          //     .catch((e) => {
          //       this.$q.notify({
          //         type: "negative",
          //         message: e.message,
          //         timeout: 5000,
          //       });
          //     });
          // }
          this.loading = false;
          this.freeLoading = false;
          this.proLoading = false;
          this.$router.push({
            name: "plans",
            query: { update_org: Date.now() },
          });
          // if (
          //   fromPro &&
          //   !this.isActiveSubscription &&
          //   !this.isProPlan &&
          //   this.hostedResponse.url
          // ) {
          //   this.subScribePlan = true;
          // }
        })
        .catch((e) => {
          this.loading = false;
          this.freeLoading = false;
          this.proLoading = false;

          this.$q.notify({
            type: "negative",
            message: e.message,
            timeout: 5000,
          });
        });
    },
  },
  setup() {
    const { t } = useI18n();
    const store = useStore();
    const $q = useQuasar();
    const frmPayment = ref();
    const planType = ref("");
    const isActiveSubscription = ref(false);
    const loading = ref(false);
    const hostedResponse: any = ref();
    const updatePaymentResponse: any = ref();
    const subscriptionref = ref();
    const listSubscriptionResponse: any = ref({});
    const Plans = Plan;
    const changePayment: any = ref(false);
    const subScribePlan: any = ref(false);
    const freeLoading: any = ref(false);
    const proLoading: any = ref(false);
    const confirm_downgrade_subscription: any = ref(false);
    const currentPlanDetail = ref();

    const retrieveHostedPage = () => {
      BillingService.retrieve_hosted_page(
        store.state.selectedOrganization.identifier,
        hostedResponse.value.id
      ).then((res) => {
        if (res.data.data.hosted_page.state == "succeeded") {
          window.location.reload();
        }
        // clearTimeout(hostedPageTimeout);
      });
    };

    return {
      t,
      store,
      frmPayment,
      planType,
      isActiveSubscription,
      loading,
      hostedResponse,
      subscriptionref,
      listSubscriptionResponse,
      updatePaymentResponse,
      retrieveHostedPage,
      Plans,
      changePayment,
      subScribePlan,
      freeLoading,
      proLoading,
      confirm_downgrade_subscription,
      currentPlanDetail,
    };
  },
});
</script>
<style lang="scss" scoped>
.subtitle {
  color: $primary;
}
</style>
