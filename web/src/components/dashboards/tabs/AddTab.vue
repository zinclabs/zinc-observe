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
  <q-card class="column full-height">
    <q-card-section class="q-px-md q-py-md">
      <div class="row items-center no-wrap">
        <div class="col">
          <div
            v-if="editMode"
            class="text-body1 text-bold"
            data-test="dashboard-tab-edit"
          >
            Edit Tab
          </div>
          <div
            v-else
            class="text-body1 text-bold"
            data-test="dashboard-tab-add"
          >
            Add Tab
          </div>
        </div>
        <div class="col-auto">
          <q-btn
            v-close-popup="true"
            round
            flat
            icon="cancel"
            data-test="dashboard-tab-cancel"
          />
        </div>
      </div>
    </q-card-section>
    <q-separator />
    <q-card-section class="q-w-md q-mx-lg">
      <q-form ref="addTabForm" @submit.stop="onSubmit.execute">
        <q-input
          v-model="tabData.name"
          label="Name*"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          :rules="[(val: any) => !!val.trim() || t('dashboard.nameRequired')]"
          :lazy-rules="true"
          data-test="dashboard-add-tab-name"
        />

        <div class="flex justify-center q-mt-lg">
          <q-btn
            v-close-popup="true"
            class="q-mb-md text-bold"
            :label="t('dashboard.cancel')"
            text-color="light-text"
            padding="sm md"
            no-caps
            data-test="dashboard-add-cancel"
          />
          <q-btn
            :disable="tabData.name.trim() === ''"
            :loading="onSubmit.isLoading.value"
            :label="t('dashboard.save')"
            class="q-mb-md text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            type="submit"
            no-caps
            data-test="dashboard-add-tab-submit"
          />
        </div>
      </q-form>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useLoading } from "@/composables/useLoading";
import { addTab, getDashboard } from "@/utils/commons";
import { useRoute } from "vue-router";
import { editTab } from "../../../utils/commons";
import useNotifications from "@/composables/useNotifications";

const defaultValue = () => {
  return {
    name: "",
    panels: [],
  };
};

export default defineComponent({
  name: "AddTab",
  props: {
    tabId: {
      validator: (value) => {
        return typeof value === "string" || value === null;
      },
      default: null,
    },
    editMode: {
      type: Boolean,
      default: false,
    },
    dashboardId: {
      validator: (value) => {
        return typeof value === "string" || value === null;
      },
      required: true,
    },
    folderId: {
      required: false,
      validator: (value) => {
        return typeof value === "string" || value === null;
      },
    },
  },
  emits: ["refresh"],
  setup(props: any, { emit }) {
    const { t } = useI18n();
    const route = useRoute();
    const store: any = useStore();
    const addTabForm: any = ref(null);
    let dashboardData: any = ref({});
    const isValidIdentifier: any = ref(true);
    const {
      showPositiveNotification,
      showErrorNotification,
      showConfictErrorNotificationWithRefreshBtn,
    } = useNotifications();
    const tabData: any = ref(defaultValue());

    const loadDashboardData = async () => {
      if (props.editMode) {
        dashboardData.value = await getDashboard(
          store,
          props.dashboardId,
          props.folderId ?? route.query.folder ?? "default"
        );
        tabData.value = JSON.parse(
          JSON.stringify(
            dashboardData?.value?.tabs?.find(
              (tab: any) => tab.tabId === props.tabId
            )
          )
        );
      }
    };
    loadDashboardData();

    const onSubmit = useLoading(async () => {
      await addTabForm.value.validate().then(async (valid: any) => {
        if (!valid) {
          return false;
        }

        try {
          //if edit mode
          if (props.editMode) {
            // only allowed to edit name
            const updatedTab = await editTab(
              store,
              props.dashboardId,
              props.folderId ?? route.query.folder ?? "default",
              tabData.value.tabId,
              tabData.value
            );

            // emit refresh to rerender
            emit("refresh", updatedTab);

            showPositiveNotification("Tab updated successfully", {
              timeout: 2000,
            });
          }
          //else new tab
          else {
            const newTab = await addTab(
              store,
              props.dashboardId,
              props.folderId ?? route.query.folder ?? "default",
              tabData.value
            );

            // emit refresh to rerender
            emit("refresh", newTab);

            showPositiveNotification("Tab added successfully", {
              timeout: 2000,
            });
          }
          tabData.value = {
            name: "",
            panels: [],
          };
          await addTabForm.value.resetValidation();
        } catch (error: any) {
          if (error?.response?.status === 409) {
            showConfictErrorNotificationWithRefreshBtn(
              error?.response?.data?.message ??
                error?.message ??
                (props.editMode ? "Failed to update tab" : "Failed to add tab")
            );
          } else {
            showErrorNotification(
              error?.message ??
                (props.editMode ? "Failed to update tab" : "Failed to add tab"),
              {
                timeout: 2000,
              }
            );
          }
        } finally {
        }
      });
    });

    return {
      t,
      tabData,
      addTabForm,
      store,
      isValidIdentifier,
      onSubmit,
    };
  },
});
</script>
