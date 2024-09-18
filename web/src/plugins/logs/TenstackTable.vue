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
  <div ref="parentRef" class="container tw-overflow-x-auto tw-relative">
    <table
      data-test="logs-search-result-logs-table"
      class="tw-w-full tw-table-auto"
      :style="{
        minWidth: '100%',
        ...columnSizeVars,
        minHeight: totalSize + 'px',
        width: !defaultColumns
          ? table.getCenterTotalSize() + 'px'
          : wrap
            ? width
              ? width - 12 + 'px'
              : '100%'
            : '100%',
      }"
    >
      <thead
        class="tw-sticky tw-top-0 tw-z-10"
        style="max-height: 44px; height: 22px"
      >
        <vue-draggable
          v-model="columnOrder"
          v-for="headerGroup in table.getHeaderGroups()"
          :key="headerGroup.id"
          :element="'table'"
          :animation="200"
          :sort="!isResizingHeader || !defaultColumns"
          handle=".table-head"
          :class="{
            'tw-cursor-move': table.getState().columnOrder.length > 1,
          }"
          :style="{
            width:
              defaultColumns && wrap
                ? width - 12 + 'px'
                : defaultColumns
                  ? tableRowSize + 'px'
                  : table.getTotalSize() + 'px',
            minWidth: '100%',
            background: store.state.theme === 'dark' ? '#565656' : '#F5F5F5',
          }"
          tag="tr"
          @start="(event) => handleDragStart(event)"
          @end="() => handleDragEnd()"
          class="tw-flex items-center"
        >
          <th
            v-for="header in headerGroup.headers"
            :key="header.id"
            :id="header.id"
            class="tw-px-2 tw-relative table-head tw-text-ellipsis"
            :style="{ width: `calc(var(--header-${header?.id}-size) * 1px)` }"
            :data-test="`log-search-result-table-th-${header.id}`"
          >
            <div
              v-if="header.column.getCanResize()"
              @dblclick="header.column.resetSize()"
              @mousedown.self.prevent.stop="header.getResizeHandler()?.($event)"
              @touchstart.self.prevent.stop="
                header.getResizeHandler()?.($event)
              "
              :class="[
                'resizer',
                store.state.theme === 'dark'
                  ? 'tw-bg-zinc-800'
                  : 'tw-bg-zinc-300',
                header.column.getIsResizing() ? 'isResizing' : '',
              ]"
              :style="{}"
              class="tw-right-0"
            />

            <div
              v-if="!header.isPlaceholder"
              :class="[
                'text-left',
                header.column.getCanSort() ? 'cursor-pointer select-none' : '',
              ]"
              @click="
                getSortingHandler(
                  $event,
                  header.column.getToggleSortingHandler(),
                )
              "
              class="tw-overflow-hidden tw-text-ellipsis"
            >
              <FlexRender
                :render="header.column.columnDef.header"
                :props="header.getContext()"
              />

              <div
                :data-test="`log-add-data-from-column-${header.column.columnDef.header}`"
                class="tw-invisible tw-items-center tw-absolute tw-right-2 tw-top-0 tw-px-2 column-actions"
                :class="
                  store.state.theme === 'dark' ? 'field_overlay_dark' : ''
                "
                v-if="
                  (header.column.columnDef.meta as any).closable ||
                  (header.column.columnDef.meta as any).showWrap
                "
              >
                <q-icon
                  v-if="(header.column.columnDef.meta as any).closable"
                  :data-test="`logs-search-result-table-th-remove-${header.column.columnDef.header}-btn`"
                  name="cancel"
                  class="q-ma-none close-icon cursor-pointer"
                  :class="
                    store.state.theme === 'dark' ? 'text-white' : 'text-grey-7'
                  "
                  :title="t('common.close')"
                  size="18px"
                  @click="closeColumn(header.column.columnDef)"
                >
                </q-icon>
              </div>
            </div>
          </th>
        </vue-draggable>

        <tr v-if="loading" class="tw-w-full">
          <td
            :colspan="columnOrder.length"
            class="text-bold"
            :style="{
              background: store.state.theme === 'dark' ? '#565656' : '#F5F5F5',
              opacity: 0.7,
            }"
          >
            <div
              class="text-subtitle2 text-weight-bold tw-flex tw-items-center"
            >
              <q-spinner-hourglass size="20px" />
              {{ t("confirmDialog.loading") }}
            </div>
          </td>
        </tr>
        <tr v-if="!loading && errMsg != ''" class="tw-w-full">
          <td
            :colspan="columnOrder.length"
            class="text-bold"
            style="opacity: 0.7"
          >
            <div class="text-subtitle2 text-weight-bold bg-warning">
              <q-icon size="xs" name="warning" class="q-mr-xs" />
              {{ errMsg }}
            </div>
          </td>
        </tr>
        <tr
          data-test="log-search-result-function-error"
          v-if="functionErrorMsg != ''"
        >
          <td
            :colspan="columnOrder.length"
            class="text-bold"
            style="opacity: 0.6"
          >
            <div
              class="text-subtitle2 text-weight-bold q-pl-sm"
              :class="
                store.state.theme === 'dark'
                  ? 'tw-bg-yellow-600'
                  : 'tw-bg-amber-300'
              "
            >
              <q-btn
                :icon="isFunctionErrorOpen ? 'expand_more' : 'chevron_right'"
                dense
                size="xs"
                flat
                class="q-mr-xs"
                data-test="table-row-expand-menu"
                @click.capture.stop="expandFunctionError"
              ></q-btn
              ><b>
                <q-icon name="warning" size="15px"></q-icon>
                {{ t("search.functionErrorLabel") }}</b
              >
            </div>
          </td>
        </tr>
        <tr v-if="functionErrorMsg != '' && isFunctionErrorOpen">
          <td
            :colspan="columnOrder.length"
            style="opacity: 0.7"
            class="q-px-sm"
            :class="
              store.state.theme === 'dark'
                ? 'tw-bg-yellow-600'
                : 'tw-bg-amber-300'
            "
          >
            <pre>{{ functionErrorMsg }}</pre>
          </td>
        </tr>
      </thead>
      <tbody
        data-test="logs-search-result-table-body"
        ref="tableBodyRef"
        class="tw-relative"
      >
        <template v-for="virtualRow in virtualRows" :key="virtualRow.id">
          <tr
            :data-test="`logs-search-result-detail-${
              (tableRows[virtualRow.index] as any)[
                store.state.zoConfig.timestamp_column || '_timestamp'
              ]
            }`"
            :style="{
              transform: `translateY(${virtualRow.start}px)`,
              minWidth: '100%',
            }"
            :data-index="virtualRow.index"
            :ref="(node: any) => rowVirtualizer.measureElement(node)"
            class="tw-absolute tw-flex tw-w-max tw-items-center tw-justify-start tw-border-b tw-cursor-pointer"
            :class="[
              store.state.theme === 'dark'
                ? 'w-border-gray-800  hover:tw-bg-zinc-800'
                : 'w-border-gray-100 hover:tw-bg-zinc-100',
              defaultColumns &&
              !wrap &&
              !(formattedRows[virtualRow.index]?.original as any)?.isExpandedRow
                ? 'tw-table-row'
                : 'tw-flex',
              (tableRows[virtualRow.index] as any)[
                store.state.zoConfig.timestamp_column
              ] === highlightTimestamp
                ? store.state.theme === 'dark'
                  ? 'tw-bg-zinc-700'
                  : 'tw-bg-zinc-300'
                : '',
            ]"
            @click="
              !(formattedRows[virtualRow.index]?.original as any)
                ?.isExpandedRow &&
                handleDataRowClick(
                  tableRows[virtualRow.index],
                  virtualRow.index,
                )
            "
          >
            <td
              v-if="
                (formattedRows[virtualRow.index]?.original as any)
                  ?.isExpandedRow
              "
              :colspan="columnOrder.length"
              :data-test="`log-search-result-expanded-row-${virtualRow.index}`"
              class="tw-w-full"
            >
              <json-preview
                :value="tableRows[virtualRow.index - 1] as any"
                show-copy-button
                class="tw-py-1"
                mode="expanded"
                @copy="copyLogToClipboard"
                @add-field-to-table="addFieldToTable"
                @add-search-term="addSearchTerm"
              />
            </td>
            <template v-else>
              <td
                v-for="(cell, cellIndex) in formattedRows[
                  virtualRow.index
                ].getVisibleCells()"
                :key="cell.id"
                :data-test="
                  'log-table-column-' +
                  virtualRow.index +
                  '-' +
                  cell.column.columnDef.id
                "
                class="tw-py-none tw-px-2 tw-items-center tw-justify-start tw-relative table-cell"
                :style="{
                  width:
                    cell.column.columnDef.id !== 'source' ||
                    cell.column.columnDef.enableResizing
                      ? `calc(var(--col-${cell.column.columnDef.id}-size) * 1px)`
                      : wrap
                        ? width - 260 - 12 + 'px'
                        : 'auto',
                  height: wrap ? '100%' : '20px',
                }"
                :class="tableCellClass"
                @mouseover="handleCellMouseOver(cell)"
                @mouseleave="handleCellMouseLeave()"
              >
                <q-btn
                  v-if="cellIndex == 0"
                  :icon="
                    expandedRowIndices.includes(virtualRow.index)
                      ? 'expand_more'
                      : 'chevron_right'
                  "
                  dense
                  size="xs"
                  flat
                  class="q-mr-xs"
                  data-test="table-row-expand-menu"
                  @click.capture.stop="handleExpandRow(virtualRow.index)"
                ></q-btn>

                <template
                  v-if="activeCellActionId === `${cell.id}_${cell.column.id}`"
                >
                  <cell-actions
                    v-if="
                      (cell.column.columnDef.meta as any)?.closable &&
                      (cell.row.original as any)[cell.column.id]
                    "
                    :column="cell.column"
                    :row="cell.row.original as any"
                    @copy="copyLogToClipboard"
                    @add-search-term="addSearchTerm"
                    @add-field-to-table="addFieldToTable"
                  />
                </template>

                <FlexRender
                  :render="cell.column.columnDef.cell"
                  :props="cell.getContext()"
                />
              </td>
            </template>
          </tr>
        </template>
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, defineEmits, watch, nextTick, onMounted } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import {
  FlexRender,
  type ColumnDef,
  type SortingState,
  useVueTable,
  getCoreRowModel,
  getSortedRowModel,
} from "@tanstack/vue-table";
import JsonPreview from "./JsonPreview.vue";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import { VueDraggableNext as VueDraggable } from "vue-draggable-next";
import CellActions from "@/plugins/logs/data-table/CellActions.vue";
import { debounce } from "quasar";

const props = defineProps({
  rows: {
    type: Array,
    required: true,
  },
  columns: {
    type: Array,
    required: true,
  },
  wrap: {
    type: Boolean,
    default: false,
  },
  width: {
    type: Number,
    default: 56,
  },
  loading: {
    type: Boolean,
    default: false,
  },
  errMsg: {
    type: String,
    default: "",
  },
  functionErrorMsg: {
    type: String,
    default: "",
  },
  expandedRows: {
    type: Array,
    default: () => [],
  },
  highlightTimestamp: {
    type: Number,
    default: -1,
  },
  defaultColumns: {
    type: Boolean,
    default: () => true,
  },
});

const { t } = useI18n();

const emits = defineEmits([
  "copy",
  "addSearchTerm",
  "addFieldToTable",
  "closeColumn",
  "click:dataRow",
  "update:columnSizes",
  "update:columnOrder",
  "expandRow",
]);

const sorting = ref<SortingState>([]);

const store = useStore();

const getSortingHandler = (e: Event, fn: any) => {
  return fn(e);
};

const setSorting = (sortingUpdater: any) => {
  const newSortVal = sortingUpdater(sorting.value);

  sorting.value = newSortVal;
};

const tableBodyRef = ref<any>(null);

const columnResizeMode = "onChange";

const tableRowSize = ref(0);

const columnOrder = ref<any>([]);

const tableRows = ref(props.rows);

const isFunctionErrorOpen = ref(false);

const activeCellActionId = ref("");

watch(
  () => props.columns,
  async (newVal) => {
    columnOrder.value = newVal.map((column: any) => column.id);

    await nextTick();

    if (props.defaultColumns) updateTableWidth();
  },
  {
    deep: true,
  },
);

watch(
  () => props.rows,
  async (newVal) => {
    if (newVal) tableRows.value = [...newVal];

    await nextTick();
    await nextTick();

    expandedRowIndices.value = [];
    setExpandedRows();

    await nextTick();
    if (props.defaultColumns) updateTableWidth();
  },
  {
    deep: true,
  },
);

watch(
  () => columnOrder.value,
  () => {
    emits("update:columnOrder", columnOrder.value, props.columns);
  },
);

const tableCellClass = computed(() => [
  hasDefaultSourceColumn.value && !props.wrap ? "tw-table-cell" : "tw-block",
  !props.wrap && "tw-overflow-hidden tw-text-ellipsis tw-whitespace-nowrap",
  props.wrap && "tw-break-words",
]);

const table = useVueTable({
  get data() {
    return tableRows.value || [];
  },
  get columns() {
    return props.columns as ColumnDef<unknown, any>[];
  },
  state: {
    get sorting() {
      return sorting.value;
    },
    get columnOrder() {
      return columnOrder.value;
    },
  },
  onSortingChange: setSorting,
  enableSorting: false,
  getCoreRowModel: getCoreRowModel(),
  getSortedRowModel: getSortedRowModel(),
  defaultColumn: {
    minSize: 60,
    maxSize: 800,
  },
  columnResizeMode,
  enableColumnResizing: true,
});

const columnSizeVars = computed(() => {
  const headers = table.getFlatHeaders();
  const colSizes: { [key: string]: number } = {};
  for (let i = 0; i < headers.length; i++) {
    const header = headers[i]!;
    colSizes[`--header-${header.id}-size`] = header.getSize();
    colSizes[`--col-${header.column.id}-size`] = header.column.getSize();
  }
  return colSizes;
});

watch(columnSizeVars, (newColSizes) => {
  debouncedUpdate(newColSizes);
});

onMounted(() => {
  setExpandedRows();
});

const hasDefaultSourceColumn = computed(
  () => props.defaultColumns && columnOrder.value.includes("source"),
);

const updateTableWidth = async () => {
  tableRowSize.value = tableBodyRef?.value?.children[0]?.scrollWidth;

  setTimeout(() => {
    let max = 0;
    let width = max;
    for (let i = 0; i < tableRows.value.length; i++) {
      width = tableBodyRef?.value?.children[i]?.scrollWidth;
      if (width > max) max = width;
    }
    tableRowSize.value = max;
  }, 0);
};

const debouncedUpdate = debounce((newColSizes) => {
  emits("update:columnSizes", newColSizes);
}, 500);

const formattedRows = computed(() => {
  return table.getRowModel().rows;
});

const isResizingHeader = ref(false);

const headerGroups = computed(() => table.getHeaderGroups()[0]);

const headers = computed(() => headerGroups.value.headers);

watch(
  () => headers.value,
  (newVal) => {
    isResizingHeader.value = newVal.some((header) =>
      header.column.getIsResizing(),
    );
  },
  {
    deep: true,
  },
);

const parentRef = ref<HTMLElement | null>(null);

const rowVirtualizerOptions = computed(() => {
  return {
    count: formattedRows.value.length,
    getScrollElement: () => parentRef.value,
    estimateSize: () => 20,
    overscan: 80,
    measureElement:
      typeof window !== "undefined" &&
      navigator.userAgent.indexOf("Firefox") === -1
        ? (element: any) => element?.getBoundingClientRect().height
        : undefined,
  };
});

const rowVirtualizer = useVirtualizer(rowVirtualizerOptions);

const virtualRows = computed(() => rowVirtualizer.value.getVirtualItems());

const totalSize = computed(() => rowVirtualizer.value.getTotalSize());

const setExpandedRows = () => {
  props.expandedRows.forEach((index: any) => {
    if (index < props.rows.length) {
      expandRow(index as number);
    }
  });
};

const copyLogToClipboard = (value: any, copyAsJson: boolean = true) => {
  emits("copy", value, copyAsJson);
};
const addSearchTerm = (value: string) => {
  emits("addSearchTerm", value);
};
const addFieldToTable = (value: string) => {
  emits("addFieldToTable", value);
};

const closeColumn = (data: any) => {
  emits("closeColumn", data);
};

const handleDragStart = (event: any) => {
  if (
    columnOrder.value[event.oldIndex] === store.state.zoConfig.timestamp_column
  ) {
    isResizingHeader.value = true;
  } else {
    isResizingHeader.value = false;
  }
};

const handleDragEnd = async () => {
  if (
    columnOrder.value.includes(store.state.zoConfig.timestamp_column) &&
    columnOrder.value[0] !== store.state.zoConfig.timestamp_column
  ) {
    const newColumnOrder = columnOrder.value.filter(
      (column: any) => column !== store.state.zoConfig.timestamp_column,
    );
    newColumnOrder.unshift(store.state.zoConfig.timestamp_column);
    columnOrder.value = [...newColumnOrder];
  }
};

const expandedRowIndices = ref<number[]>([]);

const handleExpandRow = (index: number) => {
  emits("expandRow", calculateActualIndex(index));

  expandRow(index);
};

const expandRow = async (index: number) => {
  let isCollapseOperation = false;

  if (expandedRowIndices.value.includes(index)) {
    expandedRowIndices.value = expandedRowIndices.value.filter(
      (i) => i !== index,
    );

    expandedRowIndices.value = expandedRowIndices.value.map((i) =>
      i > index ? i - 1 : i,
    );

    tableRows.value.splice(index + 1, 1);
    isCollapseOperation = true;
  } else {
    expandedRowIndices.value.push(index);

    tableRows.value.splice(index + 1, 0, {
      isExpandedRow: true,
      ...(props.rows[index] as {}),
    });

    expandedRowIndices.value = expandedRowIndices.value.map((i) =>
      i > index ? i + 1 : i,
    );
  }

  expandedRowIndices.value = expandedRowIndices.value.sort();

  tableRows.value = [...tableRows.value];

  await nextTick();

  if (isCollapseOperation)
    expandedRowIndices.value.forEach((expandedIndex) => {
      if (expandedIndex !== -1) {
        formattedRows.value[expandedIndex].toggleExpanded();
      }
    });
};

const calculateActualIndex = (index: number): number => {
  let actualIndex = index;
  expandedRowIndices.value.forEach((expandedIndex) => {
    if (expandedIndex !== -1 && expandedIndex < index) {
      actualIndex -= 1;
    }
  });
  return actualIndex;
};

const handleDataRowClick = (row: any, index: number) => {
  const actualIndex = calculateActualIndex(index);

  if (actualIndex !== -1) {
    emits("click:dataRow", row, actualIndex);
  }
};

const expandFunctionError = () => {
  isFunctionErrorOpen.value = !isFunctionErrorOpen.value;
};
// Specific function that updates the active cell action ID
const updateActiveCell = (cell?: { id: string; column: { id: string } }) => {
  if (!cell) {
    activeCellActionId.value = "";
  } else {
    activeCellActionId.value = `${cell.id}_${cell.column.id}`;
  }
};

// Debounced version of the function
const debounceCellAction = debounce(updateActiveCell, 500);

// Event handlers for mouse over and mouse leave
const handleCellMouseOver = (cell: { id: string; column: { id: string } }) => {
  debounceCellAction(cell);
};

const handleCellMouseLeave = () => {
  activeCellActionId.value = "";
};

defineExpose({
  parentRef,
});
</script>
<style scoped lang="scss">
.resizer {
  position: absolute;
  top: 0;
  height: 100%;
  width: 5px;
  cursor: col-resize;
  user-select: none;
  touch-action: none;
}

.resizer.ltr {
  right: 0;
}

.resizer.rtl {
  left: 0;
}

.resizer.isResizing {
  background: $primary;
  opacity: 1;
}

.container {
  height: calc(100vh - 294px);
  overflow: auto;
}

.cursor-pointer {
  cursor: pointer;
}

.select-none {
  user-select: none;
}

.text-left {
  text-align: left;
}

table {
  border-collapse: collapse;
  border-spacing: 0;
  font-size: 12px !important;
}

thead {
  background: lightgray;
  font-family: "Nunito Sans", sans-serif;
  font-size: 14px !important;
}

.table-row {
  border-bottom: 1px solid gray;
}

th {
  text-align: left;

  &:hover {
    .column-actions {
      visibility: visible !important;
    }
  }
}

td {
  font-family: monospace;
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

.table-cell {
  &:hover {
    .table-cell-actions {
      display: block !important;
    }
  }
}
</style>
