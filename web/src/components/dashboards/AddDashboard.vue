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
          <div v-if="beingUpdated" class="text-body1 text-bold">
            {{ t("dashboard.updatedashboard") }}
          </div>
          <div v-else class="text-body1 text-bold">
            {{ t("dashboard.createdashboard") }}
          </div>
        </div>
        <div class="col-auto">
          <q-btn
            v-close-popup="true"
            round
            flat
            icon="cancel"
            data-test="dashboard-add-cancel"
          />
        </div>
      </div>
    </q-card-section>
    <q-separator />
    <q-card-section class="q-w-md q-mx-lg">
      <q-form ref="addDashboardForm" @submit.stop="onSubmit.execute">
        <q-input
          v-if="beingUpdated"
          v-model="dashboardData.id"
          :readonly="beingUpdated"
          :disabled="beingUpdated"
          :label="t('dashboard.id')"
          data-test="dashboard-id"
        />
        <q-input
          v-model="dashboardData.name"
          :label="t('dashboard.name') + ' *'"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          data-test="add-dashboard-name"
          stack-label
          outlined
          filled
          dense
          :rules="[(val: any) => !!val.trim() || t('dashboard.nameRequired')]"
          :lazy-rules="true"
        />
        <span>&nbsp;</span>
        <q-input
          v-model="dashboardData.description"
          :label="t('dashboard.typeDesc')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          data-test="add-dashboard-description"
        />

        <span>&nbsp;</span>
        <!-- select folder or create new folder and select -->
        <select-folder-dropdown
          v-if="showFolderSelection"
          :active-folder-id="selectedFolder.value"
          @folder-selected="selectedFolder = $event"
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
            data-test="dashboard-add-submit"
            :disable="dashboardData.name.trim() === ''"
            :loading="onSubmit.isLoading.value"
            :label="t('dashboard.save')"
            class="q-mb-md text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            type="submit"
            no-caps
          />
        </div>
      </q-form>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import dashboardService from "../../services/dashboards";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { toRaw } from "vue";
import { getImageURL } from "../../utils/zincutils";
import { convertDashboardSchemaVersion } from "@/utils/dashboard/convertDashboardSchemaVersion";
import SelectFolderDropdown from "./SelectFolderDropdown.vue";
import { getAllDashboards } from "@/utils/commons";
import { useQuasar } from "quasar";
import { useLoading } from "@/composables/useLoading";
import useNotifications from "@/composables/useNotifications";

const defaultValue = () => {
  return {
    id: "",
    name: "",
    description: "",
  };
};

let callDashboard: Promise<{ data: any }>;

export default defineComponent({
  name: "ComponentAddDashboard",
  props: {
    activeFolderId: {
      validator: (value: any) => {
        return typeof value === "string" || value === null;
      },
      default: "default",
    },
    showFolderSelection: {
      type: Boolean,
      default: true,
    },
  },
  emits: ["updated"],
  setup(props, { emit }) {
    const store: any = useStore();
    const beingUpdated: any = ref(false);
    const addDashboardForm: any = ref(null);
    const disableColor: any = ref("");
    const dashboardData: any = ref(defaultValue());
    const isValidIdentifier: any = ref(true);
    const { t } = useI18n();
    const $q = useQuasar();
    const { showPositiveNotification, showErrorNotification } =
      useNotifications();

    const activeFolder: any = store.state.organizationData.folders.find(
      (item: any) => item.folderId === props.activeFolderId
    );
    const selectedFolder = ref({
      label: activeFolder.name,
      value: activeFolder.folderId,
    });

    //generate random integer number for dashboard Id
    function getRandInteger() {
      return Math.floor(Math.random() * (9999999999 - 100 + 1)) + 100;
    }

    const onSubmit = useLoading(async () => {
      await addDashboardForm.value.validate().then(async (valid: any) => {
        if (!valid) {
          return false;
        }

        const dashboardId = dashboardData.value.id;
        delete dashboardData.value.id;

        if (dashboardId == "") {
          const obj = toRaw(dashboardData.value);
          const baseObj = {
            title: obj.name,
            // NOTE: the dashboard ID is generated at the server side,
            // in "Create a dashboard" request handler. The server
            // doesn't care what value we put here as long as it's
            // a string.
            dashboardId: "",
            description: obj.description,
            variables: {
              list: [],
              showDynamicFilters: true,
            },
            defaultDatetimeDuration: {
              startTime: null,
              endTime: null,
              relativeTimePeriod: "15m",
              type: "relative",
            },
            role: "",
            owner: store.state.userInfo.name,
            created: new Date().toISOString(),
            tabs: [
              {
                panels: [],
                name: "Default",
                tabId: "default",
              },
            ],
            version: 3,
          };

          callDashboard = dashboardService.create(
            store.state.selectedOrganization.identifier,
            baseObj,
            selectedFolder.value.value ?? "default"
          );
        }
        try {
          const res = await callDashboard;

          const data = convertDashboardSchemaVersion(
            res?.data["v" + res?.data?.version]
          );

          //update store
          await getAllDashboards(store, selectedFolder.value.value);
          emit("updated", data.dashboardId, selectedFolder.value.value);
          dashboardData.value = {
            id: "",
            name: "",
            description: "",
          };
          await addDashboardForm.value.resetValidation();

          showPositiveNotification("Dashboard added successfully.");
        } catch (err: any) {
          showErrorNotification(err?.message ?? "Dashboard creation failed.");
        }
      });
    });

    return {
      t,
      disableColor,
      isPwd: ref(true),
      beingUpdated,
      status,
      dashboardData,
      addDashboardForm,
      store,
      getRandInteger,
      isValidIdentifier,
      getImageURL,
      selectedFolder,
      onSubmit,
    };
  },
  methods: {
    onRejected(rejectedEntries: string | any[]) {
      this.$q.notify({
        type: "negative",
        message: `${rejectedEntries.length} file(s) did not pass validation constraints`,
      });
    },
  },
  components: { SelectFolderDropdown },
});
</script>
