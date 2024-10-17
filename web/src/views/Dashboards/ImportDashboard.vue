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
  <div class="q-mx-md q-my-md">
    <div class="row items-center no-wrap">
      <div class="col">
        <div class="flex">
          <q-btn
            no-caps
            @click="goBack()"
            padding="xs"
            outline
            icon="arrow_back_ios_new"
          />
          <div class="text-h6 q-ml-md">
            {{ t("dashboard.importDashboard") }}
          </div>
        </div>
      </div>
    </div>
    <q-separator class="q-my-sm" />
    <div style="width: 400px">
      <q-form @submit="onSubmit">
        <div class="q-my-md">Import Dashboard from exported JSON file</div>
        <div style="width: 400px">
          <q-file
            filled
            bottom-slots
            v-model="jsonFiles"
            :label="t('dashboard.dropFileMsg')"
            accept=".json"
            multiple
            :disable="!!isLoading"
          >
            <template v-slot:prepend>
              <q-icon name="cloud_upload" @click.stop.prevent />
            </template>
            <template v-slot:append>
              <q-icon
                name="close"
                @click.stop.prevent="jsonFiles = null"
                class="cursor-pointer"
              />
            </template>
            <template v-slot:hint> .json files only </template>
          </q-file>

          <!-- select folder or create new folder and select -->
          <select-folder-dropdown
            @folder-selected="selectedFolderAtJson = $event"
          />

          <div>
            <div v-if="filesImportResults.length" class="q-py-sm">
              <div v-for="importResult in filesImportResults">
                <span v-if="importResult.status == 'rejected'" class="text-red">
                  <code style="background-color: #f2f1f1; padding: 3px">{{
                    importResult?.reason?.file
                  }}</code>
                  : {{ importResult?.reason?.error }}
                </span>
              </div>
            </div>
            <q-btn
              :disable="!!isLoading"
              :loading="isLoading == ImportType.FILES"
              :label="t('dashboard.import')"
              class="q-my-md text-bold no-border"
              color="secondary"
              padding="sm xl"
              type="submit"
              no-caps
              @click="importFiles()"
            />
          </div>
        </div>
      </q-form>
      <q-separator class="q-my-sm" />

      <q-form @submit="onSubmit">
        <div class="q-my-md">{{ t("dashboard.importURL") }}</div>
        <div style="width: 400px">
          <q-input
            v-model="url"
            :label="t('dashboard.addURL')"
            color="input-border"
            bg-color="input-bg"
            stack-label
            filled
            dense
            label-slot
            :disable="!!isLoading"
          />

          <!-- select folder or create new folder and select -->
          <select-folder-dropdown
            @folder-selected="selectedFolderAtUrl = $event"
          />

          <div class="q-my-md">
            <q-btn
              :disable="!!isLoading"
              :loading="isLoading == ImportType.URL"
              class="text-bold no-border"
              :label="t('dashboard.import')"
              color="secondary"
              type="submit"
              no-caps
              @click="importFromUrl()"
              padding="sm xl"
            />
          </div>
        </div>
      </q-form>
      <q-separator class="q-my-sm" />
      <q-form @submit="onSubmit">
        <div class="q-my-md">{{ t("dashboard.importJson") }}</div>
        <div style="width: 400px" class="flex">
          <q-input
            :disable="!!isLoading"
            v-model="jsonStr"
            style="width: 400px"
            :label="t('dashboard.jsonObject')"
            color="input-border"
            dense
            filled
            type="textarea"
          />
        </div>

        <!-- select folder or create new folder and select -->
        <select-folder-dropdown
          @folder-selected="selectedFolderAtJsonObj = $event"
        />

        <div class="q-my-md">
          <q-btn
            :disable="!!isLoading"
            :loading="isLoading == ImportType.JSON_STRING"
            class="text-bold no-border q-mr-md"
            :label="t('dashboard.import')"
            color="secondary"
            type="submit"
            padding="sm xl"
            no-caps
            @click="importFromJsonStr()"
          />
          <q-btn
            v-close-popup
            class="text-bold"
            :label="t('function.cancel')"
            text-color="light-text"
            padding="sm xl"
            no-caps
            @click="goBack()"
          />
        </div>
      </q-form>
    </div>
    <div></div>
  </div>
</template>
<script lang="ts">
// @ts-nocheck
import { defineComponent, ref, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { getAllDashboards, getFoldersList } from "../../utils/commons.ts";
import { useStore } from "vuex";
import { useRouter, useRoute } from "vue-router";
import dashboardService from "../../services/dashboards";
import axios from "axios";
import { convertDashboardSchemaVersion } from "@/utils/dashboard/convertDashboardSchemaVersion";
import SelectFolderDropdown from "@/components/dashboards/SelectFolderDropdown.vue";
import useNotifications from "@/composables/useNotifications";

export default defineComponent({
  name: "Import Dashboard",
  props: ["dashboardId"],
  setup() {
    const { t } = useI18n();
    const store = useStore();
    const router = useRouter();
    const route = useRoute();
    const { showPositiveNotification, showErrorNotification } =
      useNotifications();
    const selectedFolderAtJson = ref({
      label:
        store.state.organizationData.folders.find(
          (item: any) => item.folderId === route.query.folder ?? "default"
        )?.name ?? "default",
      value: route.query.folder,
    });
    const selectedFolderAtUrl = ref({
      label:
        store.state.organizationData.folders.find(
          (item: any) => item.folderId === route.query.folder ?? "default"
        )?.name ?? "default",
      value: route.query.folder,
    });
    const selectedFolderAtJsonObj = ref({
      label:
        store.state.organizationData.folders.find(
          (item: any) => item.folderId === route.query.folder ?? "default"
        )?.name ?? "default",
      value: route.query.folder,
    });

    // hold the values of 3 supported import types
    const jsonFiles = ref<any>();
    const url = ref("");
    const jsonStr = ref("");

    // holds the value of the loading for any of the import type
    const isLoading = ref(false);

    // import type values
    const ImportType = {
      FILES: "files",
      URL: "url",
      JSON_STRING: "json_string",
    };

    // store the results of the import (for files)
    const filesImportResults = ref([]);

    onMounted(async () => {
      filesImportResults.value = [];
      await getFoldersList(store);
      selectedFolderAtJson.value = {
        label:
          store.state.organizationData.folders.find(
            (item: any) => item.folderId === route.query.folder ?? "default"
          )?.name ?? "default",
        value: route.query.folder,
      };
      selectedFolderAtUrl.value = {
        label:
          store.state.organizationData.folders.find(
            (item: any) => item.folderId === route.query.folder ?? "default"
          )?.name ?? "default",
        value: route.query.folder,
      };
      selectedFolderAtJsonObj.value = {
        label:
          store.state.organizationData.folders.find(
            (item: any) => item.folderId === route.query.folder ?? "default"
          )?.name ?? "default",
        value: route.query.folder,
      };
    });

    //import dashboard from the json
    const importDashboardFromJSON = async (
      jsonObj: any,
      selectedFolder: any
    ) => {
      const data =
        typeof jsonObj == "string"
          ? JSON.parse(jsonObj)
          : typeof jsonObj == "object"
          ? jsonObj
          : jsonObj;

      //set owner name and creator name to import dashboard
      data.owner = store.state.userInfo.name;
      data.created = new Date().toISOString();

      //create new dashboard
      const newDashboard = await dashboardService.create(
        store.state.selectedOrganization.identifier,
        data,
        selectedFolder.value
      );

      //update store
      await getAllDashboards(store, selectedFolder.value);

      //return new dashboard
      return newDashboard;
    };

    // import multiple files
    const importFiles = async () => {
      if (!jsonFiles.value || !jsonFiles.value.length) {
        showErrorNotification("No JSON file(s) selected for import");
        isLoading.value = false;
        return;
      }
      isLoading.value = ImportType.FILES;

      const data = jsonFiles?.value?.map((it: any, index: number) => {
        return new Promise((resolve, reject) => {
          let reader = new FileReader();
          reader.onload = function (readerResult) {
            try {
              const oldImportedSchema = JSON.parse(readerResult.target.result);
              const convertedSchema =
                convertDashboardSchemaVersion(oldImportedSchema);

              importDashboardFromJSON(
                convertedSchema,
                selectedFolderAtJson.value
              )
                .then((res) => {
                  resolve({ file: it.name, result: res });
                })
                .catch((e) => {
                  reject({ file: it.name, error: e });
                });
            } catch (e) {
              reject({ file: it.name, error: "Error reading file" });
            }
          };
          reader.readAsText(it);
        });
      });

      Promise.allSettled(data ?? []).then(async (results) => {
        filesImportResults.value = results;

        const allFulfilledValues = results.filter(
          (r) => r.status === "fulfilled"
        )?.length;

        if (results.length == allFulfilledValues) {
          await resetAndRefresh(ImportType.FILES, selectedFolderAtJson.value);
        }

        if (allFulfilledValues) {
          showPositiveNotification(
            `${allFulfilledValues} Dashboard(s) Imported`
          );
        }

        if (results.length - allFulfilledValues) {
          showErrorNotification(
            `${
              results.length - allFulfilledValues
            } Dashboard(s) Failed to Import`
          );
        }

        isLoading.value = false;
      });
    };

    // reset and refresh the value based on selected type
    const resetAndRefresh = async (type, selectedFolder) => {
      switch (type) {
        case ImportType.FILES:
          jsonFiles.value = null;
          isLoading.value = false;
          break;
        case ImportType.URL:
          url.value = "";
          isLoading.value = false;
          break;
        case ImportType.JSON_STRING:
          jsonStr.value = "";
          isLoading.value = false;
          break;
        default:
          break;
      }

      return router.push({
        path: "/dashboards",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
          folder: selectedFolder.value,
        },
      });
    };

    //import dashboard from url
    const importFromUrl = async () => {
      isLoading.value = ImportType.URL;
      try {
        // get the dashboard
        const urlData = url.value.trim() ? url.value.trim() : "";

        if (!urlData) {
          showErrorNotification("Please Enter a URL for import");
          return;
        }

        const res = await axios.get(urlData);

        const oldImportedSchema = res.data;
        const convertedSchema =
          convertDashboardSchemaVersion(oldImportedSchema);

        await importDashboardFromJSON(
          convertedSchema,
          selectedFolderAtUrl.value
        ).then((res) => {
          resetAndRefresh(ImportType.URL, selectedFolderAtUrl.value);
          filesImportResults.value = [];

          showPositiveNotification(`Dashboard Imported Successfully`);
        });
      } catch (error) {
        showErrorNotification("Please Enter a URL for import");
      } finally {
        isLoading.value = false;
      }
    };

    // import dashboard from json string
    const importFromJsonStr = async () => {
      isLoading.value = ImportType.JSON_STRING;
      try {
        // get the dashboard

        const oldImportedSchema = JSON.parse(jsonStr.value);
        const convertedSchema =
          convertDashboardSchemaVersion(oldImportedSchema);

        await importDashboardFromJSON(
          convertedSchema,
          selectedFolderAtJsonObj.value
        ).then((res) => {
          resetAndRefresh(
            ImportType.JSON_STRING,
            selectedFolderAtJsonObj.value
          );
          filesImportResults.value = [];

          showPositiveNotification(`Dashboard Imported Successfully`);
        });
      } catch (error) {
        showErrorNotification("Please Enter a JSON object for import");
      } finally {
        isLoading.value = false;
      }
    };

    // back button to render dashboard List page
    const goBack = () => {
      jsonFiles.value = [];
      url.value = "";
      jsonStr.value = "";
      filesImportResults.value = [];
      return router.push({
        path: "/dashboards",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
          folder: route?.query?.folder || "default",
        },
      });
    };
    const onSubmit = () => {
      // do nothing here
    };

    return {
      t,
      goBack,
      onSubmit,
      importFiles,
      jsonFiles,
      importFromUrl,
      url,
      jsonStr,
      importFromJsonStr,
      isLoading,
      ImportType,
      filesImportResults,
      route,
      selectedFolderAtJson,
      selectedFolderAtUrl,
      selectedFolderAtJsonObj,
    };
  },
  components: { SelectFolderDropdown },
});
</script>
