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
  <q-page class="q-pa-none">
    <q-table
      ref="qTable"
      :rows="organizations"
      :columns="columns"
      row-key="id"
      :pagination="pagination"
      :filter="filterQuery"
      :filter-method="filterData"
      :loading="loading"
    >
      <template #no-data><NoData /></template>

      <template #top="scope">
        <div class="full-width q-mb-md">
          <div class="q-table__title">{{ t("organization.header") }}</div>
        </div>
        <q-input
          v-model="filterQuery"
          filled
          dense
          class="q-ml-none q-mb-xs"
          style="width: 400px"
          :placeholder="t('organization.search')"
        >
          <template #prepend>
            <q-icon name="search" />
          </template>
        </q-input>

        <QTablePagination
          :scope="scope"
          :pageTitle="t('organization.header')"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          position="top"
          @update:changeRecordPerPage="changePagination"
        />
      </template>

      <template #bottom="scope">
        <QTablePagination
          :scope="scope"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          :maxRecordToReturn="maxRecordToReturn"
          position="bottom"
          @update:changeRecordPerPage="changePagination"
          @update:maxRecordToReturn="changeMaxRecordToReturn"
        />
      </template>
    </q-table>
  </q-page>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, ref } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { useI18n } from "vue-i18n";

import organizationsService from "@/services/organizations";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import NoData from "@/components/shared/grid/NoData.vue";
import { convertToTitleCase } from "@/utils/zincutils";
import { onBeforeMount } from "vue";

export default defineComponent({
  name: "PageOrganization",
  components: {
    QTablePagination,
    NoData,
  },
  setup() {
    const store = useStore();
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const organizations = ref([]);
    const organization = ref({});
    const qTable: any = ref(null);
    const columns = ref<QTableProps["columns"]>([
      {
        name: "#",
        label: "#",
        field: "#",
        align: "left",
      },
      {
        name: "name",
        field: "name",
        label: t("organization.name"),
        align: "left",
        sortable: true,
      },
      {
        name: "identifier",
        field: "identifier",
        label: t("organization.identifier"),
        align: "left",
        sortable: true,
      },
      {
        name: "type",
        field: "type",
        label: t("organization.type"),
        align: "left",
        sortable: true,
      },
      {
        name: "owner",
        field: "owner",
        label: t("organization.owner"),
        align: "left",
        sortable: true,
      },
    ]);
    const perPageOptions = [
      { label: "5", value: 5 },
      { label: "10", value: 10 },
      { label: "20", value: 20 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "All", value: 0 },
    ];
    const resultTotal = ref<number>(0);
    const maxRecordToReturn = ref<number>(100);
    const selectedPerPage = ref<number>(20);
    const pagination: any = ref({
      rowsPerPage: 20,
    });

    onBeforeMount(() => {
      getOrganizations();
    });

    const changePagination = (val: { label: string; value: any }) => {
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value.setPagination(pagination.value);
    };
    const changeMaxRecordToReturn = (val: any) => {
      maxRecordToReturn.value = val;
      getOrganizations();
    };

    const getOrganizations = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading organizations...",
      });
      organizationsService.list(0, 100000, "name", false, "").then((res) => {
        // Updating store so that organizations in navbar also gets updated
        store.dispatch("setOrganizations", res.data.data);

        resultTotal.value = res.data.data.length;
        let counter = 1;
        organizations.value = res.data.data.map((data: any) => {
          return {
            "#": counter <= 9 ? `0${counter++}` : counter++,
            name: data.name,
            identifier: data.identifier,
            type: convertToTitleCase(data.type),
            owner: data.username,
          };
        });

        dismiss();
      });
    };

    return {
      t,
      store,
      router,
      qTable,
      loading: ref(false),
      organizations,
      organization,
      columns,
      getOrganizations,
      pagination,
      resultTotal,
      perPageOptions,
      selectedPerPage,
      changePagination,
      maxRecordToReturn,
      changeMaxRecordToReturn,
      filterQuery: ref(""),
      filterData(rows: string | any[], terms: string) {
        const filtered = [];
        terms = terms.toLowerCase();
        for (let i = 0; i < rows.length; i++) {
          if (rows[i]["name"].toLowerCase().includes(terms)) {
            filtered.push(rows[i]);
          }
        }
        return filtered;
      },
    };
  },
});
</script>

<style lang="scss" scoped>
.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}
</style>
