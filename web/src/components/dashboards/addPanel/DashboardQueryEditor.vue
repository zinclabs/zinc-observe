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
  <div class="col-auto" data-test="dashboard-panel-searchbar">
    <q-bar
      class="row sql-bar"
      style="display: flex; justify-content: space-between"
      @click.stop="onDropDownClick"
    >
      <div
        style="display: flex; flex-direction: row; align-items: center"
        data-test="dashboard-query-data"
      >
        <div>
          <q-icon
            flat
            :name="
              !dashboardPanelData.layout.showQueryBar
                ? 'arrow_right'
                : 'arrow_drop_down'
            "
            text-color="black"
            class="q-mr-sm"
            data-test="dashboard-panel-error-bar-icon"
          />
        </div>
        <q-space />
        <div style="max-width: 600px">
          <q-tabs
            v-if="promqlMode || dashboardPanelData.data.type == 'geomap'"
            v-model="dashboardPanelData.layout.currentQueryIndex"
            narrow-indicator
            dense
            inline-label
            outside-arrows
            mobile-arrows
            @click.stop
            data-test="dashboard-panel-query-tab"
          >
            <q-tab
              no-caps
              :ripple="false"
              v-for="(tab, index) in dashboardPanelData.data.queries"
              :key="index"
              :name="index"
              :label="'Query ' + (index + 1)"
              @click.stop
              :data-test="`dashboard-panel-query-tab-${index}`"
            >
              <q-icon
                v-if="
                  index > 0 ||
                  (index === 0 && dashboardPanelData.data.queries.length > 1)
                "
                name="close"
                class="q-ml-sm"
                @click.stop="removeTab(index)"
                style="cursor: pointer"
                :data-test="`dashboard-panel-query-tab-remove-${index}`"
              />
            </q-tab>
          </q-tabs>
          <!-- <div v-if="promqlMode" class="query-tabs-container">
                    <div v-for="(tab, index) in dashboardPanelData.data.queries" :key="index" class="query-tab" :class="{ 'active': index === activeTab }" @click="handleActiveTab(index)">
                        <div class="tab-label">{{ 'Query ' + (index + 1) }}</div>
                        <div v-if="index > 0 || (index === 0 && dashboardPanelData.data.queries.length > 1)" @click.stop="removeTab(index)">
                            <i class="material-icons">cancel</i>
                        </div>
                    </div>
                </div> -->
        </div>
        <span
          v-if="!(promqlMode || dashboardPanelData.data.type == 'geomap')"
          class="text-subtitle2 text-weight-bold"
          >{{ t("panel.sql") }}</span
        >
        <q-btn
          v-if="promqlMode || dashboardPanelData.data.type == 'geomap'"
          round
          flat
          @click.stop="addTab"
          icon="add"
          style="margin-right: 10px"
          data-test="`dashboard-panel-query-tab-add`"
        ></q-btn>
      </div>
      <div style="display: flex; gap: 4px">
        <q-toggle
          data-test="logs-search-bar-show-query-toggle-btn"
          v-model="dashboardPanelData.layout.vrlFunctionToggle"
          :icon="'img:' + getImageURL('images/common/function.svg')"
          title="Toggle Function Editor"
          class="float-left"
          size="28px"
          @update:model-value="onFunctionToggle"
          :disable="promqlMode"
        />
        <QueryTypeSelector></QueryTypeSelector>
      </div>
    </q-bar>
  </div>
  <div
    class="col"
    :style="
      !dashboardPanelData.layout.showQueryBar ? 'height: 0px;' : 'height: auto;'
    "
    style="overflow: hidden"
    data-test="dashboard-query"
  >
    <div class="column" style="width: 100%; height: 100%">
      <div class="col" style="width: 100%; height: 100%">
        <div class="row" style="height: 100%">
          <q-splitter
            no-scroll
            style="width: 100%; height: 100%"
            v-model="splitterModel"
            :limits="[
              30,
              promqlMode || !dashboardPanelData.layout.vrlFunctionToggle
                ? 100
                : 70,
            ]"
            :disable="
              promqlMode || !dashboardPanelData.layout.vrlFunctionToggle
            "
          >
            <template #before>
              <SqlQueryEditor
                ref="queryEditorRef"
                class="monaco-editor"
                style="width: 100%"
                v-model:query="currentQuery"
                data-test="dashboard-panel-query-editor"
                v-model:functions="dashboardPanelData.meta.stream.functions"
                v-model:fields="selectedStreamFieldsBasedOnUserDefinedSchema"
                :keywords="
                  dashboardPanelData.data.queryType === 'promql'
                    ? autoCompletePromqlKeywords
                    : []
                "
                @update-query="updateQuery"
                @run-query="searchData"
                :readOnly="
                  !dashboardPanelData.data.queries[
                    dashboardPanelData.layout.currentQueryIndex
                  ].customQuery
                "
                :language="dashboardPanelData.data.queryType"
              ></SqlQueryEditor>
            </template>
            <template #after>
              <div style="height: 100%; width: 100%">
                <div style="height: calc(100% - 40px); width: 100%">
                  <query-editor
                    v-if="!promqlMode"
                    data-test="dashboard-vrl-function-editor"
                    style="width: 100%; height: 100%"
                    ref="vrlFnEditorRef"
                    editor-id="fnEditor"
                    class="monaco-editor"
                    v-model:query="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].vrlFunctionQuery
                    "
                    :class="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ]?.vrlFunctionQuery == '' && functionEditorPlaceholderFlag
                        ? 'empty-function'
                        : ''
                    "
                    language="ruby"
                    @focus="functionEditorPlaceholderFlag = false"
                    @blur="functionEditorPlaceholderFlag = true"
                  />
                </div>
                <div style="height: 40px; width: 100%">
                  <div style="display: flex; height: 40px">
                    <q-select
                      v-model="selectedFunction"
                      label="Use Saved function"
                      :options="functionOptions"
                      data-test="dashboard-use-saved-vrl-function"
                      input-debounce="0"
                      behavior="menu"
                      use-input
                      filled
                      borderless
                      dense
                      hide-selected
                      menu-anchor="top left"
                      fill-input
                      @filter="filterFunctionOptions"
                      option-label="name"
                      option-value="function"
                      @update:modelValue="onFunctionSelect"
                      style="width: 100%"
                    >
                      <template #no-option>
                        <q-item>
                          <q-item-section>
                            {{ t("search.noResult") }}</q-item-section
                          >
                        </q-item>
                      </template>
                    </q-select>
                    <q-btn
                      no-caps
                      padding="xs"
                      class=""
                      size="sm"
                      flat
                      icon="info_outline"
                      data-test="dashboard-addpanel-config-drilldown-info"
                    >
                      <q-tooltip
                        class="bg-grey-8"
                        anchor="bottom middle"
                        self="top right"
                        max-width="250px"
                      >
                        To use extracted VRL fields in the chart, write a VRL
                        function and click on the Apply button. The fields will
                        be extracted, allowing you to use them to build the
                        chart.
                      </q-tooltip>
                    </q-btn>
                  </div>
                </div>
              </div>
            </template>
          </q-splitter>
        </div>
      </div>
      <div style="color: red; z-index: 100000" class="q-mx-sm col-auto">
        {{ dashboardPanelData.meta.errors.queryErrors.join(", ") }}
      </div>
    </div>
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import {
  defineComponent,
  ref,
  watch,
  computed,
  onMounted,
  defineAsyncComponent,
  nextTick,
  onUnmounted,
} from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import ConfirmDialog from "../../../components/ConfirmDialog.vue";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import QueryTypeSelector from "../addPanel/QueryTypeSelector.vue";
import usePromqlSuggestions from "@/composables/usePromqlSuggestions";
import { inject } from "vue";
import { onBeforeMount } from "vue";
import { getImageURL } from "@/utils/zincutils";
import useNotifications from "@/composables/useNotifications";
import { useStore } from "vuex";
import useFunctions from "@/composables/useFunctions";

export default defineComponent({
  name: "DashboardQueryEditor",
  components: {
    SqlQueryEditor: defineAsyncComponent(() => import("../QueryEditor.vue")),
    ConfirmDialog,
    QueryTypeSelector,
    queryEditor: defineAsyncComponent(
      () => import("@/components/QueryEditor.vue")
    ),
  },
  emits: ["searchdata"],
  methods: {
    searchData() {
      this.$emit("searchdata");
    },
  },
  setup() {
    const router = useRouter();
    const { t } = useI18n();
    const { showErrorNotification, showPositiveNotification } =
      useNotifications();
    const store = useStore();
    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "dashboard"
    );

    const { getAllFunctions } = useFunctions();
    const functionList = ref([]);
    const functionOptions = ref([]);
    const selectedFunction = ref("");

    const getFunctions = async () => {
      try {
        if (store.state.organizationData.functions.length == 0) {
          await getAllFunctions();
        }

        store.state.organizationData.functions.map((data: any) => {
          const args: any = [];
          for (let i = 0; i < parseInt(data.num_args); i++) {
            args.push("'${1:value}'");
          }

          const itemObj: {
            name: any;
            args: string;
          } = {
            name: data.name,
            args: "(" + args.join(",") + ")",
          };

          functionList.value.push({
            name: data.name,
            function: data.function,
          });
          // if (!data.stream_name) {
          //   searchObj.data.stream.functions.push(itemObj);
          // }
        });
        return;
      } catch (e) {
        showErrorNotification("Error while fetching functions");
      }
    };

    const filterFunctionOptions = (val: string, update: any) => {
      update(() => {
        functionOptions.value = functionList.value.filter((fn: any) => {
          return fn.name.toLowerCase().indexOf(val.toLowerCase()) > -1;
        });
      });
    };

    const onFunctionSelect = (val: any) => {
      // assign selected vrl function
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].vrlFunctionQuery = val.function;
      // clear v-model
      selectedFunction.value = "";

      // show success message
      showPositiveNotification(`${val.name} function applied successfully.`);
    };

    const {
      dashboardPanelData,
      promqlMode,
      addQuery,
      removeQuery,
      selectedStreamFieldsBasedOnUserDefinedSchema,
    } = useDashboardPanelData(dashboardPanelDataPageKey);

    const splitterModel = ref(
      promqlMode || !dashboardPanelData.layout.vrlFunctionToggle ? 100 : 70
    );

    watch(
      () => splitterModel.value,
      () => {
        window.dispatchEvent(new Event("resize"));
      }
    );
    const confirmQueryModeChangeDialog = ref(false);

    const { autoCompleteData, autoCompletePromqlKeywords, getSuggestions } =
      usePromqlSuggestions();

    const queryEditorRef = ref(null);

    const functionEditorPlaceholderFlag = ref(true);
    const vrlFnEditorRef = ref(null);

    const addTab = () => {
      addQuery();
      dashboardPanelData.layout.currentQueryIndex =
        dashboardPanelData.data.queries.length - 1;
    };
    const updateQuery = (query, fields) => {
      if (dashboardPanelData.data.queryType === "promql") {
        updatePromQLQuery(query, fields);
      }
    };

    watch(
      () => [promqlMode.value, dashboardPanelData.layout.vrlFunctionToggle],
      () => {
        if (promqlMode.value || !dashboardPanelData.layout.vrlFunctionToggle) {
          splitterModel.value = 100;
        } else {
          splitterModel.value = 70;
        }
      }
    );

    const removeTab = async (index) => {
      if (
        dashboardPanelData.layout.currentQueryIndex >=
        dashboardPanelData.data.queries.length - 1
      )
        dashboardPanelData.layout.currentQueryIndex -= 1;
      removeQuery(index);
    };

    const currentQuery = computed({
      get: () => {
        return promqlMode.value
          ? dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].query
          : dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].query;
      },
      set: (value) => {
        if (promqlMode.value) {
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].query = value;
        } else {
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].query = value;
        }
      },
    });

    // toggle show query view
    const onDropDownClick = () => {
      dashboardPanelData.layout.showQueryBar =
        !dashboardPanelData.layout.showQueryBar;
    };

    watch(
      () => dashboardPanelData.layout.showQueryBar,
      () => {
        window.dispatchEvent(new Event("resize"));
      }
    );

    // this is only for VRLs
    const resizeEventListener = async () => {
      await nextTick();
      vrlFnEditorRef?.value?.resetEditorLayout();
    };

    onMounted(async () => {
      window.removeEventListener("resize", resizeEventListener);
      window.addEventListener("resize", resizeEventListener);
      getFunctions();
    });

    onUnmounted(() => {
      window.removeEventListener("resize", resizeEventListener);
    });
    // End for VRL resize

    onMounted(() => {
      dashboardPanelData.meta.errors.queryErrors = [];
      vrlFnEditorRef?.value?.resetEditorLayout();
    });

    onBeforeMount(() => {
      if (
        !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].vrlFunctionQuery
      ) {
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].vrlFunctionQuery = "";
      }
    });

    // on queryerror change dispatch resize event to resize monaco editor
    watch(
      () => dashboardPanelData.meta.errors.queryErrors,
      () => {
        window.dispatchEvent(new Event("resize"));
      }
    );

    const updatePromQLQuery = async (event, value) => {
      autoCompleteData.value.query = value;
      autoCompleteData.value.text = event.changes[0].text;

      // set the start and end time
      if (
        dashboardPanelData.meta.dateTime.start_time &&
        dashboardPanelData.meta.dateTime.end_time
      ) {
        autoCompleteData.value.dateTime = {
          startTime: dashboardPanelData.meta.dateTime.start_time?.getTime(),
          endTime: dashboardPanelData.meta.dateTime.end_time?.getTime(),
        };
      }

      autoCompleteData.value.position.cursorIndex =
        queryEditorRef.value.getCursorIndex();
      autoCompleteData.value.popup.open =
        queryEditorRef.value.triggerAutoComplete;
      autoCompleteData.value.popup.close =
        queryEditorRef.value.disableSuggestionPopup;
      getSuggestions();
    };

    const onUpdateToggle = (value) => {
      dashboardPanelData.meta.errors.queryErrors = [];
    };

    const onFunctionToggle = (value, event) => {
      event.stopPropagation();

      // if value is false
      if (!value) {
        // hide function editor
        splitterModel.value = 100;

        // reset function query
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].vrlFunctionQuery = "";
      }

      // open query editor
      dashboardPanelData.layout.showQueryBar = true;
    };

    return {
      t,
      router,
      updatePromQLQuery,
      onDropDownClick,
      promqlMode,
      dashboardPanelData,
      confirmQueryModeChangeDialog,
      onUpdateToggle,
      addTab,
      removeTab,
      currentQuery,
      autoCompleteData,
      autoCompletePromqlKeywords,
      getSuggestions,
      queryEditorRef,
      updateQuery,
      functionEditorPlaceholderFlag,
      vrlFnEditorRef,
      splitterModel,
      getImageURL,
      onFunctionToggle,
      functionOptions,
      selectedFunction,
      filterFunctionOptions,
      onFunctionSelect,
      selectedStreamFieldsBasedOnUserDefinedSchema,
    };
  },
});
</script>

<style lang="scss" scoped>
.sql-bar {
  height: 40px !important;
  // overflow: hidden;
  cursor: pointer;
}

.q-ml-sm:hover {
  background-color: #eaeaeaa5;
  border-radius: 50%;
}

:deep(.empty-function .monaco-editor-background) {
  background-image: url("../../../assets/images/common/vrl-function.png");
  background-repeat: no-repeat;
  background-size: 170px;
}

// .query-tabs-container {
//   width: 100%;
//   display: flex;
//   flex-direction: row;
//   justify-content: flex-start;
//   align-items: center;
// }

// .query-tab {
//   display: flex;
//   flex-direction: row;
//   align-items: center;
//   margin-right: 10px;
//   padding: 5px;

//   &:hover {
//         background-color: #eaeaeaa5;
//     }
// }

// .tab-label {
//   margin-right: 5px;
// }

// .remove-button {
//   cursor: pointer;
//   display: flex;
//   align-items: center;
//   justify-content: center;
//   width: 20px;
//   height: 20px;
// }

// .query-tab.active {
//     border-bottom: 3px solid #000;
// }
</style>
