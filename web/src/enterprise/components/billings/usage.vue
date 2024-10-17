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
    <div class="row items-baseline justify-between">
      <div class="row text-body1 text-weight-medium q-pb-md">
        {{ t("billing.usage") }}
      </div>
      <div class="date-selector">
        <q-select
          dense
          outlined
          v-model="usageDate"
          :options="options"
          @update:model-value="(value: any) => selectUsageDate()"
        />
      </div>
    </div>
    <div v-if="!dataLoading">
      <div v-if="chartData?.options?.series?.length == 0">
        <div class="text-h6 text-weight-medium text-center">
          {{ t("billing.messageDataNotFound") }}
        </div>
      </div>
      <div v-else>
        <ChartRenderer
          id="billing-usage"
          :data="chartData"
          style="height: 400px"
        />
      </div>
    </div>
    <div v-else class="text-h6 text-weight-medium text-center">Loading...</div>
  </q-page>
</template>
<script lang="ts">
import { defineComponent, ref, onMounted, defineAsyncComponent } from "vue";
import { useStore } from "vuex";
import { useQuasar, date } from "quasar";
import { useI18n } from "vue-i18n";
import BillingService from "@/services/billings";
import { convertBillingData } from "@/utils/billing/convertBillingData";

let currentDate = new Date(); // Get the current date and time

// Subtract 30 days from the current date
let thirtyDaysAgo = new Date(currentDate.getTime() - 30 * 24 * 60 * 60 * 1000);

export default defineComponent({
  name: "Usage",
  components: {
    ChartRenderer: defineAsyncComponent(
      () => import("@/components/dashboards/panels/ChartRenderer.vue")
    ),
  },
  setup() {
    const { t } = useI18n();
    const $q = useQuasar();
    const store = useStore();
    const usageDate = ref("30days");
    const dataLoading = ref(false);
    let chartData: any = ref({});
    onMounted(() => {
      selectUsageDate();
    });
    const getUsage = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading usage data...",
      });
      dataLoading.value = true;
      BillingService.get_data_usage(
        store.state.selectedOrganization.identifier,
        usageDate.value
      )
        .then((res) => {
          dataLoading.value = false;
          chartData.value = convertBillingData(res?.data);
          dismiss();
        })
        .catch((e) => {
          $q.notify({
            type: "negative",
            message: e.message,
            timeout: 5000,
          });
        });
    };
    const selectUsageDate = () => {
      chartData.value = {};
      getUsage();
    };
    return {
      t,
      store,
      chartData,
      usageDate,
      dataLoading,
      options: ["30days", "60days", "3months", "6months"],
      selectUsageDate,
    };
  },
});
</script>
<style lang="scss" scoped>
.date-selector {
  width: 200px;
}
</style>
