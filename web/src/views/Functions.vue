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
  <q-page class="q-pa-none" style="min-height: inherit">
    <q-btn
      data-test="logs-search-field-list-collapse-btn"
      :icon="showSidebar ? 'chevron_left' : 'chevron_right'"
      :title="showSidebar ? 'Collapse Fields' : 'Open Fields'"
      dense
      size="12px"
      round
      class="q-mr-xs field-list-collapse-btn tw-absolute tw-top-0 tw-z-10"
      color="primary"
      :style="{
        left: showSidebar ? splitterModel - 14 + 'px' : '-8px',
      }"
      @click="collapseSidebar"
    />
    <q-splitter
      v-model="splitterModel"
      unit="px"
      :limits="[0, 300]"
      class="tw-overflow-hidden"
    >
      <template v-slot:before>
        <div v-if="showSidebar" class="functions-tabs">
          <q-tabs
            v-model="activeTab"
            indicator-color="transparent"
            inline-label
            vertical
          >
            <q-route-tab
              v-if="
                !store.state.zoConfig?.custom_hide_menus
                  ?.split(',')
                  .includes('pipelines')
              "
              data-test="stream-pipelines-tab"
              name="streamPipelines"
              :to="{
                name: 'pipelines',
                query: {
                  org_identifier: store.state.selectedOrganization.identifier,
                },
              }"
              :label="t('function.streamPipeline')"
              content-class="tab_content"
            />
            <q-route-tab
              data-test="function-stream-tab"
              default
              name="functions"
              :to="{
                name: 'functionList',
                query: {
                  org_identifier: store.state.selectedOrganization.identifier,
                },
              }"
              :label="t('function.header')"
              content-class="tab_content"
            />
            <q-route-tab
              data-test="function-enrichment-table-tab"
              name="enrichmentTables"
              :to="{
                name: 'enrichmentTables',
                query: {
                  org_identifier: store.state.selectedOrganization.identifier,
                },
              }"
              :label="t('function.enrichmentTables')"
              content-class="tab_content"
            />
          </q-tabs>
        </div>
      </template>
      <template v-slot:after>
        <div
          class="tw-overflow-auto q-mx-sm q-my-sm"
          style="height: calc(100vh - 57px)"
        >
          <!-- :templates="templates"
            :functionAssociatedStreams="functionAssociatedStreams"
            @get:functionAssociatedStreams="getFunctionAssociatedStreams"
            @get:templates="getTemplates" -->
          <RouterView />
        </div>
      </template>
    </q-splitter>
  </q-page>
</template>

<script lang="ts">
import { defineComponent, ref, onBeforeMount, watch } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";

export default defineComponent({
  name: "AppFunctions",
  setup() {
    const store = useStore();
    const { t } = useI18n();
    const router = useRouter();
    const activeTab: any = ref("streamPipelines");
    const templates = ref([]);
    const functionAssociatedStreams = ref([]);
    const splitterModel = ref(220);

    const lastSplitterPosition = ref(splitterModel.value);

    const showSidebar = ref(true);

    watch(
      () => router.currentRoute.value,
      (currentRoute: any) => {
        if (
          currentRoute.name === "functionList" &&
          currentRoute.query.action === "add"
        ) {
          if (showSidebar.value) collapseSidebar();
        }
      },
    );

    watch(
      () => router.currentRoute.value.name,
      (routeName) => {
        // This is added to redirect to functionList if the user is on functions route
        // This case happens when user clicks on functions from menu when he is already on functions page
        if (routeName === "pipeline") router.back();
      },
    );

    onBeforeMount(() => {
      redirectRoute();
    });

    const collapseSidebar = () => {
      if (showSidebar.value) lastSplitterPosition.value = splitterModel.value;
      showSidebar.value = !showSidebar.value;
      splitterModel.value = showSidebar.value ? lastSplitterPosition.value : 0;
    };

    const redirectRoute = () => {
      if (router.currentRoute.value.name === "pipeline") {
        router.replace({
          name: "pipelines",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      }
    };

    return {
      t,
      store,
      router,
      redirectRoute,
      splitterModel,
      functionAssociatedStreams,
      activeTab,
      templates,
      collapseSidebar,
      showSidebar,
    };
  },
});
</script>

<style scoped lang="scss">
:deep(.q-splitter__before) {
  overflow: visible;
}

.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}
.functions-tabs {
  .q-tabs {
    &--vertical {
      margin: 20px 16px 0 16px;
      .q-tab {
        justify-content: flex-start;
        padding: 0 1rem 0 1.25rem;
        border-radius: 0.5rem;
        margin-bottom: 0.5rem;
        text-transform: capitalize;
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
