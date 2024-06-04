<!-- Copyright 2023 Zinc Labs Inc.

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
  <div>
    <div class="column full-height">
      <DashboardHeader :title="title" backButton @back="close">
      </DashboardHeader>

      <div>
        <q-form greedy ref="addVariableForm" @submit="onSubmit">
          <div class="col">
            <div>
              <q-select
                class="textbox showLabelOnTop"
                filled
                stack-label
                input-debounce="0"
                outlined
                dense
                v-model="variableData.type"
                :options="variableTypes"
                :label="t('dashboard.typeOfVariable')"
                option-value="value"
                map-options
                emit-value
              ></q-select>
            </div>
            <div class="text-body1 text-bold q-mt-lg">
              {{ t("dashboard.addGeneralSettings") }}
            </div>
            <div class="row">
              <div class="textbox col">
                <q-input
                  v-model="variableData.name"
                  class="showLabelOnTop q-mr-sm"
                  :label="t('dashboard.nameOfVariable') + ' *'"
                  dense
                  filled
                  outlined
                  stack-label
                  :rules="[
                    (val: any) => !!(val.trim()) || 'Field is required!',
                    (val: any) => /^[a-zA-Z0-9_-]*$/.test(val) || 'Only letters, numbers, hyphens (-), and underscores (_) are allowed.'
                ]"
                ></q-input>
              </div>
              <div class="textbox col">
                <q-input
                  v-model="variableData.label"
                  class="showLabelOnTop"
                  :label="t('dashboard.labelOfVariable')"
                  dense
                  filled
                  outlined
                  stack-label
                ></q-input>
              </div>
            </div>
            <div
              class="text-body1 text-bold q-mt-lg"
              v-if="variableData.type !== 'dynamic_filters'"
            >
              {{ t("dashboard.extraOptions") }}
            </div>
            <div v-if="variableData.type == 'query_values'">
              <div class="row">
                <q-select
                  v-model="variableData.query_data.stream_type"
                  :label="t('dashboard.selectStreamType') + ' *'"
                  :options="data.streamType"
                  input-debounce="0"
                  behavior="menu"
                  filled
                  borderless
                  dense
                  stack-label
                  class="textbox showLabelOnTop col no-case q-mr-sm"
                  @update:model-value="streamTypeUpdated"
                  :rules="[(val: any) => !!val || 'Field is required!']"
                ></q-select>
                <q-select
                  v-model="variableData.query_data.stream"
                  :label="t('dashboard.selectIndex') + ' *'"
                  :options="streamsFilteredOptions"
                  input-debounce="0"
                  behavior="menu"
                  use-input
                  filled
                  borderless
                  dense
                  stack-label
                  @filter="streamsFilterFn"
                  @update:model-value="streamUpdated"
                  option-value="name"
                  option-label="name"
                  emit-value
                  class="textbox showLabelOnTop col no-case"
                  :rules="[(val: any) => !!val || 'Field is required!']"
                >
                </q-select>
              </div>
              <q-select
                v-model="variableData.query_data.field"
                :label="t('dashboard.selectField') + ' *'"
                filled
                stack-label
                use-input
                borderless
                dense
                hide-selected
                fill-input
                behavior="menu"
                input-debounce="0"
                :options="fieldsFilteredOptions"
                @filter="fieldsFilterFn"
                class="textbox showLabelOnTop no-case"
                option-value="name"
                option-label="name"
                emit-value
                :rules="[(val: any) => !!val || 'Field is required!']"
              >
              </q-select>
              <div>
                <q-input
                  class="showLabelOnTop"
                  type="number"
                  v-model.number="variableData.query_data.max_record_size"
                  :label="t('dashboard.DefaultSize')"
                  dense
                  filled
                  outlined
                  stack-label
                >
                  <q-btn
                    padding="xs"
                    round
                    flat
                    class="q-ml-sm"
                    no-caps
                    icon="info"
                  >
                    <q-tooltip>{{ t("dashboard.maxRecordSize") }}</q-tooltip>
                  </q-btn>
                </q-input>
              </div>
              <div>
                <q-toggle
                  v-model="variableData.multiSelect"
                  :label="t('dashboard.multiSelect')"
                  data-test="dashboard-query_values-show_multiple_values"
                />
              </div>
              <div>
                <div class="flex flex-row">
                  <div
                    data-test="dashboard-query-values-filter"
                    class="text-body1 text-bold q-mt-lg"
                  >
                    Filters
                  </div>
                  <q-icon
                    class=""
                    style="margin-top: 25px; margin-left: 5px"
                    size="20px"
                    name="info_outline"
                    data-test="dashboard-variables-setting-filter-info"
                  >
                    <q-tooltip style="width: 250px">
                      In filters, you can use the value of another variable to
                      filter the current variable's value. This can be done by
                      using the other variable's name. For example:
                      <span class="bg-highlight">$variableName</span>.
                    </q-tooltip>
                  </q-icon>
                </div>
                <div class="row items-center">
                  <div
                    class="row no-wrap items-center q-mb-xs"
                    v-for="(filter, index) in variableData.query_data.filter"
                    :key="index"
                  >
                    <q-select
                      filled
                      outlined
                      dense
                      v-model="filter.name"
                      :display-value="filter.name ? filter.name : ''"
                      :options="fieldsFilteredOptions"
                      input-debounce="0"
                      behavior="menu"
                      @update:model-value="filterUpdated(index, $event)"
                      use-input
                      stack-label
                      option-label="name"
                      data-test="dashboard-query-values-filter-name-selector"
                      @filter="fieldsFilterFn"
                      :placeholder="filter.name ? '' : 'Select Field'"
                      class="textbox col no-case q-ml-sm"
                      :rules="[(val: any) => !!(val.trim()) || 'Field is required!']"
                    >
                      <template v-slot:no-option>
                        <q-item>
                          <q-item-section class="text-italic text-grey"
                            >No Data Found</q-item-section
                          >
                        </q-item>
                      </template>
                    </q-select>
                    <q-select
                      dense
                      filled
                      v-model="filter.operator"
                      :display-value="filter.operator ? filter.operator : ''"
                      style="width: auto"
                      class="operator"
                      data-test="dashboard-query-values-filter-operator-selector"
                      :rules="[(val: any) => !!(val.trim()) || 'Field is required!']"
                      :options="['=', '!=', 'IN']"
                    />
                    <CommonAutoComplete
                      v-model="filter.value"
                      :items="dashboardVariablesFilterItems"
                      searchRegex="(?:^|[^$])\$?(\w+)"
                      :rules="[(val: any) => val?.length > 0 || 'Required']"
                      debounce="1000"
                      style="
                        margin-top: none !important;
                        width: auto !important;
                      "
                      placeholder="Enter Value"
                    ></CommonAutoComplete>
                    <q-btn
                      size="sm"
                      padding="12px 5px"
                      style="margin-bottom: 20px"
                      flat
                      dense
                      @click="removeFilter(index)"
                      icon="close"
                      :data-test="`dashboard-variable-adhoc-close-${index}`"
                    />
                  </div>
                </div>
                <div>
                  <q-btn
                    no-caps
                    icon="add"
                    no-outline
                    class="q-mt-md"
                    @click="addFilter"
                    data-test="dashboard-add-filter-btn"
                  >
                    Add Filter
                  </q-btn>
                </div>

                <!-- show error if filter has cycle -->
                <div v-show="filterCycleError" style="color: red">
                  {{ filterCycleError }}
                </div>
              </div>
            </div>
          </div>
          <div class="textbox" v-if="['constant'].includes(variableData.type)">
            <q-input
              class="showLabelOnTop"
              v-model="variableData.value"
              :label="t('dashboard.ValueOfVariable') + ' *'"
              dense
              filled
              outlined
              stack-label
              :rules="[(val: any) => !!(val.trim()) || 'Field is required!']"
            ></q-input>
          </div>
          <div class="textbox" v-if="['textbox'].includes(variableData.type)">
            <q-input
              class="showLabelOnTop"
              v-model="variableData.value"
              :label="t('dashboard.DefaultValue')"
              dense
              filled
              outlined
              stack-label
            ></q-input>
          </div>
          <!-- show the auto add variables for the custom fields -->
          <div v-if="variableData.type == 'custom'">
            <div>
              <q-toggle
                v-model="variableData.multiSelect"
                :label="t('dashboard.multiSelect')"
                data-test="dashboard-custom-show_multiple_values"
              />
            </div>
            <div
              v-for="(option, index) in variableData.options"
              :key="index"
              class="row"
            >
              <q-input
                dense
                filled
                outlined
                stack-label
                :rules="[(val: any) => !!(val.trim()) || 'Field is required!']"
                class="col textbox showLabelOnTop q-mr-sm"
                v-model="variableData.options[index].label"
                :label="'Label ' + (index + 1) + ' *'"
                name="label"
              />
              <q-input
                dense
                filled
                outlined
                stack-label
                :rules="[(val: any) => !!(val.trim()) || 'Field is required!']"
                class="col textbox showLabelOnTop q-mr-sm"
                v-model="variableData.options[index].value"
                :label="'Value ' + (index + 1) + ' *'"
                name="value"
              />
              <div>
                <q-btn
                  flat
                  style="margin-top: 33px"
                  round
                  @click="removeField(index)"
                  icon="cancel"
                />
              </div>
            </div>
            <div class="flex flex-col">
              <q-btn
                no-caps
                icon="add"
                no-outline
                class="q-mt-md"
                @click="addField()"
                >Add Option</q-btn
              >
            </div>
          </div>
          <div class="flex justify-center q-mt-lg">
            <q-btn
              class="q-mb-md text-bold"
              :label="t('dashboard.cancel')"
              text-color="light-text"
              padding="sm md"
              no-caps
              @click="close"
            />
            <div>
              <q-btn
                type="submit"
                :loading="saveVariableApiCall.isLoading.value"
                class="q-mb-md text-bold no-border q-ml-md"
                color="secondary"
                padding="sm xl"
                no-caps
                >Save</q-btn
              >
            </div>
          </div>
        </q-form>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  reactive,
  onMounted,
  onActivated,
  watch,
  toRef,
  toRaw,
  type Ref,
  computed,
} from "vue";
import { useI18n } from "vue-i18n";
import { useSelectAutoComplete } from "../../../composables/useSelectAutocomplete";
import { useStore } from "vuex";
import {
  addVariable,
  getDashboard,
  updateVariable,
} from "../../../utils/commons";
import { useRoute } from "vue-router";
import { useLoading } from "../../../composables/useLoading";
import DashboardHeader from "./common/DashboardHeader.vue";
import { useQuasar } from "quasar";
import useStreams from "@/composables/useStreams";
import {
  buildVariablesDependencyGraph,
  isGraphHasCycle,
} from "@/utils/dashboard/variables/variablesDependencyUtils";
import CommonAutoComplete from "@/components/dashboards/addPanel/CommonAutoComplete.vue";

export default defineComponent({
  name: "AddSettingVariable",
  props: ["variableName", "dashboardVariablesList"],
  components: { DashboardHeader, CommonAutoComplete },
  emits: ["close", "save"],
  setup(props, { emit }) {
    const $q = useQuasar();
    const { t } = useI18n();
    const store = useStore();
    const addVariableForm: Ref<any> = ref(null);
    const data: any = reactive({
      schemaResponse: [],
      streamType: ["logs", "metrics", "traces"],
      streams: [],
      currentFieldsList: [],

      // selected values
      selectedStreamFields: [],
    });
    const route = useRoute();
    const title = ref("Add Variable");
    const { getStreams, getStream } = useStreams();

    // const model = ref(null)
    // const filteredStreams = ref([]);
    const variableTypes = ref([
      {
        label: t("dashboard.queryValues"),
        value: "query_values",
      },
      {
        label: t("dashboard.constant"),
        value: "constant",
      },
      {
        label: t("dashboard.textbox"),
        value: "textbox",
      },
      {
        label: t("dashboard.custom"),
        value: "custom",
      },
    ]);

    const variableData: any = reactive({
      name: "",
      label: "",
      type: "",
      query_data: {
        stream_type: "",
        stream: "",
        field: "",
        max_record_size: null,
        filter: [],
      },
      value: "",
      options: [],
      multiSelect: false,
    });

    const filterCycleError: any = ref("");

    const addFilter = () => {
      if (!variableData.query_data.filter) {
        variableData.query_data.filter = [];
      }
      variableData.query_data.filter.push({
        name: "",
        operator: "=",
        value: "",
      });
    };
    // by default, use multiSelect as false
    if (!variableData.multiSelect) {
      variableData.multiSelect = false;
    }

    const filterUpdated = (index: number, filter: any) => {
      variableData.query_data.filter[index].name = filter.name;
    };

    const removeFilter = (index: any) => {
      variableData.query_data.filter.splice(index, 1);
    };

    const editMode = ref(false);

    watch(
      () => variableData.query_data.max_record_size,
      (newVal, oldVal) => {
        if (newVal === "") {
          variableData.query_data.max_record_size = null;
        }
      }
    );

    onMounted(async () => {
      if (props.variableName) {
        editMode.value = true;
        title.value = "Edit Variable";
        // Fetch dashboard data
        const data = JSON.parse(
          JSON.stringify(
            await getDashboard(store, route.query.dashboard, route.query.folder)
          )
        )?.variables?.list;

        // Find the variable to edit
        const edit = (data || []).find(
          (it: any) => it.name === props.variableName
        );
        // Assign edit data to variableData
        Object.assign(variableData, edit);
      } else {
        // default variable type will be query_values
        variableData.type = "query_values";
        editMode.value = false;
      }
    });

    // check if type is query_values then get stream list and field list
    watch(
      () => [variableData.type],
      async () => {
        if (variableData.type == "query_values") {
          // add query_data object if not have
          if (!variableData?.query_data) {
            variableData.query_data = {
              stream_type: "",
              stream: "",
              field: "",
              max_record_size: null,
            };
          }

          // if variable type is query_values
          // need to get the stream list
          // and followed by the field list
          try {
            // if stream type is exists
            if (variableData?.query_data?.stream_type) {
              // get all streams from current stream type
              const streamList: any = await getStreams(
                variableData?.query_data?.stream_type,
                false
              );
              data.streams = streamList.list ?? [];

              // if stream type and stream is exists
              if (variableData?.query_data?.stream) {
                // get schema of that field using getstream
                const fieldWithSchema: any = await getStream(
                  variableData?.query_data?.stream,
                  variableData.query_data.stream_type,
                  true
                );

                // assign the schema
                data.currentFieldsList = fieldWithSchema?.schema ?? [];
              } else {
                // reset field list array
                data.currentFieldsList = [];
              }
            } else {
              // reset stream and field list
              data.streams = [];
              data.currentFieldsList = [];
            }
          } catch (error: any) {
            $q.notify({
              type: "negative",
              message: error ?? "Failed to get stream fields",
              timeout: 2000,
            });
          }
        }
      }
    );

    const addField = () => {
      variableData.options.push({ label: "", value: "" });
    };

    const removeField = (index: any) => {
      variableData.options.splice(index, 1);
    };

    const saveVariableApiCall = useLoading(async () => await saveData());

    const saveData = async () => {
      const dashId = route.query.dashboard + "";

      // remove query_data if type is not query_values
      if (variableData.type !== "query_values") {
        delete variableData["query_data"];
      }

      if (editMode.value) {
        try {
          await updateVariable(
            store,
            dashId,
            props.variableName,
            toRaw(variableData),
            route.query.folder ?? "default"
          );
          emit("save");
        } catch (error: any) {
          $q.notify({
            type: "negative",
            message: error.message ?? "Variable update failed",
            timeout: 2000,
          });
        }
      } else {
        try {
          await addVariable(
            store,
            dashId,
            variableData,
            route.query.folder ?? "default"
          );
          emit("save");
        } catch (error: any) {
          $q.notify({
            type: "negative",
            message: error.message ?? "Variable creation failed",
            timeout: 2000,
          });
        }
      }
    };

    // check if filter has cycle
    const isFilterHasCycle = async () => {
      try {
        // need all variables to check for cycle
        // get all variables data.
        let variablesData: any = JSON.parse(
          JSON.stringify(
            await getDashboard(store, route.query.dashboard, route.query.folder)
          )
        )?.variables?.list;

        // current updated variable data need to merge/update in above variablesData.
        // temporary update variable list
        // if edit mode, then update the variable data
        if (editMode.value) {
          //if name already exists
          const variableIndex = variablesData.findIndex(
            (variable: any) => variable.name == props.variableName
          );

          // Update the variable data in the list
          variablesData[variableIndex] = variableData;
        }
        // else, it's a new variable.
        else {
          variablesData.push(variableData);
        }

        // now, need to check whether filter has cycle or not
        // key: variable name
        // value: { parentVariables: list of parent variable names, childVariables: list of child variable names }
        let variablesDependencyGraph: any =
          buildVariablesDependencyGraph(variablesData);

        // if graph has cycle, it will return the cycle path
        // else it will return null
        const hasCycle = isGraphHasCycle(variablesDependencyGraph);
        if (hasCycle) {
          // filter has cycle, so show error and return
          filterCycleError.value = `Variables has cycle: ${hasCycle.join(
            "->"
          )} -> ${hasCycle[0]}`;
          return true;
        }

        // above conditions passed, so remove filter cycle error and return false
        filterCycleError.value = "";
        return false;
      } catch (err: any) {
        $q.notify({
          type: "negative",
          message:
            err?.message ??
            (editMode.value
              ? "Variable update failed"
              : "Variable creation failed"),
        });
        return true;
      }
    };

    const onSubmit = () => {
      // first, validate form values
      addVariableForm.value.validate().then(async (valid: any) => {
        if (!valid) {
          return false;
        }

        // check if filter has cycle
        if (await isFilterHasCycle()) {
          // filter has cycle, so show error and return
          return false;
        }

        // above conditions passed, so remove filter cycle error
        filterCycleError.value = "";

        // save the variable
        saveVariableApiCall.execute().catch((err: any) => {
          $q.notify({
            type: "negative",
            message:
              err?.message ??
              (editMode.value
                ? "Variable update failed"
                : "Variable creation failed"),
          });
        });
      });
    };

    // select filters
    const {
      filterFn: streamsFilterFn,
      filteredOptions: streamsFilteredOptions,
    } = useSelectAutoComplete(toRef(data, "streams"), "name");
    const { filterFn: fieldsFilterFn, filteredOptions: fieldsFilteredOptions } =
      useSelectAutoComplete(toRef(data, "currentFieldsList"), "name");

    const streamTypeUpdated = async () => {
      // reset the stream and field
      variableData.query_data.stream = "";
      variableData.query_data.field = "";

      // if stream type is exists
      if (variableData.query_data.stream_type) {
        // get all streams from current stream type
        const streamList: any = await getStreams(
          variableData?.query_data?.stream_type,
          false
        );

        // assign the stream list
        data.streams = streamList.list ?? [];
      } else {
        // reset stream list
        data.streams = [];
      }
    };

    const streamUpdated = async () => {
      // reset field list value
      variableData.query_data.field = "";

      try {
        // if stream type and stream is exists
        if (
          variableData.query_data.stream &&
          variableData.query_data.stream_type
        ) {
          // get schema of that field using getstream
          const fieldWithSchema: any = await getStream(
            variableData?.query_data?.stream,
            variableData.query_data.stream_type,
            true
          );

          // assign the schema
          data.currentFieldsList = fieldWithSchema?.schema ?? [];
        } else {
          // reset field list
          data.currentFieldsList = [];
        }
      } catch (error: any) {
        $q.notify({
          type: "negative",
          message: error ?? "Failed to get stream fields",
          timeout: 2000,
        });
      }
    };

    const close = () => {
      emit("close");
    };

    const dashboardVariablesFilterItems = computed(() =>
      props.dashboardVariablesList
        .map((it: any) => ({
          label: it.name,
          value: "$" + it.name,
        }))
        .filter((it: any) => it.label !== variableData.name)
    );
    return {
      variableData,
      store,
      t,
      data,
      streamsFilterFn,
      fieldsFilterFn,
      streamsFilteredOptions,
      fieldsFilteredOptions,
      variableTypes,
      streamTypeUpdated,
      streamUpdated,
      onActivated,
      removeField,
      addField,
      saveData,
      saveVariableApiCall,
      close,
      title,
      onSubmit,
      addVariableForm,
      addFilter,
      removeFilter,
      filterUpdated,
      filterCycleError,
      dashboardVariablesFilterItems,
    };
  },
});
</script>

<style lang="scss" scoped>
:deep(.no-case .q-field__native > :first-child) {
  text-transform: none !important;
}

.textbox {
  margin-top: 5px;
  margin-bottom: 5px;
}

.theme-dark .bg-highlight {
  background-color: #747474;
}

.theme-light .bg-highlight {
  background-color: #e7e6e6;
}
</style>
