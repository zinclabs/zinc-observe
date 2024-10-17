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
    <q-card style="width: 600px" data-test="dialog-box">
      <q-card-section class="confirmBody">
        <div class="head" data-test="dashboard-tab-delete-tab-head">
          Delete
          <span
            style="text-decoration: underline"
            data-test="dashboard-tab-delete-tab-name"
            >{{
              dashboardData.tabs.find((tab: any) => tab.tabId === tabId)?.name
            }}</span
          >
        </div>
        <div class="para" data-test="dashboard-tab-delete-tab-para">
          This action cannot be undone. Are you sure you want to delete this
          tab?
        </div>
      </q-card-section>

      <!-- only show if there are panels in the tab -->
      <div
        v-if="dashboardData.tabs.find((tab: any) => tab.tabId === tabId)?.panels?.length"
        style="padding: 10px"
        data-test="dashboard-tab-delete-tab-panels-container"
      >
        <div class="radio-group">
          <div style="display: flex; flex-direction: row">
            <q-radio
              v-model="action"
              val="move"
              data-test="dashboard-tab-delete-tab-panels-move"
            >
              Move panels to another tab
            </q-radio>
            <div v-if="action === 'move'" class="select-container">
              <q-select
                dense
                filled
                v-model="selectedTabToMovePanels"
                :options="moveTabOptions"
                data-test="dashboard-tab-delete-tab-panels-move-select"
              />
            </div>
          </div>
          <q-radio
            v-model="action"
            val="delete"
            data-test="dashboard-tab-delete-tab-panels-delete"
          >
            Delete all the panels of this tab
          </q-radio>
        </div>
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
          >
            {{ t("confirmDialog.ok") }}
          </q-btn>
        </div>
      </q-card-actions>
    </q-card>
  </q-dialog>
</template>

<script lang="ts">
import { onMounted } from "vue";
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";

export default defineComponent({
  name: "TabsDeletePopUp",
  emits: ["update:ok", "update:cancel"],
  props: ["tabId", "dashboardData"],
  setup(props, { emit }) {
    const { t } = useI18n();
    const action = ref("move");
    const selectedTabToMovePanels = ref({ label: "Default", value: "default" });
    const moveTabOptions = ref([]);

    onMounted(() => {
      // update move tab options
      const newMoveTabOptions: any = [];
      props?.dashboardData?.tabs?.forEach((tab: any) => {
        // if tab is to be deleted, do not include it in the options
        if (tab.tabId != props.tabId) {
          newMoveTabOptions.push({
            label: tab.name,
            value: tab.tabId,
          });
        }
      });

      // set action to move as default
      action.value = "move";
      // set selectedTabToMovePanels to [0]th value
      selectedTabToMovePanels.value = newMoveTabOptions[0];

      // selectedTabToMovePanels.value = {
      //   label: "Default",
      //   value: "default",
      // };
      moveTabOptions.value = newMoveTabOptions;
    });

    const onCancel = () => {
      emit("update:cancel");
    };

    const onConfirm = () => {
      // if action is delete, then emit without passing the selectedTabToMovePanels as args
      // else pass the selectedTabToMovePanels
      if (action.value === "delete") {
        emit("update:ok");
        return;
      }
      emit("update:ok", selectedTabToMovePanels.value.value);
    };
    return {
      t,
      onCancel,
      onConfirm,
      action,
      selectedTabToMovePanels,
      moveTabOptions,
    };
  },
});
</script>

<style lang="scss" scoped>
.q-radio {
  margin-bottom: 10px;
}

.radio-group {
  display: flex;
  flex-direction: column;
}

.select-container {
  margin-left: 20px;
  min-width: 200px;
  max-width: 300px;
  margin-bottom: 10px;
}
</style>
