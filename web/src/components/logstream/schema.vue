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
  <q-card
    style="width: 60vw"
    class="column full-height no-wrap"
    v-if="indexData.schema"
  >
    <q-card-section class="q-ma-none">
      <div class="row items-center no-wrap">
        <div class="col">
          <div
            class="text-body1 tw-font-semibold tw-text-xl"
            data-test="schema-title-text"
          >
            {{ t("logStream.schemaHeader") }}
          </div>
        </div>
        <div class="col-auto">
          <q-btn v-close-popup="true" round flat icon="close" />
        </div>
      </div>
    </q-card-section>
    <q-separator />

    <q-card-section class="q-ma-none q-pa-none">
      <q-form ref="updateSettingsForm" @submit.prevent="onSubmit">
        <div
          v-if="loadingState"
          class="q-pt-md text-center q-w-md q-mx-lg tw-flex tw-justify-center"
          style="max-width: 450px"
        >
          <q-spinner-hourglass color="primary" size="lg" />
        </div>
        <div
          v-else-if="indexData.schema.length == 0"
          class="q-pt-md text-center q-w-md q-mx-lg"
          style="max-width: 450px"
        >
          No data available.
        </div>
        <div v-else class="indexDetailsContainer" style="height: 100vh">
          <div
            class="titleContainer tw-flex tw-flex-col tw-items-flex-start tw-gap-5"
          >
            <div
              data-test="stream-details-container"
              class="stream_details_container tw-flex tw-justify-between tw-gap-5 tw-flex-wrap"
            >
              <div data-test="schema-stream-title-text">
                {{ t("alerts.stream_name") }}
                <span class="title q-pl-xs"> {{ indexData.name }}</span>
              </div>
              <div
                v-if="store.state.zoConfig.show_stream_stats_doc_num"
                data-test="schema-stream-title-text"
              >
                {{ t("logStream.docsCount") }}
                <span class="title q-pl-xs">
                  {{
                    parseInt(indexData.stats.doc_num).toLocaleString("en-US")
                  }}
                </span>
              </div>
              <div data-test="schema-stream-title-text">
                {{ t("logStream.storageSize") }}
                <span class="title q-pl-xs">
                  {{ formatSizeFromMB(indexData.stats.storage_size) }}</span
                >
              </div>
              <div
                v-if="isCloud !== 'true'"
                data-test="schema-stream-title-text"
              >
                {{ t("logStream.compressedSize") }}
                <span class="title q-pl-xs">
                  {{ formatSizeFromMB(indexData.stats.compressed_size) }}</span
                >
              </div>
            </div>
            <div class="tw-flex tw-justify-between">
              <div
                v-if="isCloud !== 'true'"
                data-test="schema-stream-title-text"
              >
                {{ t("logStream.indexSize") }}
                <span class="title q-pl-xs">
                  {{ formatSizeFromMB(indexData.stats.index_size) }}</span
                >
              </div>
              <div
                class="stream-time-container"
                v-if="store.state.zoConfig.show_stream_stats_doc_num"
                data-test="schema-stream-title-text"
              >
                {{ t("logStream.time") }}
                <span class="title q-pl-xs">
                  {{ indexData.stats.doc_time_min }}
                </span>
                <span>to</span>
                <span class="title">{{ indexData.stats.doc_time_max }}</span>
              </div>
            </div>
          </div>

          <template v-if="showDataRetention">
            <div class="tw-flex tw-justify-between">
              <div class="row flex items-center q-pb-xs q-mt-lg">
                <div class="flex tw-flex-col">
                  <label class="q-pr-sm tw-font-medium"
                    >Data Retention in days</label
                  >
                  <span class="tw-text-xs tw-font-normal">
                    (Global retention is
                    {{ store.state.zoConfig.data_retention_days }} days)
                  </span>
                </div>
                <q-input
                  data-test="stream-details-data-retention-input"
                  v-model="dataRetentionDays"
                  type="number"
                  dense
                  filled
                  min="1"
                  round
                  class="q-mr-sm q-ml-sm data-retention-input"
                  hide-bottom-space
                  @change="formDirtyFlag = true"
                  @update:model-value="markFormDirty"
                />
                <div></div>
              </div>

              <div class="row flex items-center q-pb-xs q-mt-lg">
                <label class="q-pr-sm tw-font-medium"
                  >Max Query Range (in hours)</label
                >
                <q-input
                  data-test="stream-details-max-query-range-input"
                  v-model="maxQueryRange"
                  type="number"
                  dense
                  filled
                  min="0"
                  round
                  class="q-mr-sm data-retention-input"
                  @wheel.prevent
                  @change="formDirtyFlag = true"
                  @update:model-value="markFormDirty"
                />
              </div>

              <div class="row flex items-center q-pb-xs q-mt-lg">
                <q-toggle
                  data-test="log-stream-use_approx-toggle-btn"
                  v-model="approxPartition"
                  :label="t('logStream.approxPartition')"
                  @click="formDirtyFlag = true"
                  left-label
                  dense
                />
              </div>
            </div>
            <div
              class="q-ma-none q-pa-none text-negative"
              style="min-height: 20px"
            >
              <span v-if="dataRetentionDays <= 0 || dataRetentionDays == ''">
                Retention period must be at least 1 day
              </span>
            </div>
          </template>

          <div class="flex  q-mb-md justify-end " style="">
            <q-item
            :class="{
              'q-router-link--active':
              activeMainTab === 'schemaSettings',
            }"
             clickable @click="activeMainTab = 'schemaSettings'" :ripple="true">
              <q-item-section   avatar>
                <span class="flex"><q-icon size="16px" class="q-mr-xs" name="settings" />
                  <q-item-label>Stream Settings</q-item-label></span>
              </q-item-section>
            </q-item>
            <q-item
            :class="{
              'q-router-link--active':
              activeMainTab === 'redButton',
            }"
             @click="activeMainTab = 'redButton'"  clickable :ripple="true">
             <q-item-section   avatar>
                <span class="flex"><q-icon size="16px"  class="q-mr-xs" name="backup" />
                  <q-item-label>Extended Retention</q-item-label></span>
              </q-item-section>
            </q-item>
              </div>
          <!-- schema settings tab -->
          <div v-if="activeMainTab == 'schemaSettings'">

            <div
            class="title flex tw-justify-between tw-items-center"
            data-test="schema-log-stream-mapping-title-text"
          >
            <div style="font-weight: 400">
              <label
                v-show="indexData.defaultFts"
                style="font-weight: 600"
                class="mapping-warning-msg"
              >
                {{ t("logStream.mapping") }} Default FTS keys used (no custom
                keys set).</label
              >
            </div>
            <q-toggle
              data-test="log-stream-store-original-data-toggle-btn"
              v-model="storeOriginalData"
              :label="t('logStream.storeOriginalData')"
              @click="formDirtyFlag = true"
              left-label
              dense
            />
            </div>
            <div class="flex justify-between items-center full-width q-mb-md">
              <div>
                <app-tabs
                  v-if="isSchemaEvolutionEnabled"
                  class="schema-fields-tabs"
                  style="
                    border: 1px solid #8a8a8a;
                    border-radius: 4px;
                    overflow: hidden;
                  "
                  data-test="schema-fields-tabs"
                  :tabs="tabs"
                  :active-tab="activeTab"
                  @update:active-tab="updateActiveTab"
                />
              </div>
          </div>
          <div class="flex justify-between items-center full-width q-mb-md">
            <div>
              <app-tabs
                v-if="isSchemaEvolutionEnabled"
                class="schema-fields-tabs"
                style="
                  border: 1px solid #8a8a8a;
                  border-radius: 4px;
                  overflow: hidden;
                "
                data-test="schema-fields-tabs"
                :tabs="tabs"
                :active-tab="activeTab"
                @update:active-tab="updateActiveTab"
              />
            </div>

            <div class="flex items-center tw-gap-4">
              <q-input
                data-test="schema-field-search-input"
                v-model="filterField"
                data-cy="schema-index-field-search-input"
                filled
                borderless
                dense
                debounce="1"
                :placeholder="t('search.searchField')"
              >
                <template #prepend>
                  <q-icon name="search" />
                </template>
              </q-input>
              <q-btn
                color="primary"
                data-test="schema-add-fields-title"
                @click="openDialog"
                class="font-weight-bold"
              >
                Add Field(s)
              </q-btn>
            </div>
          </div>
          <div class="q-mb-md" v-if="isDialogOpen">
            <q-card class="add-fields-card">
              <!-- Header Section -->
              <q-card-section>
                <div class="tw-flex tw-justify-between tw-items-center">
                  <div class="text-h6">Add Field(s)</div>
                  <div>
                    <q-btn
                      data-test="add-stream-cancel-btn"
                      icon="close"
                      class="q-my-sm text-bold q-mr-md"
                      text-color="light-text"
                      padding="sm md"
                      no-caps
                      dense
                      flat
                      @click="closeDialog"
                    />
                  </div>
                </div>
              </q-card-section>
              <!-- Main Content (Scrollable if necessary) -->
              <q-card-section
                class="q-pa-none"
                style="flex: 1; overflow-y: auto"
              >
                <StreamFieldsInputs
                  :fields="newSchemaFields"
                  :showHeader="false"
                  :isInSchema="true"
                  :visibleInputs="{
                    name: true,
                    type: false,
                    index_type: false,
                  }"
                  @add="addSchemaField"
                  @remove="removeSchemaField"
                />
              </q-card-section>
            </q-card>
          </div>

          <!-- Note: Drawer max-height to be dynamically calculated with JS -->
          <div
            :class="
              store.state.theme === 'dark'
                ? 'dark-theme-table'
                : 'light-theme-table'
            "
            style="margin-bottom: 30px"
          >
            <q-table
              ref="qTable"
              data-test="schema-log-stream-field-mapping-table"
              :rows="indexData.schema"
              :columns="columns"
              :row-key="(row) => 'tr_' + row.name"
              :filter="`${filterField}@${activeTab}`"
              :filter-method="filterFieldFn"
              :pagination="pagination"
              selection="multiple"
              v-model:selected="selectedFields"
              class="q-table"
              id="schemaFieldList"
              :rows-per-page-options="[]"
              dense
            >
              <template v-slot:header="props">
                <q-tr :props="props">
                  <q-th>
                    <q-checkbox v-model="props.selected" color="primary" />
                  </q-th>
                  <q-th
                    v-for="col in props.cols"
                    :key="col.name"
                    :props="props"
                  >
                    <span v-if="col.icon">
                      <q-icon color="primary" :name="outlinedPerson"></q-icon>
                      <q-icon color="primary" :name="outlinedSchema"></q-icon>
                    </span>
                    <span v-else>
                      {{ col.label }}
                    </span>
                  </q-th>
                </q-tr>
              </template>
              <template v-slot:header-selection="scope">
                <q-td class="text-center">
                  <q-checkbox
                    v-if="
                      !(
                        scope.name == store.state.zoConfig.timestamp_column ||
                        scope.name == allFieldsName
                      )
                    "
                    :data-test="`schema-stream-delete-${scope.name}-field-fts-key-checkbox`"
                    v-model="scope.selected"
                    size="sm"
                  />
                </q-td>
              </template>

              <template v-slot:body-selection="scope">
                <q-td class="text-center q-td--no-hover">
                  <q-checkbox
                    v-if="
                      !(
                        scope.row.name ==
                          store.state.zoConfig.timestamp_column ||
                        scope.row.name == allFieldsName
                      )
                    "
                    :data-test="`schema-stream-delete-${scope.row.name}-field-fts-key-checkbox`"
                    v-model="scope.selected"
                    size="sm"
                  />
                </q-td>
              </template>

              <template v-slot:body-cell-name="props">
                <q-td class="q-td--no-hover field-name">{{
                  props.row.name
                }}</q-td>
              </template>
              <template v-slot:body-cell-type="props">
                <q-td>{{ props.row.type }}</q-td>
              </template>
              <template v-slot:body-cell-settings="props">
                <q-td class="text-left" v-if="props.row.isUserDefined">
                  <q-icon color="primary" :name="outlinedPerson"></q-icon>
                  <q-icon color="primary" :name="outlinedSchema"></q-icon>
                </q-td>
                <q-td v-else> </q-td>
              </template>
              <template v-slot:body-cell-index_type="props">
                <q-td data-test="schema-stream-index-select">
                  <q-select
                    v-if="
                      !(
                        props.row.name ==
                          store.state.zoConfig.timestamp_column ||
                        props.row.name == allFieldsName
                      )
                    "
                    v-model="props.row.index_type"
                    :options="streamIndexType"
                    :popup-content-style="{ textTransform: 'capitalize' }"
                    color="input-border"
                    bg-color="input-bg"
                    class="stream-schema-index-select q-py-xs fit q-pa-xs"
                    size="xs"
                    :option-disable="
                      (_option) => disableOptions(props.row, _option)
                    "
                    multiple
                    :max-values="2"
                    map-options
                    emit-value
                    autoclose
                    clearable
                    stack-label
                    outlined
                    filled
                    dense
                    style="min-width: 300px; max-width: 300px"
                    @update:model-value="markFormDirty(props.row.name, 'fts')"
                  />
                </q-td>
              </template>

              <template #bottom="scope">
                <QTablePagination
                  :scope="scope"
                  :position="'bottom'"
                  :resultTotal="resultTotal"
                  :perPageOptions="perPageOptions"
                  @update:changeRecordPerPage="changePagination"
                />
              </template>
            </q-table>



            </div>
          </div>
          <!-- red button tab -->
          <div v-else>
          <div class="q-mt-md">
            <div class="mapping-warning-msg q-mb-sm" style="width: fit-content;" >
              <span style="font-weight: 600;">
              <q-icon name="info" class="q-mr-xs" size="16px" />
              An additional {VALUE_FROM_CONFIG}10-day extension will be applied to the selected date ranges.</span>
            </div>

            <div class="text-center q-mt-sm tw-flex items-center ">
             <span class="text-bold"> Select Date</span>
              <date-time class="q-ml-sm" @on:date-change="dateChangeValue" :show-relative="false" :minDate="minDate" />
            </div>

            <div
              class="q-mt-md"
              style="margin-bottom: 30px"
            >
              <q-table
              ref="qTable"
                :row-key="(row,index) => 'tr_' + row.index "
                data-test="schema-log-stream-field-mapping-table"
                :rows="redBtnRows"
                :columns="redBtnColumns"
                :pagination="pagination"
                selection="multiple"
                v-model:selected="selectedDateFields"
                class="q-table"
                id="schemaFieldList"
                :rows-per-page-options="[]"
                dense
              >

              <template v-slot:header-selection="scope">
                <q-td class="text-center">
                  <q-checkbox
                    :data-test="`schema-stream-delete-${scope.name}-field-fts-key-checkbox`"
                    v-model="scope.selected"
                    size="sm"
                  />
                </q-td>
              </template>

              <template v-slot:body-selection="scope">
                <q-td class="text-center q-td--no-hover">
                  <q-checkbox
                    :data-test="`schema-stream-delete-${scope.row.name}-field-fts-key-checkbox`"
                    v-model="scope.selected"
                    size="sm"
                  />
                </q-td>
              </template>

                <template #bottom="scope">
                  <QTablePagination
                    :scope="scope"
                    :position="'bottom'"
                    :resultTotal="redBtnRows.length"
                    :perPageOptions="perPageOptions"
                    @update:changeRecordPerPage="changePagination"
                  />
                </template>
              </q-table>




            </div>
          </div>

          </div>
            <!-- floating footer for the table -->
            <div
              :class="
                store.state.theme === 'dark' ? 'dark-theme' : 'light-theme'
              "
              class="floating-buttons q-px-md q-py-xs"
            >
              <div v-if="indexData.schema.length > 0" class="q-mt-sm">
                <span class="q-px-sm q-py-md"
                  ><strong> {{ selectedFields.length }}</strong> fields
                  selected</span
                >
                <q-btn
                  v-if="isSchemaEvolutionEnabled"
                  data-test="schema-add-field-button"
                  class="q-my-sm no-border text-bold q-mr-md"
                  padding="sm md"
                  color="primary"
                  no-caps
                  v-bind:disable="!selectedFields.length"
                  @click="updateDefinedSchemaFields"
                >
                  <span class="flex items-center justify-start q-mr-sm">
                    <q-icon size="14px" :name="outlinedPerson" />
                    <q-icon size="14px" :name="outlinedSchema" />
                  </span>
                  {{
                    activeTab === "schemaFields"
                      ? t("logStream.removeSchemaField")
                      : t("logStream.addSchemaField")
                  }}
                </q-btn>
                <q-btn
                  v-bind:disable="!selectedFields.length"
                  data-test="schema-delete-button"
                  class="q-my-sm text-bold btn-delete"
                  text-color="red"
                  padding="sm md"
                  no-caps
                  dense
                  flat
                  style="border: 1px red solid"
                  @click="confirmQueryModeChangeDialog = true"
                >
                  <span class="flex items-center tw-gap-1">
                    <q-icon size="14px" :name="outlinedDelete" />
                    {{ t("logStream.delete") }}
                  </span>
                </q-btn>
                <q-btn
                  v-close-popup="true"
                  data-test="schema-cancel-button"
                  class="q-my-sm text-bold float-right q-ml-md btn-delete"
                  :label="t('logStream.cancel')"
                  text-color="red"
                  padding="sm md"
                  no-caps
                  dense
                  flat
                  style="border: 1px red solid"
                />
                <q-btn
                  v-bind:disable="!formDirtyFlag"
                  data-test="schema-update-settings-button"
                  :label="t('logStream.updateSettings')"
                  class="q-my-sm text-bold no-border q-ml-md float-right"
                  color="secondary"
                  padding="sm xl"
                  type="submit"
                  no-caps
                />
              </div>
            </div>
          </div>
        </div>
      </q-form>
      <br /><br /><br />
    </q-card-section>
  </q-card>
  <q-card v-else class="column q-pa-md full-height no-wrap">
    <h5>Wait while loading...</h5>
  </q-card>

  <ConfirmDialog
    title="Delete Action"
    :message="t('logStream.deleteActionMessage')"
    @update:ok="deleteFields()"
    @update:cancel="confirmQueryModeChangeDialog = false"
    v-model="confirmQueryModeChangeDialog"
  />
</template>

<script lang="ts">
// @ts-nocheck
import { computed, defineComponent, onBeforeMount, reactive, ref, onMounted,watch } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar, date, format } from "quasar";
import streamService from "../../services/stream";
import segment from "../../services/segment_analytics";
import { formatSizeFromMB, getImageURL,timestampToTimezoneDate } from "@/utils/zincutils";
import config from "@/aws-exports";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import useStreams from "@/composables/useStreams";
import { useRouter } from "vue-router";
import StreamFieldsInputs from "@/components/logstream/StreamFieldInputs.vue";
import AppTabs from "@/components/common/AppTabs.vue";

import QTablePagination from "@/components/shared/grid/Pagination.vue";
import {
  outlinedSchema,
  outlinedPerson,
  outlinedDelete,
} from "@quasar/extras/material-icons-outlined";

import DateTime from '@/components/DateTime.vue'

const defaultValue: any = () => {
  return {
    name: "",
    schema: [],
    stats: {},
    defaultFts: false,
    defined_schema_fields: [],
  };
};

export default defineComponent({
  name: "SchemaIndex",
  props: {
    // eslint-disable-next-line vue/require-default-prop
    modelValue: {
      type: Object,
      default: () => defaultValue(),
    },
  },
  components: {
    ConfirmDialog,
    StreamFieldsInputs,
    AppTabs,
    QTablePagination,
    DateTime,
  },
  setup({ modelValue }) {
    const { t } = useI18n();
    const store = useStore();
    const q = useQuasar();
    const indexData: any = ref(defaultValue());
    const updateSettingsForm: any = ref(null);
    const isCloud = config.isCloud;
    const dataRetentionDays = ref(0);
    const storeOriginalData = ref(false);
    const maxQueryRange = ref(0);
    const confirmQueryModeChangeDialog = ref(false);
    const formDirtyFlag = ref(false);
    const loadingState = ref(true);
    const rowsPerPage = ref(20);
    const filterField = ref("");
    const router = useRouter();
    const qTable = ref(null);
    const minDate = ref(null);
    const selectedDateFields = ref([]);
    const redBtnRows = ref(
  [
    {
      "start_time": "2024-12-06T16:44:43+05:30",
      "end_time": "2024-12-06T13:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T11:44:43+05:30",
      "end_time": "2024-12-06T10:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T09:44:43+05:30",
      "end_time": "2024-12-06T08:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T10:44:43+05:30",
      "end_time": "2024-12-06T12:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T12:44:43+05:30",
      "end_time": "2024-12-06T11:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T13:44:43+05:30",
      "end_time": "2024-12-06T12:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T10:44:43+05:30",
      "end_time": "2024-12-06T09:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T16:44:43+05:30",
      "end_time": "2024-12-06T13:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T11:44:43+05:30",
      "end_time": "2024-12-06T10:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T09:44:43+05:30",
      "end_time": "2024-12-06T08:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T10:44:43+05:30",
      "end_time": "2024-12-06T12:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T12:44:43+05:30",
      "end_time": "2024-12-06T11:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T13:44:43+05:30",
      "end_time": "2024-12-06T12:44:43+05:30",
    },
    {
      "start_time": "2024-12-06T10:44:43+05:30",
      "end_time": "2024-12-06T09:44:43+05:30",
    }
  ].map((row, index) => ({
    ...row,
    index: index + 1,  // Add index starting from 1
  }))
);

    const newSchemaFields = ref([]);
    const activeTab = ref("allFields");
    const activeMainTab = ref("schemaSettings");
    let previousSchemaVersion: any = null;
    const approxPartition = ref(false);
    const isDialogOpen = ref(false);
    const resultTotal = ref<number>(0);
    const perPageOptions: any = [
      { label: "5", value: 5 },
      { label: "10", value: 10 },
      { label: "20", value: 20 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "All", value: 0 },
    ];

    const changePagination = (val: { label: string; value: any }) => {
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value?.setPagination(pagination.value);
    };

    const selectedPerPage = ref<number>(20);
    const pagination: any = ref({
      rowsPerPage: 20,
    });

    const selectedFields = ref([]);

    const hasUserDefinedSchema = computed(() => {
      return !!indexData.value.defined_schema_fields?.length;
    });

    const allFieldsName = computed(() => {
      return store.state.zoConfig.all_fields_name;
    });

    const tabs = computed(() => [
      {
        value: "allFields",
        label: `All Fields (${indexData.value.schema.length})`,
        disabled: false,
      },
      {
        value: "schemaFields",
        label: `User Defined Schema (${indexData.value.defined_schema_fields.length})`,
        disabled: !hasUserDefinedSchema.value,
      },
    ]);
    const mainTabs = computed(() => [
      {
        value: "redButton",
        label: `Data Prolongation`,
        disabled: false
      },
      {
        value: "schemaSettings",
        label: `Schema Settings`,
        disabled: false,
      },
    ]);

    const streamIndexType = [
      { label: "Full text search", value: "fullTextSearchKey" },
      { label: "Secondary index", value: "secondaryIndexKey" },
      { label: "Bloom filter", value: "bloomFilterKey" },
      { label: "KeyValue partition", value: "keyPartition" },
      { label: "Prefix partition", value: "prefixPartition" },

      { label: "Hash partition (8 Buckets)", value: "hashPartition_8" },
      { label: "Hash partition (16 Buckets)", value: "hashPartition_16" },
      { label: "Hash partition (32 Buckets)", value: "hashPartition_32" },
      { label: "Hash partition (64 Buckets)", value: "hashPartition_64" },
      { label: "Hash partition (128 Buckets)", value: "hashPartition_128" },
    ];
    const { getStream, getUpdatedSettings } = useStreams();



    onBeforeMount(() => {
      dataRetentionDays.value = store.state.zoConfig.data_retention_days || 0;
      maxQueryRange.value = 0;
      storeOriginalData.value = false;
      approxPartition.value = false;
    });

    const isSchemaEvolutionEnabled = computed(() => {
      return store.state.zoConfig.user_defined_schemas_enabled;
    });

    const markFormDirty = () => {
      formDirtyFlag.value = true;
    };
    const deleteFields = async () => {
      loadingState.value = true;
      await streamService
        .deleteFields(
          store.state.selectedOrganization.identifier,
          indexData.value.name,
          selectedFields.value.map((field) => field.name),
        )
        .then(async (res) => {
          loadingState.value = false;
          if (res.data.code == 200) {
            q.notify({
              color: "positive",
              message: "Field(s) deleted successfully.",
              timeout: 2000,
            });
            confirmQueryModeChangeDialog.value = false;
            selectedFields.value = [];
            await getStream(
              indexData.value.name,
              indexData.value.stream_type,
              true,
              true,
            );
            getSchema();
          } else {
            q.notify({
              color: "negative",
              message: res.data.message,
              timeout: 2000,
            });
          }
        })
        .catch((err: any) => {
          loadingState.value = false;
          console.log(err);
          q.notify({
            color: "negative",
            message: err.message,
            timeout: 2000,
          });
        });
    };

    const getFieldIndices = (property, settings) => {
      const fieldIndices = [];
      if (
        (settings.full_text_search_keys.length > 0 &&
          settings.full_text_search_keys.includes(property.name)) ||
        (settings.full_text_search_keys.length == 0 &&
          store.state.zoConfig.default_fts_keys.includes(property.name))
      ) {
        fieldIndices.push("fullTextSearchKey");
      }

      if (
        settings.index_fields.length > 0 &&
        settings.index_fields.includes(property.name)
      ) {
        fieldIndices.push("secondaryIndexKey");
      }

      if (
        settings.bloom_filter_fields.length > 0 &&
        settings.bloom_filter_fields.includes(property.name)
      ) {
        fieldIndices.push("bloomFilterKey");
      }

      property["delete"] = false;

      if (
        settings.partition_keys &&
        Object.values(settings.partition_keys).some(
          (v) => !v.disabled && v.field === property.name,
        )
      ) {
        const [level, partition] = Object.entries(settings.partition_keys).find(
          ([, partition]) => partition["field"] === property.name,
        );

        property.level = level;

        if (partition.types === "value") fieldIndices.push("keyPartition");
        if (partition.types === "prefix") fieldIndices.push("prefixPartition");

        if (partition.types?.hash)
          fieldIndices.push(`hashPartition_${partition.types.hash}`);
      }

      property.index_type = [...fieldIndices];

      return fieldIndices;
    };

    const setSchema = (streamResponse) => {
      const schemaMapping = new Set([]);
      if (streamResponse?.settings) {
        // console.log("streamResponse:", streamResponse);
        previousSchemaVersion = JSON.parse(
          JSON.stringify(streamResponse.settings),
        );
      }
      if (!streamResponse.schema?.length) {
        streamResponse.schema = [];
        if (streamResponse.settings.defined_schema_fields?.length)
          streamResponse.settings.defined_schema_fields.forEach((field) => {
            streamResponse.schema.push({
              name: field,
              delete: false,
              index_type: [],
            });
          });
      }

      if (
        streamResponse.settings.full_text_search_keys.length == 0 &&
        (showFullTextSearchColumn.value || showPartitionColumn.value)
      ) {
        indexData.value.defaultFts = true;
      } else {
        indexData.value.defaultFts = false;
      }

      indexData.value.schema = streamResponse.schema || [];
      indexData.value.stats = JSON.parse(JSON.stringify(streamResponse.stats));

      indexData.value.stats.doc_time_max = date.formatDate(
        parseInt(streamResponse.stats.doc_time_max) / 1000,
        "YYYY-MM-DDTHH:mm:ss:SSZ",
      );
      indexData.value.stats.doc_time_min = date.formatDate(
        parseInt(streamResponse.stats.doc_time_min) / 1000,
        "YYYY-MM-DDTHH:mm:ss:SSZ",
      );

      indexData.value.defined_schema_fields =
        streamResponse.settings.defined_schema_fields || [];

      if (showDataRetention.value)
        dataRetentionDays.value =
          streamResponse.settings.data_retention ||
          store.state.zoConfig.data_retention_days;
        calculateDateRange();

      maxQueryRange.value = streamResponse.settings.max_query_range || 0;
      storeOriginalData.value =
        streamResponse.settings.store_original_data || false;
      approxPartition.value = streamResponse.settings.approx_partition || false;

      if (!streamResponse.schema) {
        loadingState.value = false;
        dismiss();
        return;
      }

      let fieldIndices = [];
      for (var property of streamResponse.schema) {
        schemaMapping.add(property.name);

        fieldIndices = getFieldIndices(property, streamResponse.settings);

        property.index_type = [...fieldIndices];

        fieldIndices.length = 0;
      }

      indexData.value.defined_schema_fields.forEach((field) => {
        if (!schemaMapping.has(field)) {
          const property = {
            name: field,
            delete: false,
            index_type: [],
          };

          fieldIndices = getFieldIndices(property, streamResponse.settings);

          property.index_type = [...fieldIndices];

          fieldIndices.length = 0;
          indexData.value.schema.push(property);
        }
      });
    };

    const getSchema = async () => {
      const dismiss = q.notify({
        spinner: true,
        message: "Please wait while loading stats...",
      });

      await getStream(indexData.value.name, indexData.value.stream_type, true)
        .then((streamResponse) => {
          streamResponse = updateStreamResponse(streamResponse);
          setSchema(streamResponse);
          if (activeTab.value === "schemaFields") {
            resultTotal.value =
              streamResponse.settings?.defined_schema_fields?.length;
          } else {
            resultTotal.value = streamResponse.schema?.length;
          }
          loadingState.value = false;
          dismiss();
        })
        .catch((err) => {
          loadingState.value = false;
          console.log(err);
        });
    };

    const onSubmit = async () => {
      let settings = {
        partition_keys: [],
        index_fields: [],
        full_text_search_keys: [],
        bloom_filter_fields: [],
        defined_schema_fields: [...indexData.value.defined_schema_fields],
      };
      if (showDataRetention.value && dataRetentionDays.value < 1) {
        q.notify({
          color: "negative",
          message:
            "Invalid Data Retention Period: Retention period must be at least 1 day.",
          timeout: 4000,
        });
        return;
      }
      if (Number(maxQueryRange.value) > 0) {
        settings["max_query_range"] = Number(maxQueryRange.value);
      } else {
        settings["max_query_range"] = 0;
      }

      if (showDataRetention.value) {
        settings["data_retention"] = Number(dataRetentionDays.value);
        calculateDateRange();
      }

      settings["store_original_data"] = storeOriginalData.value;
      settings["approx_partition"] = approxPartition.value;

      const newSchemaFieldsSet = new Set(
        newSchemaFields.value.map((field) =>
          field.name.trim().toLowerCase().replace(/ /g, "_").replace(/-/g, "_"),
        ),
      );

      // Push unique and normalized field names to settings.defined_schema_fields
      settings.defined_schema_fields.push(...newSchemaFieldsSet);

      let added_part_keys = [];
      for (var property of indexData.value.schema) {
        property.index_type?.forEach((index: string) => {
          if (index === "fullTextSearchKey") {
            settings.full_text_search_keys.push(property.name);
          }

          if (index === "secondaryIndexKey") {
            settings.index_fields.push(property.name);
          }

          if (property.level && index === "keyPartition") {
            settings.partition_keys.push({
              field: property.name,
              types: "value",
            });
          } else if (index === "keyPartition") {
            added_part_keys.push({
              field: property.name,
              types: "value",
            });
          }

          if (property.level && index === "prefixPartition") {
            settings.partition_keys.push({
              field: property.name,
              types: "prefix",
            });
          } else if (index === "prefixPartition") {
            added_part_keys.push({
              field: property.name,
              types: "prefix",
            });
          }

          if (index?.includes("hashPartition")) {
            const [, buckets] = index.split("_");

            if (property.level) {
              settings.partition_keys.push({
                field: property.name,
                types: {
                  hash: Number(buckets),
                },
              });
            } else {
              added_part_keys.push({
                field: property.name,
                types: {
                  hash: Number(buckets),
                },
              });
            }
          }

          if (index === "bloomFilterKey") {
            settings.bloom_filter_fields.push(property.name);
          }
        });
      }
      if (added_part_keys.length > 0) {
        settings.partition_keys =
          settings.partition_keys.concat(added_part_keys);
      }

      loadingState.value = true;

      newSchemaFields.value = [];

      let modifiedSettings = getUpdatedSettings(
        previousSchemaVersion,
        settings,
      );
      await streamService
        .updateSettings(
          store.state.selectedOrganization.identifier,
          indexData.value.name,
          indexData.value.stream_type,
          modifiedSettings,
        )
        .then(async (res) => {
          await getStream(
            indexData.value.name,
            indexData.value.stream_type,
            true,
            true,
          ).then((streamResponse) => {
            formDirtyFlag.value = false;
            streamResponse = updateStreamResponse(streamResponse);
            setSchema(streamResponse);
            loadingState.value = false;
            isDialogOpen.value = false;
            q.notify({
              color: "positive",
              message: "Stream settings updated successfully.",
              timeout: 2000,
            });
          });

          segment.track("Button Click", {
            button: "Update Settings",
            user_org: store.state.selectedOrganization.identifier,
            user_id: store.state.userInfo.email,
            stream_name: indexData.value.name,
            page: "Schema Details",
          });
        })
        .catch((err: any) => {
          loadingState.value = false;
          q.notify({
            color: "negative",
            message: err.response.data.message,
            timeout: 2000,
          });
        });
    };

    const showPartitionColumn = computed(() => {
      return (
        isCloud != "true" && modelValue.stream_type !== "enrichment_tables"
      );
    });

    const showFullTextSearchColumn = computed(
      () => modelValue.stream_type !== "enrichment_tables",
    );

    const showDataRetention = computed(
      () =>
        !!(store.state.zoConfig.data_retention_days || false) &&
        modelValue.stream_type !== "enrichment_tables",
    );

    const disableOptions = (schema, option) => {
      let selectedHashPartition = "";

      let selectedIndices = "";

      for (let i = 0; i < (schema?.index_type || []).length; i++) {
        if (schema.index_type[i].includes("hashPartition")) {
          selectedHashPartition = schema.index_type[i];
        }
        selectedIndices += schema.index_type[i];
      }

      if (
        selectedIndices.includes("prefixPartition") &&
        option.value.includes("keyPartition")
      ) {
        return true;
      }
      if (
        selectedIndices.includes("keyPartition") &&
        option.value.includes("prefixPartition")
      ) {
        return true;
      }
      if (
        selectedIndices.includes("hashPartition") &&
        selectedHashPartition !== option.value &&
        (option.value.includes("hashPartition") ||
          option.value.includes("keyPartition") ||
          option.value.includes("prefixPartition"))
      )
        return true;
      if (
        (selectedIndices.includes("keyPartition") ||
          selectedIndices.includes("prefixPartition")) &&
        option.value.includes("hashPartition")
      )
        return true;

      return false;
    };

    const filterFieldFn = (rows: any, terms: any) => {
      let [field, fieldType] = terms.split("@");

      var filtered = [];

      field = field.toLowerCase();
      for (var i = 0; i < rows.length; i++) {
        if (fieldType === "schemaFields") {
          if (indexData.value.defined_schema_fields.includes(rows[i]["name"])) {
            if (!field) {
              filtered.push(rows[i]);
            } else {
              if (rows[i]["name"].toLowerCase().includes(field)) {
                filtered.push(rows[i]);
              }
            }
          }
        } else {
          if (rows[i]["name"].toLowerCase().includes(field)) {
            filtered.push(rows[i]);
          }
        }
      }

      return filtered;
    };

    const columns = [
      {
        name: "name",
        label: t("logStream.propertyName"),
        align: "center",
        sortable: true,
        field: "name",
      },
      {
        name: "settings",
        align: "left",
        sortable: true,
        field: "isUserDefined",
        icon: "settings",
        sort: (a, b) => {
          // Ensure `isUserDefined` is properly handled
          if (a && !b) return -1; // `a` comes first
          if (!a && b) return 1; // `b` comes first
          return 0; // No change in order
        },
      },
      {
        name: "type",
        label: t("logStream.propertyType"),
        align: "left",
        sortable: true,
        field: "type",
      },
      {
        name: "index_type",
        label: t("logStream.indexType"),
        align: "left",
        sortable: false,
      },
    ];

    const redBtnColumns = [
    {
        name: "start_time",
        label: "Start Date",
        align: "center",
        sortable: true,
        field: "start_time",
      },
    {
        name: "end_time",
        label: "End Date",
        align: "center",
        sortable: true,
        field: "end_time",
      },
    ]



    const addSchemaField = () => {
      newSchemaFields.value.push({
        name: "",
        type: "",
        index_type: [],
      });
      formDirtyFlag.value = true;
    };

    const removeSchemaField = (field: any, index: number) => {
      newSchemaFields.value.splice(index, 1);
      if (newSchemaFields.value.length === 0) {
        isDialogOpen.value = false;
        newSchemaFields.value = [];
      }
    };

    const scrollToAddFields = () => {
      const el = document.getElementById("schema-add-fields-section");
      if (el) {
        el.scrollIntoView({ behavior: "smooth" });
      }
    };

    const updateActiveTab = (tab) => {
      activeTab.value = tab;
      if (tab === "schemaFields") {
        resultTotal.value = indexData.value.defined_schema_fields.length;
      } else {
        resultTotal.value = indexData.value.schema.length;
      }
    };
    const updateActiveMainTab = (tab) => {
      activeMainTab.value = tab;
    };

    const updateDefinedSchemaFields = () => {
      markFormDirty();

      const selectedFieldsSet = new Set(
        selectedFields.value.map((field) => field.name),
      );

      if (selectedFieldsSet.has(allFieldsName.value))
        selectedFieldsSet.delete(allFieldsName.value);

      if (selectedFieldsSet.has(store.state.zoConfig.timestamp_column))
        selectedFieldsSet.delete(store.state.zoConfig.timestamp_column);

      if (activeTab.value === "schemaFields") {
        indexData.value.defined_schema_fields =
          indexData.value.defined_schema_fields.filter(
            (field) => !selectedFieldsSet.has(field),
          );

        if (!indexData.value.defined_schema_fields.length) {
          activeTab.value = "allFields";
        }
      } else {
        indexData.value.defined_schema_fields = [
          ...new Set([
            ...indexData.value.defined_schema_fields,
            ...selectedFieldsSet,
          ]),
        ];
      }

      selectedFields.value = [];
    };
    const updateStreamResponse = (streamResponse) => {
      if (streamResponse.settings.hasOwnProperty("defined_schema_fields")) {
        const userDefinedSchema = streamResponse.settings.defined_schema_fields;

        // Map through the schema and add `isUserDefined` field
        const updatedSchema = streamResponse.schema.map((field) => ({
          ...field,
          isUserDefined: userDefinedSchema.includes(field.name), // Mark true if in userDefinedSchema
        }));

        // Find fields in userDefinedSchema that are not in the schema
        const additionalFields = userDefinedSchema
          .filter(
            (name) =>
              !streamResponse.schema.some((field) => field.name === name),
          )
          .map((name) => ({
            name,
            isUserDefined: true,
            // Optionally, add default values for other properties (e.g., type, index_type, etc.)
          }));

        // Combine the updated schema with additional fields
        streamResponse.schema = [...updatedSchema, ...additionalFields];
      }
      updateResultTotal(streamResponse);
      return streamResponse;
    };
    const closeDialog = () => {
      isDialogOpen.value = false;
      newSchemaFields.value = [];
    };

    const openDialog = () => {
      isDialogOpen.value = true;
      formDirtyFlag.value = true;
      newSchemaFields.value = [
        {
          name: "",
          type: "",
          index_type: [],
        },
      ];
    };
    const updateResultTotal = (streamResponse) => {
      if (activeTab.value === "schemaFields") {
        resultTotal.value =
          streamResponse.settings?.defined_schema_fields?.length;
      } else {
        resultTotal.value = streamResponse.schema?.length;
      }
    };
    function convertUnixToQuasarFormat(unixMicroseconds: any) {
      if (!unixMicroseconds) return "";
      const unixSeconds = unixMicroseconds / 1e6;
      const dateToFormat = new Date(unixSeconds * 1000);
      const formattedDate = dateToFormat.toISOString();
      return date.formatDate(formattedDate, "YYYY-MM-DDTHH:mm:ssZ");
    } 
    const dateChangeValue = (value) => {
      if(value.relativeTimePeriod == null){      
        redBtnRows.value.push(
        {
          "start_time": convertUnixToQuasarFormat(value.startTime),
          "end_time": convertUnixToQuasarFormat(value.endTime),
        }
      )
      q.notify({
              color: "positive",
              message: "Date added Successfully",
              timeout: 2000,
            });
      
      }
    };
    const calculateDateRange = () => {
    const today = new Date();
    const thirtyDaysAgo = new Date(today);
    thirtyDaysAgo.setDate(today.getDate() - dataRetentionDays.value); // Adjust to the desired number of days (10 days in this case)

  // If no min/max date is provided, default to 30 days ago and today
      const formattedDate = timestampToTimezoneDate (
        new Date().getTime(), // Current timestamp
        store.state.timezone, // Get the timezone from the store
        "yyyy/MM/dd" // Desired format
      );
      // Format minDate using timestampToTimezoneDate for a custom format
      minDate.value = timestampToTimezoneDate(
        thirtyDaysAgo.getTime(),
        store.state.timezone,
        "yyyy/MM/dd" // Desired format
      );
  };



    return {
      t,
      q,
      store,
      dateChangeValue,
      isCloud,
      indexData,
      getSchema,
      onSubmit,
      updateSettingsForm,
      format,
      showPartitionColumn,
      showFullTextSearchColumn,
      getImageURL,
      dataRetentionDays,
      storeOriginalData,
      approxPartition,
      maxQueryRange,
      showDataRetention,
      formatSizeFromMB,
      confirmQueryModeChangeDialog,
      deleteFields,
      markFormDirty,
      formDirtyFlag,
      streamIndexType,
      disableOptions,
      loadingState,
      filterFieldFn,
      rowsPerPage,
      filterField,
      columns,
      addSchemaField,
      removeSchemaField,
      newSchemaFields,
      scrollToAddFields,
      tabs,
      activeTab,
      updateActiveTab,
      hasUserDefinedSchema,
      isSchemaEvolutionEnabled,
      updateDefinedSchemaFields,
      selectedFields,
      allFieldsName,
      updateStreamResponse,
      isDialogOpen,
      closeDialog,
      resultTotal,
      perPageOptions,
      changePagination,
      selectedPerPage,
      pagination,
      qTable,
      outlinedPerson,
      outlinedSchema,
      outlinedDelete,
      openDialog,
      calculateDateRange,
      minDate,
      mainTabs,
      activeMainTab,
      updateActiveMainTab,
      redBtnColumns,
      redBtnRows,
      selectedDateFields,
    };
  },
  created() {
    if (this.modelValue && this.modelValue.name) {
      this.indexData.name = this.modelValue.name;
      this.indexData.schema = this.modelValue.schema;
      this.indexData.stream_type = this.modelValue.stream_type;

      this.getSchema();
    } else {
      this.loadingState.value = false;
    }
  },
});
</script>

<style lang="scss" scoped>
.q-card__section--vert {
  padding: 8px 16px;
}
.indexDetailsContainer {
  padding: 1.25rem;
  width: 100%;

  .title {
    margin-bottom: 1rem;
    font-weight: 700;
  }

  .titleContainer {
    background-color: #00000005;
    border: 1px solid $input-field-border-color;
    border-radius: 5px;
    padding: 1rem;
  }

  .q-table {
    border: 1px solid $input-field-border-color;
  }

  .q-table {
    border-radius: 0.5rem;
    position: relative;

    thead tr {
      height: 2.5rem;

      th {
        font-size: 0.875rem;
        font-weight: 700;
        height: 35px;
      }
    }

    .q-table tbody td:after {
      background: none !important;
    }

    tbody tr {
      height: 15px;

      td {
        font-size: 0.875rem;
        font-weight: 600;
        height: 25px;
        padding: 0px 5px;
      }
    }
  }

  .q-list {
    border-radius: 0 0 0.5rem 0.5rem;

    .q-item {
      height: 2.5rem;
      padding: 0;

      &__section {
        padding: 0.5rem 1rem;
        font-size: 0.875rem;

        &:not(:first-child) {
          border-left: 1px solid $input-field-border-color;
          align-items: flex-start;
          min-width: 29%;
        }
      }

      &.list-head {
        border: 1px solid $input-field-border-color;
        border-radius: 0.5rem 0.5rem 0 0;
        border-bottom: none;
      }

      &.list-item {
        border-right: 1px solid $input-field-border-color;
        border-left: 1px solid $input-field-border-color;

        &,
        &--side {
          font-weight: 600;
        }

        &:last-of-type {
          border-bottom: 1px solid $input-field-border-color;
          border-radius: 0 0 0.5rem 0.5rem;
        }
      }
    }
  }

  .data-retention-input {
    border: 1px solid $input-field-border-color;
    border-radius: 0.2rem;
    width: 80px;
    height: 39px;
    &.q-field {
      padding-bottom: 0 !important;
    }
  }
}

.mapping-warning-msg {
  background-color: #f9f290;
  padding: 8px 16px;
  border-radius: 4px;
  border: 1px solid #f5a623;
  color: #865300;
}

.q-item {
  padding: 3px 8px;
  margin: 0 8px;
  border-radius: 6px;

  /* Overriding default height */
  min-height: 30px;

  &.q-router-link--active {
    background-color: $primary;
    color: white;

    &::before {
      content: " ";

      position: absolute;
      top: 0;
      background-color: inherit;
    }
  }

  &.ql-item-mini {
    margin: 0;

    &::before {
      display: none;
    }
  }
}

.q-item__section--avatar {
  margin: 0;
  padding: 0;
  min-width: 40px;
}

// .sticky-buttons {
//   position: sticky;
//   bottom: 0px;
//   margin: 0 auto;
//   background-color: var(--q-accent);
//   box-shadow: 6px 6px 18px var(--q-accent);
//   justify-content: right;
//   width: 100%;
//   padding-right: 20px;
// }

// .btn-delete {
//   left: 20px;
//   position: absolute;
// }

// .sticky-table-header {
//   position: sticky;
//   top: 0px;
//   background: var(--q-accent);
//   z-index: 1;
// }

// .body--dark {
//   .sticky-table-header {
//     background: var(--q-dark);
//   }

//   .sticky-buttons {
//     background-color: var(--q-dark);
//     box-shadow: 6px 6px 18px var(--q-dark);
//   }
// }
</style>

<style lang="scss">
.add-fields-card {
  width: 100vw;
  max-width: 100%;
  display: flex;
  flex-direction: column;
}
.stream-schema-index-select {
  .q-field__control {
    .q-field__control-container {
      span {
        overflow: hidden;
        text-overflow: ellipsis;
        text-wrap: nowrap;
        display: inline-block;
      }
    }
  }
}
// background:
//             activeTab.value === "allFields" ? "#5960B2 !important" : "",
// color: activeTab.value === "allFields" ? "#ffffff !important" : "",

.indexDetailsContainer {
  .q-table__control {
    width: 100%;
  }

  .q-table {
    td:nth-child(2) {
      min-width: 20rem;
      width: 20rem;
      max-width: 20rem;
      overflow: auto;
      scrollbar-width: thin;
      scrollbar-color: #999 #f0f0f0;
    }
    td:nth-child(3) {
      padding: 4px 8px !important;
    }
  }

  th:first-child,
  td:first-child {
    padding-left: 8px !important;
  }
}
.dark-theme-table tr:hover td:nth-child(2) {
  background-color: #272a2b !important;
}
.light-theme-table tr:hover td:nth-child(2) {
  background-color: #f7f7f7 !important;
}

.schema-fields-tabs {
  height: fit-content;
  .rum-tab {
    width: fit-content !important;
    padding: 4px 12px !important;
    border: none !important;

    &.active {
      background: #5960b2;
      color: #ffffff !important;
    }
  }
}
.main-fields-tabs {
  height: fit-content;
  color: var(--q-light);
  .rum-tab {
    padding: 4px 12px !important;

    &.active {
      color: white !important;
      border-bottom: 5px solid var(--q-primary) !important;
      background-color:rgba(88, 96, 178,0.6);

    }
  }
}
.floating-buttons {
  position: sticky;
  bottom: 0;
  top: 0;
  z-index: 1; /* Ensure it stays on top of table content */
  width: 100%;
}
.dark-theme {
  background-color: var(--q-light);
  backdrop-filter: blur(10px);
}
.light-theme {
  background-color: var(--q-light);
  backdrop-filter: blur(10px);
}

.warning-text {
  color: #F5A623;
  border: 1px solid #F5A623;
  border-radius: 10px ;
  padding: 6px 3px;
}


</style>
