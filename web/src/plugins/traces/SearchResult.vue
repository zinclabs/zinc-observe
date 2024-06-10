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
  <div class="col column oveflow-hidden">
    <div class="search-list" style="width: 100%">
      <ChartRenderer
        data-test="logs-search-result-bar-chart"
        id="traces_scatter_chart"
        :data="plotChart"
        v-show="searchObj.meta.showHistogram"
        style="height: 150px"
        @updated:dataZoom="onChartUpdate"
        @click="onChartClick"
      />

      <div class="text-subtitle1 text-bold q-pt-sm q-px-sm">
        {{ searchObj.data.queryResults?.hits?.length }} Traces
      </div>

      <q-virtual-scroll
        id="tracesSearchGridComponent"
        style="height: 400px"
        :items="searchObj.data.queryResults.hits"
        class="traces-table-container"
        v-slot="{ item, index }"
        :virtual-scroll-item-size="25"
        :virtual-scroll-sticky-size-start="0"
        :virtual-scroll-sticky-size-end="0"
        :virtual-scroll-slice-size="50"
        :virtual-scroll-slice-ratio-before="10"
        @virtual-scroll="onScroll"
      >
        <q-item :key="index" dense>
          <TraceBlock
            :item="item"
            :index="index"
            @click="expandRowDetail(item)"
          />
        </q-item>
      </q-virtual-scroll>
    </div>
  </div>
</template>

<script lang="ts">
import { defineAsyncComponent, defineComponent, nextTick, ref } from "vue";
import { useQuasar } from "quasar";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";

import { byString } from "../../utils/json";
import useTraces from "../../composables/useTraces";
import { getImageURL } from "../../utils/zincutils";
import TraceDetails from "./TraceDetails.vue";
import { convertTraceData } from "@/utils/traces/convertTraceData";
import TraceBlock from "./TraceBlock.vue";
import { useRouter } from "vue-router";

export default defineComponent({
  name: "SearchResult",
  components: {
    TraceDetails,
    ChartRenderer: defineAsyncComponent(
      () => import("@/components/dashboards/panels/ChartRenderer.vue")
    ),
    TraceBlock,
  },
  emits: [
    "update:scroll",
    "update:datetime",
    "remove:searchTerm",
    "search:timeboxed",
    "get:traceDetails",
  ],
  methods: {
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
      if (!(start && end)) return;
      this.searchObj.meta.showDetailTab = false;
      this.$emit("update:datetime", {
        start,
        end,
      });
    },
    onScroll(info: any) {
      if (
        info.ref.items.length / info.index <= 1.2 &&
        this.searchObj.loading == false &&
        this.searchObj.data.resultGrid.currentPage <=
          this.searchObj.data.queryResults.from /
            this.searchObj.meta.resultGrid.rowsPerPage &&
        this.searchObj.data.queryResults.hits.length >
          this.searchObj.meta.resultGrid.rowsPerPage *
            this.searchObj.data.resultGrid.currentPage
      ) {
        this.searchObj.data.resultGrid.currentPage += 1;
        this.$emit("update:scroll");
      }
    },
    onTimeBoxed(obj: any) {
      this.searchObj.meta.showDetailTab = false;
      this.searchObj.data.searchAround.indexTimestamp = obj.key;
      this.$emit("search:timeboxed", obj);
    },
  },
  setup(props, { emit }) {
    // Accessing nested JavaScript objects and arrays by string path
    // https://stackoverflow.com/questions/6491463/accessing-nested-javascript-objects-and-arrays-by-string-path
    const { t } = useI18n();
    const store = useStore();
    const $q = useQuasar();
    const router = useRouter();

    const { searchObj, updatedLocalLogFilterField } = useTraces();
    const totalHeight = ref(0);

    const searchTableRef: any = ref(null);

    const plotChart: any = ref({});

    const reDrawChart = () => {
      if (
        // eslint-disable-next-line no-prototype-builtins
        searchObj.data.histogram.data &&
        searchObj.data.histogram.layout
      ) {
        nextTick(() => {
          plotChart.value = convertTraceData(
            searchObj.data.histogram,
            store.state.timezone
          );
          // plotChart.value.forceReLayout();
        });
      }
    };

    const changeMaxRecordToReturn = (val: any) => {
      // searchObj.meta.resultGrid.pagination.rowsPerPage = val;
    };

    const expandRowDetail = (props: any) => {
      router.push({
        name: "traceDetails",
        query: {
          stream: router.currentRoute.value.query.stream,
          trace_id: props.trace_id,
          from: props.trace_start_time - 10000000,
          to: props.trace_end_time + 10000000,
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });

      emit("get:traceDetails", props);
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

    const closeTraceDetails = () => {
      const query = cloneDeep(router.currentRoute.value.query);
      delete query.trace_id;
      if (query.span_id) delete query.span_id;

      router.push({
        query: {
          ...query,
        },
      });
      setTimeout(() => {
        searchObj.meta.showTraceDetails = false;
        searchObj.data.traceDetails.showSpanDetails = false;
        searchObj.data.traceDetails.selectedSpanId = null;
      }, 100);
    };

    const onChartClick = (data: any) => {
      expandRowDetail(searchObj.data.queryResults.hits[data.dataIndex]);
    };

    return {
      t,
      store,
      plotChart,
      searchObj,
      updatedLocalLogFilterField,
      byString,
      searchTableRef,
      addSearchTerm,
      removeSearchTerm,
      expandRowDetail,
      changeMaxRecordToReturn,
      navigateRowDetail,
      totalHeight,
      reDrawChart,
      getImageURL,
      onChartClick,
    };
  },
});
</script>

<style lang="scss" scoped>
.traces-table-container {
  height: calc(100vh - 326px) !important;
}
.max-result {
  width: 170px;
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
      background-color: $accent;
      padding: 0px;

      .q-chip__content {
        margin-right: 0.5rem;
        font-size: 0.75rem;
        color: $dark;
      }

      .q-chip__icon--remove {
        height: 1rem;
        width: 1rem;
        opacity: 1;
        margin: 0;

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
      padding-right: 26px;
      position: relative;

      .q-table-col-close {
        transform: translateX(26px);
        position: absolute;
        margin-top: 2px;
        color: grey;
        transition: transform 0.3s cubic-bezier(0.25, 0.8, 0.5, 1);
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
.thead-sticky tr:last-child > * {
  top: 0;
}

.tfoot-sticky tr:first-child > * {
  bottom: 0;
}

.field_list {
  padding: 0px;
  margin-bottom: 0.125rem;
  position: relative;
  overflow: visible;
  cursor: default;

  .field_overlay {
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

    .q-icon {
      cursor: pointer;
      opacity: 0;
      transition: all 0.3s linear;
      margin: 0 1px;
    }
  }

  &:hover {
    .field_overlay {
      visibility: visible;

      .q-icon {
        opacity: 1;
      }
    }
  }
}
</style>
