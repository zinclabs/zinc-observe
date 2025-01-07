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
  <div class="full-width">
    <div class="row items-center no-wrap q-mx-md q-my-sm">
      <div class="flex items-center">
        <div
          data-test="add-alert-back-btn"
          class="flex justify-center items-center q-mr-md cursor-pointer"
          style="
            border: 1.5px solid;
            border-radius: 50%;
            width: 22px;
            height: 22px;
          "
          title="Go Back"
          @click="router.back()"
        >
          <q-icon name="arrow_back_ios_new" size="14px" />
        </div>
        <div v-if="beingUpdated" class="text-h6" data-test="add-alert-title">
          {{ t("alerts.updateTitle") }}
        </div>
        <div v-else class="text-h6" data-test="add-alert-title">
          {{ t("alerts.addTitle") }}
        </div>
      </div>
    </div>

    <q-separator />
    <div
      ref="addAlertFormRef"
      class="q-px-lg q-my-md"
      style="
        max-height: calc(100vh - 138px);
        overflow: auto;
        scroll-behavior: smooth;
      "
    >
      <div class="row justify-start items-start" style="width: 1024px">
        <div style="width: calc(100% - 401px)">
          <q-form class="add-alert-form" ref="addAlertForm" @submit="onSubmit">
            <div
              class="flex justify-start items-center q-pb-sm q-col-gutter-md flex-wrap"
            >
              <div
                data-test="add-alert-name-input"
                class="alert-name-input o2-input"
                style="padding-top: 12px"
              >
                <q-input
                  v-model="formData.name"
                  :label="t('alerts.name') + ' *'"
                  color="input-border"
                  bg-color="input-bg"
                  class="showLabelOnTop"
                  stack-label
                  outlined
                  filled
                  dense
                  v-bind:readonly="beingUpdated"
                  v-bind:disable="beingUpdated"
                  :rules="[
                    (val: any) =>
                      !!val
                        ? isValidResourceName(val) ||
                          `Characters like :, ?, /, #, and spaces are not allowed.`
                        : t('common.nameRequired'),
                  ]"
                  tabindex="0"
                  style="min-width: 480px"
                />
              </div>
              <div
                class="flex justify-start items-center"
                style="padding-top: 0px"
              >
                <div
                  data-test="add-alert-stream-type-select"
                  class="alert-stream-type o2-input q-mr-sm"
                  style="padding-top: 0"
                >
                  <q-select
                    v-model="formData.stream_type"
                    :options="streamTypes"
                    :label="t('alerts.streamType') + ' *'"
                    :popup-content-style="{ textTransform: 'lowercase' }"
                    color="input-border"
                    bg-color="input-bg"
                    class="q-py-sm showLabelOnTop no-case"
                    stack-label
                    outlined
                    filled
                    dense
                    v-bind:readonly="beingUpdated"
                    v-bind:disable="beingUpdated"
                    @update:model-value="updateStreams()"
                    :rules="[(val: any) => !!val || 'Field is required!']"
                    style="min-width: 220px"
                  />
                </div>
                <div
                  data-test="add-alert-stream-select"
                  class="o2-input"
                  style="padding-top: 0"
                >
                  <q-select
                    v-model="formData.stream_name"
                    :options="filteredStreams"
                    :label="t('alerts.stream_name') + ' *'"
                    :loading="isFetchingStreams"
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
                    v-bind:readonly="beingUpdated"
                    v-bind:disable="beingUpdated"
                    @filter="filterStreams"
                    @update:model-value="
                      updateStreamFields(formData.stream_name)
                    "
                    behavior="menu"
                    :rules="[(val: any) => !!val || 'Field is required!']"
                    style="min-width: 250px !important; width: 250px !important"
                  />
                </div>
              </div>
            </div>
            <div class="q-gutter-sm">
              <q-radio
                data-test="add-alert-scheduled-alert-radio"
                v-bind:readonly="beingUpdated"
                v-bind:disable="beingUpdated"
                v-model="formData.is_real_time"
                :checked="formData.is_real_time"
                val="false"
                :label="t('alerts.standard')"
                class="q-ml-none"
              />
              <q-radio
                data-test="add-alert-realtime-alert-radio"
                v-bind:readonly="beingUpdated"
                v-bind:disable="beingUpdated"
                v-model="formData.is_real_time"
                :checked="!formData.is_real_time"
                val="true"
                :label="t('alerts.realTime')"
                class="q-ml-none"
              />
            </div>
            <div
              v-if="formData.is_real_time === 'true'"
              class="q-py-sm showLabelOnTop text-bold text-h7"
              data-test="add-alert-query-input-title"
            >
              <real-time-alert
                :columns="filteredColumns"
                :conditions="formData.query_condition.conditions"
                @field:add="addField"
                @field:remove="removeField"
                @input:update="onInputUpdate"
              />
            </div>
            <div v-else>
              <scheduled-alert
                ref="scheduledAlertRef"
                :columns="filteredColumns"
                :conditions="formData.query_condition.conditions"
                :alertData="formData"
                :sqlQueryErrorMsg="sqlQueryErrorMsg"
                :vrlFunctionError="vrlFunctionError"
                :showTimezoneWarning="showTimezoneWarning"
                v-model:trigger="formData.trigger_condition"
                v-model:sql="formData.query_condition.sql"
                v-model:promql="formData.query_condition.promql"
                v-model:query_type="formData.query_condition.type"
                v-model:aggregation="formData.query_condition.aggregation"
                v-model:promql_condition="
                  formData.query_condition.promql_condition
                "
                v-model:multi_time_range="
                  formData.query_condition.multi_time_range
                "
                v-model:vrl_function="formData.query_condition.vrl_function"
                v-model:isAggregationEnabled="isAggregationEnabled"
                v-model:showVrlFunction="showVrlFunction"
                @field:add="addField"
                @field:remove="removeField"
                @input:update="onInputUpdate"
                @validate-sql="validateSqlQuery"
                @update:showVrlFunction="updateFunctionVisibility"
                @update:multi_time_range="updateMultiTimeRange"
                class="q-mt-sm"
              />
            </div>

            <div class="col-12 flex justify-start items-center q-mt-xs">
              <div
                class="q-py-sm showLabelOnTop text-bold text-h7 q-pb-md flex items-center"
                data-test="add-alert-delay-title"
                style="width: 190px"
              >
                {{ t("alerts.silenceNotification") + " *" }}
                <q-icon
                  :name="outlinedInfo"
                  size="17px"
                  class="q-ml-xs cursor-pointer"
                  :class="
                    store.state.theme === 'dark' ? 'text-grey-5' : 'text-grey-7'
                  "
                >
                  <q-tooltip
                    anchor="center right"
                    self="center left"
                    max-width="300px"
                  >
                    <span style="font-size: 14px">
                      If the alert triggers then how long should it wait before
                      sending another notification.
                      <br />
                      e.g. if the alert triggers at 4:00 PM and the silence
                      notification is set to 10 minutes then it will not send
                      another notification until 4:10 PM even if the alert is
                      still after 1 minute. This is to avoid spamming the user
                      with notifications.</span
                    >
                  </q-tooltip>
                </q-icon>
              </div>
              <div style="min-height: 58px">
                <div class="col-8 row justify-left align-center q-gutter-sm">
                  <div
                    class="flex items-center"
                    style="border: 1px solid rgba(0, 0, 0, 0.05)"
                  >
                    <div
                      data-test="add-alert-delay-input"
                      style="width: 87px; margin-left: 0 !important"
                      class="silence-notification-input"
                    >
                      <q-input
                        v-model="formData.trigger_condition.silence"
                        type="number"
                        dense
                        filled
                        min="0"
                        style="background: none"
                      />
                    </div>
                    <div
                      data-test="add-alert-delay-unit"
                      style="
                        min-width: 90px;
                        margin-left: 0 !important;
                        background: #f2f2f2;
                        height: 40px;
                      "
                      :class="
                        store.state.theme === 'dark'
                          ? 'bg-grey-10'
                          : 'bg-grey-2'
                      "
                      class="flex justify-center items-center"
                    >
                      {{ t("alerts.minutes") }}
                    </div>
                  </div>
                </div>
                <div
                  data-test="add-alert-delay-error"
                  v-if="formData.trigger_condition.silence < 0"
                  class="text-red-8 q-pt-xs"
                  style="font-size: 11px; line-height: 12px"
                >
                  Field is required!
                </div>
              </div>
            </div>

            <div class="o2-input flex justify-start items-start q-mt-sm">
              <div
                data-test="add-alert-destination-title"
                class="text-bold q-pb-sm"
                style="width: 190px"
              >
                {{ t("alerts.destination") + " *" }}
              </div>
              <div data-test="add-alert-destination-select">
                <q-select
                  v-model="formData.destinations"
                  :options="getFormattedDestinations"
                  color="input-border"
                  bg-color="input-bg "
                  class="no-case"
                  stack-label
                  outlined
                  filled
                  dense
                  multiple
                  use-input
                  fill-input
                  :rules="[(val: any) => !!val || 'Field is required!']"
                  style="width: 200px"
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
                            v-model="formData.destinations"
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
              <div class="q-pl-sm">
                <q-btn
                data-test="create-destination-btn"
                icon="refresh"
                title="Refresh latest Destinations"
                class="text-bold no-border"
                no-caps
                flat
                dense
                @click="$emit('refresh:destinations')"
              />
              </div>
              <div class="q-pl-sm">
                <q-btn
                data-test="create-destination-btn"
                label="Create Destination"
                class="text-bold no-border"
                color="secondary"
                no-caps
                @click="routeToCreateDestination"

              />
              </div>
            </div>

            <div class="q-mt-md">
              <div class="text-bold">{{ t("alerts.additionalVariables") }}</div>
              <variables-input
                class="o2-input"
                :variables="formData.context_attributes"
                @add:variable="addVariable"
                @remove:variable="removeVariable"
              />
            </div>

            <div class="o2-input">
              <div data-test="add-alert-description-input">
                <q-input
                  v-model="formData.description"
                  :label="t('alerts.description')"
                  color="input-border"
                  bg-color="input-bg"
                  class="showLabelOnTop q-mb-sm"
                  stack-label
                  outlined
                  filled
                  dense
                  tabindex="0"
                  style="width: 550px"
                />
              </div>
              <div data-test="add-alert-row-input">
                <q-input
                  v-model="formData.row_template"
                  :label="t('alerts.row')"
                  color="input-border"
                  bg-color="input-bg"
                  class="showLabelOnTop"
                  stack-label
                  outlined
                  filled
                  dense
                  tabindex="0"
                  style="width: 550px"
                />
              </div>
            </div>

            <div class="flex justify-start q-mt-lg">
              <q-btn
                data-test="add-alert-cancel-btn"
                v-close-popup="true"
                class="q-mb-md text-bold"
                :label="t('alerts.cancel')"
                text-color="light-text"
                padding="sm md"
                no-caps
                @click="$emit('cancel:hideform')"
              />
              <q-btn
                data-test="add-alert-submit-btn"
                :label="t('alerts.save')"
                class="q-mb-md text-bold no-border q-ml-md"
                color="secondary"
                padding="sm xl"
                type="submit"
                no-caps
              />
            </div>
          </q-form>
        </div>
        <div
          style="width: 400px; height: 200px; position: sticky; top: 0"
          class="q-mb-lg q-px-md"
        >
          <div class="text-bold q-pb-xs">Preview</div>
          <preview-alert
            style="border: 1px solid #ececec"
            ref="previewAlertRef"
            :formData="formData"
            :query="previewQuery"
            :selectedTab="scheduledAlertRef?.tab || 'custom'"
            :aggregationEnabled="isAggregationEnabled"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onMounted,
  watch,
  type Ref,
  computed,
  nextTick,
  defineAsyncComponent,
  onBeforeMount,
} from "vue";

import "monaco-editor/esm/vs/editor/editor.all.js";
import "monaco-editor/esm/vs/basic-languages/sql/sql.contribution.js";
import "monaco-editor/esm/vs/basic-languages/sql/sql.js";

import alertsService from "../../services/alerts";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar, debounce } from "quasar";
import segment from "../../services/segment_analytics";
import {
  getUUID,
  getTimezoneOffset,
  b64EncodeUnicode,
  b64DecodeUnicode,
  isValidResourceName,
  getTimezonesByOffset,
} from "@/utils/zincutils";
import { cloneDeep } from "lodash-es";
import { useRouter } from "vue-router";
import useStreams from "@/composables/useStreams";
import { outlinedInfo } from "@quasar/extras/material-icons-outlined";
import useFunctions from "@/composables/useFunctions";
import useQuery from "@/composables/useQuery";
import searchService from "@/services/search";
import { convertDateToTimestamp } from "@/utils/date";

const defaultValue: any = () => {
  return {
    name: "",
    stream_type: "",
    stream_name: "",
    is_real_time: "false",
    query_condition: {
      conditions: [
        {
          column: "",
          operator: "=",
          value: "",
          type: "",
          id: getUUID(),
        },
      ],
      sql: "",
      promql: "",
      type: "custom",
      aggregation: {
        group_by: [""],
        function: "avg",
        having: {
          column: "",
          operator: ">=",
          value: 1,
        },
      },
      promql_condition: null,
      vrl_function: null,
      multi_time_range: [],
    },
    trigger_condition: {
      period: 10,
      operator: ">=",
      frequency: 1,
      cron: "",
      threshold: 3,
      silence: 10,
      frequency_type: "minutes",
      timezone: "UTC",
    },
    destinations: [],
    context_attributes: {},
    enabled: true,
    description: "",
    lastTriggeredAt: 0,
    createdAt: "",
    updatedAt: "",
    owner: "",
    lastEditedBy: "",
  };
};
let callAlert: Promise<{ data: any }>;
export default defineComponent({
  name: "ComponentAddUpdateAlert",
  props: {
    modelValue: {
      type: Object,
      default: () => defaultValue(),
    },
    isUpdated: {
      type: Boolean,
      default: false,
    },
    destinations: {
      type: Array,
      default: () => [],
    },
  },
  emits: ["update:list", "cancel:hideform", "refresh:destinations"],
  components: {
    ScheduledAlert: defineAsyncComponent(() => import("./ScheduledAlert.vue")),
    RealTimeAlert: defineAsyncComponent(() => import("./RealTimeAlert.vue")),
    VariablesInput: defineAsyncComponent(() => import("./VariablesInput.vue")),
    PreviewAlert: defineAsyncComponent(() => import("./PreviewAlert.vue")),
  },
  setup(props) {
    const store: any = useStore();
    let beingUpdated: boolean = false;
    const addAlertForm: any = ref(null);
    const disableColor: any = ref("");
    const formData: any = ref(defaultValue());
    const indexOptions = ref([]);
    const schemaList = ref([]);
    const streams: any = ref({});
    const { t } = useI18n();
    const q = useQuasar();
    const editorRef: any = ref(null);
    const filteredColumns: any = ref([]);
    const filteredStreams: Ref<string[]> = ref([]);
    let editorobj: any = null;
    var sqlAST: any = ref(null);
    const selectedRelativeValue = ref("1");
    const selectedRelativePeriod = ref("Minutes");
    const relativePeriods: any = ref(["Minutes"]);
    var triggerCols: any = ref([]);
    const selectedDestinations = ref("slack");
    const originalStreamFields: any = ref([]);
    const isAggregationEnabled = ref(false);
    var triggerOperators: any = ref([
      "=",
      "!=",
      ">=",
      "<=",
      ">",
      "<",
      "Contains",
      "NotContains",
    ]);
    const showVrlFunction = ref(false);
    const isFetchingStreams = ref(false);
    const streamTypes = ["logs", "metrics", "traces"];
    const editorUpdate = (e: any) => {
      formData.value.sql = e.target.value;
    };
    const { getAllFunctions } = useFunctions();

    const { getStreams, getStream } = useStreams();

    const { buildQueryPayload } = useQuery();

    const previewQuery = ref("");

    const sqlQueryErrorMsg = ref("");

    const validateSqlQueryPromise = ref<Promise<unknown>>();

    const addAlertFormRef = ref(null);

    const router = useRouter();
    const scheduledAlertRef: any = ref(null);

    const plotChart: any = ref(null);

    const previewAlertRef: any = ref(null);

    let parser: any = null;

    const vrlFunctionError = ref("");

    const showTimezoneWarning = ref(false);

    onBeforeMount(async () => {
      await importSqlParser();
      await getAllFunctions();
    });

    const importSqlParser = async () => {
      const useSqlParser: any = await import("@/composables/useParser");
      const { sqlParser }: any = useSqlParser.default();
      parser = await sqlParser();
    };

    const streamFieldsMap = computed(() => {
      const map: any = {};
      originalStreamFields.value.forEach((field: any) => {
        map[field.value] = field;
      });
      return map;
    });

    const showPreview = computed(() => {
      return formData.value.stream_type && formData.value.stream_name;
    });

    const updateConditions = (e: any) => {
      try {
        const ast = parser.astify(e.target.value);
        if (ast) sqlAST.value = ast;
        else return;

        // If sqlAST.value.columns is not type of array then return
        if (!sqlAST.value) return;
        if (!Array.isArray(sqlAST.value?.columns)) return;

        sqlAST.value.columns.forEach(function (item: any) {
          let val;
          if (item["as"] === undefined || item["as"] === null) {
            val = item["expr"]["column"];
          } else {
            val = item["as"];
          }
          if (!triggerCols.value.includes(val)) {
            triggerCols.value.push(val);
          }
        });
      } catch (e) {
        console.log("Alerts: Error while parsing SQL query");
      }
    };
    const editorData = ref("");
    const prefixCode = ref("");
    const suffixCode = ref("");

    onMounted(async () => {});

    const updateEditorContent = async (stream_name: string) => {
      triggerCols.value = [];
      if (!stream_name) return;

      if (editorData.value) {
        editorData.value = editorData.value
          .replace(prefixCode.value, "")
          .trim();
        editorData.value = editorData.value
          .replace(suffixCode.value, "")
          .trim();
      }

      if (!props.isUpdated) {
        prefixCode.value = `select * from`;
        suffixCode.value = "'" + formData.value.stream_name + "'";
        const someCode = `${prefixCode.value} ${editorData.value} ${suffixCode.value}`;
      }

      const selected_stream: any = await getStream(
        stream_name,
        formData.value.stream_type,
        true,
      );
      selected_stream.schema.forEach(function (item: any) {
        triggerCols.value.push(item.name);
      });
    };

    const updateStreamFields = async (stream_name: any) => {
      let streamCols: any = [];
      const streams: any = await getStream(
        stream_name,
        formData.value.stream_type,
        true,
      );

      if (streams && Array.isArray(streams.schema)) {
        streamCols = streams.schema.map((column: any) => ({
          label: column.name,
          value: column.name,
          type: column.type,
        }));
      }

      originalStreamFields.value = [...streamCols];
      filteredColumns.value = [...streamCols];

      onInputUpdate("stream_name", stream_name);
    };
    watch(
      () => props.destinations.length, // Watch for length changes
      (newLength, oldLength) => {
        formData.value.destinations  = formData.value.destinations.filter((destination : any) => {
          return props.destinations.some((dest:any) => {
            return dest.name === destination});
        });
      }
    );

    watch(
      triggerCols.value,
      () => {
        filteredColumns.value = [...triggerCols.value];
      },
      { immediate: true },
    );
    const filterColumns = (options: any[], val: String, update: Function) => {
      let filteredOptions: any[] = [];
      if (val === "") {
        update(() => {
          filteredOptions = [...options];
        });
        return filteredOptions;
      }
      update(() => {
        const value = val.toLowerCase();
        filteredOptions = options.filter(
          (column: any) => column.toLowerCase().indexOf(value) > -1,
        );
      });
      return filteredOptions;
    };
    const updateStreams = (resetStream = true) => {
      if (resetStream) formData.value.stream_name = "";
      if (streams.value[formData.value.stream_type]) {
        schemaList.value = streams.value[formData.value.stream_type];
        indexOptions.value = streams.value[formData.value.stream_type].map(
          (data: any) => {
            return data.name;
          },
        );
        return;
      }

      if (!formData.value.stream_type) return Promise.resolve();

      isFetchingStreams.value = true;
      return getStreams(formData.value.stream_type, false)
        .then((res: any) => {
          streams.value[formData.value.stream_type] = res.list;
          schemaList.value = res.list;
          indexOptions.value = res.list.map((data: any) => {
            return data.name;
          });

          if (formData.value.stream_name)
            updateStreamFields(formData.value.stream_name);
          return Promise.resolve();
        })
        .catch(() => Promise.reject())
        .finally(() => (isFetchingStreams.value = false));
    };

    const filterStreams = (val: string, update: any) => {
      filteredStreams.value = filterColumns(indexOptions.value, val, update);
    };

    const addField = () => {
      formData.value.query_condition.conditions.push({
        column: "",
        operator: "=",
        value: "",
        id: getUUID(),
      });
    };

    const removeField = (field: any) => {
      formData.value.query_condition.conditions =
        formData.value.query_condition.conditions.filter(
          (_field: any) => _field.id !== field.id,
        );
    };

    const addVariable = () => {
      formData.value.context_attributes.push({
        name: "",
        value: "",
        id: getUUID(),
      });
    };

    const removeVariable = (variable: any) => {
      formData.value.context_attributes =
        formData.value.context_attributes.filter(
          (_variable: any) => _variable.id !== variable.id,
        );
    };

    const getSelectedTab = computed(() => {
      return scheduledAlertRef.value?.tab || null;
    });

    const previewAlert = async () => {
      if (getSelectedTab.value === "custom")
        previewQuery.value = generateSqlQuery();
      else if (getSelectedTab.value === "sql")
        previewQuery.value = formData.value.query_condition.sql.trim();
      else if (getSelectedTab.value === "promql")
        previewQuery.value = formData.value.query_condition.promql.trim();

      if (formData.value.is_real_time === "true") {
        previewQuery.value = generateSqlQuery();
      }

      await nextTick();
      if (getSelectedTab.value !== "sql") {
        previewAlertRef.value.refreshData();
      }
    };

    const getFormattedCondition = (
      column: string,
      operator: string,
      value: number | string,
    ) => {
      let condition = "";
      switch (operator) {
        case "=":
        case "<>":
        case "<":
        case ">":
        case "<=":
        case ">=":
          condition = column + ` ${operator} ${value}`;
          break;
        case "Contains":
          condition = column + ` LIKE '%${value}%'`;
          break;
        case "NotContains":
          condition = column + ` NOT LIKE '%${value}%'`;
          break;
        default:
          condition = column + ` ${operator} ${value}`;
          break;
      }

      return condition;
    };

    const generateSqlQuery = () => {
      // SELECT histgoram(_timestamp, '1 minute') AS zo_sql_key, COUNT(*) as zo_sql_val FROM _rundata WHERE geo_info_country='india' GROUP BY zo_sql_key ORDER BY zo_sql_key ASC

      // SELECT histgoram(_timestamp, '1 minute') AS zo_sql_key, avg(action_error_count) as zo_sql_val, geo_info_city FROM _rundata WHERE geo_info_country='india' GROUP BY zo_sql_key,geo_info_city ORDER BY zo_sql_key ASC;
      let query = `SELECT histogram(${
        store.state.zoConfig.timestamp_column || "_timestamp"
      }) AS zo_sql_key,`;

      let whereClause = formData.value.query_condition.conditions
        .map((condition: any) => {
          if (condition.column && condition.operator && condition.value) {
            // If value is string then add single quotes
            const value =
              streamFieldsMap.value[condition.column].type === "Int64" ||
              condition.operator === "Contains" ||
              condition.operator === "NotContains"
                ? condition.value
                : `'${condition.value}'`;

            return getFormattedCondition(
              condition.column,
              condition.operator,
              value,
            );
          }
        })
        .filter((condition: any) => condition && condition?.trim()?.length)
        .join(" AND ");

      whereClause = whereClause?.trim().length ? " WHERE " + whereClause : "";

      if (!isAggregationEnabled.value) {
        query +=
          ` COUNT(*) as zo_sql_val FROM \"${formData.value.stream_name}\" ` +
          whereClause +
          " GROUP BY zo_sql_key ORDER BY zo_sql_key ASC";
      } else {
        const aggFn = formData.value.query_condition.aggregation.function;
        const column = formData.value.query_condition.aggregation.having.column;

        const isAggValid = aggFn?.trim()?.length && column?.trim()?.length;

        let groupByAlias = "";
        let groupByCols: string[] = [];
        formData.value.query_condition.aggregation.group_by.forEach(
          (column: any) => {
            if (column.trim().length) groupByCols.push(column);
          },
        );

        let concatGroupBy = "";
        if (groupByCols.length) {
          groupByAlias = ", x_axis_2";
          concatGroupBy = `, concat(${groupByCols.join(
            ",' : ',",
          )}) as x_axis_2`;
        }

        const percentileFunctions: any = {
          p50: 0.5,
          p75: 0.75,
          p90: 0.9,
          p95: 0.95,
          p99: 0.99,
        };

        if (isAggValid) {
          if (percentileFunctions[aggFn]) {
            query +=
              ` approx_percentile_cont(${column}, ${percentileFunctions[aggFn]}) as zo_sql_val ${concatGroupBy} FROM \"${formData.value.stream_name}\" ` +
              whereClause +
              ` GROUP BY zo_sql_key ${groupByAlias} ORDER BY zo_sql_key ASC`;
          } else {
            query +=
              ` ${aggFn}(${column}) as zo_sql_val ${concatGroupBy} FROM \"${formData.value.stream_name}\" ` +
              whereClause +
              ` GROUP BY zo_sql_key ${groupByAlias} ORDER BY zo_sql_key ASC`;
          }
        } else {
          query +=
            ` COUNT(*) as zo_sql_val ${concatGroupBy} FROM \"${formData.value.stream_name}\" ` +
            whereClause +
            ` GROUP BY zo_sql_key ${groupByAlias} ORDER BY zo_sql_key ASC`;
        }
      }

      return query;
    };

    const debouncedPreviewAlert = debounce(previewAlert, 500);

    const onInputUpdate = async (name: string, value: any) => {
      if (showPreview.value) {
        debouncedPreviewAlert();
      }
    };
    const getParser = (sqlQuery: string) => {
      try {
        // As default is a reserved keyword in sql-parser, we are replacing it with default1
        const regex = /\bdefault\b/g;
        const columns = parser.astify(
          sqlQuery.replace(regex, "default1"),
        ).columns;
        for (const column of columns) {
          if (column.expr.column === "*") {
            sqlQueryErrorMsg.value = "Selecting all columns is not allowed";
            return false;
          }
        }
        return true;
      } catch (error) {
        // In catch block we are returning true, as we just wanted to validate if user have added * in the query to select all columns
        // select field from default // here default is not wrapped in "" so node sql parser will throw error as default is a reserved keyword. But our Backend supports this query without quotes
        // Query will be validated in the backend
        console.log(error);
        return true;
      }
    };

    const getAlertPayload = () => {
      const payload = cloneDeep(formData.value);

      // Deleting uuid from payload as it was added for reference of frontend
      if (payload.uuid) delete payload.uuid;

      payload.is_real_time = payload.is_real_time === "true";

      payload.context_attributes = {};

      payload.query_condition.type = payload.is_real_time
        ? "custom"
        : formData.value.query_condition.type;

      formData.value.context_attributes.forEach((attr: any) => {
        if (attr.key?.trim() && attr.value?.trim())
          payload.context_attributes[attr.key] = attr.value;
      });

      payload.trigger_condition.threshold = parseInt(
        formData.value.trigger_condition.threshold,
      );

      payload.trigger_condition.period = parseInt(
        formData.value.trigger_condition.period,
      );

      payload.trigger_condition.frequency = parseInt(
        formData.value.trigger_condition.frequency,
      );

      payload.trigger_condition.silence = parseInt(
        formData.value.trigger_condition.silence,
      );

      payload.description = formData.value.description.trim();

      if (!isAggregationEnabled.value || getSelectedTab.value !== "custom") {
        payload.query_condition.aggregation = null;
      }

      if (getSelectedTab.value === "sql" || getSelectedTab.value === "promql")
        payload.query_condition.conditions = [];

      if (getSelectedTab.value === "sql" || getSelectedTab.value === "custom") {
        payload.query_condition.promql_condition = null;
      }

      if (getSelectedTab.value === "promql") {
        payload.query_condition.sql = "";
      }

      if (formData.value.query_condition.vrl_function) {
        payload.query_condition.vrl_function = b64EncodeUnicode(
          formData.value.query_condition.vrl_function.trim(),
        );
      }

      if (beingUpdated) {
        payload.updatedAt = new Date().toISOString();
        payload.lastEditedBy = store.state.userInfo.email;
      } else {
        payload.createdAt = new Date().toISOString();
        payload.owner = store.state.userInfo.email;
        payload.lastTriggeredAt = new Date().getTime();
        payload.lastEditedBy = store.state.userInfo.email;
        formData.value.updatedAt = new Date().toISOString();
      }

      return payload;
    };

    const validateInputs = (input: any, notify: boolean = true) => {
      if (isNaN(Number(input.trigger_condition.silence))) {
        notify &&
          q.notify({
            type: "negative",
            message: "Silence Notification should not be empty",
            timeout: 1500,
          });
        return false;
      }

      if (input.is_real_time) return true;

      if (
        Number(input.trigger_condition.period) < 1 ||
        isNaN(Number(input.trigger_condition.period))
      ) {
        notify &&
          q.notify({
            type: "negative",
            message: "Period should be greater than 0",
            timeout: 1500,
          });
        return false;
      }

      if (input.query_condition.aggregation) {
        if (
          isNaN(input.trigger_condition.threshold) ||
          !input.query_condition.aggregation.having.value.toString().trim()
            .length ||
          !input.query_condition.aggregation.having.column ||
          !input.query_condition.aggregation.having.operator
        ) {
          notify &&
            q.notify({
              type: "negative",
              message: "Threshold should not be empty",
              timeout: 1500,
            });
          return false;
        }

        return true;
      }

      if (
        isNaN(input.trigger_condition.threshold) ||
        input.trigger_condition.threshold < 1 ||
        !input.trigger_condition.operator
      ) {
        notify &&
          q.notify({
            type: "negative",
            message: "Threshold should not be empty",
            timeout: 1500,
          });
        return false;
      }

      return true;
    };

    const validateSqlQuery = async () => {
      // Delaying the validation by 300ms, as editor has debounce of 300ms. Else old value will be used for validation
      await new Promise((resolve) => setTimeout(resolve, 300));

      if (!getParser(formData.value.query_condition.sql)) {
        return;
      }

      const query = buildQueryPayload({
        sqlMode: true,
        streamName: formData.value.stream_name,
      });

      delete query.aggs;

      // We get 15 minutes time range for the query, so reducing it by 13 minutes to get 2 minute data
      query.query.start_time = query.query.start_time + 780000000;

      query.query.sql = formData.value.query_condition.sql;

      if (formData.value.query_condition.vrl_function)
        query.query.query_fn = b64EncodeUnicode(
          formData.value.query_condition.vrl_function,
        );

      validateSqlQueryPromise.value = new Promise((resolve, reject) => {
        searchService
          .search({
            org_identifier: store.state.selectedOrganization.identifier,
            query,
            page_type: "logs",
          })
          .then((res: any) => {
            sqlQueryErrorMsg.value = "";

            if (res.data?.function_error) {
              vrlFunctionError.value = res.data.function_error;
              q.notify({
                type: "negative",
                message: "Invalid VRL Function",
                timeout: 3000,
              });
              reject("function_error");
            } else vrlFunctionError.value = "";

            resolve("");
          })
          .catch((err: any) => {
            sqlQueryErrorMsg.value = err.response?.data?.message
              ? err.response?.data?.message
              : "Invalid SQL Query";

            // Show error only if it is not real time alert
            // This case happens when user enters invalid query and then switches to real time alert
            if (formData.value.query_condition.type === "sql")
              q.notify({
                type: "negative",
                message: "Invalid SQL Query : " + err.response?.data?.message,
                timeout: 3000,
              });

            reject("sql_error");
          });
      });
    };

    const updateFunctionVisibility = () => {
      // if validateSqlQueryPromise has error "function_error" then reset the promise when function is disabled
      if (!showVrlFunction.value && vrlFunctionError.value) {
        validateSqlQueryPromise.value = Promise.resolve("");
        vrlFunctionError.value = "";
      }
    };
    const updateMultiTimeRange = (value: any) => {
      if (value) {
        formData.value.query_condition.multi_time_range = value;
      }
    };

    const routeToCreateDestination = () => {
      const url = router.resolve({
          name: "alertDestinations",
          query: {
            action: "add",
            org_identifier: store.state.selectedOrganization.identifier,
          },
        }).href;
      window.open(url, "_blank");
    };

    return {
      t,
      q,
      disableColor,
      beingUpdated,
      formData,
      addAlertForm,
      store,
      indexOptions,
      editorRef,
      editorobj,
      prefixCode,
      suffixCode,
      editorData,
      selectedRelativeValue,
      selectedRelativePeriod,
      relativePeriods,
      editorUpdate,
      updateConditions,
      updateStreamFields,
      updateEditorContent,
      triggerCols,
      triggerOperators,
      sqlAST,
      schemaList,
      filteredColumns,
      streamTypes,
      streams,
      updateStreams,
      isFetchingStreams,
      filteredStreams,
      filterStreams,
      addField,
      removeField,
      removeVariable,
      addVariable,
      selectedDestinations,
      scheduledAlertRef,
      router,
      isAggregationEnabled,
      plotChart,
      previewAlert,
      addAlertFormRef,
      validateInputs,
      getAlertPayload,
      getParser,
      onInputUpdate,
      showPreview,
      streamFieldsMap,
      previewQuery,
      previewAlertRef,
      outlinedInfo,
      getTimezoneOffset,
      showVrlFunction,
      validateSqlQuery,
      validateSqlQueryPromise,
      isValidResourceName,
      sqlQueryErrorMsg,
      vrlFunctionError,
      updateFunctionVisibility,
      convertDateToTimestamp,
      getTimezonesByOffset,
      showTimezoneWarning,
      updateMultiTimeRange,
      routeToCreateDestination,
    };
  },

  created() {
    // TODO OK: Refactor this code
    this.formData.ingest = ref(false);
    this.formData = { ...defaultValue, ...cloneDeep(this.modelValue) };
    this.formData.is_real_time = this.formData.is_real_time.toString();
    this.beingUpdated = this.isUpdated;
    this.updateStreams(false)?.then(() => {
      this.updateEditorContent(this.formData.stream_name);
    });
    if (
      this.modelValue &&
      this.modelValue.name != undefined &&
      this.modelValue.name != ""
    ) {
      this.beingUpdated = true;
      this.disableColor = "grey-5";
      this.formData = cloneDeep(this.modelValue);
      this.isAggregationEnabled = !!this.formData.query_condition.aggregation;

      if (!this.formData.trigger_condition?.timezone) {
        if (this.formData.tz_offset === 0) {
          this.formData.trigger_condition.timezone = "UTC";
        } else {
          this.getTimezonesByOffset(this.formData.tz_offset).then(
            (res: any) => {
              if (res.length > 1) this.showTimezoneWarning = true;
              this.formData.trigger_condition.timezone = res[0];
            },
          );
        }
      }

      if (this.formData.query_condition.vrl_function) {
        this.showVrlFunction = true;
        this.formData.query_condition.vrl_function = b64DecodeUnicode(
          this.formData.query_condition.vrl_function,
        );
      }
    }

    this.formData.is_real_time = this.formData.is_real_time.toString();
    this.formData.context_attributes = Object.keys(
      this.formData.context_attributes,
    ).map((attr) => {
      return {
        key: attr,
        value: this.formData.context_attributes[attr],
        id: getUUID(),
      };
    });
    this.formData.query_condition.conditions =
      this.formData.query_condition.conditions.map((condition: any) => {
        return {
          ...condition,
          id: getUUID(),
        };
      });
  },

  computed: {
    getFormattedDestinations: function () {
      return this.destinations.map((destination: any) => {
        return destination.name;
      });
    },
  },
  methods: {
    onRejected(rejectedEntries: string | any[]) {
      this.q.notify({
        type: "negative",
        message: `${rejectedEntries.length} file(s) did not pass validation constraints`,
      });
    },

    async onSubmit() {
      // Delaying submission by 500ms to allow the form to validate, as query is validated in validateSqlQuery method
      // When user updated query and click on save
      await new Promise((resolve) => setTimeout(resolve, 500));

      if (
        this.formData.is_real_time == "false" &&
        this.formData.query_condition.type == "sql" &&
        !this.getParser(this.formData.query_condition.sql)
      ) {
        this.q.notify({
          type: "negative",
          message: "Selecting all Columns in SQL query is not allowed.",
          timeout: 1500,
        });
        return false;
      }

      if (this.formData.stream_name == "") {
        this.q.notify({
          type: "negative",
          message: "Please select stream name.",
          timeout: 1500,
        });
        return false;
      }

      if (
        this.formData.is_real_time == "false" &&
        this.formData.trigger_condition.frequency_type == "cron"
      ) {
        const now = new Date();

        // Get the day, month, and year from the date object
        const day = String(now.getDate()).padStart(2, "0");
        const month = String(now.getMonth() + 1).padStart(2, "0"); // January is 0!
        const year = now.getFullYear();

        // Combine them in the DD-MM-YYYY format
        const date = `${day}-${month}-${year}`;

        // Get the hours and minutes, ensuring they are formatted with two digits
        const hours = String(now.getHours()).padStart(2, "0");
        const minutes = String(now.getMinutes()).padStart(2, "0");

        // Combine them in the HH:MM format
        const time = `${hours}:${minutes}`;

        const convertedDateTime = this.convertDateToTimestamp(
          date,
          time,
          this.formData.trigger_condition.timezone,
        );

        this.formData.tz_offset = convertedDateTime.offset;
      }

      this.addAlertForm.validate().then(async (valid: any) => {
        if (!valid) {
          return false;
        }

        const payload = this.getAlertPayload();
        if (!this.validateInputs(payload)) return;

        const dismiss = this.q.notify({
          spinner: true,
          message: "Please wait...",
          timeout: 2000,
        });

        if (
          this.formData.is_real_time == "false" &&
          this.formData.query_condition.type == "sql"
        ) {
          try {
            // Wait for the promise to resolve
            // Storing the SQL query validation promise in a variable
            // Case: When user edits the query and directly saves it without waiting for the validation to complete
            // So waiting here for sql validation to complete
            await this.validateSqlQueryPromise;
          } catch (error) {
            dismiss();
            console.log("Error while validating sql query");
            return false;
          }
        }

        if (this.beingUpdated) {
          callAlert = alertsService.update(
            this.store.state.selectedOrganization.identifier,
            payload.stream_name,
            payload.stream_type,
            payload,
          );
          callAlert
            .then((res: { data: any }) => {
              this.formData = { ...defaultValue };
              this.$emit("update:list");
              this.addAlertForm.resetValidation();
              dismiss();
              this.q.notify({
                type: "positive",
                message: `Alert updated successfully.`,
              });
            })
            .catch((err: any) => {
              dismiss();
              if(err.response?.status != 403){
                this.q.notify({
                type: "negative",
                message:
                  err.response?.data?.error || err.response?.data?.message,
                });
              }
              
            });
          segment.track("Button Click", {
            button: "Update Alert",
            user_org: this.store.state.selectedOrganization.identifier,
            user_id: this.store.state.userInfo.email,
            stream_name: this.formData.stream_name,
            alert_name: this.formData.name,
            page: "Add/Update Alert",
          });
          return;
        } else {
          callAlert = alertsService.create(
            this.store.state.selectedOrganization.identifier,
            payload.stream_name,
            payload.stream_type,
            payload,
          );

          callAlert
            .then((res: { data: any }) => {
              this.formData = { ...defaultValue };
              this.$emit("update:list");
              this.addAlertForm.resetValidation();
              dismiss();
              this.q.notify({
                type: "positive",
                message: `Alert saved successfully.`,
              });
            })
            .catch((err: any) => {
              dismiss();
              if(err.response?.status != 403){
                this.q.notify({
                type: "negative",
                message:
                  err.response?.data?.error || err.response?.data?.message,
                });
              }
              
            });
          segment.track("Button Click", {
            button: "Save Alert",
            user_org: this.store.state.selectedOrganization.identifier,
            user_id: this.store.state.userInfo.email,
            stream_name: this.formData.stream_name,
            alert_name: this.formData.name,
            page: "Add/Update Alert",
          });
        }
      });
    },
  },
});
</script>

<style scoped lang="scss">
#editor {
  width: 100%;
  min-height: 5rem;
  // padding-bottom: 14px;
  resize: both;
}

.alert-condition {
  .__column,
  .__value {
    width: 250px;
  }

  .__operator {
    width: 100px;
  }
}
</style>
<style lang="scss">
.no-case .q-field__native span {
  text-transform: none !important;
}

.no-case .q-field__input {
  text-transform: none !important;
}

.add-alert-form {
  .q-field--dense .q-field__control {
    .q-field__native span {
      overflow: hidden;
    }
  }

  .alert-condition .__column .q-field__control .q-field__native span {
    max-width: 152px;
    text-overflow: ellipsis;
    text-align: left;
    white-space: nowrap;
  }

  .q-field__bottom {
    padding: 2px 0;
  }
}

.silence-notification-input,
.threshould-input {
  .q-field--filled .q-field__control {
    background-color: transparent !important;
  }

  .q-field--dark .q-field__control {
    background-color: rgba(255, 255, 255, 0.07) !important;
  }
}
</style>
