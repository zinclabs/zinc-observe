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
          <div v-if="editMode" class="text-body1 text-bold">
            {{ t("dashboard.updateFolder") }}
          </div>
          <div v-else class="text-body1 text-bold">
            {{ t("dashboard.newFolder") }}
          </div>
        </div>
        <div class="col-auto">
          <q-btn
            v-close-popup="true"
            round
            flat
            icon="cancel"
            data-test="dashboard-folder-cancel"
          />
        </div>
      </div>
    </q-card-section>
    <q-separator />
    <q-card-section class="q-w-md q-mx-lg">
      <q-form ref="addFolderForm" @submit.stop="onSubmit.execute">
        <q-input
          v-model="folderData.name"
          :label="t('dashboard.nameOfVariable') + '*'"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          data-test="dashboard-folder-add-name"
          stack-label
          outlined
          filled
          dense
          :rules="[(val: any) => !!val.trim() || t('dashboard.nameRequired')]"
          :lazy-rules="true"
        />
        <span>&nbsp;</span>
        <q-input
          v-model="folderData.description"
          :label="t('dashboard.typeDesc')"
          color="input-border"
          bg-color="input-bg"
          data-test="dashboard-folder-add-description"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
        />

        <div class="flex justify-center q-mt-lg">
          <q-btn
            v-close-popup="true"
            class="q-mb-md text-bold"
            :label="t('dashboard.cancel')"
            text-color="light-text"
            padding="sm md"
            no-caps
            data-test="dashboard-folder-add-cancel"
          />
          <q-btn
            data-test="dashboard-folder-add-save"
            :disable="folderData.name.trim() === ''"
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
import { createFolder, updateFolder } from "@/utils/commons";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { getImageURL } from "../../utils/zincutils";
import { useLoading } from "@/composables/useLoading";
import useNotifications from "@/composables/useNotifications";

const defaultValue = () => {
  return {
    folderId: "",
    name: "",
    description: "",
  };
};

export default defineComponent({
  name: "AddFolder",
  props: {
    folderId: {
      type: String,
      default: "default",
    },
    editMode: {
      type: Boolean,
      default: false,
    },
  },
  emits: ["update:modelValue"],
  setup(props, { emit }) {
    const store: any = useStore();
    const addFolderForm: any = ref(null);
    const disableColor: any = ref("");
    const folderData: any = ref(
      props.editMode
        ? JSON.parse(
            JSON.stringify(
              store.state.organizationData.folders.find(
                (item: any) => item.folderId === props.folderId
              )
            )
          )
        : defaultValue()
    );
    const isValidIdentifier: any = ref(true);
    const { t } = useI18n();
    const { showPositiveNotification, showErrorNotification } =
      useNotifications();

    const onSubmit = useLoading(async () => {
      await addFolderForm.value.validate().then(async (valid: any) => {
        if (!valid) {
          return false;
        }

        try {
          //if edit mode
          if (props.editMode) {
            await updateFolder(
              store,
              folderData.value.folderId,
              folderData.value
            );
            showPositiveNotification("Folder updated successfully", {
              timeout: 2000,
            });
            emit("update:modelValue", folderData.value);
          }
          //else new folder
          else {
            const newFolder: any = await createFolder(store, folderData.value);
            emit("update:modelValue", newFolder);
            showPositiveNotification("Folder added successfully", {
              timeout: 2000,
            });
          }
          folderData.value = {
            folderId: "",
            name: "",
            description: "",
          };
          await addFolderForm.value.resetValidation();
        } catch (err: any) {
          showErrorNotification(
            err?.message ??
              (props.editMode
                ? "Folder updation failed"
                : "Folder creation failed"),
            { timeout: 2000 }
          );
        }
      });
    });

    return {
      t,
      disableColor,
      isPwd: ref(true),
      status,
      folderData,
      addFolderForm,
      store,
      isValidIdentifier,
      getImageURL,
      onSubmit,
    };
  },
});
</script>
