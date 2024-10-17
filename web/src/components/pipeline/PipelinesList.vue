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
  <div v-if="currentRouteName === 'pipelines'">
    <div class="full-wdith">
      <q-table
        data-test="pipeline-list-table"
        ref="qTableRef"
        :rows="pipelines"
        :columns="columns"
        row-key="name"
        :pagination="pagination"
        :filter="filterQuery"
        :filter-method="filterData"
        style="width: 100%"
      >
        <template #no-data>
          <no-data />
        </template>
        <template v-slot:body-cell-actions="props">
          <q-td :props="props">
            <q-btn
              :data-test="`pipeline-list-${props.row.name}-update-pipeline`"
              icon="edit"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('alerts.edit')"
              @click="editPipeline(props.row)"
            ></q-btn>
            <q-btn
              :data-test="`pipeline-list-${props.row.name}-delete-pipeline`"
              :icon="outlinedDelete"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('alerts.delete')"
              @click="openDeleteDialog(props.row)"
            ></q-btn>
          </q-td>
        </template>

        <template v-slot:body-cell-function="props">
          <q-td :props="props">
            <q-tooltip>
              <pre>{{ props.row.sql }}</pre>
            </q-tooltip>
            <pre style="white-space: break-spaces">{{ props.row.sql }}</pre>
          </q-td>
        </template>
        <template #top="scope">
          <div class="q-table__title" data-test="pipeline-list-title">
            {{ t("pipeline.header") }}
          </div>
          <q-input
            data-test="pipeline-list-search-input"
            v-model="filterQuery"
            borderless
            filled
            dense
            class="q-ml-auto q-mb-xs no-border"
            :placeholder="t('pipeline.search')"
          >
            <template #prepend>
              <q-icon name="search" class="cursor-pointer" />
            </template>
          </q-input>
          <q-btn
            data-test="pipeline-list-add-pipeline-btn"
            class="q-ml-md q-mb-xs text-bold no-border"
            padding="sm lg"
            color="secondary"
            no-caps
            :label="t(`pipeline.addPipeline`)"
            @click="createPipeline"
          />

          <q-table-pagination
            :scope="scope"
            :pageTitle="t('pipeline.header')"
            :position="'top'"
            :resultTotal="resultTotal"
            :perPageOptions="perPageOptions"
            @update:changeRecordPerPage="changePagination"
          />
        </template>

        <template #bottom="scope">
          <q-table-pagination
            :scope="scope"
            :position="'bottom'"
            :resultTotal="resultTotal"
            :perPageOptions="perPageOptions"
            @update:changeRecordPerPage="changePagination"
          />
        </template>
      </q-table>
    </div>
  </div>

  <router-view v-else />

  <q-dialog v-model="showCreatePipeline" position="right" full-height maximized>
    <stream-selection @save="savePipeline" />
  </q-dialog>

  <confirm-dialog
    :title="confirmDialogMeta.title"
    :message="confirmDialogMeta.message"
    @update:ok="confirmDialogMeta.onConfirm()"
    @update:cancel="resetConfirmDialog"
    v-model="confirmDialogMeta.show"
  />
</template>
<script setup lang="ts">
import { ref, onBeforeMount, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import StreamSelection from "./StreamSelection.vue";
import pipelineService from "@/services/pipelines";
import { useStore } from "vuex";
import { useQuasar, type QTableProps } from "quasar";
import NoData from "../shared/grid/NoData.vue";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

interface Pipeline {
  name: string;
  description: string;
  stream_type: string;
  stream_name: string;
}

const { t } = useI18n();
const router = useRouter();

const qTableRef: any = ref({});

const q = useQuasar();

const filterQuery = ref("");

const showCreatePipeline = ref(false);

const pipelines = ref([]);

const store = useStore();

const confirmDialogMeta: any = ref({
  show: false,
  title: "",
  message: "",
  data: null,
  onConfirm: () => {},
});

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
  qTableRef.value?.setPagination(pagination.value);
};

const currentRouteName = computed(() => {
  return router.currentRoute.value.name;
});

const editingPipeline = ref<Pipeline | null>(null);

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
    label: t("common.name"),
    align: "left",
    sortable: true,
  },
  {
    name: "description",
    field: "description",
    label: t("common.description"),
    align: "left",
    sortable: true,
  },
  {
    name: "stream_name",
    field: "stream_name",
    label: t("alerts.stream_name"),
    align: "left",
    sortable: true,
  },
  {
    name: "stream_type",
    field: "stream_type",
    label: t("alerts.streamType"),
    align: "left",
    sortable: true,
  },
  {
    name: "actions",
    field: "actions",
    label: t("alerts.actions"),
    align: "center",
    sortable: false,
  },
]);

onBeforeMount(() => {
  getPipelines();
});

const createPipeline = () => {
  showCreatePipeline.value = true;
};

const getPipelines = () => {
  pipelineService
    .getPipelines(store.state.selectedOrganization.identifier)
    .then((response) => {
      pipelines.value = response.data.list.map(
        (pipeline: any, index: number) => {
          return {
            ...pipeline,
            "#": index + 1,
          };
        }
      );
    })
    .catch((error) => {
      console.error(error);
    });
};

const editPipeline = (pipeline: Pipeline) => {
  router.push({
    name: "pipelineEditor",
    query: {
      name: pipeline.name,
      stream: pipeline.stream_name,
      stream_type: pipeline.stream_type,
      org_identifier: store.state.selectedOrganization.identifier,
    },
  });
};

const openDeleteDialog = (pipeline: Pipeline) => {
  confirmDialogMeta.value.show = true;
  confirmDialogMeta.value.title = t("pipeline.deletePipeline");
  confirmDialogMeta.value.message =
    "Are you sure you want to delete this pipeline?";
  confirmDialogMeta.value.onConfirm = deletePipeline;
  confirmDialogMeta.value.data = pipeline;
};

const savePipeline = (data: Pipeline) => {
  const dismiss = q.notify({
    message: "saving pipeline...",
    position: "bottom",
    spinner: true,
  });

  pipelineService
    .createPipeline({
      ...data,
      org_identifier: store.state.selectedOrganization.identifier,
    })
    .then(() => {
      getPipelines();
      showCreatePipeline.value = false;
      q.notify({
        message: "Pipeline created successfully",
        color: "positive",
        position: "bottom",
        timeout: 3000,
      });
    })
    .catch((error) => {
      q.notify({
        message: error.response?.data?.message || "Error while saving pipeline",
        color: "negative",
        position: "bottom",
        timeout: 3000,
      });
    })
    .finally(() => {
      dismiss();
    });
};

const deletePipeline = () => {
  const dismiss = q.notify({
    message: "deleting pipeline...",
    position: "bottom",
    spinner: true,
  });

  pipelineService
    .deletePipeline({
      ...confirmDialogMeta.value.data,
      org_identifier: store.state.selectedOrganization.identifier,
    })
    .then(() => {
      getPipelines();
      q.notify({
        message: "Pipeline deleted successfully",
        color: "positive",
        position: "bottom",
        timeout: 3000,
      });
    })
    .catch((error) => {
      q.notify({
        message: error.response?.data?.message || "Error while saving pipeline",
        color: "negative",
        position: "bottom",
        timeout: 3000,
      });
    })
    .finally(() => {
      dismiss();
    });

  resetConfirmDialog();
};

const resetConfirmDialog = () => {
  confirmDialogMeta.value.show = false;
  confirmDialogMeta.value.title = "";
  confirmDialogMeta.value.message = "";
  confirmDialogMeta.value.onConfirm = () => {};
  confirmDialogMeta.value.data = null;
};

const filterData = (rows: any, terms: any) => {
  var filtered = [];
  terms = terms.toLowerCase();
  for (var i = 0; i < rows.length; i++) {
    if (rows[i]["name"].toLowerCase().includes(terms)) {
      filtered.push(rows[i]);
    }
  }
  return filtered;
};
</script>
<style lang=""></style>
