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
  <q-card class="cols q-py-md q-pl-md q-pr-sm my-card">
    <q-card-section>
      <div class="row q-mt-sm justify-between items-center">
        <div class="row">
          <div v-if="planData.for == 'individuals'">
            <q-icon
              :name="'img:' + getImageURL('images/common/basic-icon.svg')"
              size="72px"
            />
          </div>
          <div v-if="planData.for == 'startups'">
            <q-icon
              :name="'img:' + getImageURL('images/common/pro_plan_icon.svg')"
              size="72px"
            />
          </div>

          <div class="q-ml-md q-mt-xs">
            <span class="row text-body1 text-weight-redular">
              <span v-if="planData.for == 'individuals'">{{
                t("billing.individuals")
              }}</span>
              <span v-if="planData.for == 'startups'">{{
                t("billing.startups")
              }}</span>
              <span v-if="planData.for == 'business'"
                >For {{ t("billing.business") }}</span
              >
            </span>
            <span class="row text-h5 text-weight-bold">
              <span class="secondaryColor" v-if="planData.type == config.freePlan">{{
                t("billing.basic")
              }}</span>
              <span class="primaryColor" v-if="planData.type == config.paidPlan">{{
                t("billing.pro")
              }}</span>
              <span class="primaryColor" v-if="planData.type == config.enterprisePlan">{{
                t("billing.business")
              }}</span>
            </span>
          </div>
        </div>
        <q-icon
          v-if="
            isPaidPlan == planData.type ||
            !isPaidPlan == planData.type ||
            isPaidPlan == planData.business
          "
          name="check_circle"
          size="md"
          color="primary"
        />
      </div>
      <!-- <div class="q-mt-md text-body1 text-weight-redular">
        {{ planData.description }}
      </div> -->
      <div class="q-mt-md text-body text-weight-redular text-h6">
        <span
          class="text-weight-bold free text-body"
          style="font-size: 30px"
          :class="{
            secondaryColor: planData.price == 'Free',
            primaryColor: planData.price != 'Free',
          }"
          >{{ planData.price }}</span
        >
        <span class="text-weight-regular text-body">
          /&nbsp;{{ planData.duration }}
        </span>
      </div>
      <div class="text-body1 text-weight-bold q-mt-md">
        {{ t("billing.whatIsIncluded") }}
      </div>
      <div
        class="text-body1 text-weight-bold q-mt-md"
        v-if="planData.type == config.paidPlan || planData.type == config.enterprisePlan"
      >
        {{ t("billing.everythingDeveloperPlan") }}
      </div>
      <div class="q-mt-lg q-mb-xl">
        <div class="q-mt-sm" v-for="rule in planData.included">
          <q-icon name="check_circle" size="20px" :color="cardColor" />
          <span class="q-ml-sm text-body1 text-weight-redular">{{ rule }}</span>
        </div>
      </div>
      <q-btn
        v-if="isPaidPlan !== planData.type"
        class="full-width card-btn"
        type="button"
        :color="cardColor"
        no-caps
        @click="onGetStartedSubscription(planData.type)"
        :disable="freeLoading || proLoading"
      >
        <div v-if="freeLoading || proLoading">
          <q-spinner-dots
            color="white"
            size="40px"
            style="margin: 0 auto; display: block"
          />
        </div>
        <div v-else>
          {{ planData.getStarted }}
        </div>
      </q-btn>
      <div
        v-if="isPaidPlan == planData.type"
        class="row justify-center items-center card-btn"
        :class="cardColor"
      >
        {{ planData.subscribed }}
      </div>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { computed, defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import { getImageURL } from "@/utils/zincutils";
import config from "@/aws-exports";

export default defineComponent({
  name: "PlanCard",
  props: ["plan", "isPaidPlan", "freeLoading", "proLoading"],
  setup(props, { emit }) {
    const { t } = useI18n();
    const store = useStore();
    const $q = useQuasar();
    const planData = ref(props?.plan);
    const cardColor = ref(props?.plan?.color);
    const loading = ref(false);

    const isPaidPlan = computed(() => {
      return props.isPaidPlan;
    });

    const onGetStartedSubscription = (planType: string) => {
      loading.value = true;
      if (planType == config.paidPlan) {
        emit("update:proSubscription");
      } else if (planType == config.freePlan) {
        emit("update:freeSubscription");
      } else if (planType == config.enterprisePlan) {
        emit("update:businessSubscription");
      }
    };

    return {
      t,
      planData,
      config,
      cardColor,
      isPaidPlan,
      onGetStartedSubscription,
      loading,
      getImageURL,
    };
  },
});
</script>
<style lang="scss" scoped>
.my-card {
  width: 394px;
  max-width: 394px;
  height: auto;
  border: 1px solid $card-border;
  box-shadow: 0px 2px 12px rgba(20, 20, 43, 0.08);
  border-radius: 24px;
  .secondaryColor {
    color: $secondary;
  }
  .primaryColor {
    color: $primary;
  }
  .free {
    font-size: 42px;
  }
  .card-btn {
    height: 72px;
    font-size: 18px;
    font-weight: 700;
    border-radius: 10px;
  }
  .secondary {
    border: 1px solid $secondary;
    color: $secondary;
  }
  .primary {
    border: 1px solid $primary;
    color: $primary;
  }
}
</style>
