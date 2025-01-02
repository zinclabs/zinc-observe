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
  <div class="q-mx-sm q-mt-md">
    <div class="flex q-mx-md items-center no-wrap">
      <div class="col">
        <div class="flex">
          <q-btn
            no-caps
            padding="xs"
            outline
            @click="goBack()"
            icon="arrow_back_ios_new"
          />
          <div class="text-h6 q-ml-md">
            {{ t("dashboard.importDashboard") }}
          </div>
        </div>
      </div>
      <div class="flex justify-center">
        <q-btn
          outline
          padding="sm"
          class="q-mr-md"
          no-caps
          :label="t('dashboard.communityDashboard')"
          @click="goToCommunityDashboards"
          data-test="dashboard-panel-tutorial-btn"
        ></q-btn>
        <q-btn
          v-close-popup
          class="text-bold q-mr-md"
          :label="t('function.cancel')"
          text-color="light-text"
          padding="sm xl"
          no-caps
          @click="goBack()"
        />
        <q-btn
          :disable="!!isLoading"
          class="text-bold no-border"
          :label="t('dashboard.import')"
          color="secondary"
          type="submit"
          padding="sm xl"
          no-caps
          @click="importDashboard"
        />
      </div>
    </div>
    <q-separator class="q-my-sm" />
    <div class="flex">
      <div
        class="dashboard-import-type-tabs flex items-center justify-center q-mx-md"
      >
        <app-tabs
          data-test="dashboard-import-type-tabs"
          class="q-mr-md"
          :tabs="tabs"
          v-model:active-tab="activeTab"
          @update:active-tab="updateActiveTab"
        />
      </div>

      <div class="flex">
        <q-splitter
          no-scroll
          v-model="splitterModel"
          :limits="[40, 80]"
          style="width: calc(95vw - 20px); height: 100%"
        >
          <template #before>
            <div
              v-if="activeTab == 'import_json_url'"
              class="editor-container-url"
            >
              <q-form class="q-mx-md q-mt-md" @submit="onSubmit">
                <div
                  style="width: calc(100% - 10px)"
                  class="q-mb-md flex full-width editor-form"
                >
                  <div
                    data-test="dashboard-import-url-input"
                    style="width: 69%"
                    class="q-pr-sm"
                  >
                    <q-input
                      v-model="url"
                      :label="t('dashboard.addURL')"
                      color="input-border"
                      bg-color="input-bg"
                      stack-label
                      filled
                      label-slot
                      :loading="isLoading == ImportType.URL"
                    />
                  </div>

                  <div
                    style="width: calc(30%)"
                    class="dashboard-folder-dropdown"
                    data-test="dashboard-folder-dropdown"
                  >
                    <select-folder-dropdown
                      @folder-selected="selectedFolderAtJson = $event"
                    />
                  </div>
                </div>
                <query-editor
                  data-test="dashboard-import-url-editor"
                  ref="queryEditorFileRef"
                  editor-id="dashboards-query-editor-file"
                  class="monaco-editor"
                  :debounceTime="300"
                  v-model:query="jsonStr"
                  language="json"
                  :class="
                    jsonStr == '' && queryEditorPlaceholderFlag
                      ? 'empty-query'
                      : ''
                  "
                  @focus="queryEditorPlaceholderFlag = false"
                  @blur="queryEditorPlaceholderFlag = true"
                />
              </q-form>
            </div>
            <div
              v-if="activeTab == 'import_json_file'"
              class="dashboard-import-json-container"
            >
              <q-form class="q-mx-md q-mt-md" @submit="onSubmit">
                <div
                  style="width: calc(100% - 10px)"
                  class="q-mb-md flex full-width editor-form"
                >
                  <div
                    data-test="dashboard-import-file-input"
                    style="width: 69%"
                    class="q-pr-sm"
                  >
                    <q-file
                      v-model="jsonFiles"
                      filled
                      bottom-slots
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
                  </div>
                  <div
                    style="width: calc(30%)"
                    class="dashboard-folder-dropdown"
                  >
                    <select-folder-dropdown
                      @folder-selected="selectedFolderAtJson = $event"
                    />
                  </div>
                  <div v-if="filesImportResults.length" class="q-py-sm">
                    <div v-for="importResult in filesImportResults">
                      <span
                        v-if="importResult.status == 'rejected'"
                        class="text-red"
                      >
                        <code style="background-color: #f2f1f1; padding: 3px">{{
                          importResult?.reason?.file
                        }}</code>
                        : {{ importResult?.reason?.error }}
                      </span>
                    </div>
                  </div>
                </div>
                <query-editor
                  data-test="dashboard-import-json-file-editor"
                  ref="queryEditorJsonRef"
                  editor-id="dashboards-query-editor-json"
                  class="monaco-editor"
                  :debounceTime="300"
                  v-model:query="jsonStr"
                  language="json"
                  :class="
                    jsonStr == '' && queryEditorPlaceholderFlag
                      ? 'empty-query'
                      : ''
                  "
                  @focus="queryEditorPlaceholderFlag = false"
                  @blur="queryEditorPlaceholderFlag = true"
                />

                <div></div>
              </q-form>
            </div>
          </template>
          <template #after>
            <div
              data-test="dashboard-import-error-container"
              style="width: 100%; height: 100%"
            >
              <div class="text-center text-h6">Error Validations</div>
              <q-separator class="q-mt-md" />
              <div
                class="error-section"
                v-if="dashboardErrorsToDisplay.length > 0"
              >
                <div class="error-list">
                  <!-- Iterate through the outer array -->

                  <!-- Iterate through each inner array (the individual error message) -->
                  <div
                    v-for="(
                      errorMessage, errorIndex
                    ) in dashboardErrorsToDisplay"
                    :key="errorIndex"
                    class="error-item"
                  >
                    <span
                      v-if="errorMessage.field == 'dashboard_title'"
                      class="text-red"
                    >
                      {{ errorMessage.message }}
                      <div style="width: 300px">
                        <q-input
                          v-model="dashboardTitles[errorIndex]"
                          :label="'Dashboard Title'"
                          color="input-border"
                          bg-color="input-bg"
                          class="showLabelOnTop"
                          stack-label
                          outlined
                          filled
                          dense
                          tabindex="0"
                          @update:model-value="
                            updateDashboardTitle(
                              dashboardTitles[errorIndex],
                              errorMessage.dashboardIndex,
                            )
                          "
                        />
                      </div>
                    </span>
                    <span
                      v-else-if="errorMessage.field == 'stream_type'"
                      class="text-red"
                    >
                      {{ errorMessage.message }}
                      <div style="width: 300px">
                        <q-select
                          v-model="streamTypes[errorIndex]"
                          :options="streamTypeOptions"
                          :label="'Stream Type'"
                          :popup-content-style="{
                            textTransform: 'lowercase',
                          }"
                          color="input-border"
                          bg-color="input-bg"
                          class="q-py-sm showLabelOnTop no-case"
                          filled
                          stack-label
                          dense
                          use-input
                          hide-selected
                          fill-input
                          :input-debounce="400"
                          @update:model-value="
                            updateStreamType(
                              streamTypes[errorIndex],
                              errorMessage.dashboardIndex,
                              errorMessage.tabIndex,
                              errorMessage.panelIndex,
                              errorMessage.queryIndex,
                            )
                          "
                          behavior="menu"
                        />
                      </div>
                    </span>

                    <span v-else>{{ errorMessage }}</span>
                  </div>
                </div>
              </div>
            </div>
          </template>
        </q-splitter>
      </div>
    </div>

    <div></div>
  </div>
</template>
<script lang="ts">
// @ts-nocheck
import { defineComponent, ref, onMounted, reactive, watch } from "vue";
import { useI18n } from "vue-i18n";
import { getAllDashboards, getFoldersList } from "../../utils/commons.js";
import { useStore } from "vuex";
import { useRouter, useRoute } from "vue-router";
import dashboardService from "../../services/dashboards.js";
import axios from "axios";
import { convertDashboardSchemaVersion } from "@/utils/dashboard/convertDashboardSchemaVersion";
import SelectFolderDropdown from "@/components/dashboards/SelectFolderDropdown.vue";
import useNotifications from "@/composables/useNotifications";
import AppTabs from "@/components/common/AppTabs.vue";
import QueryEditor from "@/components/QueryEditor.vue";
import stream from "@/services/stream.js";
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
          (item: any) => item.folderId === route.query.folder ?? "default",
        )?.name ?? "default",
      value: route.query.folder,
    });
    const selectedFolderAtUrl = ref({
      label:
        store.state.organizationData.folders.find(
          (item: any) => item.folderId === route.query.folder ?? "default",
        )?.name ?? "default",
      value: route.query.folder,
    });
    const selectedFolderAtJsonObj = ref({
      label:
        store.state.organizationData.folders.find(
          (item: any) => item.folderId === route.query.folder ?? "default",
        )?.name ?? "default",
      value: route.query.folder,
    });

    // hold the values of 3 supported import types
    const jsonFiles = ref<any>();
    const url = ref("");
    const jsonStr = ref("");

    const tabs = reactive([
      {
        label: "File Upload / JSON",
        value: "import_json_file",
      },
      {
        label: "URL Import",
        value: "import_json_url",
      },
    ]);
    const activeTab = ref("import_json_file");

    const dashboardErrorsToDisplay = ref([]);
    const splitterModel = ref(60);

    // holds the value of the loading for any of the import type
    const isLoading = ref(false);
    const queryEditorPlaceholderFlag = ref(true);
    const dashboardTitles = reactive({});
    const streamTypes = reactive({});
    const streamTypeOptions = ["logs", "metrics", "traces"];
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
            (item: any) => item.folderId === route.query.folder ?? "default",
          )?.name ?? "default",
        value: route.query.folder,
      };
      selectedFolderAtUrl.value = {
        label:
          store.state.organizationData.folders.find(
            (item: any) => item.folderId === route.query.folder ?? "default",
          )?.name ?? "default",
        value: route.query.folder,
      };
      selectedFolderAtJsonObj.value = {
        label:
          store.state.organizationData.folders.find(
            (item: any) => item.folderId === route.query.folder ?? "default",
          )?.name ?? "default",
        value: route.query.folder,
      };
    });

    watch(jsonFiles, (newVal) => {
      if (newVal && newVal.length > 0) {
        const fileContents = []; // Array to store parsed JSON objects

        newVal.forEach((file) => {
          const reader = new FileReader();
          reader.onload = (e) => {
            try {
              const jsonObject = JSON.parse(e.target.result);
              fileContents.push(jsonObject);

              // Check if all files have been processed
              if (fileContents.length === newVal.length) {
                jsonStr.value = JSON.stringify(fileContents, null, 2);
              }
            } catch (error) {
              console.error("Error parsing JSON:", error);
            }
          };
          reader.readAsText(file);
        });
      }
    });

    watch(jsonStr, (newVal) => {
      if (newVal) {
        try {
          if (typeof newVal === "object") {
            jsonStr.value = JSON.stringify(newVal, null, 2);
          } else {
            const jsonObject = JSON.parse(newVal);
            jsonStr.value = JSON.stringify(jsonObject, null, 2);
          }
        } catch (error) {
          showErrorNotification("Invalid JSON format");
        }
      }
    });

    watch(url, async (newVal) => {
      try {
        if (newVal) {
          const response = await axios.get(newVal);

          // Check if the response body is valid JSON
          try {
            if (
              response.headers["content-type"].includes("application/json") ||
              response.headers["content-type"].includes("text/plain")
            ) {
              jsonStr.value = JSON.stringify(response.data, null, 2);
            } else {
              showErrorNotification("Invalid JSON format in the URL");
            }
          } catch (parseError) {
            // If parsing fails, display an error message
            showErrorNotification("Invalid JSON format");
          }
        }
      } catch (error) {
        showErrorNotification("Error fetching data");
      }
    });

    //import dashboard from the json
    const importDashboardFromJSON = async (
      jsonObj: any,
      selectedFolder: any,
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
        selectedFolder.value,
      );

      //update store
      await getAllDashboards(store, selectedFolder.value);

      //return new dashboard
      return newDashboard;
    };

    // import multiple files
    const importFiles = async () => {
      if (!jsonStr.value || !jsonStr.value.length) {
        showErrorNotification("No JSON file(s) selected for import");
        isLoading.value = false;
        return;
      }
      isLoading.value = ImportType.FILES;

      jsonStr.value = JSON.parse(jsonStr.value);

      const data = jsonStr.value.map((parsedObject, index) => {
        return new Promise((resolve, reject) => {
          try {
            const convertedSchema = convertDashboardSchemaVersion(parsedObject);

            importDashboardFromJSON(convertedSchema, selectedFolderAtJson.value)
              .then((res) => {
                resolve({ file: jsonFiles.value[index].name, result: res });
              })
              .catch((e) => {
                reject({ file: jsonFiles.value[index].name, error: e });
              });
          } catch (e) {
            reject({
              file: jsonFiles.value[index].name,
              error: "Error processing file",
            });
          }
        });
      });

      Promise.allSettled(data).then(async (results) => {
        filesImportResults.value = results;

        const allFulfilledValues = results.filter(
          (r) => r.status === "fulfilled",
        ).length;

        if (results.length === allFulfilledValues) {
          await resetAndRefresh(ImportType.FILES, selectedFolderAtJson.value);
        }

        if (allFulfilledValues) {
          showPositiveNotification(
            `${allFulfilledValues} Dashboard(s) Imported`,
          );
        }

        if (results.length - allFulfilledValues) {
          showErrorNotification(
            `${results.length - allFulfilledValues} Dashboard(s) Failed to Import`,
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
          jsonStr.value = "";
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

        const oldImportedSchema = JSON.parse(jsonStr.value);
        const convertedSchema =
          convertDashboardSchemaVersion(oldImportedSchema);

        await importDashboardFromJSON(
          convertedSchema,
          selectedFolderAtUrl.value,
        ).then((res) => {
          resetAndRefresh(ImportType.URL, selectedFolderAtUrl.value);
          filesImportResults.value = [];
          jsonStr.value = "";

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
          selectedFolderAtJsonObj.value,
        ).then((res) => {
          resetAndRefresh(
            ImportType.JSON_STRING,
            selectedFolderAtJsonObj.value,
          );
          filesImportResults.value = [];
          jsonStr.value = "";

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
    const updateActiveTab = () => {
      jsonStr.value = "";
      jsonFiles.value = null;
      url.value = "";
    };
    const importDashboard = () => {
      dashboardErrorsToDisplay.value = [];
      const jsonObj = JSON.parse(jsonStr.value);
      if (Array.isArray(jsonObj)) {
        jsonObj.forEach((input, index) => {
          validateBasicInputs(input, index);
        });
      } else {
        validateBasicInputs(jsonObj);
      }
      if (dashboardErrorsToDisplay.value.length > 0) {
        return;
      }
      if (activeTab.value === "import_json_file") {
        if (jsonFiles.value == undefined) {
          importFromJsonStr();
        } else {
          importFiles();
        }
      } else {
        importFromUrl();
      }
    };
    const validateBasicInputs = (input, index = 0) => {
      if (input.title === "" || typeof input.title !== "string") {
        dashboardErrorsToDisplay.value.push({
          message: `Title is required for dashboard - ${index ? index + 1 : 1}  and should be a string`,
          field: "dashboard_title",
          dashboardIndex: index,
        });
      }
      checkStreamType(input.tabs, index);
    };
    function checkStreamType(tabs, dashboardIndex) {
      const streamTypeOptions = ["logs", "metrics", "traces"];
      tabs.forEach((tab, tabIndex) => {
        tab.panels.forEach((panel, panelIndex) => {
          panel.queries.forEach((query, queryIndex) => {
            // Check if stream is defined and is a valid string
            if (
              !query.fields.stream_type ||
              typeof query.fields.stream_type !== "string" ||
              query.fields.stream_type.trim() === "" ||
              !streamTypeOptions.includes(query.fields.stream_type)
            ) {
              dashboardErrorsToDisplay.value.push({
                message: `Missing or invalid 'stream_type' of dashboard - ${dashboardIndex + 1} at tab index ${tabIndex}, panel index ${panelIndex}, query index ${queryIndex}`,
                field: "stream_type",
                tabIndex: tabIndex,
                panelIndex: panelIndex,
                queryIndex: queryIndex,
                dashboardIndex,
              });
            }
          });
        });
      });
    }

    const goToCommunityDashboards = () => {
      window.open("https://github.com/openobserve/dashboards", "_blank");
    };
    const updateDashboardTitle = (selectedDashboard, dashboardIndex) => {
      const jsonObj = JSON.parse(jsonStr.value);
      if (Array.isArray(jsonObj)) {
        jsonObj[dashboardIndex].title = selectedDashboard;
      } else {
        jsonObj.title = selectedDashboard;
      }
      jsonStr.value = JSON.stringify(jsonObj, null, 2);
    };
    const updateStreamType = (
      selectedStreamType,
      dashboardIndex,
      tabIndex,
      panelIndex,
      queryIndex,
    ) => {
      const jsonObj = JSON.parse(jsonStr.value);
      if (Array.isArray(jsonObj)) {
        jsonObj[dashboardIndex].tabs[tabIndex].panels[panelIndex].queries[
          queryIndex
        ].fields.stream_type = selectedStreamType;
      } else {
        jsonObj.tabs[tabIndex].panels[panelIndex].queries[
          queryIndex
        ].fields.stream_type = selectedStreamType;
      }
      jsonStr.value = JSON.stringify(jsonObj, null, 2);
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
      tabs,
      activeTab,
      dashboardErrorsToDisplay,
      splitterModel,
      updateActiveTab,
      queryEditorPlaceholderFlag,
      importDashboard,
      goToCommunityDashboards,
      updateDashboardTitle,
      updateStreamType,
      streamTypeOptions,
      dashboardTitles,
      streamTypes,
    };
  },
  components: { SelectFolderDropdown, AppTabs, QueryEditor },
});
</script>

<style scoped lang="scss">
.empty-query .monaco-editor-background {
  background-image: url("../../assets/images/common/query-editor.png");
  background-repeat: no-repeat;
  background-size: 115px;
}

.empty-function .monaco-editor-background {
  background-image: url("../../assets/images/common/vrl-function.png");
  background-repeat: no-repeat;
  background-size: 170px;
}
.editor-container {
  height: calc(80vh - 20px) !important;
}
.editor-container-url {
  .monaco-editor {
    height: calc(71vh - 14px) !important; /* Total editor height */
    overflow: auto; /* Allows scrolling if content overflows */
    resize: none; /* Remove resize behavior */
  }
}
.dashboard-import-json-container {
  .monaco-editor {
    height: calc(71vh - 20px) !important; /* Total editor height */
    overflow: auto; /* Allows scrolling if content overflows */
    resize: none; /* Remove resize behavior */
  }
}
.monaco-editor {
  height: calc(81vh - 14px) !important; /* Total editor height */
  overflow: auto; /* Allows scrolling if content overflows */
  resize: none; /* Remove resize behavior */
}
.error-report-container {
  height: calc(78vh - 8px) !important; /* Total editor height */
  overflow: auto; /* Allows scrolling if content overflows */
  resize: none;
}
.error-container {
  display: flex;
  overflow-y: auto;

  flex-direction: column;
  border: 1px solid #ccc;
  height: calc(100% - 100px) !important; /* Total container height */
}

.error-section {
  padding: 10px;
  margin-bottom: 10px;
}

.section-title {
  font-size: 16px;
  margin-bottom: 10px;
  text-transform: uppercase;
}

.error-list {
}

.error-item {
  padding: 5px 0px;
  font-size: 14px;
}
.dashboard-import-type-tabs {
  height: fit-content;

  :deep(.rum-tabs) {
    border: 1px solid #464646;
  }

  :deep(.rum-tab) {
    &:hover {
      background: #464646;
    }

    &.active {
      background: #5960b2;
      color: #ffffff !important;
    }
  }
}
.dashboard-import-type-tabs {
  height: fit-content;

  :deep(.rum-tabs) {
    border: 1px solid #eaeaea;
    height: fit-content;
    border-radius: 4px;
    overflow: hidden;
  }

  :deep(.rum-tab) {
    width: fit-content !important;
    padding: 4px 12px !important;
    border: none !important;

    &:hover {
      background: #eaeaea;
    }

    &.active {
      background: #5960b2;
      color: #ffffff !important;
    }
  }
}
.dashboard-folder-dropdown {
  :deep(.q-field--labeled.showLabelOnTop) {
    padding-top: 12px; /* Example override */
  }
}
.dashboard-folder-dropdown {
  :deep(
      .q-field--labeled.showLabelOnTop.q-select
        .q-field__control-container
        .q-field__native
    )
    > :first-child {
    overflow: hidden;
    text-overflow: ellipsis;
  }
}
</style>
