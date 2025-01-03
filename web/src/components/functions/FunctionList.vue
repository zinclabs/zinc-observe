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
        :rows="jsTransforms"
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
              icon="edit"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('function.updateTitle')"
              @click="showAddUpdateFn(props)"
            ></q-btn>
            <q-btn
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
              :icon="outlinedAccountTree"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="'Associated Pipelines'"
              @click="getAssociatedPipelines(props)"
            ></q-btn>
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
            {{ t("function.header") }}
          </div>
          <div class="q-ml-auto" data-test="functions-list-search-input">
            <q-input
              v-model="filterQuery"
              borderless
              filled
              dense
              class="q-ml-auto q-mb-xs no-border"
              :placeholder="t('function.search')"
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
            :label="t(`function.add`)"
            @click="showAddUpdateFn({})"
          />

          <QTablePagination
            :scope="scope"
            :pageTitle="t('function.header')"
            :position="'top'"
            :resultTotal="resultTotal"
            :perPageOptions="perPageOptions"
            @update:changeRecordPerPage="changePagination"
          />
        </template>

        <template #bottom="scope">
          <QTablePagination
            :scope="scope"
            :position="'bottom'"
            :resultTotal="resultTotal"
            :perPageOptions="perPageOptions"
            @update:changeRecordPerPage="changePagination"
          />
        </template>
      </q-table>
    </div>
    <div v-else>
      <AddFunction
        v-model="formData"
        :isUpdated="isUpdated"
        class="q-pa-sm"
        @update:list="refreshList"
        @cancel:hideform="hideForm"
      />
    </div>
    <ConfirmDialog
      title="Delete Transform"
      message="Are you sure you want to delete transform?"
      @update:ok="deleteFn"
      @update:cancel="confirmDelete = false"
      v-model="confirmDelete"
    />
    <q-dialog v-model="confirmForceDelete" persistent>
      <q-card style="width: 40vw; max-height: 90vh; overflow-y: auto">
        <q-card-section
          class="text-h6 dialog-heading tw-flex tw-justify-between tw-items-center"
        >
          <div>
            Pipelines Associated with
            <strong> {{ selectedDelete.name }}</strong>
          </div>
          <q-icon
            name="close"
            size="18px"
            @click="closeDialog"
            style="cursor: pointer"
          />
        </q-card-section>
        <q-card-section>
          <div
            v-if="transformedPipelineList.length > 0"
            class="pipeline-list-container"
          >
            <q-list class="scrollable-list">
              <q-item
                v-for="(pipeline, index) in transformedPipelineList"
                :key="pipeline.value"
                clickable
                @click="onPipelineSelect(pipeline)"
              >
                <q-item-section>
                  {{ index + 1 }}. {{ pipeline.label }}
                </q-item-section>
              </q-item>
            </q-list>
          </div>
          <div v-else>
            <div class="text-h6 text-center">
              No pipelines associated with this function
            </div>
          </div>
        </q-card-section>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<script lang="ts">
import {
  defineAsyncComponent,
  defineComponent,
  ref,
  computed,
  watch,
  onMounted,
} from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar, type QTableProps } from "quasar";
import { useI18n } from "vue-i18n";

import QTablePagination from "../shared/grid/Pagination.vue";
import jsTransformService from "../../services/jstransform";
import NoData from "../shared/grid/NoData.vue";
import ConfirmDialog from "../ConfirmDialog.vue";
import segment from "../../services/segment_analytics";
import { getImageURL, verifyOrganizationStatus } from "../../utils/zincutils";
import {
  outlinedDelete,
  outlinedAccountTree,
} from "@quasar/extras/material-icons-outlined";
import useLogs from "@/composables/useLogs";

export default defineComponent({
  name: "functionList",
  components: {
    QTablePagination,
    AddFunction: defineAsyncComponent(() => import("./AddFunction.vue")),
    NoData,
    ConfirmDialog,
  },
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
    const formData: any = ref({});
    const showAddJSTransformDialog: any = ref(false);
    const qTable: any = ref(null);
    const selectedDelete: any = ref(null);
    const isUpdated: any = ref(false);
    const confirmDelete = ref<boolean>(false);
    const confirmForceDelete = ref<boolean>(false);
    const { searchObj } = useLogs();
    const pipelineList = ref([
      "Pipeline Alpha",
      "Pipeline Beta",
      "Pipeline Gamma",
      "Pipeline Delta",
      "Pipeline Epsilon",
      "Pipeline Zeta",
      "Pipeline Eta",
      "Pipeline Theta",
      "Pipeline Iota",
      "Pipeline Kappa",
      "Pipeline Lambda",
      "Pipeline Mu",
      "Pipeline Nu",
      "Pipeline Xi",
      "Pipeline Omicron",
      "Pipeline Pi",
      "Pipeline Rho",
      "Pipeline Sigma",
      "Pipeline Tau",
      "Pipeline Upsilon",
      "Pipeline Phi",
      "Pipeline Chi",
      "Pipeline Psi",
      "Pipeline Omega",
      "Pipeline One",
      "Pipeline Two",
      "Pipeline Three",
      "Pipeline Four",
      "Pipeline Five",
      "Pipeline Six",
      "Pipeline Seven",
      "Pipeline Eight",
      "Pipeline Nine",
      "Pipeline Ten",
      "Pipeline Eleven",
      "Pipeline Twelve",
      "Pipeline Thirteen",
      "Pipeline Fourteen",
      "Pipeline Fifteen",
      "Pipeline Sixteen",
      "Pipeline Seventeen",
      "Pipeline Eighteen",
      "Pipeline Nineteen",
      "Pipeline Twenty",
      "Pipeline Twenty-One",
      "Pipeline Twenty-Two",
      "Pipeline Twenty-Three",
      "Pipeline Twenty-Four",
      "Pipeline Twenty-Five",
      "Pipeline Twenty-Six",
      "Pipeline Twenty-Seven",
      "Pipeline Twenty-Eight",
      "Pipeline Twenty-Nine",
      "Pipeline Thirty",
    ]);
    const selectedPipeline = ref("");
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

    const onPipelineSelect = (pipeline: any) => {
      const routeUrl = router.resolve({
        name: "pipelineEditor",
        query: {
          id: pipeline.value,
          name: pipeline.label,
        },
      }).href;

      window.open(routeUrl, "_blank");
    };

    const getJSTransforms = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading functions...",
      });

      jsTransformService
        .list(
          1,
          100000,
          "name",
          false,
          "",
          store.state.selectedOrganization.identifier,
        )
        .then((res) => {
          var counter = 1;
          resultTotal.value = res.data.list.length;
          if (router.currentRoute.value.query.action == "add") {
            showAddUpdateFn({ row: undefined });
          }
          jsTransforms.value = res.data.list.map((data: any) => {
            if (router.currentRoute.value.query.action == "update") {
              if (router.currentRoute.value.query.name == data.name) {
                showAddUpdateFn({ row: data });
              }
            }

            return {
              "#": counter <= 9 ? `0${counter++}` : counter++,
              name: data.name,
              function: data.function,
              params: data.params,
              // order: data.order ? data.order : 1,
              // stream_name: data.stream_name ? data.stream_name : "--",
              // stream_type: data.stream_type ? data.stream_type : "--",
              transType: data.transType.toString(),
              // ingest: data.stream_name ? true : false,
              actions: "",
            };
          });

          searchObj.data.transforms = jsTransforms.value;

          dismiss();
        })
        .catch((err) => {
          console.log("--", err);

          dismiss();
          if (err.response.status != 403) {
            $q.notify({
              type: "negative",
              message: "Error while pulling function.",
              timeout: 2000,
            });
          }
        });
    };

    if (jsTransforms.value == "" || jsTransforms.value == undefined) {
      getJSTransforms();
    }

    interface OptionType {
      label: String;
      value: number | String;
    }
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
    const changeMaxRecordToReturn = (val: any) => {
      maxRecordToReturn.value = val;
    };

    const addTransform = () => {
      showAddJSTransformDialog.value = true;
    };

    const transformedPipelineList = computed(() => {
      return pipelineList.value.map((pipeline: any) => ({
        label: pipeline.name,
        value: pipeline.id,
      }));
    });

    const showAddUpdateFn = (props: any) => {
      formData.value = props.row;
      let action;
      if (!props.row) {
        isUpdated.value = false;
        action = "Add Function";
        router.push({
          name: "functionList",
          query: {
            action: "add",
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      } else {
        isUpdated.value = true;
        action = "Update Function";
        router.push({
          name: "functionList",
          query: {
            action: "update",
            name: props.row.name,
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      }
      addTransform();

      segment.track("Button Click", {
        button: action,
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        page: "Functions",
      });
    };

    const refreshList = () => {
      router.push({
        name: "functionList",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
      showAddJSTransformDialog.value = false;
      getJSTransforms();
    };

    const hideForm = () => {
      showAddJSTransformDialog.value = false;
      router.replace({
        name: "functionList",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    const deleteFn = () => {
      jsTransformService
        .delete(
          store.state.selectedOrganization.identifier,
          selectedDelete.value.name,
        )
        .then((res: any) => {
          if (res.data.code == 200) {
            $q.notify({
              type: "positive",
              message: res.data.message,
              timeout: 2000,
            });
            getJSTransforms();
          } else {
            $q.notify({
              type: "negative",
              message: res.data.message,
              timeout: 2000,
            });
          }
        })
        .catch((err) => {
          if (err.response.data.code == 409) {
            $q.notify({
              type: "negative",
              message:
                "Function deletion failed as it is associated with pipelines. Click on view button to get associated pipelines.",
              timeout: 10000,
              actions: [
                {
                  label: "View",
                  color: "white",
                  handler: () => {
                    forceRemoveFunction(err.response.data["message"]);
                  },
                },
              ],
            });
            return;
          }
          if (err.response.status != 403) {
            $q.notify({
              type: "negative",
              message:
                JSON.stringify(err.response.data["message"]) ||
                "Function deletion failed.",
            });
          }
        });

      segment.track("Button Click", {
        button: "Delete Function",
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        function_name: selectedDelete.value.name,
        is_ingest_func: selectedDelete.value.ingest,
        page: "Functions",
      });
    };

    const showDeleteDialogFn = (props: any) => {
      selectedDelete.value = props.row;
      confirmDelete.value = true;
    };

    const getAssociatedPipelines = (props: any) => {
      selectedDelete.value = props.row;
      jsTransformService
        .getAssociatedPipelines(
          store.state.selectedOrganization.identifier,
          props.row.name,
        )
        .then((res: any) => {
          pipelineList.value = res.data.list;
          confirmForceDelete.value = true;
        })
        .catch((err) => {
          console.log(err);
        });
    };

    const forceRemoveFunction = (message: any) => {
      const match = message.match(/\[([^\]]+)\]/);
      if (match) {
        // Convert the matched string to an array of pipeline names
        pipelineList.value = JSON.parse(match[0].replace(/'/g, '"'));
      }

      confirmForceDelete.value = true;
    };

    const closeDialog = () => {
      confirmForceDelete.value = false;
    };

    const forceDeleteFn = () => {};

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
      getJSTransforms,
      pagination,
      resultTotal,
      refreshList,
      perPageOptions,
      selectedPerPage,
      addTransform,
      deleteFn,
      isUpdated,
      showAddUpdateFn,
      showDeleteDialogFn,
      changePagination,
      maxRecordToReturn,
      showAddJSTransformDialog,
      changeMaxRecordToReturn,
      outlinedDelete,
      outlinedAccountTree,
      forceDeleteFn,
      confirmForceDelete,
      pipelineList,
      selectedPipeline,
      closeDialog,
      onPipelineSelect,
      transformedPipelineList,
      getAssociatedPipelines,
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
    };
  },
  computed: {
    // selectedOrg() {
    //   return this.store.state.selectedOrganization.identifier;
    // },
  },
  watch: {
    // selectedOrg(newVal: any, oldVal: any) {
    //   this.verifyOrganizationStatus(
    //     this.store.state.organizations,
    //     this.router
    //   );
    //   if (
    //     (newVal != oldVal || this.jsTransforms.value == undefined) &&
    //     this.router.currentRoute.value.name == "AppFunctions"
    //   ) {
    //     this.resultTotal = 0;
    //     this.jsTransforms = [];
    //     this.getJSTransforms();
    //   }
    // },
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
.pipeline-list-container {
  max-height: 200px; /* Adjust based on item height to fit 5 items */
  overflow-y: auto;
}
.dialog-heading {
  border-bottom: 1px solid $border-color;
}

.scrollable-list::-webkit-scrollbar {
  width: 8px;
}

.scrollable-list::-webkit-scrollbar-thumb {
  background-color: #888; /* Desired thumb color */
  border-radius: 4px;
}

.scrollable-list::-webkit-scrollbar-thumb:hover {
  background-color: blue; /* Darker shade on hover */
}

.scrollable-list::-webkit-scrollbar-track {
  background-color: blue; /* Track color */
}
</style>
