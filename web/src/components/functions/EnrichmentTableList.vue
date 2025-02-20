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

<!-- eslint-disable vue/v-on-event-hyphenation -->
<!-- eslint-disable vue/attribute-hyphenation -->
<template>
  <q-page class="q-pa-none" style="min-height: inherit">
    <div v-if="!showAddJSTransformDialog">
      <q-table
        ref="qTable"
        :rows="selectedStreamType == 'all' ? jsTransforms : jobsValue"
        :columns="columns"
        row-key="id"
        :pagination="pagination"
        :filter="filterQuery"
        :filter-method="filterData"
        style="width: 100%"
      >
        <template #no-data>
          <NoData />
        </template>
        <template v-slot:body-cell-actions="props">
          <q-td :props="props">
            <q-btn
              v-if="selectedStreamType == 'all'"
              icon="search"
              :title="t('logStream.explore')"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              @click="exploreEnrichmentTable(props)"
            />
            <q-btn
              v-if="selectedStreamType == 'all'"
              icon="edit"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('function.enrichmentTables')"
              @click="showAddUpdateFn(props)"
            ></q-btn>
            <q-btn
              v-if="selectedStreamType == 'all'"
              :icon="outlinedDelete"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('function.delete')"
              @click="showDeleteDialogFn(props)"
            ></q-btn>
            <q-btn
              v-if="selectedStreamType == 'pending'"
              :icon="outlinedDelete"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('function.delete')"
              @click="deleteJob(props.row)"
            />
          </q-td>
        </template>

        <template v-slot:body-cell-function="props">
          <q-td :props="props">
            <q-tooltip>
              <pre>{{ props.row.function }}</pre>
            </q-tooltip>
            <pre style="white-space: break-spaces">{{
              props.row.function
            }}</pre>
          </q-td>
        </template>
        <template #top="scope">
          <div class="q-table__title">
            {{ t("function.enrichmentTables") }}
          </div>
          <div class="flex items-center q-ml-auto" data-test="enrichment-tables-search-input">
            <div class="flex justify-between items-end q-px-md">
              <div
                style="
                  border: 1px solid #cacaca;
                  padding: 4px;
                  border-radius: 2px;
                "
              >
                <template
                  v-for="visual in streamFilterValues"
                  :key="visual.value"
                >
                  <q-btn
                    :color="
                      visual.value === selectedStreamType ? 'primary' : ''
                    "
                    :flat="visual.value === selectedStreamType ? false : true"
                    dense
                    emit-value
                    no-caps
                    class="visual-selection-btn"
                    style="height: 30px; margin: 0 2px; padding: 4px 12px"
                    @click="onChangeStreamFilter(visual.value)"
                  >
                    {{ visual.label }}</q-btn
                  >
                </template>
              </div>
            </div>
            <q-input
              v-model="filterQuery"
              borderless
              filled
              dense
              class="q-ml-auto q-mb-xs no-border search-en-table-input"
              :placeholder="t('function.searchEnrichmentTable')"
            >
              <template #prepend>
                <q-icon name="search" class="cursor-pointer" />
              </template>
            </q-input>
          </div>
          <q-btn
            class="q-ml-md q-mb-xs text-bold no-border"
            padding="sm lg"
            color="secondary"
            no-caps
            :label="t(`function.addEnrichmentTable`)"
            @click="showAddUpdateFn({})"
          />

          <QTablePagination
            :scope="scope"
            :pageTitle="t('function.enrichmentTables')"
            :position="'top'"
            :resultTotal="selectedStreamType == 'all' ? resultTotal : jobsValue.length"
            :perPageOptions="perPageOptions"
            @update:changeRecordPerPage="changePagination"
          />
        </template>

        <template #bottom="scope">
          <QTablePagination
            :scope="scope"
            :position="'bottom'"
            :resultTotal="selectedStreamType == 'all' ? resultTotal : jobsValue.length"
            :perPageOptions="perPageOptions"
            @update:changeRecordPerPage="changePagination"
          />
        </template>
      </q-table>
    </div>
    <div v-else>
      <add-enrichment-table
        v-model="formData"
        :isUpdating="isUpdated"
        @update:list="refreshList"
        @cancel:hideform="hideForm"
      />
    </div>
    <ConfirmDialog
      title="Delete Enrichment Table"
      message="Are you sure you want to delete enrichment table?"
      @update:ok="deleteLookupTable"
      @update:cancel="confirmDelete = false"
      v-model="confirmDelete"
    />
  </q-page>
</template>

<script lang="ts">
import { defineComponent, onBeforeMount, ref } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar, type QTableProps } from "quasar";
import { useI18n } from "vue-i18n";

import QTablePagination from "../shared/grid/Pagination.vue";
import AddEnrichmentTable from "./AddEnrichmentTable.vue";
import NoData from "../shared/grid/NoData.vue";
import ConfirmDialog from "../ConfirmDialog.vue";
import segment from "../../services/segment_analytics";
import { getImageURL, verifyOrganizationStatus } from "../../utils/zincutils";
import streamService from "@/services/stream";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";
import useStreams from "@/composables/useStreams";
import jstransform from "@/services/jstransform";

export default defineComponent({
  name: "EnrichmentTableList",
  components: { QTablePagination, AddEnrichmentTable, NoData, ConfirmDialog },
  emits: [
    "updated:fields",
    "update:changeRecordPerPage",
    "update:maxRecordToReturn",
  ],
  setup(props, { emit }) {
    const store = useStore();
    const { t } = useI18n();
    const $q = useQuasar();
    const router = useRouter();
    const jsTransforms: any = ref([]);
    const jobsValue: any = ref([]);
    const formData: any = ref({});
    const showAddJSTransformDialog: any = ref(false);
    const qTable: any = ref(null);
    const selectedDelete: any = ref(null);
    const isUpdated: any = ref(false);
    const confirmDelete = ref<boolean>(false);
    const columns: any = ref<QTableProps["columns"]>([
      {
        name: "#",
        label: "#",
        field: "#",
        align: "left",
      },
      {
        name: "name",
        field: "name",
        label: t("function.name"),
        align: "left",
        sortable: true,
      },
      {
        name: "actions",
        field: "actions",
        label: t("function.actions"),
        align: "center",
        sortable: false,
      },
    ]);
    const { getStreams, resetStreamType ,getStream} = useStreams();
    const selectedStreamType = ref("all");
    const streamFilterValues = [
      // { label: t("logStream.labelAll"), value: "all" },
      { label: "Tables", value: "all" },
      { label: "Jobs", value: "pending" }
    ];
    const onChangeStreamFilter = (value:any) =>{
      console.log(value);
      selectedStreamType.value = value;
    }
    const fetchJobs = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading functions...",
      });
      streamService
        .getJobs({
          org_identifier: store.state.selectedOrganization.identifier,
        })
        .then((res: any) => {
          console.log(res);
          let counter = 1;

          jobsValue.value = res.data.map((data: any) => {
            return {
              "#": counter <= 9 ? `0${counter++}` : counter++,
              id: data.stream_name + counter,
              name: data.stream_name,
              actions: "action buttons",
              task_id: data.task_id,
            };
          });
          dismiss();
        })
    };

    onBeforeMount(() => {
      getLookupTables();
      fetchJobs();
    });

    const getLookupTables = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading functions...",
      });

      getStreams("enrichment_tables", false)
        .then((res: any) => {
          let counter = 1;
          resultTotal.value = res.list.length;
          jsTransforms.value = res.list.map((data: any) => {
            return {
              "#": counter <= 9 ? `0${counter++}` : counter++,
              id: data.name + counter,
              name: data.name,
              actions: "action buttons",
              stream_type: data.stream_type,
            };
          });
          dismiss();
        })
        .catch((err) => {
          console.log("--", err);
          dismiss();
          if (err.response.status != 403) {
            $q.notify({
              type: "negative",
              message:
                err.response?.data?.message ||
                "Error while fetching functions.",
              timeout: 2000,
            });
          }
        });
    };

    const perPageOptions: any = [
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
    const changePagination = (val: { label: string; value: any }) => {
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value.setPagination(pagination.value);
    };

    const addLookupTable = () => {
      showAddJSTransformDialog.value = true;
    };

    const showAddUpdateFn = (props: any) => {
      formData.value = props.row;
      let action;
      if (!props.row) {
        isUpdated.value = false;
        action = "Add Enrichment Table";
        router.push({
          name: "enrichmentTables",
          query: {
            action: "add",
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      } else {
        isUpdated.value = true;
        action = "Update Enrichment Table";
        router.push({
          name: "enrichmentTables",
          query: {
            action: "update",
            name: props.row.name,
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      }
      addLookupTable();

      segment.track("Button Click", {
        button: action,
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        page: "Functions",
      });
    };

    const refreshList = () => {
      router.push({
        name: "enrichmentTables",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
      showAddJSTransformDialog.value = false;
      resetStreamType("enrichment_tables");
      getLookupTables();
    };

    const hideForm = () => {
      showAddJSTransformDialog.value = false;
      router.replace({
        name: "enrichmentTables",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    const deleteLookupTable = () => {
      streamService
        .delete(
          store.state.selectedOrganization.identifier,
          selectedDelete.value.name,
          "enrichment_tables",
        )
        .then((res: any) => {
          if (res.data.code == 200) {
            $q.notify({
              color: "positive",
              message: `${selectedDelete.value.name} deleted successfully.`,
            });
            resetStreamType("enrichment_tables");
            getLookupTables();
          }
        })
        .catch((err: any) => {
          if (err.response.status != 403) {
            $q.notify({
              color: "negative",
              message:
                err.response?.data?.message || "Error while deleting stream.",
            });
          }
        });

      segment.track("Button Click", {
        button: "Delete Enrichment Table",
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        function_name: selectedDelete.value.name,
        is_ingest_func: selectedDelete.value.ingest,
        page: "Functions",
      });
    };
    const deleteJob = (job:any) => {
      streamService
        .deleteJob(
          store.state.selectedOrganization.identifier,
          job.task_id
        )
        .then((res: any) => {
          if (res.data.code == 200) {
            $q.notify({
              color: "positive",
              message: `${job.name} deleted successfully.`,
            });
            fetchJobs();
          }
        })
        .catch((err: any) => {
          if(err.response.status != 403){
            $q.notify({
            color: "negative",
            message: err.response?.data?.message || "Error while deleting stream.",
          });
          }
        });
    };
    const showDeleteDialogFn = (props: any) => {
      selectedDelete.value = props.row;
      confirmDelete.value = true;
    };

    /**
     * Get time range for stream explorer, for enrichment tables it will get the time range from the stream data min and max time
     * @param stream: Stream object
     */
    const getTimeRange = async (stream: any) => {
      const dateTime: { period?: string; from?: number; to?: number } = {};

      const dismiss = $q.notify({
        spinner: true,
        message: "Redirecting to explorer...",
        color: "secondary",
      });

      try {
        await getStream(stream.name, stream.stream_type, true)
          .then((streamResponse) => {
            if (
              streamResponse.stats.doc_time_min &&
              streamResponse.stats.doc_time_max
            ) {
              dateTime["from"] = streamResponse.stats.doc_time_min - 60000000;
              dateTime["to"] = streamResponse.stats.doc_time_max + 60000000;
            } else if (streamResponse.stats.created_at) {
              // When enrichment table is uploaded, stats will not have doc_time_min and doc_time_max.
              // Stats will be available asynchronously, so we can use created_at time to get the time range.
              dateTime["from"] = streamResponse.stats.created_at - 60000000;
              dateTime["to"] = streamResponse.stats.created_at + 3600000000;
            } else {
              dateTime["period"] = "15m";
            }
          })
          .catch((err) => {
            console.error("Error while getting enrichment table: ", err);
            dateTime["period"] = "15m";
          })
          .finally(() => {
            dismiss();
          });
      } catch (err) {
        console.error("Error while getting enrichment table: ", err);
        dateTime["period"] = "15m";
        dismiss();
      }

      return dateTime;
    };

    const exploreEnrichmentTable = async (props: any) => {
      const timestamps = await getTimeRange(props.row);
      router.push({
        name: "logs",
        query: {
          stream_type: props.row.stream_type,
          stream: props.row.name,
          refresh: "0",
          query: "",
          type: "stream_explorer",
          org_identifier: store.state.selectedOrganization.identifier,
          ...timestamps,
        },
      });
    };

    return {
      t,
      qTable,
      store,
      router,
      jsTransforms,
      columns,
      formData,
      hideForm,
      confirmDelete,
      selectedDelete,
      getLookupTables,
      pagination,
      resultTotal,
      refreshList,
      perPageOptions,
      selectedPerPage,
      addLookupTable,
      deleteLookupTable,
      isUpdated,
      showAddUpdateFn,
      showDeleteDialogFn,
      changePagination,
      maxRecordToReturn,
      showAddJSTransformDialog,
      outlinedDelete,
      filterQuery: ref(""),
      filterData(rows: any, terms: any) {
        var filtered = [];
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (rows[i]["name"].toLowerCase().includes(terms)) {
            filtered.push(rows[i]);
          }
        }
        return filtered;
      },
      getImageURL,
      verifyOrganizationStatus,
      exploreEnrichmentTable,
      selectedStreamType,
      streamFilterValues,
      onChangeStreamFilter,
      fetchJobs,
      jobsValue,
      deleteJob
    };
  },
  computed: {
    selectedOrg() {
      return this.store.state.selectedOrganization.identifier;
    },
    // filteredData() {
    //   if (this.selectedStreamType === "all") {
    //     return this.jsTransforms; 
    //   } else if (this.selectedStreamType === "pending") {
    //     return this.jobsValue; 
    //   } else {
    //     return []; // Handle unexpected cases
    //   }
    // }
  },
  watch: {
    selectedOrg(newVal: any, oldVal: any) {
      this.verifyOrganizationStatus(
        this.store.state.organizations,
        this.router,
      );
      if (
        (newVal != oldVal || this.jsTransforms.value == undefined) &&
        this.router.currentRoute.value.name == "pipeline"
      ) {
        this.resultTotal = 0;
        this.jsTransforms = [];
        this.getLookupTables();
      }
    },
  },
});
</script>

<style lang="scss">
.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}

.search-en-table-input {
  .q-field__inner {
    width: 250px;
  }
}
</style>
