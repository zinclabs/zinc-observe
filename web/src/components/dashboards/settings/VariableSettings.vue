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
  <div>
    <div v-if="isAddVariable" class="column full-height">
      <AddSettingVariable
        v-if="isAddVariable"
        @save="handleSaveVariable"
        :variableName="selectedVariable"
        @close="goBackToDashboardList"
        :dashboardVariablesList="dashboardVariablesList"
      />
    </div>
    <div v-else class="column full-height">
      <DashboardHeader title="Variables">
        <template #right>
          <div>
            <!-- show variables dependencies if variables exist -->
            <q-btn
              v-if="dashboardVariablesList.length > 0"
              class="text-bold no-border q-ml-md"
              no-caps
              no-outline
              rounded
              color="primary"
              label="Show Dependencies"
              @click="showVariablesDependenciesGraphPopUp = true"
              data-test="dashboard-variable-dependencies-btn"
            />
            <q-btn
              class="text-bold no-border q-ml-md"
              no-caps
              no-outline
              rounded
              color="secondary"
              :label="t(`dashboard.newVariable`)"
              @click="addVariables"
              data-test="dashboard-variable-add-btn"
            />
          </div>
        </template>
      </DashboardHeader>
      <div>
        <div class="variables-list-header">
          <div class="header-item"></div>
          <div class="header-item">#</div>
          <div class="header-item">{{ t("dashboard.name") }}</div>
          <div class="header-item">{{ t("dashboard.type") }}</div>
          <div class="header-item q-ml-lg q-pl-lg">
            {{ t("dashboard.actions") }}
          </div>
        </div>

        <draggable
          v-model="dashboardVariablesList"
          :options="dragOptions"
          @end="handleDragEnd"
          @mousedown.stop="() => {}"
          data-test="dashboard-variable-settings-drag"
        >
          <div
            v-for="(variable, index) in dashboardVariablesList"
            :key="variable.name"
            class="draggable-row"
            data-test="dashboard-variable-settings-draggable-row"
          >
            <div class="draggable-handle">
              <q-icon
                name="drag_indicator"
                color="grey-13"
                class="'q-mr-xs"
                data-test="dashboard-variable-settings-drag-handle"
              />
            </div>
            <div class="draggable-content">
              <div>
                {{ index < 9 ? `0${index + 1}` : index + 1 }}
              </div>
              <div class="item-name">{{ variable.name }}</div>
              <div>
                {{ getVariableTypeLabel(variable.type) }}
              </div>
              <div class="item-actions">
                <q-btn
                  icon="edit"
                  padding="sm"
                  unelevated
                  size="sm"
                  round
                  flat
                  :title="t('dashboard.edit')"
                  @click="editVariableFn(variable.name)"
                  data-test="dashboard-edit-variable"
                />
                <q-btn
                  :icon="outlinedDelete"
                  :title="t('dashboard.delete')"
                  padding="sm"
                  unelevated
                  size="sm"
                  round
                  flat
                  @click.stop="
                    showDeleteDialogFn({ row: { name: variable.name } })
                  "
                  data-test="dashboard-delete-variable"
                />
              </div>
            </div>
          </div>
        </draggable>

        <ConfirmDialog
          title="Delete Variable"
          message="Are you sure you want to delete the variable?"
          @update:ok="deleteVariableFn"
          @update:cancel="confirmDeleteDialog = false"
          v-model="confirmDeleteDialog"
        />
        <q-dialog v-model="showVariablesDependenciesGraphPopUp">
          <q-card
            style="width: 60vw; min-width: 60vw; height: 70vh; min-height: 70vh"
          >
            <q-toolbar>
              <q-toolbar-title>Variables Dependency Graph</q-toolbar-title>
              <q-btn flat round dense icon="close" v-close-popup="true" />
            </q-toolbar>
            <q-card-section style="width: 100%; height: calc(100% - 50px)">
              <VariablesDependenciesGraph
                :variablesList="dashboardVariablesList"
                :class="store.state.theme == 'dark' ? 'dark-mode' : 'bg-white'"
                @closePopUp="
                  () => (showVariablesDependenciesGraphPopUp = false)
                "
              />
            </q-card-section>
          </q-card>
        </q-dialog>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, onMounted, onActivated, reactive } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useRoute } from "vue-router";
import { getImageURL } from "../../../utils/zincutils";
import {
  getDashboard,
  deleteVariable,
  updateDashboard,
} from "../../../utils/commons";
import AddSettingVariable from "./AddSettingVariable.vue";
import DashboardHeader from "./common/DashboardHeader.vue";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";
import NoData from "../../shared/grid/NoData.vue";
import ConfirmDialog from "../../ConfirmDialog.vue";
import VariablesDependenciesGraph from "./VariablesDependenciesGraph.vue";
import useNotifications from "@/composables/useNotifications";
import { VueDraggableNext } from "vue-draggable-next";

export default defineComponent({
  name: "VariableSettings",
  components: {
    draggable: VueDraggableNext,
    AddSettingVariable,
    NoData,
    ConfirmDialog,
    DashboardHeader,
    VariablesDependenciesGraph,
  },
  emits: ["save"],
  setup(props, { emit }) {
    const store: any = useStore();
    const { t } = useI18n();
    const route = useRoute();
    const isAddVariable = ref(false);

    const dashboardVariableData: any = reactive({
      data: {},
    });

    const dragOptions = ref({
      animation: 200,
    });

    const {
      showPositiveNotification,
      showErrorNotification,
      showConfictErrorNotificationWithRefreshBtn,
    } = useNotifications();
    // list of all variables, which will be same as the dashboard variables list
    const dashboardVariablesList: any = ref([]);
    const selectedVariable = ref(null);
    const confirmDeleteDialog = ref<boolean>(false);
    const selectedDelete: any = ref(null);
    const variableTypes = [
      {
        label: t("dashboard.queryValues"),
        value: "query_values",
      },
      {
        label: t("dashboard.constant"),
        value: "constant",
      },
      {
        label: t("dashboard.textbox"),
        value: "textbox",
      },
      {
        label: t("dashboard.custom"),
        value: "custom",
      },
    ];

    // show variables dependencies graph pop up
    const showVariablesDependenciesGraphPopUp = ref(false);

    const getVariableTypeLabel = (type: string) => {
      return variableTypes.find((vType) => vType.value === type)?.label || type;
    };

    const handleDragEnd = async () => {
      try {
        dashboardVariableData.data.variables = {
          list: dashboardVariablesList.value,
        };

        await updateDashboard(
          store,
          store.state.selectedOrganization.identifier,
          dashboardVariableData.data.dashboardId,
          dashboardVariableData.data,
          route.query.folder ?? "default",
        );

        showPositiveNotification("Dashboard updated successfully.", {
          timeout: 2000,
        });

        emit("save");
      } catch (error: any) {
        if (error?.response?.status === 409) {
          showConfictErrorNotificationWithRefreshBtn(
            error?.response?.data?.message ??
              error?.message ??
              "Variable reorder failed",
          );
        } else {
          showErrorNotification(error?.message ?? "Variable reorder failed");
        }
        await getDashboardData();
      }
    };

    onMounted(async () => {
      await getDashboardData();
    });

    onActivated(async () => {
      await getDashboardData();
    });

    const getDashboardData = async () => {
      dashboardVariableData.data = await getDashboard(
        store,
        route.query.dashboard,
        route.query.folder ?? "default",
      );

      dashboardVariablesList.value =
        dashboardVariableData.data?.variables?.list ?? [];
    };

    const addVariables = () => {
      selectedVariable.value = null;
      isAddVariable.value = true;
    };

    const showDeleteDialogFn = (props: any) => {
      selectedDelete.value = props.row;
      confirmDeleteDialog.value = true;
    };

    const deleteVariableFn = async () => {
      try {
        if (selectedDelete.value) {
          const variableName = selectedDelete?.value?.name;

          await deleteVariable(
            store,
            route.query.dashboard,
            variableName,
            route.query.folder ?? "default",
          );

          await getDashboardData();
          emit("save");
        }

        showPositiveNotification("Variable deleted successfully", {
          timeout: 2000,
        });
      } catch (error: any) {
        if (error?.response?.status === 409) {
          showConfictErrorNotificationWithRefreshBtn(
            error?.response?.data?.message ??
              error?.message ??
              "Variable deletion failed",
          );
        } else {
          showErrorNotification(error?.message ?? "Variable deletion failed", {
            timeout: 2000,
          });
        }
      }
    };
    const handleSaveVariable = async () => {
      isAddVariable.value = false;
      await getDashboardData();
      emit("save");
    };
    const goBackToDashboardList = () => {
      isAddVariable.value = false;
    };
    const editVariableFn = async (name: any) => {
      selectedVariable.value = name;

      isAddVariable.value = true;
    };

    return {
      t,
      store,
      getDashboardData,
      addVariables,
      dashboardVariablesList,
      isAddVariable,
      outlinedDelete,
      showDeleteDialogFn,
      confirmDeleteDialog,
      deleteVariableFn,
      goBackToDashboardList,
      editVariableFn,
      selectedVariable,
      handleSaveVariable,
      showVariablesDependenciesGraphPopUp,
      dragOptions,
      handleDragEnd,
      getVariableTypeLabel,
    };
  },
});
</script>

<style lang="scss" scoped>
.column {
  &.full-height {
    height: 100%;
  }
}

.variables-list-header {
  display: grid;
  grid-template-columns: 48px 80px minmax(200px, 1fr) 150px 120px;
  padding: 8px 0;
  font-weight: 900;
  border-bottom: 1px solid #cccccc70;

  .header-item {
    &:first-child {
      padding-left: 16px;
    }
  }
}

.draggable-row {
  display: grid;
  grid-template-columns: 48px minmax(0, 1fr);
  align-items: center;
  border-radius: 4px;
  border-bottom: 1px solid #cccccc70;
  &:hover {
    background-color: #cccccc10;
  }
}

.draggable-handle {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  cursor: move;
  box-sizing: border-box;
}

.draggable-content {
  display: grid;
  grid-template-columns: 80px minmax(200px, 1fr) 150px 120px;
  align-items: center;

  .item-name {
    padding-right: 16px;
  }

  .item-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
}

:deep(.dark-mode) {
  .draggable-row {
    background-color: #1e1e1e;
  }

  .draggable-row:nth-child(odd) {
    background-color: #242424;
  }
}
</style>
