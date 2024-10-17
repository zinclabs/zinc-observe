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
  <div class="logs-search-bar-component" id="searchBarComponent">
    <div class="row flex justify-end align-center">
      <div class="col-auto q-my-xs">
        <q-btn
          class="q-mr-sm download-logs-btn q-px-sm"
          size="sm"
          :disabled="
            queryData.queryResults.hasOwnProperty('hits') &&
            !queryData.queryResults.hits.length
          "
          icon="download"
          :title="t('search.exportLogs')"
          @click="downloadLogs"
        ></q-btn>
        <div
          class="float-left"
          v-show="queryData.streamType !== 'enrichment_tables'"
        >
          <date-time
            data-test="logs-search-bar-date-time-dropdown"
            auto-apply
            :default-type="searchObj.data.datetime.type"
            :default-absolute-time="{
              startTime: searchObj.data.datetime.startTime,
              endTime: searchObj.data.datetime.endTime,
            }"
            :default-relative-time="searchObj.data.datetime.relativeTimePeriod"
            @on:date-change="updateDateTime"
          />
        </div>
        <div class="search-time q-pl-sm float-left q-mr-sm">
          <q-btn
            data-test="logs-search-bar-refresh-btn"
            data-cy="search-bar-refresh-button"
            dense
            flat
            :title="t('search.runQuery')"
            class="q-pa-none search-button"
            @click="searchData"
            >{{ t("search.runQuery") }}</q-btn
          >
        </div>
      </div>
    </div>
    <div class="row query-editor-container">
      <div class="col" style="border-top: 1px solid #dbdbdb; height: 100px">
        <b>Query Editor:</b>
        <query-editor
          editor-id="logsStreamQueryEditor"
          ref="queryEditorRef"
          class="monaco-editor"
          v-model:query="query"
          @update:query="updateQueryValue"
          @run-query="searchData"
        ></query-editor>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, ref, onMounted, onBeforeMount } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useStore } from "vuex";
import DateTime from "@/components/DateTime.vue";
import useLogs from "@/composables/useLogs";
import type { IDateTime } from "@/ts/interfaces";

const defaultValue: any = () => {
  return {
    name: "",
    function: "",
    params: "row",
    transType: "0",
  };
};

export default defineComponent({
  name: "ComponentSearchSearchBar",
  props: {
    queryData: {
      type: Object,
      default: () => {
        return {};
      },
    },
    isLoading: {
      type: Boolean,
      default: false,
    },
  },
  components: {
    DateTime,
    QueryEditor: defineAsyncComponent(
      () => import("@/components/QueryEditor.vue")
    ),
  },
  emits: ["searchdata", "update-query", "change:date-time"],
  methods: {
    searchData() {
      if (!this.isLoading) {
        // this.searchObj.runQuery = true;
        this.$emit("searchdata");
      }
    },
  },
  setup(props, { emit }) {
    const router = useRouter();
    const { t } = useI18n();
    const store = useStore();
    const btnRefreshInterval = ref(null);

    const { searchObj } = useLogs();
    const queryEditorRef = ref(null);

    const functionOptions = ref(searchObj.data.transforms);

    const functionModel: string = ref(null);
    const fnEditorRef: any = ref(null);
    const confirmDialogVisible: boolean = ref(false);
    const query = ref("");

    let confirmCallback;
    let fnEditorobj: any = null;

    onBeforeMount(() => {
      query.value = props.queryData.query;
    });

    const updateQueryValue = (value: string) => {
      emit("update-query", value);
    };

    const updateDateTime = (value: IDateTime) => {
      emit("change:date-time", value);
    };

    const updateQuery = () => {
      // alert(searchObj.data.query);
      if (queryEditorRef.value?.setValue)
        queryEditorRef.value.setValue(props.queryData.query);
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
      const filename = "logs-data.csv";
      const data = jsonToCsv(props.queryData.queryResults.hits);
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

    const resetEditorLayout = () => {
      setTimeout(() => {
        queryEditorRef.value.resetEditorLayout();
        fnEditorobj.layout();
      }, 100);
    };

    return {
      t,
      store,
      router,
      fnEditorRef,
      fnEditorobj,
      searchObj,
      queryEditorRef,
      btnRefreshInterval,
      confirmDialogVisible,
      confirmCallback,
      refreshTimes: searchObj.config.refreshTimes,
      updateQueryValue,
      updateDateTime,
      updateQuery,
      downloadLogs,
      resetEditorLayout,
      functionModel,
      functionOptions,
      query,
    };
  },
});
</script>

<style lang="scss">
#logsStreamQueryEditor,
#fnEditor {
  height: calc(100% - 20px) !important;
}
#fnEditor {
  width: 100%;
  border-radius: 5px;
  border: 0px solid #dbdbdb;
  overflow: hidden;
}

.q-field--standard .q-field__control:before,
.q-field--standard .q-field__control:focus:before,
.q-field--standard .q-field__control:hover:before {
  border: 0px !important;
  border-color: none;
  transition: none;
}

.logs-search-bar-component > .row:nth-child(2) {
  height: calc(100% - 38px); /* or any other height you want to set */
}

.logs-search-bar-component {
  padding-bottom: 1px;
  height: 100%;
  overflow: visible;

  .function-dropdown {
    width: 205px;
    padding-bottom: 0px;
    border: 1px solid #dbdbdb;
    border-radius: 5px;
    cursor: pointer;

    .q-field__input {
      cursor: pointer;
      font-weight: 600;
      font-size: 12px;
    }
    .q-field__native,
    .q-field__control {
      min-height: 29px;
      height: 29px;
      padding: 0px 0px 0px 4px;
    }

    .q-field__marginal {
      height: 30px;
    }
  }

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

  .search-button {
    width: 96px;
    line-height: 29px;
    font-weight: bold;
    text-transform: initial;
    font-size: 11px;
    color: white;

    .q-btn__content {
      background: $secondary;
      border-radius: 3px 3px 3px 3px;

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

.query-editor-container {
  height: calc(100% - 40px) !important;
}
</style>
