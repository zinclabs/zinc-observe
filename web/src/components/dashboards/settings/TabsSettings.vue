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
  <div class="column full-height" data-test="dashboard-tab-settings">
    <DashboardHeader :title="t('dashboard.tabSettingsTitle')"
      ><template #right>
        <q-btn
          class="text-bold no-border q-ml-md"
          no-caps
          no-outline
          rounded
          color="secondary"
          :label="t(`dashboard.newTab`)"
          @click.stop="addNewItem"
          data-test="dashboard-tab-settings-add-tab"
        /> </template
    ></DashboardHeader>
    <div class="flex justify-between q-py-md q-mb-sm text-bold border-bottom">
      <div class="q-ml-xl" data-test="dashboard-tab-settings-name">
        {{ t("dashboard.name") }}
      </div>
      <div class="q-mr-lg" data-test="dashboard-tab-settings-actions">
        {{ t("dashboard.actions") }}
      </div>
    </div>
    <div>
      <draggable
        v-model="currentDashboardData.data.tabs"
        :options="dragOptions"
        @end.stop="handleDragEnd"
        @mousedown.stop="() => {}"
        data-test="dashboard-tab-settings-drag"
      >
        <div
          v-for="(tab, index) in currentDashboardData.data.tabs"
          :key="index"
          class="draggable-row"
        >
          <div class="draggable-handle">
            <q-icon
              name="drag_indicator"
              color="grey-13"
              class="'q-mr-xs"
              data-test="dashboard-tab-settings-drag-handle"
            />
          </div>
          <div class="draggable-content">
            <span
              v-if="tab.tabId !== editTabId"
              data-test="dashboard-tab-settings-tab-name"
              >{{ tab.name }}</span
            >
            <div v-else style="display: flex; flex-direction: row">
              <input
                v-model="editTabObj.data.name"
                class="edit-input"
                data-test="dashboard-tab-settings-tab-name-edit"
              />
              <q-btn
                icon="check"
                class="q-ml-xs"
                unelevated
                size="sm"
                round
                flat
                :title="t('dashboard.save')"
                @click.stop="saveEdit"
                :disable="!editTabObj.data.name.trim()"
                data-test="dashboard-tab-settings-tab-name-edit-save"
              ></q-btn>
              <q-btn
                icon="close"
                class="q-ml-xs"
                unelevated
                size="sm"
                round
                flat
                :title="t('dashboard.cancel')"
                @click.stop="cancelEdit"
                data-test="dashboard-tab-settings-tab-name-edit-cancel"
              ></q-btn>
            </div>
            <span class="q-ml-lg">
              <q-btn
                icon="edit"
                class="q-ml-xs"
                padding="sm"
                unelevated
                size="sm"
                round
                flat
                :disabled="tab.tabId === editTabId"
                :title="t('dashboard.edit')"
                @click.stop="editItem(tab.tabId)"
                data-test="dashboard-tab-settings-tab-edit-btn"
              ></q-btn>
              <q-btn
                v-if="currentDashboardData.data.tabs.length !== 1"
                :icon="outlinedDelete"
                :title="t('dashboard.delete')"
                class="q-ml-xs"
                padding="sm"
                unelevated
                size="sm"
                round
                flat
                @click.stop="deleteItem(tab.tabId)"
                data-test="dashboard-tab-settings-tab-delete-btn"
              ></q-btn>
            </span>
          </div>
        </div>
      </draggable>
    </div>

    <!-- add tab dialog -->
    <q-dialog v-model="showAddTabDialog" position="right" full-height maximized>
      <AddTab
        :edit-mode="isTabEditMode"
        :tabId="selectedTabIdToEdit"
        :dashboard-id="currentDashboardData.data.dashboardId"
        @refresh="refreshRequired"
      />
    </q-dialog>
    <!-- delete tab dialog -->
    <TabsDeletePopUp
      v-model="deletePopupVisible"
      :key="tabIdToBeDeleted"
      @update:cancel="deletePopupVisible = false"
      @update:ok="confirmDelete"
      :tabId="tabIdToBeDeleted"
      :dashboardData="currentDashboardData.data"
    />
  </div>
</template>

<script lang="ts">
import { defineComponent, nextTick, ref } from "vue";
import { VueDraggableNext } from "vue-draggable-next";
import { useI18n } from "vue-i18n";
import DashboardHeader from "./common/DashboardHeader.vue";
import { useStore } from "vuex";
import { deleteTab, editTab, getDashboard } from "@/utils/commons";
import { useRoute } from "vue-router";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";
import { reactive } from "vue";
import { onMounted } from "vue";
import AddTab from "@/components/dashboards/tabs/AddTab.vue";
import TabsDeletePopUp from "./TabsDeletePopUp.vue";
import { updateDashboard } from "../../../utils/commons";
import useNotifications from "@/composables/useNotifications";

export default defineComponent({
  name: "TabsSettings",
  components: {
    draggable: VueDraggableNext,
    DashboardHeader,
    AddTab,
    TabsDeletePopUp,
  },
  emits: ["refresh"],
  setup(props, { emit }) {
    const store = useStore();
    const route = useRoute();

    const showAddTabDialog = ref(false);
    const isTabEditMode = ref(false);
    const selectedTabIdToEdit: any = ref("");

    const deletePopupVisible = ref(false);
    const tabIdToBeDeleted: any = ref(null);

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
    const {
      showPositiveNotification,
      showErrorNotification,
      showConfictErrorNotificationWithRefreshBtn,
    } = useNotifications();
    onMounted(async () => {
      await getDashboardData();
    });

    const { t } = useI18n();
    const dragOptions = ref({
      animation: 200,
    });

    const editTabId = ref(null);
    const editTabObj: any = reactive({
      data: {},
    });

    const handleDragEnd = async () => {
      try {
        await updateDashboard(
          store,
          store.state.selectedOrganization.identifier,
          currentDashboardData.data.dashboardId,
          currentDashboardData.data,
          route.query.folder ?? "default"
        );

        // emit refresh to rerender
        emit("refresh");

        showPositiveNotification("Dashboard updated successfully.", {
          timeout: 2000,
        });
      } catch (error: any) {
        if (error?.response?.status === 409) {
          showConfictErrorNotificationWithRefreshBtn(
            error?.response?.data?.message ??
              error?.message ??
              "Tab reorder failed"
          );
        } else {
          showErrorNotification(error?.message ?? "Tab reorder failed");
        }
        // emit refresh to rerender
        emit("refresh");
        await getDashboardData();
      }
    };

    const editItem = (tabId: any) => {
      editTabId.value = tabId;
      editTabObj.data = JSON.parse(
        JSON.stringify(
          currentDashboardData.data.tabs.find((tab: any) => tab.tabId === tabId)
        )
      );
    };

    const saveEdit = async () => {
      try {
        if (editTabId.value) {
          // only allowed to edit name
          await editTab(
            store,
            currentDashboardData.data.dashboardId,
            route.query.folder ?? "default",
            editTabObj.data.tabId,
            editTabObj.data
          );

          // emit refresh to rerender
          emit("refresh");
          await getDashboardData();

          showPositiveNotification("Tab updated successfully", {
            timeout: 2000,
          });
          // reset edit mode
          editTabId.value = null;
          editTabObj.data = {};
        }
      } catch (error: any) {
        if (error?.response?.status === 409) {
          showConfictErrorNotificationWithRefreshBtn(
            error?.response?.data?.message ??
              error?.message ??
              "Tab updation failed"
          );
        } else {
          showErrorNotification(error?.message ?? "Tab updation failed");
        }

        // emit refresh to rerender
        emit("refresh");
        await getDashboardData();
      }
    };

    const cancelEdit = () => {
      // reset edit mode
      editTabId.value = null;
      editTabObj.data = {};
    };

    const addNewItem = () => {
      isTabEditMode.value = false;
      showAddTabDialog.value = true;
    };

    const deleteItem = async (tabId: any) => {
      tabIdToBeDeleted.value = tabId;
      await nextTick();
      // call cancelEdit to reset edit mode
      cancelEdit();
      deletePopupVisible.value = true;
    };

    const confirmDelete = async (moveTabId: any) => {
      try {
        await deleteTab(
          store,
          route.query.dashboard,
          route.query.folder,
          tabIdToBeDeleted.value,
          moveTabId
        );
        await getDashboardData();

        // emit event
        emit("refresh");

        tabIdToBeDeleted.value = null;
        deletePopupVisible.value = false;

        showPositiveNotification("Tab deleted successfully", {
          timeout: 2000,
        });
      } catch (error: any) {
        if (error?.response?.status === 409) {
          showConfictErrorNotificationWithRefreshBtn(
            error?.response?.data?.message ??
              error?.message ??
              "Tab deletion failed"
          );
        } else {
          showErrorNotification(error?.message ?? "Tab deletion failed", {
            timeout: 2000,
          });
        }
      }
    };

    const refreshRequired = async () => {
      await getDashboardData();
      emit("refresh");
      showAddTabDialog.value = false;
      isTabEditMode.value = false;
    };

    return {
      dragOptions,
      t,
      editTabId,
      editTabObj,
      editItem,
      saveEdit,
      cancelEdit,
      deleteItem,
      confirmDelete,
      deletePopupVisible,
      tabIdToBeDeleted,
      handleDragEnd,
      outlinedDelete,
      addNewItem,
      showAddTabDialog,
      isTabEditMode,
      selectedTabIdToEdit,
      currentDashboardData,
      refreshRequired,
    };
  },
});
</script>

<style lang="scss" scoped>
.draggable-row {
  display: flex;
  border-bottom: 1px solid #cccccc70;
  margin-bottom: 8px;
  cursor: move;
}

.draggable-handle {
  flex: 0 0 30px;
  padding: 8px;
  box-sizing: border-box;
}

.draggable-content {
  flex: 1;
  display: flex;
  justify-content: space-between;
  padding: 8px;
  box-sizing: border-box;
}

.edit-input {
  border: 1px solid $primary;
  border-radius: 4px;
  padding: 4px;
  outline: none;
  transition: border-color 0.3s;
  width: 100%;
}

.edit-input:focus {
  border-color: $secondary;
}
</style>
