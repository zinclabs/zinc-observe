<template>
  <div v-if="currentRouteName === 'synthetics'">
    <div class="full-wdith">
      <q-table
        data-test="alert-list-table"
        ref="qTable"
        :rows="tests"
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
            <div
              data-test="alert-list-loading-alert"
              v-if="testStatusLoadingMap[props.row.uuid]"
              style="display: inline-block; width: 33.14px; height: auto"
              class="flex justify-center items-center q-ml-xs"
              :title="`Turning ${props.row.enabled ? 'Off' : 'On'}`"
            >
              <q-circular-progress
                indeterminate
                rounded
                size="16px"
                :value="1"
                color="secondary"
              />
            </div>
            <q-btn
              v-else
              :data-test="`alert-list-${props.row.name}-pause-start-alert`"
              :icon="props.row.enabled ? outlinedPause : outlinedPlayArrow"
              class="q-ml-xs material-symbols-outlined"
              padding="sm"
              unelevated
              size="sm"
              :color="props.row.enabled ? 'negative' : 'positive'"
              round
              flat
              :title="props.row.enabled ? t('alerts.pause') : t('alerts.start')"
              @click="toggleTestStatus(props.row)"
            />
            <q-btn
              :data-test="`alert-list-${props.row.name}-udpate-alert`"
              icon="search"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('alerts.edit')"
              @click="editPipeline(props.row)"
            />
            <q-btn
              :data-test="`alert-list-${props.row.name}-udpate-alert`"
              icon="edit"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('alerts.edit')"
              @click="editPipeline(props.row)"
            />
            <q-btn
              :data-test="`alert-list-${props.row.name}-delete-alert`"
              :icon="outlinedDelete"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('alerts.delete')"
              @click="openDeleteDialog(props.row)"
            />
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
          <div class="q-table__title" data-test="alerts-list-title">
            {{ t("synthetics.header") }}
          </div>
          <q-input
            data-test="alert-list-search-input"
            v-model="filterQuery"
            borderless
            filled
            dense
            class="q-ml-auto q-mb-xs no-border"
            :placeholder="t('common.search')"
          >
            <template #prepend>
              <q-icon name="search" class="cursor-pointer" />
            </template>
          </q-input>
          <q-btn
            data-test="alert-list-add-alert-btn"
            class="q-ml-md q-mb-xs text-bold no-border"
            padding="sm lg"
            color="secondary"
            no-caps
            :label="t(`logStream.explore`)"
            icon="search"
            @click="createPipeline"
          />
          <q-btn
            data-test="alert-list-add-alert-btn"
            class="q-ml-md q-mb-xs text-bold no-border"
            padding="sm lg"
            color="secondary"
            no-caps
            :label="t(`synthetics.addTest`)"
            @click="createPipeline"
          />

          <q-table-pagination
            :scope="scope"
            :pageTitle="t('synthetics.tests')"
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
    <create-test @save="savePipeline" />
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
import { ref, onBeforeMount, computed, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import CreateTest from "./CreateTest.vue";
import syntheticService from "@/services/synthetics";
import { useStore } from "vuex";
import { useQuasar, type QTableProps } from "quasar";
import NoData from "../shared/grid/NoData.vue";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import {
  outlinedPause,
  outlinedPlayArrow,
} from "@quasar/extras/material-icons-outlined";
import syntheticsService from "@/services/synthetics";

interface Test {
  name: string;
  type: string;
  domain: string;
  state: boolean;
}

const { t } = useI18n();
const router = useRouter();

// const qTable: any = ref(null);

const q = useQuasar();

const filterQuery = ref("");

const showCreatePipeline = ref(false);

const testStatusLoadingMap: Ref<{ [key: string]: boolean }> = ref({});

const tests = ref([
  {
    "#": 1,
    name: "test1",
    type: "type1",
    domain: "domain1",
  },
  {
    "#": 2,
    name: "test2",
    type: "type2",
    domain: "domain2",
  },
]);

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
  // qTable.value?.setPagination(pagination.value);
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
    name: "type",
    field: "type",
    label: t("common.type"),
    align: "left",
    sortable: true,
  },
  {
    name: "domain",
    field: "domain",
    label: t("synthetics.domain"),
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
  gettests();
});

const createPipeline = () => {
  showCreatePipeline.value = true;
};

const gettests = () => {
  syntheticsService
    .list(store.state.selectedOrganization.identifier)
    .then((response) => {
      tests.value = response.data.list.map((test: any, index: number) => {
        return {
          ...test,
          "#": index + 1,
        };
      });
    })
    .catch((error) => {
      console.error(error);
    });
};

const editPipeline = (test: Test) => {
  router.push({
    name: "editSyntheticsTest",
    query: {
      name: test.name,
      org_identifier: store.state.selectedOrganization.identifier,
    },
  });
};

const deleteTest = (test: Test) => {
  syntheticsService
    .delete(store.state.selectedOrganization.identifier, test.name)
    .then(() => {
      gettests();
    })
    .catch((error) => {
      console.error(error);
    });
};

const exploreTest = (test: Test) => {
  router.push({
    name: "exploreSyntheticsTest",
    query: {
      name: test.name,
      org_identifier: store.state.selectedOrganization.identifier,
    },
  });
};

const toggleTestStatus = (test: Test) => {
  testStatusLoadingMap.value[test.name] = true;
  syntheticsService
    .toggleTestState(
      store.state.selectedOrganization.identifier,
      test.name,
      !test.state
    )
    .then(() => {
      test.state = !test.state;
    })
    .catch((error) => {
      console.error(error);
    })
    .finally(() => {
      testStatusLoadingMap.value[test.name] = false;
    });
};

const openDeleteDialog = (pipeline: Test) => {
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

  syntheticService
    .create(store.state.selectedOrganization.identifier, data)
    .then(() => {
      gettests();
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

  syntheticService
    .delete(
      store.state.selectedOrganization.identifier,
      confirmDialogMeta.value.data.name
    )
    .then(() => {
      gettests();
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
