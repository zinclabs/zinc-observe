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
    <div
      :class="store.state.theme === 'dark' ? 'dark-mode dark-theme' : 'light-theme light-mode'"
     class="full-wdith pipeline-list-table">
      <q-table
        data-test="pipeline-list-table"
        ref="qTableRef"
        :rows="filteredPipelines"
        :columns="columns"
        row-key="name"
        :pagination="pagination"
        :filter="filterQuery"
        :filter-method="filterData"
        style="width: 100%"
      >
      <template v-slot:body="props">
        <q-tr
          :data-test="`pipeline-list-table-${props.row.pipeline_id}-row`"
          :props="props"
          style="cursor: pointer"
          @click="triggerExpand(props)"
        >
        <q-tooltip position="bottom">
                <PipelineView :pipeline="props.row" />
              </q-tooltip>
          <q-td v-if="activeTab == 'scheduled' "  >
            
            <q-btn
              dense
              flat
              size="xs"
              :icon="
                expandedRow != props.row.pipeline_id
                  ? 'expand_more'
                  : 'expand_less'
              "
            />
          </q-td>
          <q-td v-for="col in filterColumns()" :key="col.name " :props="props">

          <template v-if="col.name  !== 'actions'">
            {{ props.row[col.field] }}
          </template>
          <template v-else>
            <!-- Actions Buttons -->
            <q-btn
              :data-test="`pipeline-list-${props.row.name}-pause-start-alert`"
              :icon="props.row.enabled ? outlinedPause : outlinedPlayArrow"
              class="q-ml-xs material-symbols-outlined"
              padding="sm"
              unelevated
              size="sm"
              :color="props.row.enabled ? 'negative' : 'positive'"
              round
              flat
              :title="props.row.enabled ? t('alerts.pause') : t('alerts.start')"
              @click.stop="toggleAlertState(props.row)"
            />
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
              @click.stop="editPipeline(props.row)"
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
              @click.stop="openDeleteDialog(props.row)"
            ></q-btn>
          </template>
        </q-td>

      
        </q-tr>
        <q-tr data-test="scheduled-pipeline-row-expand" v-show="expandedRow === props.row.pipeline_id" :props="props" >
         
          <q-td v-if="props.row?.sql_query"  colspan="100%">

            <div data-test="scheduled-pipeline-expanded-content" class="text-left tw-px-2 q-mb-sm  expanded-content">
            <div class="tw-flex tw-items-center q-py-sm  ">
              <strong >SQL Query : <span></span></strong>
            </div>
              <div class="tw-flex tw-items-start  tw-justify-center" >
            
              <div data-test="scheduled-pipeline-expanded-sql" class="scrollable-content  expanded-sql ">
                <pre style="text-wrap: wrap;">{{ props.row?.sql_query  }} </pre>

              </div>
              
              
              </div>
            </div>

          </q-td>
        </q-tr>
      </template>
        <template #no-data>
          <no-data />
        </template>
 
        <template v-slot:body-cell-function="props">
          <q-td :props="props">
            <q-tooltip>
              <pre data-test="scheduled-pipeline-expanded-tooltip-sql">{{ props.row.sql }}</pre>
            </q-tooltip>
            <pre style="white-space: break-spaces">{{ props.row.sql }}</pre>
          </q-td>
        </template>
        <template #top="scope">
          <div class="q-table__title" data-test="pipeline-list-title">
            {{ t("pipeline.header") }}
            
          </div>
          <div class="tw-flex tw-items-center report-list-tabs q-ml-auto">

          <app-tabs
                data-test="pipeline-list-tabs"
                class="q-mr-md "
                :tabs="tabs"
                v-model:active-tab="activeTab"
                @update:active-tab="updateActiveTab"
              />
          <q-input
            data-test="pipeline-list-search-input"
            v-model="filterQuery"
            borderless
            filled
            dense
            class=" q-mb-xs no-border"
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
            :to="{ name: 'createPipeline' }"
          />
        </div>

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
import { ref, onBeforeMount, computed, watch, reactive, onActivated, onMounted } from "vue";
import {MarkerType} from "@vue-flow/core";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import StreamSelection from "./StreamSelection.vue";
import pipelineService from "@/services/pipelines";
import { useStore } from "vuex";
import { useQuasar, type QTableProps  } from "quasar";
import type { QTableColumn } from 'quasar';

import NoData from "../shared/grid/NoData.vue";
import { outlinedDelete , outlinedPause , outlinedPlayArrow } from "@quasar/extras/material-icons-outlined";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import useDragAndDrop from "@/plugins/pipelines/useDnD";
import AppTabs from "@/components/common/AppTabs.vue";
import PipelineView from "./PipelineView.vue";

import { filter, update } from "lodash-es";

interface Column {
  name: string;
  field: string;
  label: string;
  align: string;
  sortable?: boolean;
}



const { t } = useI18n();
const router = useRouter();

const qTableRef: any = ref({});

const q = useQuasar();

const filterQuery = ref("");

const showCreatePipeline = ref(false);

const  expandedRow : any = ref( []); // Array to track expanded rows

const pipelines = ref([]);

const store = useStore();
const isEnabled = ref(false);

const { pipelineObj } = useDragAndDrop();

watch(
  () => router.currentRoute.value,
  async () => {
    await getPipelines();
    updateActiveTab();
  },
);

const confirmDialogMeta: any = ref({
  show: false,
  title: "",
  message: "",
  data: null,
  onConfirm: () => {},
});
const activeTab = ref("all");
const filteredPipelines : any = ref([]);
const columns = ref<Column[]>([]);

const tabs = reactive ([
{
    label: "All",
    value: "all",
  },
  {
    label: "Realtime",
    value: "realtime",
  },
  {
    label: "Scheduled",
    value: "scheduled",
  },
]);

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

const filterColumns   = () : Column[] => {
  if(activeTab.value === "realtime" || activeTab.value === "all"){
    return columns.value;
  }

  return columns.value.slice(1);
};

const updateActiveTab = () => {
  if (activeTab.value === "all") {
    filteredPipelines.value = pipelines.value.map((pipeline : any, index : any) => ({
      ...pipeline,
      "#": index + 1,
    }));
    filteredPipelines.value = pipelines.value;
    resultTotal.value = pipelines.value.length;
    columns.value = getColumnsForActiveTab(activeTab.value);
    return;
  }




  filteredPipelines.value = pipelines.value
    .filter((pipeline : any) => pipeline.source.source_type === activeTab.value)
    .map((pipeline : any, index) => ({
      ...pipeline,
      "#": index + 1,
    }));

  resultTotal.value = filteredPipelines.value.length;
  columns.value = getColumnsForActiveTab(activeTab.value);

};


const toggleAlertState = (row : any) =>{
  row.enabled = !row.enabled;
  pipelineService.toggleState(store.state.selectedOrganization.identifier,row.pipeline_id,row.enabled).then((response) => {
    const message = row.enabled 
    ? `${row.name} state resumed successfully` 
    : `${row.name} state paused successfully`;
    q.notify({
      message: message,
      color: "positive",
      position: "bottom",
      timeout: 3000,
    });
  }).catch((error) => {
    q.notify({
      message: error.response?.data?.message || "Error while updating pipeline state",
      color: "negative",
      position: "bottom",
      timeout: 3000,
    });
  });
}

const triggerExpand = (props : any) =>{
  if (expandedRow.value === props.row.pipeline_id) {
      expandedRow.value = null;
    } else {
      // Otherwise, expand the clicked row and collapse any other row
      expandedRow.value = props.row.pipeline_id;
  }
}

const editingPipeline = ref<any | null>(null);


// const updateActiveTab = (tab: string) => {
//   isRealTime.value = tab === "realTime";
// };


const getColumnsForActiveTab = (tab : any) => {
  let realTimeColumns = [
  { name: "#", label: "#", field: "#", align: "left" },

  { name: "name", field: "name", label: t("common.name"), align: "left", sortable: true },
    { name: "stream_name", field: "stream_name", label: t("alerts.stream_name"), align: "left", sortable: true },
    { name: "stream_type", field: "stream_type", label: t("alerts.streamType"), align: "left", sortable: true },
  ];

  let scheduledColumns = [
    { name: "#", label: "#", field: "#", align: "left" },

    { name: "name", field: "name", label: t("common.name"), align: "left", sortable: true },
    { name: "stream_type", field: "stream_type", label: "Stream Type", align: "left", sortable: true },
    { name: "frequency", field: "frequency", label: "Frequency", align: "left", sortable: true },
    { name: "period", field: "period", label: "Period", align: "left", sortable: true },
    { name: "silence", field: "silence", label: "Silence", align: "left", sortable: true },
    { name: "cron", field: "cron", label: "Cron", align: "left", sortable: false },
    { name: "sql_query", field: "sql_query", label: "SQL Query", align: "left", sortable: false
      ,
      style: "white-space: no-wrap; max-width: 200px; overflow: hidden; text-overflow: ellipsis;"

     },
  ];

  const actionsColumn = {
    name: "actions",
    field: "actions",
    label: t("alerts.actions"),
    align: "center",
    sortable: false,
  };
if(tab === "all"){

  
  const allColumns = [ ...scheduledColumns, actionsColumn];
  allColumns.splice(2,0 , { name: "type", field: "type", label: 'Type', align: "left", sortable: true });

  allColumns.splice(3,0 , { name: "stream_name", field: "stream_name", label: t("alerts.stream_name"), align: "left", sortable: true });

  return allColumns;
}
  return tab === "realtime"
    ? [ ...realTimeColumns, actionsColumn]
    : [ ...scheduledColumns, actionsColumn];
};


onMounted(async () => {
      await getPipelines(); // Ensure pipelines are fetched before updating
      updateActiveTab();
    });

const createPipeline = () => {
  showCreatePipeline.value = true;
};

const getPipelines = async () => {
      try {
        const response = await pipelineService.getPipelines(store.state.selectedOrganization.identifier);
        pipelines.value = [];
        // resultTotal.value = response.data.list.length;
        pipelines.value = response.data.list.map((pipeline : any, index : any) => {
          const updatedEdges = pipeline.edges.map((edge : any) => ({
            ...edge,
            markerEnd: {
                type: MarkerType.ArrowClosed,
                width: 20,  // Increase arrow width
                height: 20, // Increase arrow height
              },
              type: 'button',
              
              style:{
                strokeWidth: 2,
              },
              animated: true,
            updatable: true 
          }));
          pipeline.type = pipeline.source.source_type;
          if (pipeline.source.source_type === 'realtime') {
            pipeline.stream_name = pipeline.source.stream_name;
            pipeline.stream_type = pipeline.source.stream_type;
          } else {
            pipeline.stream_type = pipeline.source.stream_type;
            pipeline.frequency = pipeline.source.trigger_condition.frequency + " Mins";
            pipeline.period = pipeline.source.trigger_condition.period + " Mins";
            pipeline.silence = pipeline.source.trigger_condition.silence + " Mins";
            pipeline.cron = pipeline.cron && pipeline.cron !== "" ? pipeline.source.trigger_condition.cron : 'False';
            pipeline.sql_query = pipeline.source.query_condition.sql;
          }

          pipeline.edges = updatedEdges;
          return {
            ...pipeline,
            "#": index + 1,
          };
        });
      } catch (error) {
        console.error(error);
      }
    };
const editPipeline = (pipeline: any) => {
  pipeline.nodes.forEach((node : any) => {
    node.type = node.io_type;
  });

  pipelineObj.currentSelectedPipeline = pipeline;
  pipelineObj.pipelineWithoutChange = pipeline;
  router.push({
    name: "pipelineEditor",
    query: {
      id: pipeline.pipeline_id,
      name: pipeline.name,
      stream: pipeline.stream_name,
      stream_type: pipeline.stream_type,
      org_identifier: store.state.selectedOrganization.identifier,
    },
  });
};

const openDeleteDialog = (pipeline: any) => {
  confirmDialogMeta.value.show = true;
  confirmDialogMeta.value.title = t("pipeline.deletePipeline");
  confirmDialogMeta.value.message =
    "Are you sure you want to delete this pipeline?";
  confirmDialogMeta.value.onConfirm = deletePipeline;
  confirmDialogMeta.value.data = pipeline;
};

const savePipeline = (data: any) => {
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

const deletePipeline = async () => {
  const dismiss = q.notify({
    message: "deleting pipeline...",
    position: "bottom",
    spinner: true,
  });
  const { pipeline_id} = confirmDialogMeta.value.data;
  const org_id = store.state.selectedOrganization.identifier;
  pipelineService
    .deletePipeline({
      pipeline_id,
      org_id
    })
    .then(async () => {
      await getPipelines();
      updateActiveTab();
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
      getPipelines();
      updateActiveTab(); 
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
<style lang="scss" scoped>
.pipeline-list-table {
  th:last-child,
  td:last-child {
    position: sticky;
    right: 0;
    z-index: 1;
    box-shadow: -4px 0px 4px 0 rgba(0, 0, 0, 0.1);
    width: 100px;
    
  }
}

.dark-theme {
  th:last-child,
  td:last-child {
    background: var(--q-dark);
    box-shadow: -4px 0px 4px 0 rgba(144, 144, 144, 0.1);
    width: 100px;


  }
}

.light-theme {

  th:last-child,
  td:last-child {
    background: #ffffff;
    width: 100px;

  }
}
.dark-mode {
  background-color: $dark-page;

  .report-list-tabs {
    height: fit-content;

    :deep(.rum-tabs) {
      border: 1px solid #464646;
    }

    :deep(.rum-tab) {
      &:hover {
        background: #464646;
      }

      &.active {
        background: #5960b2;
        color: #ffffff !important;
      }
    }
  }
}
  .report-list-tabs {
  height: fit-content;

  :deep(.rum-tabs) {
    border: 1px solid #eaeaea;
    height: fit-content;
    border-radius: 4px;
    overflow: hidden;
  }

  :deep(.rum-tab) {
    width: fit-content !important;
    padding: 4px 12px !important;
    border: none !important;

    &:hover {
      background: #eaeaea;
    }

    &.active {
      background: #5960b2;
      color: #ffffff !important;
    }
  }
}

.expanded-content {
  padding: 0  3rem;
  max-height: 100vh; /* Set a fixed height for the container */
  overflow: hidden; /* Hide overflow by default */
}

.scrollable-content {
  width: 100%; /* Use the full width of the parent */
  overflow-y: auto; /* Enable vertical scrolling for long content */
  padding: 10px; /* Optional: padding for aesthetics */
  border: 1px solid #ddd; /* Optional: border for visibility */
  height: 100%;
  max-height: 200px;
   /* Use the full height of the parent */
  text-wrap: normal;
  background-color: #e8e8e8;
  color: black;
}
.expanded-sql{
  border-left: #7A54A2 3px solid;
}


:deep(.pipeline-list-table thead th:last-child) {
  position: sticky;
  right: 0;
    z-index: 1;
    box-shadow: -4px 0px 4px 0 rgba(0, 0, 0, 0.1);
    width: 100px;
}

:deep(.dark-theme.pipeline-list-table thead th:last-child) {
  background: var(--q-dark);
}
:deep(.light-theme.pipeline-list-table thead th:last-child) {
  background: #ffffff;
}




</style>
