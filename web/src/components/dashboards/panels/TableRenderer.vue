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
  <q-table
    :class="[
      'my-sticky-virtscroll-table',
      { 'no-position-absolute': store.state.printMode },
    ]"
    virtual-scroll
    v-model:pagination="pagination"
    :rows-per-page-options="[0]"
    :virtual-scroll-sticky-size-start="48"
    dense
    :wrap-cells="wrapCells"
    :rows="data.rows || []"
    :columns="data.columns"
    row-key="id"
    ref="tableRef"
    data-test="dashboard-panel-table"
    @row-click="(...args: any) => $emit('row-click', ...args)"
  >
    <template v-slot:body-cell="props">
      <q-td :props="props" :style="getStyle(props)">
        {{ props.value }}
      </q-td>
    </template>
  </q-table>
</template>

<script lang="ts">
import useNotifications from "@/composables/useNotifications";
import { exportFile } from "quasar";
import { defineComponent, ref } from "vue";
import { findFirstValidMappedValue } from "@/utils/dashboard/convertDataIntoUnitValue";
import { useStore } from "vuex";

export default defineComponent({
  name: "TableRenderer",
  props: {
    data: {
      required: true,
      type: Object,
      default: () => ({ rows: [], columns: {} }),
    },
    wrapCells: {
      required: false,
      type: Boolean,
      default: false,
    },
    valueMapping: {
      required: false,
      type: Object,
      default: () => [],
    },
  },
  emits: ["row-click"],
  setup(props: any) {
    const tableRef: any = ref(null);
    const store = useStore();

    const { showErrorNotification, showPositiveNotification } =
      useNotifications();
    function wrapCsvValue(val: any, formatFn?: any, row?: any) {
      let formatted = formatFn !== void 0 ? formatFn(val, row) : val;

      formatted =
        formatted === void 0 || formatted === null ? "" : String(formatted);

      formatted = formatted.split('"').join('""');
      /**
       * Excel accepts \n and \r in strings, but some other CSV parsers do not
       * Uncomment the next two lines to escape new lines
       */
      // .split('\n').join('\\n')
      // .split('\r').join('\\r')

      return `"${formatted}"`;
    }

    const downloadTableAsCSV = (title?: any) => {
      // naive encoding to csv format
      const content = [
        props?.data?.columns?.map((col: any) => wrapCsvValue(col.label)),
      ]
        .concat(
          tableRef?.value?.filteredSortedRows?.map((row: any) =>
            props?.data?.columns
              ?.map((col: any) =>
                wrapCsvValue(
                  typeof col.field === "function"
                    ? col.field(row)
                    : row[col.field === void 0 ? col.name : col.field],
                  col.format,
                  row
                )
              )
              .join(",")
          )
        )
        .join("\r\n");

      const status = exportFile(
        (title ?? "table-export") + ".csv",
        content,
        "text/csv"
      );

      if (status === true) {
        showPositiveNotification("Table downloaded as a CSV file", {
          timeout: 2000,
        });
      } else {
        showErrorNotification("Browser denied file download...");
      }
    };

    const getStyle = (rowData: any) => {
      const value = rowData?.row[rowData?.col?.field] ?? rowData?.value;

      // Find the first valid mapping with a valid color
      const foundValue = findFirstValidMappedValue(
        value,
        props?.valueMapping,
        "color"
      );

      if (foundValue && foundValue?.color) {
        const hex = foundValue.color;

        // Check if hex is valid
        const isValidHex = /^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})$/i.test(hex);
        if (!isValidHex) {
          return "";
        }

        const isDark = isDarkColor(hex);
        return `background-color: ${hex}; color: ${
          isDark ? "#ffffff" : "#000000"
        }`;
      }
      return "";
    };

    const isDarkColor = (hex: any) => {
      const result: any = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
      const r = parseInt(result[1], 16);
      const g = parseInt(result[2], 16);
      const b = parseInt(result[3], 16);
      const luminance = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
      return luminance < 0.5;
    };

    return {
      pagination: ref({
        rowsPerPage: 0,
      }),
      downloadTableAsCSV,
      tableRef,
      getStyle,
      store,
    };
  },
});
</script>

<style lang="scss" scoped>
.my-sticky-virtscroll-table {
  /* height or max-height is important */
  height: calc(100% - 1px);
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  overflow: auto;

  :deep(.q-table__top),
  :deep(.q-table__bottom),
  :deep(thead tr:first-child th) {
    /* bg color is important for th; just specify one */
    background-color: #fff;
  }

  :deep(thead tr th) {
    will-change: auto !important;
    position: sticky;
    z-index: 1;
  }

  /* this will be the loading indicator */
  :deep(thead tr:last-child th) {
    /* height of all previous header rows */
    top: 48px;
  }

  :deep(thead tr:first-child th) {
    top: 0;
  }

  :deep(.q-virtual-scroll) {
    will-change: auto !important;
  }
}

.no-position-absolute {
  position: static !important;
}

.my-sticky-virtscroll-table.q-dark {
  :deep(.q-table__top),
  :deep(.q-table__bottom),
  :deep(thead tr:first-child th) {
    /* bg color is important for th; just specify one */
    //   background-color: #fff;
    background-color: $dark-page !important;
  }
}
</style>
