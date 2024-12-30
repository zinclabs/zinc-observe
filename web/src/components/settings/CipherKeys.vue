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

<!-- eslint-disable vue/x-invalid-end-tag -->
<template>
  <q-page class="q-pa-none">
    <div v-if="!showAddDialog">
      <q-table
        ref="qTable"
        :rows="tabledata"
        :columns="columns"
        row-key="id"
        :pagination="pagination"
        :filter="filterQuery"
        :filter-method="filterData"
        :loading="loading"
      >
        <template #no-data><NoData /></template>
        <template #top="scope">
          <div class="row full-width">
            <div
              class="col q-table__title items-start"
              data-test="cipher-keys-list-title"
            >
              {{ t("cipherKey.header") }}
            </div>
            <div class="col-auto flex">
              <q-input
                v-model="filterQuery"
                filled
                dense
                class="q-ml-none q-mb-xs"
                style="width: 400px"
                :placeholder="t('cipherKey.search')"
              >
                <template #prepend>
                  <q-icon name="search" />
                </template>
              </q-input>
              <q-btn
                class="q-mb-xs text-bold no-border q-ml-md"
                padding="sm lg"
                color="secondary"
                no-caps
                dense
                :label="t(`cipherKey.add`)"
                @click="addCipherKey"
              />
            </div>
          </div>
          <div class="row full-width">
            <QTablePagination
              :scope="scope"
              :pageTitle="t('cipherKey.header')"
              :resultTotal="resultTotal"
              :perPageOptions="perPageOptions"
              position="top"
              @update:changeRecordPerPage="changePagination"
            />
          </div>
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
    </div>
    <div v-else>
      <add-cipher-key @cancel:hideform="hideAddDialog" />
    </div>
  </q-page>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, ref, onMounted, onUpdated, watch } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar, date, copyToClipboard } from "quasar";
import { useI18n } from "vue-i18n";

import QTablePagination from "@/components/shared/grid/Pagination.vue";
import NoData from "@/components/shared/grid/NoData.vue";
import segment from "@/services/segment_analytics";
import { convertToTitleCase } from "@/utils/zincutils";
import config from "@/aws-exports";
import AddCipherKey from "@/components/cipherkeys/AddCipherKey.vue";
import cipherKeysService from "@/services/cipher_keys";

export default defineComponent({
  name: "PageOrganization",
  components: {
    QTablePagination,
    NoData,
    AddCipherKey,
  },
  setup() {
    const store = useStore();
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const tabledata = ref([]);
    const showAddDialog = ref(false);
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

    watch(
      () => router.currentRoute.value.query?.action,
      (action) => {
        if (action == "add") {
          showAddDialog.value = true;
        }
      },
    );

    onMounted(() => {
      if (router.currentRoute.value.query.action == "add") {
        showAddDialog.value = true;
      }

      getCipherKeysData();
    });

    onUpdated(() => {
      if (router.currentRoute.value.query.action == "add") {
        showAddDialog.value = true;
      }
    });

    const getCipherKeysData = () => {
      cipherKeysService
        .list(store.state.selectedOrganization.identifier)
        .then((response) => {
          tabledata.value = response.data;
          resultTotal.value = response.data.length;
        })
        .catch((error) => {
          $q.notify({
            type: "negative",
            message: error.response.data.message,
          });
        });
    };

    const changePagination = (val: { label: string; value: any }) => {
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value.setPagination(pagination.value);
    };
    const changeMaxRecordToReturn = (val: any) => {
      maxRecordToReturn.value = val;
      getOrganizations();
    };

    const addCipherKey = (evt) => {
      router.push({
        query: {
          action: "add",
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });

      if (evt) {
        let button_txt = evt.target.innerText;
        segment.track("Button Click", {
          button: button_txt,
          user_org: store.state.selectedOrganization.identifier,
          user_id: store.state.userInfo.email,
          page: "Organizations",
        });
      }
    };

    const getData = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading data...",
      });
    };

    getData();

    const hideAddDialog = () => {
      showAddDialog.value = !showAddDialog.value;
      router.push({
        name: "cipherKeys",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    return {
      t,
      store,
      router,
      qTable,
      loading: ref(false),
      tabledata,
      columns,
      showAddDialog,
      addCipherKey,
      getData,
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
      hideAddDialog,
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
