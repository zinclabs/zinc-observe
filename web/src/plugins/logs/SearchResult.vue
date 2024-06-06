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

<!-- eslint-disable vue/v-on-event-hyphenation -->
<!-- eslint-disable vue/attribute-hyphenation -->
<template>
  <div class="col column oveflow-hidden full-height">
    <div
      class="search-list full-height"
      ref="searchListContainer"
      style="width: 100%"
    >
      <div class="row">
        <div class="col-6 text-left q-pl-lg q-mt-xs">
          {{ noOfRecordsTitle }}
        </div>
        <div class="col-6 text-right q-pr-md q-gutter-xs pagination-block">
          <q-pagination
            v-if="searchObj.meta.resultGrid.showPagination"
            :disable="searchObj.loading == true"
            v-model="pageNumberInput"
            :key="
              searchObj.data.queryResults.total +
              '-' +
              searchObj.data.resultGrid.currentPage
            "
            :max="
              Math.max(
                1,
                searchObj.data.queryResults?.partitionDetail?.paginations
                  ?.length || 0
              )
            "
            :input="false"
            direction-links
            :boundary-numbers="false"
            :max-pages="5"
            :ellipses="false"
            icon-first="skip_previous"
            icon-last="skip_next"
            icon-prev="fast_rewind"
            icon-next="fast_forward"
            class="float-right paginator-section"
            @update:model-value="getPageData('pageChange')"
            rowsPerPageLabel="Rows per page"
            :rows-per-page-options="rowsPerPageOptions"
            :rows-per-page="searchObj.meta.resultGrid.rowsPerPage"
            style="line-height: 30px; max-height: 30px"
            data-test="logs-search-result-pagination"
          />
          <q-select
            v-if="searchObj.meta.resultGrid.showPagination"
            data-test="logs-search-result-records-per-page"
            v-model="searchObj.meta.resultGrid.rowsPerPage"
            :options="rowsPerPageOptions"
            class="float-right select-pagination"
            size="sm"
            dense
            @update:model-value="getPageData('recordsPerPage')"
            style="line-height: 20px"
          ></q-select>
        </div>
      </div>
      <div v-if="searchObj.data.histogram.errorMsg == ''">
        <ChartRenderer
          v-if="searchObj.meta.showHistogram"
          data-test="logs-search-result-bar-chart"
          :data="plotChart"
          style="max-height: 100px"
          @updated:dataZoom="onChartUpdate"
        />
        <div
          class="q-pb-lg"
          style="top: 50px; position: absolute; left: 45%"
          v-if="searchObj.loadingHistogram == true"
        >
          <q-spinner-hourglass
            color="primary"
            size="25px"
            style="margin: 0 auto; display: block"
          />
        </div>
      </div>
      <div
        v-else-if="
          searchObj.data.histogram.errorMsg != '' &&
          searchObj.meta.showHistogram
        "
      >
        <h6 class="text-center" v-if="searchObj.data.histogram.errorCode != 0">
          <q-icon name="warning" color="warning" size="30px"></q-icon> Error
          while fetching histogram data.
          <q-btn
            @click="toggleErrorDetails"
            size="sm"
            data-test="logs-page-histogram-error-details-btn"
            >{{ t("search.histogramErrorBtnLabel") }}</q-btn
          ><br />
          <span v-if="disableMoreErrorDetails">
            {{ searchObj.data.histogram.errorMsg }}
            {{ searchObj.data.histogram.errorDetail }}
          </span>
        </h6>
        <h6 class="text-center" v-else>
          {{ searchObj.data.histogram.errorMsg }}
          {{ searchObj.data.histogram.errorDetail }}
        </h6>
      </div>
      <q-virtual-scroll
        data-test="logs-search-result-logs-table"
        id="searchGridComponent"
        type="table"
        ref="searchTableRef"
        class="logs-search-result-table"
        :virtual-scroll-item-size="25"
        :virtual-scroll-sticky-size-start="0"
        :virtual-scroll-sticky-size-end="0"
        :virtual-scroll-slice-size="100"
        :virtual-scroll-slice-ratio-before="100"
        :items="searchObj.data.queryResults.filteredHit"
        :wrap-cells="
          searchObj.meta.toggleSourceWrap && searchObj.meta.flagWrapContent
            ? true
            : false
        "
        :style="{
          wordBreak: 'break-word',
          height:
            !searchObj.meta.showHistogram
              ? 'calc(100% - 40px)'
              : 'calc(100% - 140px)',
        }"
      >
        <template v-slot:before>
          <thead class="thead-sticky text-left">
            <tr>
              <th
                v-for="(col, index) in searchObj.data.resultGrid.columns"
                :key="'result_' + index"
                class="table-header"
                :data-test="`log-search-result-table-th-${col.label}`"
                :style="col.wrapContent ? { 'max-width': '200px' } : {}"
              >
                <div class="flex items-center no-wrap table-head-chip q-dark">
                  <span
                    :class="
                      store.state.theme === 'dark' ? 'text-white' : 'text-dark'
                    "
                    class="header-col-title"
                  >
                    {{ col.label }}</span
                  >
                  <div
                    class="flex items-center no-wrap field_overlay"
                    :class="
                      store.state.theme === 'dark' ? 'field_overlay_dark' : ''
                    "
                    style="left: 0 !important"
                    v-if="col.closable || col.showWrap"
                  >
                    <span
                      v-if="col.showWrap"
                      style="font-weight: normal"
                      :class="
                        store.state.theme === 'dark'
                          ? 'text-white'
                          : 'text-grey-9'
                      "
                      >{{ t("common.wrap") }}</span
                    >
                    <q-toggle
                      v-if="col.showWrap"
                      class="text-normal q-ml-xs q-mr-sm"
                      :data-test="`logs-search-result-table-th-remove-${col.label}-btn`"
                      v-model="col.wrapContent"
                      color="primary"
                      :class="
                        store.state.theme === 'dark'
                          ? 'text-white'
                          : 'text-grey-7'
                      "
                      size="xs"
                      dense
                    />

                    <q-icon
                      v-if="col.closable"
                      :data-test="`logs-search-result-table-th-remove-${col.label}-btn`"
                      name="cancel"
                      class="q-ma-none close-icon cursor-pointer"
                      :class="
                        store.state.theme === 'dark'
                          ? 'text-white'
                          : 'text-grey-7'
                      "
                      :title="t('common.close')"
                      size="18px"
                      @click="closeColumn(col)"
                    >
                    </q-icon>
                  </div>
                </div>
              </th>
            </tr>
            <tr v-if="searchObj.loading == true">
              <td
                :colspan="searchObj.data.resultGrid.columns.length"
                class="text-bold"
                style="opacity: 0.7"
              >
                <div class="text-subtitle2 text-weight-bold">
                  <q-spinner-hourglass size="20px" />
                  {{ t("confirmDialog.loading") }}
                </div>
              </td>
            </tr>
            <tr
              v-if="
                searchObj.loading == false && searchObj.data.missingStreamMessage != ''
              "
            >
              <td
                :colspan="searchObj.data.resultGrid.columns.length"
                class="text-bold"
                style="opacity: 0.7"
              >
                <div class="text-subtitle2 text-weight-bold bg-warning">
                  <q-icon size="xs" name="warning" class="q-mr-xs" />
                  {{ searchObj.data.missingStreamMessage }}
                </div>
              </td>
            </tr>
          </thead>
          <tr
            data-test="log-search-result-function-error"
            v-if="searchObj.data.functionError != ''"
          >
            <td
              :colspan="searchObj.data.resultGrid.columns.length"
              class="text-bold"
              style="opacity: 0.6"
            >
              <div class="text-subtitle2 text-weight-bold bg-warning">
                <q-btn
                  :icon="expandedLogs['-1'] ? 'expand_more' : 'chevron_right'"
                  dense
                  size="xs"
                  flat
                  class="q-mr-xs"
                  data-test="table-row-expand-menu"
                  @click.stop="expandLog('function_error', -1)"
                ></q-btn
                ><b>
                  <q-icon name="warning" size="15px"></q-icon>
                  {{ t("search.functionErrorLabel") }}</b
                >
              </div>
            </td>
          </tr>
          <q-tr v-if="expandedLogs['-1']">
            <td
              :colspan="searchObj.data.resultGrid.columns.length"
              class="bg-warning"
              style="opacity: 0.7"
            >
              <pre>{{ searchObj.data.functionError }}</pre>
            </td>
          </q-tr>
        </template>
        <template v-slot="{ item: row, index }">
          <q-tr
            :data-test="`logs-search-result-detail-${
              row[store.state.zoConfig.timestamp_column]
            }`"
            :key="'expand_' + index"
            @click="expandRowDetail(row, index)"
            style="cursor: pointer"
            class="pointer"
            :style="
              row[store.state.zoConfig.timestamp_column] ==
              searchObj.data.searchAround.indexTimestamp
                ? 'background-color:lightgray'
                : ''
            "
          >
            <q-td
              v-for="(column, colIndex) in searchObj.data.resultGrid.columns"
              :key="index + '-' + column.name"
              :data-test="'log-table-column-' + index + '-' + column.name"
              class="field_list ellipsis"
              style="cursor: pointer"
              :style="column.wrapContent ? { 'max-width': '200px' } : {}"
            >
              <div class="flex row items-center no-wrap">
                <q-btn
                  v-if="colIndex == 0"
                  :icon="
                    expandedLogs[index.toString()]
                      ? 'expand_more'
                      : 'chevron_right'
                  "
                  dense
                  size="xs"
                  flat
                  class="q-mr-xs"
                  data-test="table-row-expand-menu"
                  @click.stop="expandLog(row, index)"
                ></q-btn>
                <!-- <high-light
                  :content="
                    column.name == 'source'
                      ? column.prop(row)
                      : searchObj.data.resultGrid.columns.length > 2 &&
                        (column.prop(row, column.name)?.length || 0) > 100
                      ? (column.prop(row, column.name)?.substr(0, 100) || '') +
                        '...'
                      : column.name != '@timestamp'
                      ? row[column.name]
                      : column.prop(row, column.name)
                  "
                  :query-string="
                    searchObj.meta.sqlMode
                      ? searchObj.data.query.split('where')[1]
                      : searchObj.data.query
                  "
                  :title="
                    (column.prop(row, column.name)?.length || 0) > 100 &&
                    column.name != 'source'
                      ? column.prop(row, column.name)
                      : ''
                  "
                ></high-light> -->
                <span class="ellipsis" :title="column.prop(row, column.name)">{{
                  column.prop(row, column.name)
                }}</span>
              </div>
              <div
                v-if="column.closable && row[column.name]"
                class="field_overlay"
                :class="
                  store.state.theme === 'dark' ? 'field_overlay_dark' : ''
                "
                :title="row.name"
                :data-test="`log-add-data-from-column-${row[column.name]}`"
              >
                <q-btn
                  class="q-mr-xs"
                  size="6px"
                  @click.prevent.stop="
                    copyLogToClipboard(
                      column.prop(row, column.name).toString(),
                      false
                    )
                  "
                  title="Copy"
                  round
                  icon="content_copy"
                />
                <q-btn
                  class="q-mr-xs"
                  size="6px"
                  @click.prevent.stop="
                    addSearchTerm(`${column.name}='${row[column.name]}'`)
                  "
                  :data-test="`log-details-include-field-${row[column.name]}`"
                  title="Include Term"
                  round
                >
                  <q-icon color="currentColor">
                    <EqualIcon></EqualIcon>
                  </q-icon>
                </q-btn>
                <q-btn
                  size="6px"
                  @click.prevent.stop="
                    addSearchTerm(`${column.name}!='${row[column.name]}'`)
                  "
                  title="Exclude Term"
                  :data-test="`log-details-exclude-field-${row[column.name]}`"
                  round
                >
                  <q-icon color="currentColor">
                    <NotEqualIcon></NotEqualIcon>
                  </q-icon>
                </q-btn>
              </div>
            </q-td>
          </q-tr>
          <q-tr v-if="expandedLogs[index.toString()]">
            <td :colspan="searchObj.data.resultGrid.columns.length">
              <json-preview
                :value="searchObj.data.queryResults.hits[index]"
                show-copy-button
                @copy="copyLogToClipboard"
                @add-field-to-table="addFieldToTable"
                @add-search-term="addSearchTerm"
              />
            </td>
          </q-tr>
        </template>
      </q-virtual-scroll>
      <q-dialog
        data-test="logs-search-result-detail-dialog"
        v-model="searchObj.meta.showDetailTab"
        position="right"
        full-height
        maximized
        @escap.stop="reDrawChart"
        @hide="reDrawChart"
        @before-hide="reDrawChart"
      >
        <DetailTable
          :key="
            'dialog_' + searchObj.meta.resultGrid.navigation.currentRowIndex
          "
          v-model="
            searchObj.data.queryResults.hits[
              searchObj.meta.resultGrid.navigation.currentRowIndex
            ]
          "
          :stream-type="searchObj.data.stream.streamType"
          style="margin-bottom: 15px"
          :currentIndex="searchObj.meta.resultGrid.navigation.currentRowIndex"
          :totalLength="parseInt(searchObj.data.queryResults.hits.length)"
          @showNextDetail="navigateRowDetail"
          @showPrevDetail="navigateRowDetail"
          @add:searchterm="addSearchTerm"
          @remove:searchterm="removeSearchTerm"
          @search:timeboxed="onTimeBoxed"
          @add:table="addFieldToTable"
        />
      </q-dialog>
    </div>
  </div>
</template>

<script lang="ts">
import {
  computed,
  defineComponent,
  ref,
  onMounted,
  onUpdated,
  onRenderTracked,
  defineAsyncComponent,
} from "vue";
import { copyToClipboard, useQuasar } from "quasar";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";

import HighLight from "../../components/HighLight.vue";
import { byString } from "../../utils/json";
import { getImageURL, useLocalWrapContent } from "../../utils/zincutils";
import EqualIcon from "../../components/icons/EqualIcon.vue";
import NotEqualIcon from "../../components/icons/NotEqualIcon.vue";
import useLogs from "../../composables/useLogs";
import { convertLogData } from "@/utils/logs/convertLogData";

export default defineComponent({
  name: "SearchResult",
  components: {
    HighLight,
    DetailTable: defineAsyncComponent(() => import("./DetailTable.vue")),
    EqualIcon,
    NotEqualIcon,
    JsonPreview: defineAsyncComponent(() => import("./JsonPreview.vue")),
    ChartRenderer: defineAsyncComponent(
      () => import("@/components/dashboards/panels/ChartRenderer.vue")
    ),
  },
  emits: [
    "update:scroll",
    "update:datetime",
    "remove:searchTerm",
    "search:timeboxed",
    "expandlog",
    "update:recordsPerPage",
  ],
  props: {
    expandedLogs: {
      type: Object,
      default: () => ({}),
    },
  },
  methods: {
    getPageData(actionType: string) {
      if (actionType == "prev") {
        if (this.searchObj.data.resultGrid.currentPage > 1) {
          this.searchObj.data.resultGrid.currentPage =
            this.searchObj.data.resultGrid.currentPage - 1;
          this.pageNumberInput = this.searchObj.data.resultGrid.currentPage;
          this.$emit("update:scroll");
          this.searchTableRef.scrollTo(0);
        }
      } else if (actionType == "next") {
        if (
          this.searchObj.data.resultGrid.currentPage <=
          Math.round(
            this.searchObj.data.queryResults.total /
              this.searchObj.meta.resultGrid.rowsPerPage
          )
        ) {
          this.searchObj.data.resultGrid.currentPage =
            this.searchObj.data.resultGrid.currentPage + 1;
          this.pageNumberInput = this.searchObj.data.resultGrid.currentPage;
          this.$emit("update:scroll");
          this.searchTableRef.scrollTo(0);
        }
      } else if (actionType == "recordsPerPage") {
        this.searchObj.data.resultGrid.currentPage = 1;
        this.pageNumberInput = this.searchObj.data.resultGrid.currentPage;
        this.refreshPartitionPagination(true);
        this.$emit("update:recordsPerPage");
        this.searchTableRef.scrollTo(0);
      } else if (actionType == "pageChange") {
        if (
          this.pageNumberInput >
          Math.ceil(
            this.searchObj.data.queryResults.partitionDetail.paginations.length
          )
        ) {
          this.$q.notify({
            type: "negative",
            message:
              "Page number is out of range. Please provide valid page number.",
            timeout: 1000,
          });
          this.pageNumberInput = this.searchObj.data.resultGrid.currentPage;
          return false;
        }

        this.searchObj.data.resultGrid.currentPage = this.pageNumberInput;
        this.$emit("update:scroll");
        this.searchTableRef.scrollTo(0);
      }
    },
    closeColumn(col: any) {
      const RGIndex = this.searchObj.data.resultGrid.columns.indexOf(col.name);
      this.searchObj.data.resultGrid.columns.splice(RGIndex, 1);

      const SFIndex = this.searchObj.data.stream.selectedFields.indexOf(
        col.name
      );

      this.searchObj.data.stream.selectedFields.splice(SFIndex, 1);
      this.searchObj.organizationIdetifier =
        this.store.state.selectedOrganization.identifier;
      this.updatedLocalLogFilterField();
    },
    onChartUpdate({ start, end }: { start: any; end: any }) {
      this.searchObj.meta.showDetailTab = false;
      this.$emit("update:datetime", { start, end });
    },
    onTimeBoxed(obj: any) {
      this.searchObj.meta.showDetailTab = false;
      this.searchObj.data.searchAround.indexTimestamp = obj.key;
      // this.$emit("search:timeboxed", obj);
      this.searchAroundData(obj);
    },
    toggleErrorDetails() {
      this.disableMoreErrorDetails = !this.disableMoreErrorDetails;
    },
  },
  setup(props, { emit }) {
    // Accessing nested JavaScript objects and arrays by string path
    // https://stackoverflow.com/questions/6491463/accessing-nested-javascript-objects-and-arrays-by-string-path
    const { t } = useI18n();
    const store = useStore();
    const $q = useQuasar();
    const searchListContainer = ref(null);
    const noOfRecordsTitle = ref("");
    const scrollPosition = ref(0);
    const rowsPerPageOptions = [10, 25, 50, 100, 250, 500];
    const disableMoreErrorDetails = ref(false);

    const {
      searchObj,
      updatedLocalLogFilterField,
      searchAroundData,
      extractFTSFields,
      evaluateWrapContentFlag,
      refreshPartitionPagination,
      filterHitsColumns,
    } = useLogs();
    const pageNumberInput = ref(1);
    const totalHeight = ref(0);

    const searchTableRef: any = ref(null);

    const plotChart: any = ref({});

    onMounted(() => {
      reDrawChart();
    });

    onUpdated(() => {
      pageNumberInput.value = searchObj.data.resultGrid.currentPage;
    });

    const reDrawChart = () => {
      if (
        // eslint-disable-next-line no-prototype-builtins
        searchObj.data.histogram.hasOwnProperty("xData") &&
        searchObj.data.histogram.xData.length > 0
        // && plotChart.value?.reDraw
      ) {
        //format data in form of echarts options
        plotChart.value = convertLogData(
          searchObj.data.histogram.xData,
          searchObj.data.histogram.yData,
          searchObj.data.histogram.chartParams
        );
        // plotChart.value.forceReLayout();
      }
    };

    const changeMaxRecordToReturn = (val: any) => {
      // searchObj.meta.resultGrid.pagination.rowsPerPage = val;
    };

    const expandRowDetail = (props: any, index: number) => {
      searchObj.meta.showDetailTab = true;
      searchObj.meta.resultGrid.navigation.currentRowIndex = index;
    };

    const getRowIndex = (next: boolean, prev: boolean, oldIndex: number) => {
      if (next) {
        return oldIndex + 1;
      } else {
        return oldIndex - 1;
      }
    };

    const navigateRowDetail = (isNext: boolean, isPrev: boolean) => {
      const newIndex = getRowIndex(
        isNext,
        isPrev,
        Number(searchObj.meta.resultGrid.navigation.currentRowIndex)
      );
      searchObj.meta.resultGrid.navigation.currentRowIndex = newIndex;
    };

    const addSearchTerm = (term: string) => {
      // searchObj.meta.showDetailTab = false;
      searchObj.data.stream.addToFilter = term;
    };

    const removeSearchTerm = (term: string) => {
      emit("remove:searchTerm", term);
    };

    const expandLog = async (row: any, index: number) => {
      emit("expandlog", index);
    };

    const getWidth = computed(() => {
      console.log("get search width", searchListContainer);
      return "";
    });

    function addFieldToTable(fieldName: string) {
      if (searchObj.data.stream.selectedFields.includes(fieldName)) {
        searchObj.data.stream.selectedFields =
          searchObj.data.stream.selectedFields.filter(
            (v: any) => v !== fieldName
          );
      } else {
        searchObj.data.stream.selectedFields.push(fieldName);
      }
      searchObj.organizationIdetifier =
        store.state.selectedOrganization.identifier;
      updatedLocalLogFilterField();
      evaluateWrapContentFlag();
      filterHitsColumns();
    }

    const copyLogToClipboard = (log: any, copyAsJson: boolean = true) => {
      const copyData = copyAsJson ? JSON.stringify(log) : log;
      copyToClipboard(copyData).then(() =>
        $q.notify({
          type: "positive",
          message: "Content Copied Successfully!",
          timeout: 1000,
        })
      );
    };

    return {
      t,
      store,
      plotChart,
      searchObj,
      updatedLocalLogFilterField,
      byString,
      searchTableRef,
      searchAroundData,
      addSearchTerm,
      removeSearchTerm,
      expandRowDetail,
      changeMaxRecordToReturn,
      navigateRowDetail,
      totalHeight,
      reDrawChart,
      expandLog,
      getImageURL,
      addFieldToTable,
      searchListContainer,
      getWidth,
      copyLogToClipboard,
      extractFTSFields,
      evaluateWrapContentFlag,
      useLocalWrapContent,
      noOfRecordsTitle,
      scrollPosition,
      rowsPerPageOptions,
      pageNumberInput,
      refreshPartitionPagination,
      disableMoreErrorDetails,
    };
  },
  computed: {
    toggleWrapFlag() {
      return this.searchObj.meta.toggleSourceWrap;
    },
    findFTSFields() {
      return this.searchObj.data.stream.selectedStreamFields;
    },
    updateTitle() {
      return this.searchObj.data.histogram.chartParams.title;
    },
  },
  watch: {
    toggleWrapFlag() {
      this.useLocalWrapContent(this.searchObj.meta.toggleSourceWrap);
    },
    findFTSFields() {
      this.extractFTSFields();
      this.evaluateWrapContentFlag();
    },
    updateTitle() {
      this.noOfRecordsTitle = this.searchObj.data.histogram.chartParams.title;
    },
  },
});
</script>

<style lang="scss" scoped>
.max-result {
  width: 170px;
}

.pagination-block {
  .q-field--dense .q-field__control,
  .q-field--dense .q-field__marginal {
    height: 30px !important;
  }

  .select-pagination {
    position: relative;
    top: -5px;
  }
}

.search-list {
  width: 100%;

  .chart {
    width: 100%;
    border-bottom: 1px solid rgba(0, 0, 0, 0.12);
  }

  .my-sticky-header-table {
    .q-table__top,
    .q-table__bottom,
    thead tr:first-child th {
      /* bg color is important for th; just specify one */
      background-color: white;
    }

    thead tr th {
      position: sticky;
      z-index: 1;
    }

    thead tr:first-child th {
      top: 0;
    }

    /* this is when the loading indicator appears */
    &.q-table--loading thead tr:last-child th {
      /* height of all previous header rows */
      top: 48px;
    }
  }

  .q-table__top {
    padding-left: 0;
    padding-top: 0;
  }

  .q-table thead tr,
  .q-table tbody td,
  .q-table th,
  .q-table td {
    height: 25px;
    padding: 0px 5px;
    font-size: 0.75rem;
  }

  .q-table__bottom {
    width: 100%;
  }

  .q-table__bottom {
    min-height: 40px;
    padding-top: 0;
    padding-bottom: 0;
  }

  .q-td {
    overflow: hidden;
    min-width: 100px;

    .expanded {
      margin: 0;
      white-space: pre-wrap;
      word-wrap: break-word;
      word-break: break-all;
    }
  }

  .highlight {
    background-color: rgb(255, 213, 0);
  }

  .table-header {
    // text-transform: capitalize;

    .table-head-chip {
      background-color: #f5f5f5;
      padding: 0px;

      .header-col-title {
        margin-right: 0.5rem;
        font-size: 14px;
        color: $dark;
      }

      .close-icon {
        &:hover {
          opacity: 0.7;
        }
      }

      .q-table th.sortable {
        cursor: pointer;
        text-transform: capitalize;
        font-weight: bold;
      }
    }

    &.isClosable {
      padding-right: 30px;
      position: relative;

      .q-table-col-close {
        transform: translateX(26px);
        position: absolute;
        margin-top: 2px;
        color: #808080;
      }
    }

    .q-table th.sortable {
      cursor: pointer;
      text-transform: capitalize;
      font-weight: bold;
    }
  }
}
.thead-sticky tr > *,
.tfoot-sticky tr > * {
  position: sticky;
  opacity: 1;
  z-index: 1;
  background: #f5f5f5;
}

.q-table--dark .thead-sticky tr > *,
.q-table--dark .tfoot-sticky tr > * {
  background: #565656;
}

.q-table--dark .table-header {
  // text-transform: capitalize;

  .table-head-chip {
    background-color: #565656;
  }
}

.thead-sticky tr:last-child > * {
  top: 0;
}

.tfoot-sticky tr:first-child > * {
  bottom: 0;
}

.field_list,
.table-head-chip {
  padding: 0px;
  margin-bottom: 0.125rem;
  position: relative;
  overflow: visible;
  cursor: default;
  font-size: 12px;
  font-family: monospace;

  .field_overlay {
    width: fit-content;
    position: absolute;
    height: 100%;
    right: 0;
    top: 0;
    background-color: #ffffff;
    border-radius: 6px;
    padding: 0 6px;
    visibility: hidden;
    display: flex;
    align-items: center;
    transition: all 0.3s linear;

    &.field_overlay_dark {
      background-color: #181a1b;
    }

    .q-icon,
    .q-toggle__inner {
      cursor: pointer;
      opacity: 0;
      transition: all 0.3s linear;
      margin: 0 1px;
    }
  }

  &:hover {
    .field_overlay {
      visibility: visible;

      .q-icon,
      .q-toggle__inner {
        opacity: 1;
      }
    }
  }
}

.table-head-chip {
  font-family: "Nunito Sans", sans-serif;
  .field_overlay {
    background-color: #f5f5f5;

    &.field_overlay_dark {
      background-color: #565656;
    }
  }
}
</style>

<style lang="scss">
.search-list {
  .copy-log-btn {
    .q-btn .q-icon {
      font-size: 12px !important;
    }
  }

  .q-pagination__content input {
    border: 1px solid lightgrey;
    top: 7px;
    position: relative;
    height: 30px;
  }
}
</style>
