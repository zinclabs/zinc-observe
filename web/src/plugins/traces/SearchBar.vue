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
  <div class="search-bar-component" id="searchBarComponent">
    <div class="row q-my-xs">
      <div class="float-right col flex items-center">
        <syntax-guide
          class="q-mr-lg"
          data-test="logs-search-bar-sql-mode-toggle-btn"
          :sqlmode="searchObj.meta.sqlMode"
        />
        <div class="flex items-center">
          <div class="q-mr-xs text-bold">Filters:</div>
          <app-tabs
            style="
              border: 1px solid #8a8a8a;
              border-radius: 4px;
              overflow: hidden;
            "
            :tabs="[
              {
                label: 'Basic',
                value: 'basic',
                style: {
                  width: 'fit-content',
                  padding: '0px 8px',
                  background:
                    searchObj.meta.filterType === 'basic' ? '#5960B2' : '',
                  border: 'none !important',
                  color:
                    searchObj.meta.filterType === 'basic'
                      ? '#ffffff !important'
                      : '',
                },
              },
              {
                label: 'Advanced',
                value: 'advance',
                style: {
                  width: 'fit-content',
                  padding: '0px 8px',
                  background:
                    searchObj.meta.filterType === 'advance' ? '#5960B2' : '',
                  border: 'none !important',
                  color:
                    searchObj.meta.filterType === 'advance'
                      ? '#ffffff !important'
                      : '',
                },
              },
            ]"
            :active-tab="searchObj.meta.filterType"
            @update:active-tab="updateFilterType"
          />
        </div>
        <q-btn
          v-if="searchObj.meta.filterType === 'basic'"
          label="Reset Filters"
          no-caps
          size="sm"
          icon="restart_alt"
          class="q-pr-sm q-pl-xs reset-filters q-ml-md"
          @click="resetFilters"
        />
      </div>
      <div class="float-right col-auto">
        <div class="float-left">
          <date-time
            ref="dateTimeRef"
            auto-apply
            :default-type="searchObj.data.datetime.type"
            :default-absolute-time="{
              startTime: searchObj.data.datetime.startTime,
              endTime: searchObj.data.datetime.endTime,
            }"
            :default-relative-time="searchObj.data.datetime.relativeTimePeriod"
            data-test="logs-search-bar-date-time-dropdown"
            @on:date-change="updateDateTime"
            @on:timezone-change="updateTimezone"
          />
        </div>
        <div class="search-time q-pl-sm float-left">
          <q-btn
            data-test="logs-search-bar-refresh-btn"
            data-cy="search-bar-refresh-button"
            dense
            flat
            :title="t('search.runQuery')"
            class="search-button bg-secondary"
            @click="searchData"
            :disable="isLoading"
            >{{ t("search.runQuery") }}</q-btn
          >
        </div>
        <q-btn
          class="q-mr-sm float-left download-logs-btn q-pa-sm"
          size="sm"
          :disable="!searchObj.data.queryResults?.hits?.length"
          icon="download"
          title="Export Traces"
          @click="downloadLogs"
        />
        <q-btn
          data-test="logs-search-bar-share-link-btn"
          class="q-mr-sm download-logs-btn q-px-sm"
          size="sm"
          icon="share"
          :title="t('search.shareLink')"
          @click="shareLink"
        />
      </div>
    </div>
    <div
      class="row"
      v-if="searchObj.meta.showQuery"
      style="border-top: 1px solid #dbdbdb"
    >
      <div class="col">
        <query-editor
          ref="queryEditorRef"
          editor-id="traces-query-editor"
          class="monaco-editor"
          v-model:query="searchObj.data.editorValue"
          :keywords="autoCompleteKeywords"
          v-model:functions="searchObj.data.stream.functions"
          :read-only="searchObj.meta.filterType === 'basic'"
          @update:query="updateQueryValue"
          @run-query="searchData"
        />
      </div>
    </div>
    <template>
      <confirm-dialog
        title="Change Filter Type"
        message="Query will be wiped off and reset to default."
        @update:ok="changeToggle()"
        @update:cancel="showWarningDialog = false"
        v-model="showWarningDialog"
      />
    </template>
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import {
  defineComponent,
  ref,
  watch,
  nextTick,
  defineAsyncComponent,
  onBeforeUnmount,
} from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";

import DateTime from "@/components/DateTime.vue";
import useTraces from "@/composables/useTraces";
import SyntaxGuide from "./SyntaxGuide.vue";

import segment from "@/services/segment_analytics";
import config from "@/aws-exports";
import useSqlSuggestions from "@/composables/useSuggestions";
import AppTabs from "@/components/common/AppTabs.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import useStreams from "@/composables/useStreams";

export default defineComponent({
  name: "ComponentSearchSearchBar",
  components: {
    DateTime,
    QueryEditor: defineAsyncComponent(
      () => import("@/components/QueryEditor.vue")
    ),
    SyntaxGuide,
    AppTabs,
    ConfirmDialog,
  },
  emits: ["searchdata", "shareLink"],
  props: {
    fieldValues: {
      type: Object,
      default: () => {},
    },
    // Using loading key directly from traces composable was not working
    // so have added as prop
    // Run query btn was not getting disabled while loading
    isLoading: {
      type: Boolean,
      default: false,
    },
  },
  methods: {
    searchData() {
      if (this.searchObj?.loading == false) {
        // this.searchObj.runQuery = true;
        this.$emit("searchdata");
      }
    },
  },
  setup(props, { emit }) {
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const btnRefreshInterval = ref(null);

    const { searchObj } = useTraces();
    const queryEditorRef = ref(null);

    const showWarningDialog = ref(false);

    let parser: any;
    let streamName = "";
    const dateTimeRef = ref(null);

    const { getStream } = useStreams();

    const {
      autoCompleteData,
      autoCompleteKeywords,
      getSuggestions,
      updateFieldKeywords,
    } = useSqlSuggestions();

    const importSqlParser = async () => {
      const useSqlParser: any = await import("@/composables/useParser");
      const { sqlParser }: any = useSqlParser.default();
      parser = await sqlParser();
    };

    onBeforeUnmount(async () => {
      await importSqlParser();
    });

    const refreshTimeChange = (item) => {
      searchObj.meta.refreshInterval = item.value;
      searchObj.meta.refreshIntervalLabel = item.label;
      btnRefreshInterval.value = false;
    };

    watch(
      () => searchObj.data.stream.selectedStreamFields,
      (fields) => {
        if (fields.length) updateFieldKeywords(fields);
      },
      { immediate: true, deep: true }
    );

    const updateAutoComplete = (value) => {
      autoCompleteData.value.query = value;
      autoCompleteData.value.cursorIndex =
        queryEditorRef.value.getCursorIndex();
      autoCompleteData.value.fieldValues = props.fieldValues;
      autoCompleteData.value.popup.open =
        queryEditorRef.value.triggerAutoComplete;
      getSuggestions();
    };

    const updateQueryValue = async (value: string) => {
      updateAutoComplete(value);
      if (searchObj.meta.sqlMode == true) {
        searchObj.data.parsedQuery = parser.astify(value);
        if (searchObj.data.parsedQuery?.from?.length > 0) {
          if (
            searchObj.data.parsedQuery.from[0].table !==
              searchObj.data.stream.selectedStream.value &&
            searchObj.data.parsedQuery.from[0].table !== streamName
          ) {
            let streamFound = false;
            streamName = searchObj.data.parsedQuery.from[0].table;
            await getStream(streamName, "traces", true).then(
              (streamResponse) => {
                streamFound = true;
                let itemObj = {
                  label: streamResponse.name,
                  value: streamResponse.name,
                };
                searchObj.data.stream.selectedStream = itemObj;
                streamResponse.schema.forEach((field) => {
                  searchObj.data.stream.selectedStreamFields.push({
                    name: field.name,
                  });
                });
              }
            );

            if (streamFound == false) {
              searchObj.data.stream.selectedStream = { label: "", value: "" };
              searchObj.data.stream.selectedStreamFields = [];
              $q.notify({
                message: "Stream not found",
                color: "negative",
                position: "top",
                timeout: 2000,
              });
            }
          }
        }
      }
    };

    const updateDateTime = async (value: object) => {
      searchObj.data.datetime = {
        startTime: value.startTime,
        endTime: value.endTime,
        relativeTimePeriod: value.relativeTimePeriod
          ? value.relativeTimePeriod
          : searchObj.data.datetime.relativeTimePeriod,
        type: value.relativeTimePeriod ? "relative" : "absolute",
      };

      await nextTick();
      await nextTick();
      await nextTick();
      await nextTick();

      if (config.isCloud == "true" && value.userChangedValue) {
        segment.track("Button Click", {
          button: "Date Change",
          tab: value.tab,
          value: value,
          //user_org: this.store.state.selectedOrganization.identifier,
          //user_id: this.store.state.userInfo.email,
          stream_name: searchObj.data.stream.selectedStream.value,
          page: "Search Logs",
        });
      }

      if (value.valueType === "relative") emit("searchdata");
    };

    const udpateQuery = () => {
      // alert(searchObj.data.query);
      if (queryEditorRef.value?.setValue)
        queryEditorRef.value.setValue(searchObj.data.query);
    };

    // This method is used in parent component using ref
    const setEditorValue = (value: String) => {
      if (queryEditorRef.value?.setValue) queryEditorRef.value.setValue(value);
    };

    const jsonToCsv = (jsonData) => {
      const replacer = (key, value) => (value === null ? "" : value);
      const header = Object.keys(jsonData[0]);
      let csv = header.join(",") + "\r\n";

      for (let i = 0; i < jsonData.length; i++) {
        const row = header
          .map((fieldName) => JSON.stringify(jsonData[i][fieldName], replacer))
          .join(",");
        csv += row + "\r\n";
      }

      return csv;
    };

    const downloadLogs = () => {
      const filename = "traces-data.csv";
      const data = jsonToCsv(searchObj.data.queryResults.hits);
      const file = new File([data], filename, {
        type: "text/csv",
      });
      const url = URL.createObjectURL(file);
      const link = document.createElement("a");
      link.href = url;
      link.download = filename;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
    };

    const updateTimezone = () => {
      emit("onChangeTimezone");
    };

    const updateFilterType = (value) => {
      if (value === "basic") {
        searchObj.meta.filterType = "basic";
        searchObj.data.editorValue = searchObj.data.advanceFiltersQuery;
      } else {
        searchObj.meta.filterType = value;
      }
    };

    const changeToggle = () => {
      showWarningDialog.value = false;
      searchObj.meta.filterType = "basic";
    };

    const resetFilters = () => {
      searchObj.data.editorValue = "";
      searchObj.data.advanceFiltersQuery = "";
      Object.values(searchObj.data.stream.fieldValues).forEach((field) => {
        field.selectedValues = [];
        field.searchKeyword = "";
      });
    };

    const shareLink = () => {
      emit("shareLink");
    };

    return {
      t,
      router,
      searchObj,
      queryEditorRef,
      btnRefreshInterval,
      refreshTimes: searchObj.config.refreshTimes,
      refreshTimeChange,
      updateQueryValue,
      updateDateTime,
      udpateQuery,
      downloadLogs,
      setEditorValue,
      autoCompleteKeywords,
      updateTimezone,
      dateTimeRef,
      updateFilterType,
      showWarningDialog,
      changeToggle,
      resetFilters,
      shareLink,
    };
  },
  computed: {
    addSearchTerm() {
      return this.searchObj.data.stream.addToFilter;
    },
  },
  watch: {
    addSearchTerm() {
      if (this.searchObj.data.stream.addToFilter != "") {
        let currentQuery = this.searchObj.data.editorValue.split("|");
        let filter = this.searchObj.data.stream.addToFilter;

        const isFilterValueNull = filter.split(/=|!=/)[1] === "'null'";

        if (isFilterValueNull) {
          filter = filter
            .replace(/=|!=/, (match) => {
              return match === "=" ? " is " : " is not ";
            })
            .replace(/'null'/, "null");
        }

        if (currentQuery.length > 1) {
          if (currentQuery[1].trim() != "") {
            currentQuery[1] += " and " + filter;
          } else {
            currentQuery[1] = filter;
          }
          this.searchObj.data.query = currentQuery.join("| ");
        } else {
          if (currentQuery != "") {
            currentQuery += " and " + filter;
          } else {
            currentQuery = filter;
          }
          this.searchObj.data.query = currentQuery;
        }
        this.searchObj.data.stream.addToFilter = "";
        if (this.queryEditorRef?.setValue)
          this.queryEditorRef.setValue(this.searchObj.data.query);
      }
    },
    filters() {},
  },
});
</script>

<style lang="scss" scoped>
.search-bar-component {
  border-bottom: 1px solid #e0e0e0;
  padding-bottom: 1px;

  .q-toggle__inner {
    font-size: 30px;
  }

  .q-toggle__label {
    font-size: 12px;
  }

  .casesensitive-btn {
    padding: 8px;
    margin-left: -6px;
    background-color: #d5d5d5;
    border-radius: 0px 3px 3px 0px;
  }
  .search-field .q-field {
    &__control {
      border-radius: 3px 0px 0px 3px !important;
    }
    &__native {
      font-weight: 600;
    }
  }
  .search-time {
    // width: 120px;
    margin-right: 10px;
    .q-btn-group {
      border-radius: 3px;

      .q-btn {
        min-height: auto;
      }
    }
  }
  .search-dropdown {
    padding: 0px;
    .block {
      color: $dark-page;
      font-weight: 600;
      font-size: 12px;
    }
    .q-btn-dropdown__arrow-container {
      color: $light-text2;
    }
  }
  .refresh-rate-dropdown-container {
    width: 220px;
    * .q-btn {
      font-size: 12px !important;
      padding-left: 8px;
      padding-right: 8px;
    }
  }

  .flex-start {
    justify-content: flex-start;
    align-items: flex-start;
    display: flex;
  }

  .resultsOverChart {
    margin-bottom: 0.75rem;
    font-size: 0.875rem;
    color: $dark-page;
    font-weight: 700;
  }

  .ddlWrapper {
    position: relative;
    z-index: 10;

    .listWrapper {
      box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
      transition: height 0.25s ease;
      height: calc(100vh - 146px);
      background-color: white;
      position: absolute;
      top: 2.75rem;
      width: 100%;
      left: 0;

      &:empty {
        height: 0;
      }

      &,
      .q-list {
        border-radius: 3px;
      }
    }
  }
  .fields_autocomplete {
    max-height: 250px;
  }
  .monaco-editor {
    width: 100% !important;
    height: 40px !important;
  }

  .search-button {
    min-width: 96px;
    line-height: 29px;
    font-weight: bold;
    text-transform: initial;
    font-size: 11px;
    color: white;
    padding: 0px 5px;

    .q-btn__content {
      background: $secondary;
      border-radius: 3px 3px 3px 3px;
      padding: 0px 5px;

      .q-icon {
        font-size: 15px;
        color: #ffffff;
      }
    }
  }

  .download-logs-btn {
    height: 30px;
  }
}
</style>

<style lang="scss">
.reset-filters {
  font-size: 22px;

  .block {
    font-size: 12px;
  }
  .q-icon {
    margin-right: 4px;
  }
}
</style>
