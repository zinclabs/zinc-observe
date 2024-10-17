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
  <div
    style="display: flex"
    @mouseover="isHovered = true"
    @mouseleave="isHovered = false"
  >
    <q-tabs
      v-model="selectedTabId"
      :align="'left'"
      dense
      inline-label
      outside-arrows
      mobile-arrows
      active-color="primary"
      @click.stop
      style="max-width: calc(100% - 40px)"
      data-test="dashboard-tab-list"
    >
      <q-tab
        no-caps
        :ripple="false"
        v-for="(tab, index) in tabs"
        :key="index"
        :name="tab.tabId"
        @click.stop
        content-class="tab_content"
        :data-test="`dashboard-tab-${tab.tabId}`"
      >
        <div class="full-width row justify-between no-wrap">
          <span
            style="
              white-space: nowrap;
              overflow: hidden;
              text-overflow: ellipsis;
              width: 100%;
            "
            :title="tab?.name"
            :data-test="`dashboard-tab-${tab.tabId}-name`"
            >{{ tab?.name }}</span
          >
        </div>
      </q-tab>
    </q-tabs>
    <q-btn
      v-if="!viewOnly"
      v-show="isHovered"
      class="text-bold no-border q-ml-xs"
      no-caps
      no-outline
      rounded
      icon="add"
      padding="xs"
      @click="
        () => {
          showAddTabDialog = true;
        }
      "
      data-test="dashboard-tab-add-btn"
      ><q-tooltip>Add Tab</q-tooltip></q-btn
    >
    <q-dialog v-model="showAddTabDialog" position="right" full-height maximized>
      <AddTab
        :dashboard-id="dashboardData?.dashboardId"
        @refresh="refreshDashboard"
      />
    </q-dialog>
  </div>
</template>

<script lang="ts">
import { computed, inject, ref } from "vue";
import { defineComponent } from "vue";
import AddTab from "@/components/dashboards/tabs/AddTab.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import { useRoute } from "vue-router";

export default defineComponent({
  name: "TabList",
  components: {
    AddTab,
    ConfirmDialog,
  },
  props: {
    dashboardData: {
      required: true,
      type: Object,
    },
    viewOnly: {
      type: Boolean,
      default: false,
    },
  },
  emits: ["refresh"],
  setup(props, { emit }) {
    const route = useRoute();
    const showAddTabDialog = ref(false);
    const isHovered = ref(false);

    // inject selected tab, default will be default tab
    const selectedTabId: any = inject("selectedTabId", ref("default"));

    const tabs: any = computed(() => {
      return props.dashboardData?.tabs ?? [];
    });

    const refreshDashboard = () => {
      emit("refresh");
      showAddTabDialog.value = false;
    };

    return {
      showAddTabDialog,
      refreshDashboard,
      tabs,
      route,
      isHovered,
      selectedTabId,
    };
  },
});
</script>

<style lang="scss" scoped>
.q-tabs {
  &--vertical {
    margin: 5px;
    .q-tab {
      justify-content: flex-start;
      padding: 0 1rem 0 1.25rem;
      border-radius: 0.5rem;
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
</style>
