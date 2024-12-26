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
<div class=" q-mt-md full-width">
  <div class="flex q-mx-md items-center no-wrap">
      <div class="col">
        <div class="flex">
          <q-btn
            no-caps
            padding="xs"
            outline
            @click="arrowBackFn"
            icon="arrow_back_ios_new"
          />
          <div class="text-h6 q-ml-md">
           Import Destination
          </div>
        </div>
      </div>
      <div class=" flex justify-center">
        <q-btn
        v-close-popup
        class="text-bold q-mr-md"
        :label="t('function.cancel')"
        text-color="light-text"
        padding="sm xl"
        no-caps
        @click="router.back()"
      />
      <q-btn
        class="text-bold no-border "
        :label="t('dashboard.import')"
        color="secondary"
        type="submit"
        padding="sm xl"
        no-caps
        @click="importJson"
      />

    </div>
    </div>

    <q-separator class="q-my-sm q-mx-md" />
</div>
<div class="flex" >
 <div class="report-list-tabs flex items-center justify-center q-mx-md">
  <app-tabs
          data-test="pipeline-list-tabs"
          class="q-mr-md "
          :tabs="tabs"
          v-model:active-tab="activeTab"
          @update:active-tab="updateActiveTab"
        />
 </div>
 
  <div class="flex" style="width: 100%;">
    <q-splitter
          class="logs-search-splitter"
          no-scroll
          v-model="splitterModel"
          :style="{
          width: '100%',
          height: '100%',
        }"
        >
          <template #before>
            <div v-if="activeTab == 'import_json_url'" class="editor-container-url">
              <q-form class="q-mx-md q-mt-md" @submit="onSubmit"> 
                <div style="width: 100%" class="q-mb-md">
                    <q-input
                      v-model="url"
                      :label="t('dashboard.addURL')"
                      color="input-border"
                      bg-color="input-bg"
                      stack-label
                      filled
  
                      label-slot
                    />
              </div>
                    <query-editor
                          data-test="scheduled-alert-sql-editor"
                          ref="queryEditorRef"
                          editor-id="alerts-query-editor"
                          class="monaco-editor"
                          :debounceTime="300"
                          v-model:query="jsonStr"
                          language="json"
                          :class="
                            jsonStr == '' && queryEditorPlaceholderFlag ? 'empty-query' : ''
                          "
                          @focus="queryEditorPlaceholderFlag = false"
                          @blur="queryEditorPlaceholderFlag = true"
                        />

                <div>
                </div>
              </q-form>
            </div>
            <div v-if="activeTab == 'import_json_file'" class="editor-container-json">
              <q-form class="q-mx-md q-mt-md" @submit="onSubmit"> 
                <div style="width: 100%" class="q-mb-md">
                <q-file
                v-model="jsonFiles"
                  filled
                  bottom-slots
                  :label="t('dashboard.dropFileMsg')"
                  accept=".json"
                  multiple
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
                    <query-editor
                          data-test="scheduled-alert-sql-editor"
                          ref="queryEditorRef"
                          editor-id="alerts-query-editor"
                          class="monaco-editor"
                          :debounceTime="300"
                          v-model:query="jsonStr"
                          language="json"
                          :class="
                            jsonStr == '' && queryEditorPlaceholderFlag ? 'empty-query' : ''
                          "
                          @focus="queryEditorPlaceholderFlag = false"
                          @blur="queryEditorPlaceholderFlag = true"
                        />

                <div>
                </div>
              </q-form>
            </div>
          </template>
          
          <template #after>
            <div
              data-test="logs-vrl-function-editor"
              style="width: 100%; height: 100%"
            >
            <div v-if="alertErrorsToDisplay.length > 0" class="text-center text-h6 ">
  Error Validations
</div>
<div v-else class="text-center text-h6">
  Output Messages
</div>
<q-separator class="q-mx-md q-mt-md" />
            <div class="error-report-container">

  <!-- Alert Errors Section -->
  <div class="error-section" v-if="alertErrorsToDisplay.length > 0">
    <div class="error-list">
      <!-- Iterate through the outer array -->
      <div v-for="(errorGroup, index) in alertErrorsToDisplay" :key="index">
        <!-- Iterate through each inner array (the individual error message) -->
        <div v-for="(errorMessage, errorIndex) in errorGroup" :key="errorIndex" class="error-item">
          <span class="text-red" v-if="typeof errorMessage === 'object' && errorMessage.field == 'destination_name'">
            {{ errorMessage.message }}

            <div style="width: 300px;">
              <q-input
                  v-model="userSelectedDestinationName"
                  :label="'Destination *'"
                  color="input-border"
                  bg-color="input-bg"
                  class="showLabelOnTop"
                  stack-label
                  outlined
                  filled
                  dense
                  tabindex="0"
                  @update:model-value="updateAlertName(userSelectedDestinationName)"
                />
            </div>
          </span>
          <span class="text-red" v-else-if="typeof errorMessage === 'object' && errorMessage.field == 'url'">
            {{ errorMessage.message }}

            <div style="width: 300px;">
              <q-input
                  v-model="userSelectedDestinationUrl"
                  :label="'Destination URL *'"
                  color="input-border"
                  bg-color="input-bg"
                  class="showLabelOnTop"
                  stack-label
                  outlined
                  filled
                  dense
                  tabindex="0"
                  @update:model-value="updateDestinationUrl(userSelectedDestinationUrl)"
                />
            </div>
          </span>
          <!-- Check if the errorMessage is an object, if so, display the 'message' property -->
          <span class="text-red" v-else-if="typeof errorMessage === 'object' && errorMessage.field == 'type'">
            {{ errorMessage.message }}
            <div style="width: 300px;">
              <q-select
                    v-model="userSelectedDestinationType"
                    :options="destinationTypes"
                    :label="'Destination Type *'"
                    :popup-content-style="{ textTransform: 'lowercase' }"
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
                      updateDestinationType(userSelectedDestinationType)
                    "
                    behavior="menu"
                  
                  />
            </div>
          </span>
          <span class="text-red" v-else-if="typeof errorMessage === 'object' && errorMessage.field == 'method'">
            {{ errorMessage.message }}
            <div style="width: 300px;">
              <q-select
                    v-model="userSelectedDestinationMethod"
                    :options="destinationMethods"
                    :label="'Destination Method *'"
                    :popup-content-style="{ textTransform: 'lowercase' }"
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
                      updateDestinationMethod(userSelectedDestinationMethod)
                    "
                    behavior="menu"
                  
                  />
            </div>
          </span>
          <span class="text-red" v-else-if="typeof errorMessage === 'object' && errorMessage.field == 'template_name'">
            {{ errorMessage.message }}
            <div>
              <q-select
                  v-model="userSelectedTemplates"
                  :options="getFormattedTemplates"
                  label="Templates *"
                    :popup-content-style="{ textTransform: 'lowercase' }"
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
                    behavior="menu"
                  :rules="[(val: any) => !!val || 'Field is required!']"
                  style="width: 300px"
                >
                </q-select>
            </div>
          </span>
  

          <span class="text-red" v-else>{{ errorMessage }}</span>
        </div>
      </div>
    </div>
  </div>

      <div class="error-section" v-if="alertCreators.length > 0">
      <div class="section-title text-primary" >Alert Creation</div>
      <div class="error-list" v-for="(val, index) in alertCreators " :key="index">
      <div
      :class="{
        'error-item text-bold': true,
        'text-green ': val.success && !val?.rollback,
        'text-red': !val.success,
        'text-orange': val.success && val?.rollback
      }"><pre>{{ val.message }}</pre></div>
      </div>
      </div>
      </div>
            </div>
          </template>
        </q-splitter>
</div>
</div>

</template>

<script lang="ts">
import { defineComponent, ref, onMounted, reactive, computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useRouter, useRoute } from "vue-router";
import axios from "axios";
import router from "@/router";
import { useQuasar } from "quasar";
import alertsService from "../../services/alerts";

import QueryEditor from "../QueryEditor.vue";
import { json } from "stream/consumers";
import useStreams from "@/composables/useStreams";
import templateService from "@/services/alert_templates";
import destinationService from "@/services/alert_destination";

import AppTabs from "../common/AppTabs.vue";




export default defineComponent({
  name: "ImportDestination",
  props: {
    destinations: {
        type: Array,
        default: () => [],
    },
    templates: {
        type:Array,
        default:() => [],
    },
    alerts: {
        type:Array,
        default:() => [],
    }
  },
   emits :  ["update:destinations", "update:templates", "update:alerts"],
  setup(props, { emit }) {
    const { t } = useI18n();
    const store = useStore();
    const router = useRouter();
    const route = useRoute();

    const jsonStr = ref("");
    const q = useQuasar();
    const { getStreams } = useStreams();

    const templateErrorsToDisplay = ref([]);

    const destinationErrorsToDisplay = ref([]);

    const alertErrorsToDisplay = ref([]);
    const userSelectedTemplates = ref([]);
    const destinationTypes = ["http","email"]
    const destinationMethods = ["post","get","put"]

    const tempalteCreators = ref([])
    const destinationCreators = ref([])
    const alertCreators = ref([])
    const queryEditorPlaceholderFlag = ref(true);
    const streamList = ref([]);
    const userSelectedDestinationType = ref("");
    const userSelectedDestinationMethod = ref("");
    const jsonFiles = ref(null);
    const userSelectedDestinationName = ref("");
    const userSelectedDestinationUrl  = ref("")
    const jsonArrayOfObj = ref([
      {

      },
    ]);
    const activeTab = ref("import_json_file");
    const splitterModel = ref(60);
    const getFormattedTemplates = computed(() => {
      return props.templates.map((template) => {
        return template.name;
      });
    });

    watch(() => userSelectedTemplates.value , (newVal, oldVal) => {
      if(newVal){
        jsonArrayOfObj.value.template = newVal;
        jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
      }
    })

   const  updateDestinationType = (type) => {
      jsonArrayOfObj.value.type = type;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
   }
   const  updateDestinationMethod = (method) => {
      jsonArrayOfObj.value.method = method;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
   }

   const updateAlertName = (alertName) => {
    jsonArrayOfObj.value.name = alertName;
    jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
   }
   const updateDestinationUrl = (url) => {
    jsonArrayOfObj.value.url = url;
    jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
   }

   watch(jsonFiles, (newVal, oldVal) => {
    if (newVal) {
      const reader = new FileReader();
      reader.onload = (e) => {
        jsonStr.value = e.target.result;
      };
      reader.readAsText(newVal[0]);
    }
   })

    const tabs = reactive ([

{
    label: "File Upload / JSON",
    value: "import_json_file",
},
{
    label: "URL Import",
    value: "import_json_url",
}


    ]);

    const updateActiveTab = () =>{
      console.log('here')
      jsonStr.value = "";
      jsonFiles.value = null;
      jsonArrayOfObj.value = [
        {

        },
      ];
    }


    const importJson = async () => {
      alertErrorsToDisplay.value = [];
      templateErrorsToDisplay.value = [];
      destinationErrorsToDisplay.value = [];
      destinationCreators.value = [];
      alertCreators.value = [];
      // userSelectedTemplates.value = [];

      try {
  // Check if jsonStr.value is empty or null
        if (!jsonStr.value || jsonStr.value.trim() === "") {
          throw new Error("JSON string is empty");
        }

        // Try to parse the JSON string
        jsonArrayOfObj.value = JSON.parse(jsonStr.value);


      } catch (e) {
        // Handle parsing errors and other issues
        q.notify({
          message: e.message || "Invalid JSON format",
          color: "negative",
          position: "bottom",
          timeout: 2000,
        });
        return;
    }



      // Check if jsonArrayOfObj is an array or a single object
      const isArray = Array.isArray(jsonArrayOfObj.value);

      // If it's an array, process each object sequentially
      if (isArray) {
        for (const [index, jsonObj] of jsonArrayOfObj.value.entries()) {
          await processJsonObject(jsonObj, index+1);  // Pass the index along with jsonObj
        }
      } else {
        // If it's a single object, just process it
        await processJsonObject(jsonArrayOfObj.value,1);
      }


    }

    const processJsonObject = async (jsonObj: any,index: number) => {


  const isValidDestination = await validateDestinationInputs(jsonObj,0,index);
  if (!isValidDestination) {
    return;
  }

  if (alertErrorsToDisplay.value.length === 0 ) {

      const hasCreatedDestination = await createDestination(jsonObj,index);

      if(hasCreatedDestination) {
        q.notify({
            message: "Destination imported successfully",
            color: "positive",
            position: "bottom",
            timeout: 2000,
          });
          router.push({
            name: "alertDestinations",
            query:{
              org_identifier: store.state.selectedOrganization.identifier
            }
          })
        }
  }
};

    const validateDestinationInputs = async (input: any, destinationIndex:  number = 1, index: any) => {  
        let destinationErrors: string[] = [];

        // Check if 'url' is required for webhook and should not exist for email
        if (!input.hasOwnProperty('type') && !input.url) {
          destinationErrors.push(`Destination - ${index} 'url' is required for webhook`);
        }

        if (input.type === 'email' && input.url) {
          destinationErrors.push(`Destination - ${index} 'url' should not be provided for email`);
        }
        if (checkDestinationInList(props.destinations, input.name)) {
          destinationErrors.push(
            {
              message:`Destination - ${index} "${input.name}" already exists`,
              field:"destination_name"
            }
          );
        }
        if(input.type == 'http' && (typeof input.url != 'string' || input.url == "")  ){
          destinationErrors.push({
            message:`Destination - ${index} 'URL' should not be empty for type 'http'`,
            field:"url"
          });
        }
        if (!input.method || (input.method !== 'post' && input.method !== 'get' && input.method !== 'put')) {
          destinationErrors.push({
            message:`Destination - ${index} 'method' is required and should be either 'post', 'get', or 'put'`,
            field:'method'
          });
        }

        if(!checkTemplatesInList(props.templates,input.template)){
          destinationErrors.push({
            message:`Destination - ${index} ---> template "${input.template}" does not exists`,
            field:"template_name"
          })
        }


        // Check type for email and it should be present only for email
        if (input.type != 'email' && input.type != 'http' ) {
          
          destinationErrors.push({
            message:`Destination - ${index} 'type' should be email or http`,
            field:"type"
          });
        }


        // Check if 'method' is required for both webhook and email
 
        // Check if 'skip_tls_verify' is required for both webhook and email, and it should be a boolean
        if (input.skip_tls_verify === undefined || typeof input.skip_tls_verify !== 'boolean') {
          destinationErrors.push(`Destination - ${index} 'skip_tls_verify' is required and should be a boolean value`);
        }

        // Check if 'headers' is required for webhook but not for email
        if (!input.hasOwnProperty('type') && Object.keys(input.headers).length === 0) {
          destinationErrors.push(`Destination - ${index} 'headers' is required for webhook`);
        }

        if (input.type === 'email' && Object.keys(input.headers).length !== 0) {
          destinationErrors.push(`Destination - ${index} 'headers' should not be provided for email`);
        }

        // Check if 'name' is required for both webhook and email
        if (!input.name || typeof input.name !== 'string') {
          destinationErrors.push(`Destination - ${index} 'name' is required and should be a string`);
        }

        // 'emails' should be required for email type and should be an array of strings
        if (input.type === 'email') {
          if (!Array.isArray(input.emails) || input.emails.some((email: any) => typeof email !== 'string') || input.emails.length ==0 ) {
            destinationErrors.push(`Destination - ${index} 'emails' should be an array of strings for email`);
          }
        }

        // Check if 'template' is required for both webhook and email
        if (!input.template || typeof input.template !== 'string') {
          destinationErrors.push(`Destination - ${index} 'template' is required and should be a string`);
        }





        // Log all destination errors at the end if any exist
        if (destinationErrors.length > 0) {
          alertErrorsToDisplay.value.push(destinationErrors);
          return false;
      }

  // If all validations pass
  return true;
    };


    const checkDestinationInList = (destinations, destinationName) =>{
      const destinationsList = destinations.map(destination => destination.name);
      return destinationsList.includes(destinationName);
    }
    const checkTemplatesInList = (templates, templateName) =>{
      const templatesList = templates.map(template => template.name);
      return templatesList.includes(templateName);
    }


    const createDestination = async (input: any,index: any) => {
      try {
        // Await the destination creation service call
        await destinationService.create({
          org_identifier: store.state.selectedOrganization.identifier,
          destination_name: input.name,
          data: input,
        });

        // Success block
        alertCreators.value.push({
          message: `Destination - ${index}: "${input.name}" created successfully`,
          success: true,
        });
        emit("update:destinations");
        return true; // Return true for success
      } catch (error) {
        // Error block
        console.log(error, "error");
        alertCreators.value.push({
          message: `Destination - ${index}: "${input.name}" creation failed --> \n Reason: ${error?.response?.data?.message || "Unknown Error"}`,
          success: false,
        });
        return false; // Return false for failure
      }
    }
    const arrowBackFn = () => {
      router.push({
        name: "alertDestinations",
        query:{
          org_identifier:store.state.selectedOrganization.identifier,
        }
      })
    }




    const onSubmit = (e) => {
      e.preventDefault();
    };

    return {
      t,
      jsonStr,
      importJson,
      onSubmit,
      router,
      q,
      templateErrorsToDisplay,
      destinationErrorsToDisplay,
      alertErrorsToDisplay,
      tempalteCreators,
      destinationCreators,
      alertCreators,
      queryEditorPlaceholderFlag,
      splitterModel,
      tabs,
      activeTab,
      userSelectedTemplates,
      getFormattedTemplates,
      jsonArrayOfObj,
      streamList,
      userSelectedDestinationType,
      userSelectedDestinationMethod,
      updateDestinationType,
      updateDestinationMethod,
      updateAlertName,
      updateDestinationUrl,
      jsonFiles,
      updateActiveTab,
      arrowBackFn,
      userSelectedDestinationName,
      userSelectedDestinationUrl,
      destinationTypes,
      destinationMethods,
    };
  },
  components: {
    QueryEditor,
    AppTabs,
  },
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
  .editor-container{
    height: calc(70vh - 20px) !important; 
  }
  .editor-container-url {
    .monaco-editor {
      height: calc(66vh - 8px) !important; /* Total editor height */
      overflow: auto;             /* Allows scrolling if content overflows */
      resize: none;               /* Remove resize behavior */
    }
  }
  .editor-container-json {
    .monaco-editor {
      height: calc(65vh - 20px) !important; /* Total editor height */
      overflow: auto;             /* Allows scrolling if content overflows */
      resize: none;               /* Remove resize behavior */
    }
  }
  .monaco-editor {
  height: calc(60vh - 14px) !important; /* Total editor height */
  overflow: auto;             /* Allows scrolling if content overflows */
  resize: none;               /* Remove resize behavior */
}
  .error-report-container {
  height: calc(60vh - 8px) !important; /* Total editor height */
  overflow: auto;             /* Allows scrolling if content overflows */
  resize: none;      
}
.error-container {
  display: flex;
  overflow-y: auto;

  flex-direction: column;
  border: 1px solid #ccc;
  height: calc(100% - 100px) !important /* Total container height */
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
.report-list-tabs {
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
  .report-list-tabs {
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
</style>
