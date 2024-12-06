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
  <q-page data-test="alerts-page" class="q-pa-none" style="min-height: inherit">
    <RouterView
      :templates="templates"
      :destinations="destinations"
      @get:destinations="getDestinations"
      @get:templates="getTemplates"
    />
  </q-page>
</template>

<script lang="ts">
import { defineComponent, ref, onBeforeMount, watch } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import templateService from "@/services/alert_templates";
import destinationService from "@/services/alert_destination";

export default defineComponent({
  name: "AppAlerts",
  setup() {
    const store = useStore();
    const { t } = useI18n();
    const router = useRouter();
    const activeTab: any = ref("destinations");
    const templates = ref([]);
    const destinations = ref([]);
    const splitterModel = ref(160);

    onBeforeMount(() => {
      redirectRoute();
    });

    watch(
      () => router.currentRoute.value.name,
      (routeName) => {
        // This is handled for browser back button, as it was redirecting to this app alerts page and not alert list page
        if (routeName === "alerts") router.back();
      }
    );

    const redirectRoute = () => {
      if (router.currentRoute.value.name === "alerts") {
        router.push({
          name: "alertList",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      }
    };
    const getTemplates = () => {
      // if (store.state.selectedOrganization.status == "active") {
      templateService
        .list({
          org_identifier: store.state.selectedOrganization.identifier,
        })
        .then((res) => (templates.value = res.data));
      // }
    };
    const getDestinations = () => {
      // if (store.state.selectedOrganization.status == "active") {
      destinationService
        .list({
          org_identifier: store.state.selectedOrganization.identifier,
        })
        .then((res) => (destinations.value = res.data));
      // }
    };

    return {
      activeTab,
      templates,
      destinations,
      splitterModel,
      getTemplates,
      getDestinations,
      redirectRoute,
      t,
      store,
    };
  },
});
</script>

<style scoped lang="scss">
.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}
.alerts-tabs {
  .q-tabs {
    &--vertical {
      margin: 16px 8px 0 8px;
      .q-tab {
        justify-content: flex-start;
        padding: 0 1rem 0 1.25rem;
        border-radius: 0.5rem;
        margin-bottom: 0.5rem;
        text-transform: capitalize;
        min-height: 40px !important;
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
