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
            @click="router.back()"
            icon="arrow_back_ios_new"
          />
          <div class="text-h6 q-ml-md">
           Import Alert
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
<div class="flex">
 <div class="report-list-tabs flex items-center justify-center q-mx-md">
  <app-tabs
          data-test="pipeline-list-tabs"
          class="q-mr-md "
          :tabs="tabs"
          v-model:active-tab="activeTab"
          @update:active-tab="updateActiveTab"
        />
 </div>

  <div class="flex">
    <q-splitter
          class="logs-search-splitter"
          no-scroll
          v-model="splitterModel"
          style="width: calc(95vw - 20px); height: 100%"
        >
          <template #before>
            <div v-if="activeTab == 'import_json_clipboard'" class="editor-container">

              <q-form class="q-mx-md q-mt-md" @submit="onSubmit"> 
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
          <span class="text-red" v-if="typeof errorMessage === 'object' && !errorMessage.isDestination && errorMessage.isAlert">
            {{ errorMessage.message }}

            <div style="width: 300px;">
              <q-input
                  v-model="userSelectedAlertName"
                  :label="t('alerts.name') + ' *'"
                  color="input-border"
                  bg-color="input-bg"
                  class="showLabelOnTop"
                  stack-label
                  outlined
                  filled
                  dense
                  tabindex="0"
                  @update:model-value="updateAlertName(userSelectedAlertName)"
                />
            </div>
          </span>
          <!-- Check if the errorMessage is an object, if so, display the 'message' property -->
          <span class="text-red" v-else-if="typeof errorMessage === 'object' && !errorMessage.isDestination && !errorMessage.isAlert">
            {{ errorMessage.message }}
            <div style="width: 300px;">
              <q-select
                    v-model="userSelectedStreamName"
                    :options="streamList"
                    :label="t('alerts.stream_name') + ' *'"
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
                      updateStreamFields(userSelectedStreamName)
                    "
                    behavior="menu"
                  
                  />
            </div>
          </span>
          <span class="text-red" v-else-if="typeof errorMessage === 'object' && errorMessage.isDestination">
            {{ errorMessage.message }}
            <div>
              <q-select
                  v-model="userSelectedDestinations"
                  :options="getFormattedDestinations"
                  label="Destinations *"
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
                  <template v-slot:option="option">
                    <q-list dense>
                      <q-item
                        tag="label"
                        :data-test="`add-alert-destination-${option.opt}-select-item`"
                      >
                        <q-item-section avatar>
                          <q-checkbox
                            size="xs"
                            dense
                            v-model="userSelectedDestinations"
                            :val="option.opt"
                          />
                        </q-item-section>
                        <q-item-section>
                          <q-item-label class="ellipsis"
                            >{{ option.opt }}
                          </q-item-label>
                        </q-item-section>
                      </q-item>
                    </q-list>
                  </template>
                </q-select>
            </div>
          </span>
  

          <span v-else>{{ errorMessage }}</span>
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
  name: "ImportAlert",
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
    const userSelectedDestinations = ref([]);

    const tempalteCreators = ref([])
    const destinationCreators = ref([])
    const alertCreators = ref([])
    const queryEditorPlaceholderFlag = ref(true);
    const streamList = ref([]);
    const userSelectedStreamName = ref("");
    const jsonFiles = ref(null);
    const jsonArrayOfObj = ref([
      {

      },
    ]);
    const activeTab = ref("import_json_file");
    const splitterModel = ref(60);
    const getFormattedDestinations = computed(() => {
      return props.destinations.map((destination) => {
        return destination.name;
      });
    });

    watch(() => userSelectedDestinations.value , (newVal, oldVal) => {
      if(newVal){
        jsonArrayOfObj.value.destinations = newVal;
        jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
      }
    })

   const  updateStreamFields = (stream_name) => {
      jsonArrayOfObj.value.stream_name = stream_name;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
   }

   const updateAlertName = (alertName) => {
    jsonArrayOfObj.value.name = alertName;
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
    label: "File Upload",
    value: "import_json_file",
},
{
    label: "URL Import",
    value: "import_json_url",
},
{
    label: "JSON Clipboard",
    value: "import_json_clipboard",
},


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

    onMounted(()=>{
    })


    const importJson = async () => {
      alertErrorsToDisplay.value = [];
      templateErrorsToDisplay.value = [];
      destinationErrorsToDisplay.value = [];
      destinationCreators.value = [];
      alertCreators.value = [];
      // userSelectedDestinations.value = [];

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
  // Step 1: Validate Template
  // await validateTemplateInputs(jsonObj.templatePayload,index);

  // Step 2: Validate All Destinations Sequentially
  // for (const [destinationIndex, destination] of jsonObj.destinationPayload.entries()) {
  //   await validateDestinationInputs(destination, destinationIndex, jsonObj.alertPayload.destinations, jsonObj.templatePayload.name,index);
  // }

  // Step 3: Validate Alert
  const isValidAlert = await validateAlertInputs(jsonObj,index);
  if (!isValidAlert) {
    return;
  }

  // Step 4: Check if there are no errors before proceeding
  if (alertErrorsToDisplay.value.length === 0 ) {

    // Step 5: Create Template
    // const hasCreatedTemplate = await createTemplate(jsonObj.templatePayload,index);
    // if (!hasCreatedTemplate) {
    //   return;
    // }

    // Step 6: Create Destinations
    // const results = await Promise.all(
    //   jsonObj.destinationPayload.map((destination: any) => createDestination(destination,index))
    // );

    // // Check if all results are successful
    // const allSuccessful = results.every(result => result === true);

    // if (!allSuccessful && !hasRolledBack) {
    //   // Step 7: Rollback Destinations if Creation Failed
    //   for (const [destinationIndex, destination] of jsonObj.destinationPayload.entries()) {
    //     await rollbackDestination(destination.name,  index, destinationIndex+1);
    //   }


      // Step 8: Rollback Template
      // await rollbackTemplate(jsonObj.templatePayload.name,index);

    //   hasRolledBack = true;
    //   return; // Exit after rollback
    // }

    // Step 9: Create Alert if No Rollback
    // if (!hasRolledBack) {
      const hasCreatedAlert = await createAlert(jsonObj,index);

      if(hasCreatedAlert) {
        q.notify({
            message: "Alert imported successfully",
            color: "positive",
            position: "bottom",
            timeout: 2000,
          });
          router.push({
            name: "alertList",
            query:{
              org_identifier: store.state.selectedOrganization.identifier
            }
          })
      }

      // if (!hasCreatedAlert) {
      //   // Step 10: Rollback Destinations if Alert Creation Failed
      //   for (const [destinationIndex, destination] of jsonObj.destinationPayload.entries()) {
      //   await rollbackDestination(destination.name,  index, destinationIndex+1);
      // }

      //   // Step 11: Rollback Template
      //   await rollbackTemplate(jsonObj.templatePayload.name,index);

      //   hasRolledBack = true;
      //   return; // Exit after rollback
      // }
    // }
  }
};

    const validateAlertInputs = async (input: any, index: any) => {
      let alertErrors: string[] = [];

      // 1. Validate 'name' field
      if (!input.name || typeof input.name !== 'string' || input.name.trim() === '') {
        alertErrors.push({
          message: `Alert - ${index}: Name is mandatory and should be a valid string.`,
          isDestination: false,
          isAlert: true,
        });
      }

      if (checkAlertsInList(props.alerts, input.name)) {
        alertErrors.push({
          message: `Alert - ${index}: "${input.name}" already exists`,
          isDestination: false,
          isAlert: true,
        });
      }
      const organizationData = store.state.organizations;
      const orgList = organizationData.map(org => org.identifier);

      // 2. Validate 'org_id' field
      if (!input.org_id || typeof input.org_id !== 'string' || input.org_id.trim() === '' || !orgList.includes(input.org_id)) {
        alertErrors.push(`Alert - ${index}: Organization Id is mandatory, should exist in organization list and should be a valid string.`);
      }

      // 3. Validate 'stream_type' field
      const validStreamTypes = ["logs", "metrics", "traces"];
      if (!input.stream_type || !validStreamTypes.includes(input.stream_type)) {
        alertErrors.push(`Alert - ${index}: Stream Type is mandatory and should be one of: 'logs', 'metrics', 'traces'.`);
      }

      try {
        const streamResponse = await getStreams(input.stream_type, false);
        streamList.value = streamResponse.list.map(stream => stream.name);
      } catch (e) {
        alertErrorsToDisplay.value = alertErrors;
      }

      // 4. Validate 'stream_name' field
      if (!input.stream_name || typeof input.stream_name !== 'string' || !streamList.value.includes(input.stream_name)) {
        alertErrors.push({
          message: `Alert - ${index}: Stream Name is mandatory, should exist in the stream list and should be a valid string.`,
          isDestination: false,
          isAlert: false,
        });
      }

      // 5. Validate 'is_real_time' field
      if (typeof input.is_real_time !== 'boolean') {
        alertErrors.push(`Alert - ${index}: Is Real-Time is mandatory and should be a boolean value.`);
      }

      // 6. Validate 'query_condition' field
      if (input.query_condition && input.query_condition.conditions) {
        if (!Array.isArray(input.query_condition.conditions)) {
          alertErrors.push(`Alert - ${index}: Query conditions should be an array.`);
        }

        for (let condition of input.query_condition.conditions) {
          if (!condition.column || !condition.operator || !condition.value) {
            alertErrors.push(`Alert - ${index}: Each query condition must have 'column', 'operator', and 'value'.`);
          }

          if (input.query_condition.type === 'custom' && !["=", ">", "<", ">=", "<=", "Contains", "NotContains"].includes(condition.operator)) {
            alertErrors.push(`Alert - ${index}: Invalid operator in query condition. Allowed operators: '=', '>', '<', '>=', '<=', 'Contains', 'NotContains'.`);
          }
        }
      }

      // 7. Validate 'sql' and 'promql'
      if (input.query_condition.type === 'sql' && typeof input.query_condition.sql !== 'string') {
        alertErrors.push(`Alert - ${index}: SQL should be provided when the type is 'sql'.`);
      }

      if (input.query_condition.type === 'promql' && typeof input.query_condition.promql !== 'string') {
        alertErrors.push(`Alert - ${index}: PromQL should be provided when the type is 'promql'.`);
      }

      // 8. Validate 'vrl_function'
      if (input.query_condition.vrl_function && typeof input.query_condition.vrl_function !== 'string') {
        alertErrors.push(`Alert - ${index}: VRL function should be a string or null.`);
      }

      // 9. Validate 'multi_time_range'
      if (!Array.isArray(input.query_condition.multi_time_range)) {
        alertErrors.push(`Alert - ${index}: Multi Time Range should be an empty array.`);
      }

      // 10. Validate 'trigger_condition'
      const triggerCondition = input.trigger_condition;
      if (!triggerCondition) {
        alertErrors.push(`Alert - ${index}: Trigger Condition is required.`);
      }

      if (isNaN(Number(triggerCondition.period)) || triggerCondition.period < 1 || typeof triggerCondition.period !== 'number') {
        alertErrors.push(`Alert - ${index}: Period should be a positive number greater than 0 and should be a number.`);
      }

      const validOperators = ["=", "!=", ">=", "<=", ">", "<", "Contains", "NotContains"];
      if (!validOperators.includes(triggerCondition.operator)) {
        alertErrors.push(`Alert - ${index}: Operator should be one of: '=', '!=', '>=', '<=', '>', '<', 'Contains', 'NotContains'.`);
      }

      if (isNaN(Number(triggerCondition.frequency)) || triggerCondition.frequency < 1 || typeof triggerCondition.frequency !== 'number') {
        alertErrors.push(`Alert - ${index}: Frequency should be a positive number greater than 0 and should be a number.`);
      }

      if (triggerCondition.cron && typeof triggerCondition.cron !== 'string') {
        alertErrors.push(`Alert - ${index}: Cron expression should be a string.`);
      }

      if (isNaN(Number(triggerCondition.threshold)) || triggerCondition.threshold < 1 || typeof triggerCondition.threshold !== 'number') {
        alertErrors.push(`Alert - ${index}: Threshold should be a positive number greater than 0 and should be a number.`);
      }

      if (isNaN(Number(triggerCondition.silence)) || triggerCondition.silence < 1 || typeof triggerCondition.silence !== 'number') {
        alertErrors.push(`Alert - ${index}: Silence should be a positive number greater than 0 and should be a number.`);
      }

      if (triggerCondition.frequency_type !== 'minutes' || typeof triggerCondition.frequency_type !== 'string') {
        alertErrors.push(`Alert - ${index}: Frequency Type must be 'minutes' and should be a string.`);
      }

      if (!input.destinations || !Array.isArray(input.destinations) || input.destinations.length === 0) {
        alertErrors.push({
          message: `Alert - ${index}: Destinations are required and should be an array.`,
          isDestination: true,
        });
      }

      if (typeof input.enabled !== 'boolean') {
        alertErrors.push(`Alert - ${index}: Enabled should be Boolean.`);
      }

      if (input.tz_offset && (typeof input.tz_offset !== 'number' || input.tz_offset < 0)) {
        alertErrors.push(`Alert - ${index}: Timezone offset should be a number.`);
      }


      input.destinations.forEach((destination: any)=>{
        if(!checkDestinationInList(props.destinations, destination)){
        alertErrors.push({
          message: `Alert - ${index}: "${destination}" destination does not exist`,
          isDestination: true,
        });
      }
      })

      // Log all alert errors at the end
      if (alertErrors.length > 0) {
        alertErrorsToDisplay.value.push(alertErrors);
        return false;
      }

      return true;
    };
    const validateTemplateInputs = async (input: any, index: any) => {
      let templateErrors: string[] = [];

      // Validate name: should be a non-empty string
      if (!input.name || typeof input.name !== 'string' || input.name.trim() === '') {
        templateErrors.push(`Template - ${index}: The "name" field is required and should be a valid string.`);
      }

      // Validate body: should be a non-empty string
      if (!input.body || typeof input.body !== 'string' || input.body.trim() === '') {
        templateErrors.push(`Template - ${index}: The "body" field is required and should be a valid string.`);
      }

      // Validate type: should be either "email" or "http"
      if (!input.type || (input.type !== 'email' && input.type !== 'http')) {
        templateErrors.push(`Template - ${index}: The "type" field must be either "email" or "http".`);
      }

      // Validate title based on type
      if (input.type === 'email') {
        // For email type, title should be a non-empty string
        if (!input.title || typeof input.title !== 'string' || input.title.trim() === '') {
          templateErrors.push(`Template - ${index}: The "title" field is required and should be a non-empty string for "email" type.`);
        }
      } else if (input.type === 'http') {
        // For http type, title should be empty
        if (input.title && input.title.trim() !== '') {
          templateErrors.push(`Template - ${index}: The "title" field should be empty for "http" type.`);
        }
      }

      if (checkTemplatesInList(props.templates, input.name)) {
        templateErrors.push(`Template - ${index}: "${input.name}" already exists`);
      }

      // If there are errors, log them at the end
      if (templateErrors.length > 0) {
        alertErrorsToDisplay.value.push(templateErrors);
        return false;
      }

      // If all validations pass
      return true;
    };
    const validateDestinationInputs = async (input: any, destinationIndex: number, destinationList: any, templateName: any, index: any) => {  
        let destinationErrors: string[] = [];

        // Check if 'url' is required for webhook and should not exist for email
        if (!input.hasOwnProperty('type') && !input.url) {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'url' is required for webhook`);
        }

        if (input.type === 'email' && input.url) {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'url' should not be provided for email`);
        }

        // Check type for email and it should be present only for email
        if (input.type !== 'email' && input.hasOwnProperty('type')) {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'type' should be email for email`);
        }

        // Check if 'method' is required for both webhook and email
        if (!input.method || (input.method !== 'post' && input.method !== 'get' && input.method !== 'put')) {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'method' is required and should be either 'post', 'get', or 'put'`);
        }

        // Check if 'skip_tls_verify' is required for both webhook and email, and it should be a boolean
        if (input.skip_tls_verify === undefined || typeof input.skip_tls_verify !== 'boolean') {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'skip_tls_verify' is required and should be a boolean value`);
        }

        // Check if 'headers' is required for webhook but not for email
        if (!input.hasOwnProperty('type') && Object.keys(input.headers).length === 0) {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'headers' is required for webhook`);
        }

        if (input.type === 'email' && Object.keys(input.headers).length !== 0) {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'headers' should not be provided for email`);
        }

        // Check if 'name' is required for both webhook and email
        if (!input.name || typeof input.name !== 'string') {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'name' is required and should be a string`);
        }

        // 'emails' should be required for email type and should be an array of strings
        if (input.type === 'email') {
          if (!Array.isArray(input.emails) || input.emails.some((email: any) => typeof email !== 'string')) {
            destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'emails' should be an array of strings for email`);
          }
        }

        // Check if 'template' is required for both webhook and email
        if (!input.template || typeof input.template !== 'string') {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'template' is required and should be a string`);
        }

        if (!destinationList.includes(input.name)) {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'name' should match one of the destinations provided in the alert`);
        }

        if (input.template !== templateName) {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: 'template' should match the template name provided in the template`);
        }

        if (checkDestinationInList(props.destinations, input.name)) {
          destinationErrors.push(`Destination - ${index} ---> [${destinationIndex + 1}]: "${input.name}" already exists`);
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

    const checkAlertsInList = (alerts, alertName) =>{
      const alertsList = alerts.map(alert => alert.name);
      console.log(alertsList,alertName);
      return alertsList.includes(alertName);
    }


    const createTemplate = async (input: any,index: any) => {
      try {
        // Await the template creation service call
        await templateService.create({
          org_identifier: store.state.selectedOrganization.identifier,
          template_name: input.name,
          data: {
            name: input.name.trim(),
            body: input.body,
            type: input.type,
            title: input.title,
          },
        });

    // Success block
    alertCreators.value.push({
      message: `Template - ${index}: "${input.name}" created successfully`,
      success: true,
    });
    emit("update:templates");
    return true; // Return true for success
  } catch (error) {
    // Error block
    alertCreators.value.push({
      message: `Template - ${index}: "${input.name}" creation failed`,
      success: false,
    });
    return false; // Return false for failure
  }
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


    const createAlert = async (input: any,index: any) => {
      try {
        await alertsService.create(
          store.state.selectedOrganization.identifier,
          input.stream_name,
          input.stream_type,
          input
        );
        
        // Success
        alertCreators.value.push({
          message: `Alert - ${index}: "${input.name}" created successfully \nNote: please remove the created alert object ${input.name} from the json file`,
          success: true,
        });
        emit("update:alerts");
        return true; // Return true if the alert creation is successful
      } catch (error) {
        // Failure
        alertCreators.value.push({
          message: `Alert - ${index}: "${input.name}" creation failed --> \n Reason: ${error?.response?.data?.message || "Unknown Error"}`,
          success: false,
        });
        return false; // Return false if there was an error
      }
    }


    const rollbackTemplate = async (templateName: any,index: any) => {
  try {
    // Await the delete service call for template
    await templateService.delete({
      org_identifier: store.state.selectedOrganization.identifier,
      template_name: templateName,
    });

    // Success block
    alertCreators.value.push({
      message: `Template - ${index}: "${templateName}" rolled back successfully`,
      success: true,
      rollback:true,
    });
    emit("update:templates");
    return true; // Return true for success
  } catch (error) {
    // Error block
    alertCreators.value.push({
      message: `Template - ${index}: "${templateName}" roll back failed`,
      success: false,
      rollback:false,
    });
    return false; // Return false for failure
  }
}

    const rollbackDestination = async (destinationName: any,index: any, destinationIndex: any) => {
      try {
        // Await the delete service call for destination
        // if(checkDestinationInList(props.destinations,destinationName)){
          await destinationService.delete({
            org_identifier: store.state.selectedOrganization.identifier,
            destination_name: destinationName,
          });
          alertCreators.value.push({
          message: `Destination - ${index}: ---> [${destinationIndex}] "${destinationName}" rolled back successfully`,
          success: true,
          rollback:true
        });
        emit("update:destinations");
        // }
        // Success block
        return true; // Return true for success
      } catch (error) {
        // Error block
        alertCreators.value.push({
          message: `Destination - ${index}: ---> [${destinationIndex}] "${destinationName}" roll back failed`,
          success: false,
          rollback:false,
        });
        return false; // Return false for failure
      }
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
      userSelectedDestinations,
      getFormattedDestinations,
      jsonArrayOfObj,
      streamList,
      userSelectedStreamName,
      updateStreamFields,
      updateAlertName,
      jsonFiles,
      updateActiveTab,
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
    height: calc(80vh - 20px) !important; 
  }
  .editor-container-url {
    .monaco-editor {
      height: calc(72vh - 8px) !important; /* Total editor height */
      overflow: auto;             /* Allows scrolling if content overflows */
      resize: none;               /* Remove resize behavior */
    }
  }
  .editor-container-json {
    .monaco-editor {
      height: calc(71vh - 20px) !important; /* Total editor height */
      overflow: auto;             /* Allows scrolling if content overflows */
      resize: none;               /* Remove resize behavior */
    }
  }
  .monaco-editor {
  height: calc(81vh - 14px) !important; /* Total editor height */
  overflow: auto;             /* Allows scrolling if content overflows */
  resize: none;               /* Remove resize behavior */
}
  .error-report-container {
  height: calc(78vh - 8px) !important; /* Total editor height */
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
