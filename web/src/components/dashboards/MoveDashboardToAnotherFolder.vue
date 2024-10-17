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
    <q-card-section
      class="q-px-md q-py-md"
      data-test="dashboard-folder-move-header"
    >
      <div class="row items-center no-wrap">
        <div class="col">
          <div class="text-body1 text-bold">
            Move Dashboard To Another Folder
          </div>
        </div>
        <div class="col-auto">
          <q-btn
            v-close-popup="true"
            round
            flat
            icon="cancel"
            data-test="dashboard-folder-move-cancel"
          />
        </div>
      </div>
    </q-card-section>
    <q-separator />
    <q-card-section
      class="q-w-md q-mx-lg"
      data-test="dashboard-folder-move-body"
    >
      <q-form
        ref="moveFolderForm"
        @submit.stop="onSubmit.execute()"
        data-test="dashboard-folder-move-form"
      >
        <q-input
          v-model="store.state.organizationData.folders.find((item: any) => item.folderId === activeFolderId).name"
          :label="t('dashboard.currentFolderLabel')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          :disable="true"
          data-test="dashboard-folder-move-name"
        />
        <span>&nbsp;</span>

        <!-- select folder or create new folder and select -->
        <SelectFolderDropdown @folder-selected="selectedFolder = $event" />

        <div class="flex justify-center q-mt-lg">
          <q-btn
            v-close-popup="true"
            class="q-mb-md text-bold"
            :label="t('dashboard.cancel')"
            text-color="light-text"
            padding="sm md"
            no-caps
            data-test="dashboard-folder-move-cancel"
          />
          <q-btn
            data-test="dashboard-folder-move"
            :disable="activeFolderId === selectedFolder.value"
            :loading="onSubmit.isLoading.value"
            :label="t('common.move')"
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
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { getImageURL } from "../../utils/zincutils";
import { moveDashboardToAnotherFolder } from "../../utils/commons";
import SelectFolderDropdown from "./SelectFolderDropdown.vue";
import { useLoading } from "@/composables/useLoading";
import useNotifications from "@/composables/useNotifications";

export default defineComponent({
  name: "MoveDashboardToAnotherFolder",
  components: { SelectFolderDropdown },
  props: {
    activeFolderId: {
      type: String,
      default: "default",
    },
    dashboardId: {
      type: String,
      default: "",
    },
  },
  emits: ["updated"],
  setup(props, { emit }) {
    const store: any = useStore();
    const moveFolderForm: any = ref(null);
    //dropdown selected folder
    const selectedFolder = ref({
      label: store.state.organizationData.folders.find(
        (item: any) => item.folderId === props.activeFolderId
      ).name,
      value: props.activeFolderId,
    });
    const { t } = useI18n();
    const { showPositiveNotification, showErrorNotification } =
      useNotifications();

    const onSubmit = useLoading(async () => {
      await moveFolderForm.value.validate().then(async (valid: any) => {
        if (!valid) {
          return false;
        }

        try {
          await moveDashboardToAnotherFolder(
            store,
            props.dashboardId,
            props.activeFolderId,
            selectedFolder.value.value
          );

          showPositiveNotification("Dashboard Moved successfully", {
            timeout: 2000,
          });

          emit("updated");
          moveFolderForm.value.resetValidation();
        } catch (err: any) {
          showErrorNotification(err?.message ?? "Dashboard move failed.", {
            timeout: 2000,
          });
        }
      });
    });

    return {
      t,
      moveFolderForm,
      store,
      getImageURL,
      selectedFolder,
      onSubmit,
    };
  },
});
</script>
