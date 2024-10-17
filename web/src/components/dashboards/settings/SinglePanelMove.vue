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

<!-- eslint-disable vue/v-on-event-hyphenation -->
<!-- eslint-disable vue/attribute-hyphenation -->
<template>
  <q-dialog>
    <q-card style="width: 300px" data-test="dialog-box">
      <q-card-section class="confirmBody">
        <div class="head" data-test="dashboard-tab-move-title">{{ title }}</div>
        <div class="para" data-test="dashboard-tab-move-message">
          {{ message }}
        </div>
      </q-card-section>

      <div
        style="
          display: flex;
          flex-direction: row;
          width: 100%;
          height: 40px;
          padding-left: 10px;
          padding-right: 10px;
          padding-top: 5px;
        "
      >
        <q-select
          dense
          filled
          label="Select Tab"
          v-model="selectedMoveTabId"
          :options="moveTabOptions"
          class="select-container"
          data-test="dashboard-tab-move-select"
        >
          <!-- template when on options -->
          <template v-slot:no-option>
            <q-item data-test="dashboard-tab-move-select-no-option">
              <q-item-section class="text-italic text-grey">
                No Other Tabs Available
              </q-item-section>
            </q-item>
          </template>
        </q-select>

        <q-btn
          class="text-bold"
          no-caps
          outline
          rounded
          icon="add"
          style="padding: 10px; height: 40px; margin-left: 2px"
          @click="
            () => {
              isTabEditMode = false;
              showAddTabDialog = true;
            }
          "
          data-test="dashboard-tab-move-add-tab-btn"
          ><q-tooltip>Add Tab</q-tooltip></q-btn
        >
        <q-dialog
          v-model="showAddTabDialog"
          position="right"
          full-height
          maximized
        >
          <AddTab
            :edit-mode="isTabEditMode"
            :tabId="selectedTabIdToEdit"
            :dashboard-id="currentDashboardData.data.dashboardId"
            @refresh="refreshRequired"
            data-test="dashboard-tab-move-add-tab-dialog"
          />
        </q-dialog>
      </div>
      <q-card-actions class="confirmActions">
        <div class="button-container">
          <q-btn
            v-close-popup="true"
            unelevated
            no-caps
            class="q-mr-sm"
            @click="onCancel"
            data-test="cancel-button"
          >
            {{ t("confirmDialog.cancel") }}
          </q-btn>
          <q-btn
            v-close-popup="true"
            unelevated
            no-caps
            class="no-border"
            color="primary"
            @click="onConfirm"
            data-test="confirm-button"
            :disable="selectedMoveTabId === null"
          >
            Move
          </q-btn>
        </div>
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script lang="ts">
import { getDashboard } from "@/utils/commons";
import { reactive } from "vue";
import { onMounted } from "vue";
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";
import { useStore } from "vuex";
import AddTab from "@/components/dashboards/tabs/AddTab.vue";

export default defineComponent({
  name: "SinglePanelMove",
  components: { AddTab },
  emits: ["update:ok", "update:cancel", "refresh"],
  props: ["title", "message"],
  setup(props, { emit }) {
    const { t } = useI18n();
    const store = useStore();
    const route = useRoute();
    const action = ref("delete");
    const selectedMoveTabId: any = ref(null);
    const showAddTabDialog = ref(false);
    const isTabEditMode = ref(false);
    const selectedTabIdToEdit = ref(null);

    const moveTabOptions = ref([]);

    const currentDashboardData: any = reactive({
      data: {},
    });

    const getDashboardData = async () => {
      currentDashboardData.data = await getDashboard(
        store,
        route.query.dashboard,
        route.query.folder ?? "default"
      );
    };

    const getTabOptions = async () => {
      await getDashboardData();

      // update move tab options
      const newMoveTabOptions: any = [];
      currentDashboardData.data.tabs?.forEach((tab: any) => {
        // if tab is to be deleted, do not include it in the options
        if (tab.tabId != route.query.tab) {
          newMoveTabOptions.push({
            label: tab.name,
            value: tab.tabId,
          });
        }
      });

      moveTabOptions.value = newMoveTabOptions;
    };

    const refreshRequired = async (tabData: any) => {
      emit("refresh");
      // update move tab options
      await getTabOptions();

      // set selectedMoveTabId to newly created tab
      selectedMoveTabId.value = { label: tabData.name, value: tabData.tabId };

      // close add tab dialog
      showAddTabDialog.value = false;
    };

    onMounted(async () => {
      await getTabOptions();
      // set selectedMoveTabId to first tab from move tab options
      selectedMoveTabId.value =
        moveTabOptions.value.length > 0 ? moveTabOptions.value[0] : null;
    });

    const onCancel = () => {
      emit("update:cancel");
    };

    const onConfirm = () => {
      emit("update:ok", selectedMoveTabId.value.value);
    };
    return {
      t,
      onCancel,
      onConfirm,
      action,
      selectedMoveTabId,
      moveTabOptions,
      currentDashboardData,
      showAddTabDialog,
      isTabEditMode,
      selectedTabIdToEdit,
      refreshRequired,
    };
  },
});
</script>

<style lang="scss" scoped>
.select-container {
  width: 100%;
}
</style>
