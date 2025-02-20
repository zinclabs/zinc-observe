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
    ref="qTable"
    :rows="invoiceHistory"
    :columns="columns"
    row-key="id"
    :pagination="pagination"
  >
    <template #no-data><NoData /></template>
    <template #body-cell-actions="props">
      <q-td :props="props">
        <q-btn
          :href="props.row.pdf"
          target="_blank"
          :title="t('billing.downloadInvoice')"
          class="q-ml-xs"
          padding="sm"
          unelevated
          size="sm"
          round
          flat
          icon="download"
        />
      </q-td>
    </template>
    <template #bottom="scope">
      <QTablePagination
        :scope="scope"
        :resultTotal="resultTotal"
        :perPageOptions="perPageOptions"
        position="bottom"
        @update:changeRecordPerPage="changePagination"
        @update:maxRecordToReturn="changeMaxRecordToReturn"
      />
    </template>
  </q-table>
</template>

<script lang="ts">
// @ts-nocheck
import { computed, defineComponent, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar, date } from "quasar";
import NoData from "@/components/shared/grid/NoData.vue";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import BillingService from "@/services/billings";
import { getImageURL } from "@/utils/zincutils";

export default defineComponent({
  name: "InvoiceHistory",
  components: {
    NoData,
    QTablePagination,
  },
  props: [],
  setup(props) {
    const { t } = useI18n();
    const $q = useQuasar();
    const store = useStore();
    const qTable: any = ref(null);
    const columns = ref<QTableProps["columns"]>([
      {
        name: "id",
        field: "id",
        label: "#",
        align: "left",
        sortable: true,
      },
      {
        name: "amount",
        field: "amount",
        label: t("billing.amount"),
        align: "left",
        sortable: true,
      },
      {
        name: "paid",
        field: "paid",
        label: t("billing.amountPaid"),
        align: "left",
        sortable: true,
      },
      {
        name: "start_date",
        field: "start_date",
        label: t("billing.invoiceStartDate"),
        align: "left",
        sortable: true,
      },
      {
        name: "end_date",
        field: "end_date",
        label: t("billing.invoiceEndDate"),
        align: "left",
        sortable: true,
      },
      {
        name: "status",
        field: "status",
        label: t("billing.status"),
        align: "left",
        sortable: true,
      },
      {
        name: "actions",
        field: "actions",
        label: t("billing.action"),
        align: "center",
      },
    ]);
    const resultTotal = ref<number>(0);
    const invoiceHistory = ref([]);
    const pagination: any = ref({
      rowsPerPage: 5,
    });
    // const selectedPerPage = ref<number>(20);
    const perPageOptions = [
      { label: "5", value: 5 },
      { label: "10", value: 10 },
      { label: "20", value: 20 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "All", value: 0 },
    ];

    onMounted(() => {
      getInvoiceHistory();
    });

    const getInvoiceHistory = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading invoice history...",
      });

      BillingService.list_invoice_history(
        store.state.selectedOrganization.identifier
      )
        .then((res) => {
          dismiss();
          const invoiceList = res.data;
          if (invoiceList.length > 0) {
            resultTotal.value = invoiceList.length;
            invoiceHistory.value = invoiceList.map((invoice, index) => {
              return {
                id: ++index,
                start_date: invoice.period_start,
                end_date: invoice.period_end,
                paid: invoice.paid ? "Yes" : "No",
                amount: invoice.total + " " + invoice.currency.toUpperCase(),
                amount_paid: invoice.amount_paid,
                amount_due: invoice.amount_due,
                attempt_count: invoice.attempt_count,
                status: invoice.status,
                pdf: invoice.invoice_pdf,
                action: "Download",
              };
            });
          }
        })
        .catch((e) => {
          dismiss();
          $q.notify({
            type: "negative",
            message: e.message,
            timeout: 5000,
          });
        });
    };

    const changePagination = (val: { label: string; value: any }) => {
      // selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value.setPagination(pagination.value);
    };
    const changeMaxRecordToReturn = (val: any) => {
      // maxRecordToReturn.value = val;
      // getOrganizations();
    };

    return {
      t,
      // $q,
      store,
      qTable,
      columns,
      resultTotal,
      invoiceHistory,
      pagination,
      changePagination,
      // selectedPerPage,
      changeMaxRecordToReturn,
      perPageOptions,
      getInvoiceHistory,
      getImageURL,
    };
  },
});
</script>
<style lang="scss" scoped>
.download-icon {
  cursor: pointer;
}
</style>
