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

<!-- eslint-disable vue/x-invalid-end-tag -->
<template>
  <q-page class="page">
    <div class="head q-table__title q-pb-md">
      {{ t("billing.header") }}
    </div>
    <q-separator class="separator" />
    <q-splitter
      v-model="splitterModel"
      unit="px"
      style="min-height: calc(100vh - 130px)"
    >
      <template v-slot:before>
        <q-tabs
          v-model="billingtab"
          indicator-color="transparent"
          inline-label
          vertical
        >
          <q-route-tab
            exact
            default
            name="usage"
            :to="
              '/billings/usage?org_identifier=' +
              store.state.selectedOrganization.identifier
            "
            :icon="'img:' + getImageURL('images/common/usage_icon.svg')"
            :label="t('billing.usageLabel')"
            content-class="tab_content"
          />
          <q-route-tab
            exact
            name="plans"
            :to="
              '/billings/plans?org_identifier=' +
              store.state.selectedOrganization.identifier
            "
            :icon="'img:' + getImageURL('images/common/plan_icon.svg')"
            :label="t('billing.plansLabel')"
            content-class="tab_content"
          />
          <q-route-tab
            exact
            name="invoice_history"
            :to="
              '/billings/invoice_history?org_identifier=' +
              store.state.selectedOrganization.identifier
            "
            :icon="'img:' + getImageURL('images/common/invoice_icon.svg')"
            :label="t('billing.invoiceHistoryLabel')"
            content-class="tab_content"
          />
        </q-tabs>
      </template>

      <template v-slot:after>
        <router-view title=""> </router-view>
      </template>
    </q-splitter>
  </q-page>
</template>

<script lang="ts">
// @ts-ignore
import { defineComponent, ref, onBeforeMount } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import config from "@/aws-exports";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import Usage from "./usage.vue";
import { getImageURL } from "@/utils/zincutils";

export default defineComponent({
  name: "PageIngestion",
  components: { ConfirmDialog, Usage },
  setup() {
    const { t } = useI18n();
    const store = useStore();
    const q = useQuasar();
    const router: any = useRouter();
    const billingtab = ref("usage");

    onBeforeMount(() => {
      if (router.currentRoute.value.name == "billings") {
        billingtab.value = "usage";
        router.push({ path: "/billings/usage" });
      }
      // else {
      //   billingtab.value = router.currentRoute.value.name;
      //   router.push({ path: "/billings/" + router.currentRoute.value.name });
      // }
    });

    return {
      t,
      store,
      router,
      config,
      billingtab,
      getImageURL,
      splitterModel: ref(200),
    };
  },
});
</script>

<style scoped lang="scss">
.page {
  padding: 1.5rem 1.5rem 0;
  .head {
    padding-bottom: 1rem;
  }
  .q-tabs {
    &--vertical {
      margin: 1.5rem 1rem 0 0;
      .q-tab {
        justify-content: flex-start;
        // padding: 0 1rem 0 1.25rem;
        border-radius: 0.5rem;
        margin-bottom: 0.5rem;
        // color: $dark;

        &__content.tab_content {
          .q-tab {
            &__icon + &__label {
              padding-left: 0.875rem;
              font-weight: 600;
            }
          }
        }
        &--active {
          background-color: $accent;
          color: black;
        }
      }
    }
  }
}
</style>
