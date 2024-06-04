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
  <div
    data-test="alert-list-page"
    class="q-pa-none flex"
    style="height: calc(100vh - 57px)"
  >
    <div v-if="!showAddAlertDialog" class="full-width">
      <q-table
        data-test="alert-list-table"
        ref="qTable"
        :rows="alertsRows"
        :columns="columns"
        row-key="id"
        :pagination="pagination"
        :filter="filterQuery"
        :filter-method="filterData"
        style="width: 100%"
      >
        <template #no-data>
          <div
            v-if="!templates.length || !destinations.length"
            class="full-width flex column justify-center items-center text-center"
          >
            <div style="width: 600px" class="q-mt-xl">
              <template v-if="!templates.length">
                <div
                  class="text-subtitle1"
                  data-test="alert-list-create-template-text"
                >
                  It looks like you haven't created any Templates yet. To create
                  an Alert, you'll need to have at least one Destination and one
                  Template in place
                </div>
                <q-btn
                  data-test="alert-list-create-template-btn"
                  class="q-mt-md"
                  label="Create Template"
                  size="md"
                  color="primary"
                  no-caps
                  style="border-radius: 4px"
                  @click="routeTo('alertTemplates')"
                />
              </template>
              <template v-if="!destinations.length && templates.length">
                <div
                  class="text-subtitle1"
                  data-test="alert-list-create-destination-text"
                >
                  It looks like you haven't created any Destinations yet. To
                  create an Alert, you'll need to have at least one Destination
                  and one Template in place
                </div>
                <q-btn
                  data-test="alert-list-create-destination-btn"
                  class="q-mt-md"
                  label="Create Destination"
                  size="md"
                  color="primary"
                  no-caps
                  style="border-radius: 4px"
                  @click="routeTo('alertDestinations')"
                />
              </template>
            </div>
          </div>
          <template v-else>
            <NoData />
          </template>
        </template>
        <template v-slot:body-cell-actions="props">
          <q-td :props="props">
            <div
              data-test="alert-list-loading-alert"
              v-if="alertStateLoadingMap[props.row.uuid]"
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
              @click="toggleAlertState(props.row)"
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
              @click="showAddUpdateFn(props)"
            ></q-btn>
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
              @click="showDeleteDialogFn(props)"
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
          <div class="q-table__title" data-test="alerts-list-title">
            {{ t("alerts.header") }}
          </div>
          <q-input
            data-test="alert-list-search-input"
            v-model="filterQuery"
            borderless
            filled
            dense
            class="q-ml-auto q-mb-xs no-border"
            :placeholder="t('alerts.search')"
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
            :disable="!destinations.length"
            :title="!destinations.length ? t('alerts.noDestinations') : ''"
            :label="t(`alerts.add`)"
            @click="showAddUpdateFn({})"
          />

          <QTablePagination
            :scope="scope"
            :pageTitle="t('alerts.header')"
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
    <template v-else>
      <AddAlert
        v-model="formData"
        :isUpdated="isUpdated"
        :destinations="destinations"
        @update:list="refreshList"
        @cancel:hideform="hideForm"
      />
    </template>
    <ConfirmDialog
      title="Delete Alert"
      message="Are you sure you want to delete alert?"
      @update:ok="deleteAlert"
      @update:cancel="confirmDelete = false"
      v-model="confirmDelete"
    />
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onBeforeMount,
  onActivated,
  watch,
  defineAsyncComponent,
} from "vue";
import type { Ref } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { QTable, useQuasar, type QTableProps } from "quasar";
import { useI18n } from "vue-i18n";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import alertsService from "@/services/alerts";
import destinationService from "@/services/alert_destination";
import templateService from "@/services/alert_templates";
import NoData from "@/components/shared/grid/NoData.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import segment from "@/services/segment_analytics";
import config from "@/aws-exports";
import {
  getImageURL,
  getUUID,
  verifyOrganizationStatus,
} from "@/utils/zincutils";
import type { Alert, AlertListItem } from "@/ts/interfaces/index";
import {
  outlinedDelete,
  outlinedPause,
  outlinedPlayArrow,
} from "@quasar/extras/material-icons-outlined";
// import alertList from "./alerts";

export default defineComponent({
  name: "AlertList",
  components: {
    QTablePagination,
    AddAlert: defineAsyncComponent(
      () => import("@/components/alerts/AddAlert.vue")
    ),
    NoData,
    ConfirmDialog,
  },
  emits: [
    "updated:fields",
    "update:changeRecordPerPage",
    "update:maxRecordToReturn",
  ],
  setup() {
    const store = useStore();
    const { t } = useI18n();
    const $q = useQuasar();
    const router = useRouter();
    const alerts: Ref<Alert[]> = ref([]);
    const alertsRows: Ref<AlertListItem[]> = ref([]);
    const formData: Ref<Alert | {}> = ref({});
    const showAddAlertDialog: any = ref(false);
    const qTable: Ref<InstanceType<typeof QTable> | null> = ref(null);
    const selectedDelete: any = ref(null);
    const isUpdated: any = ref(false);
    const confirmDelete = ref<boolean>(false);
    const splitterModel = ref(220);
    const alertStateLoadingMap: Ref<{ [key: string]: boolean }> = ref({});
    const folders = ref([
      {
        name: "folder1",
      },
      {
        name: "folder2",
      },
    ]);
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
        label: t("alerts.name"),
        align: "left",
        sortable: true,
      },
      {
        name: "alert_type",
        field: "alert_type",
        label: t("alerts.alertType"),
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
        name: "stream_name",
        field: "stream_name",
        label: t("alerts.stream_name"),
        align: "left",
        sortable: true,
      },
      {
        name: "conditions",
        field: "conditions",
        label: t("alerts.condition"),
        align: "left",
        sortable: false,
      },
      {
        name: "description",
        field: "description",
        label: t("alerts.description"),
        align: "center",
        sortable: false,
      },
      {
        name: "actions",
        field: "actions",
        label: t("alerts.actions"),
        align: "center",
        sortable: false,
      },
    ]);
    const activeTab: any = ref("alerts");
    const destinations = ref([0]);
    const templates = ref([0]);
    const getAlerts = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading alerts...",
      });
      alertsService
        .list(
          1,
          1000,
          "name",
          false,
          "",
          store.state.selectedOrganization.identifier
        )
        .then((res) => {
          var counter = 1;
          resultTotal.value = res.data.list.length;
          alerts.value = res.data.list.map((alert: any) => {
            return {
              ...alert,
              uuid: getUUID(),
            };
          });
          alertsRows.value = alerts.value.map((data: any) => {
            let conditions = "--";
            if (data.query_condition.conditions?.length) {
              conditions = data.query_condition.conditions
                .map((condition: any) => {
                  return `${condition.column} ${condition.operator} ${condition.value}`;
                })
                .join(" AND ");
            } else if (data.query_condition.sql) {
              conditions = data.query_condition.sql;
            } else if (data.query_condition.promql) {
              conditions = data.query_condition.promql;
            }
            if (conditions.length > 50) {
              conditions = conditions.substring(0, 32) + "...";
            }
            return {
              "#": counter <= 9 ? `0${counter++}` : counter++,
              name: data.name,
              alert_type: data.is_real_time ? "Real Time" : "Scheduled",
              stream_name: data.stream_name ? data.stream_name : "--",
              stream_type: data.stream_type,
              enabled: data.enabled,
              conditions: conditions,
              description: data.description,
              uuid: data.uuid,
            };
          });
          alertsRows.value.forEach((alert: AlertListItem) => {
            alertStateLoadingMap.value[alert.uuid as string] = false;
          });
          if (router.currentRoute.value.query.action == "add") {
            showAddUpdateFn({ row: undefined });
          }
          if (router.currentRoute.value.query.action == "update") {
            const alertName = router.currentRoute.value.query.name as string;
            showAddUpdateFn({
              row: getAlertByName(alertName),
            });
          }
          dismiss();
        })
        .catch((e) => {
          console.error(e);
          dismiss();
          $q.notify({
            type: "negative",
            message: "Error while pulling alerts.",
            timeout: 2000,
          });
        });
    };
    const getAlertByName = (name: string) => {
      return alerts.value.find((alert) => alert.name === name);
    };
    if (!alerts.value.length) {
      getAlerts();
    }
    onBeforeMount(async () => {
      await getTemplates();
      getDestinations();
    });
    onActivated(() => getDestinations());
    watch(
      () => router.currentRoute.value.query.action,
      (action) => {
        if (!action) showAddAlertDialog.value = false;
      }
    );
    const getDestinations = () => {
      destinationService
        .list({
          org_identifier: store.state.selectedOrganization.identifier,
        })
        .then((res) => {
          destinations.value = res.data;
        })
        .catch(() =>
          $q.notify({
            type: "negative",
            message: "Error while fetching destinations.",
            timeout: 3000,
          })
        );
    };

    const getTemplates = () => {
      templateService
        .list({
          org_identifier: store.state.selectedOrganization.identifier,
        })
        .then((res) => {
          templates.value = res.data;
        })
        .catch(() =>
          $q.notify({
            type: "negative",
            message: "Error while fetching templates.",
            timeout: 3000,
          })
        );
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
      qTable.value?.setPagination(pagination.value);
    };
    const changeMaxRecordToReturn = (val: any) => {
      maxRecordToReturn.value = val;
    };
    const addAlert = () => {
      showAddAlertDialog.value = true;
    };
    const showAddUpdateFn = (props: any) => {
      formData.value = alerts.value.find(
        (alert: any) => alert.uuid === props.row?.uuid
      ) as Alert;
      let action;
      if (!props.row) {
        isUpdated.value = false;
        action = "Add Alert";
        router.push({
          name: "alertList",
          query: {
            action: "add",
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      } else {
        isUpdated.value = true;
        action = "Update Alert";
        router.push({
          name: "alertList",
          query: {
            action: "update",
            name: props.row.name,
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      }
      addAlert();
      if (config.enableAnalytics == "true") {
        segment.track("Button Click", {
          button: action,
          user_org: store.state.selectedOrganization.identifier,
          user_id: store.state.userInfo.email,
          page: "Alerts",
        });
      }
    };
    const refreshList = () => {
      getAlerts();
      hideForm();
    };
    const hideForm = () => {
      showAddAlertDialog.value = false;
      router.push({
        name: "alertList",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };
    const deleteAlert = () => {
      alertsService
        .delete(
          store.state.selectedOrganization.identifier,
          selectedDelete.value.stream_name,
          selectedDelete.value.name,
          selectedDelete.value.stream_type
        )
        .then((res: any) => {
          if (res.data.code == 200) {
            $q.notify({
              type: "positive",
              message: res.data.message,
              timeout: 2000,
            });
            getAlerts();
          } else {
            $q.notify({
              type: "negative",
              message: res.data.message,
              timeout: 2000,
            });
          }
        })
        .catch((err) => {
          $q.notify({
            type: "negative",
            message: err?.data?.message || "Error while deleting alert.",
            timeout: 2000,
          });
        });
      if (config.enableAnalytics == "true") {
        segment.track("Button Click", {
          button: "Delete Alert",
          user_org: store.state.selectedOrganization.identifier,
          user_id: store.state.userInfo.email,
          alert_name: selectedDelete.value.name,
          page: "Alerts",
        });
      }
    };
    const showDeleteDialogFn = (props: any) => {
      selectedDelete.value = props.row;
      confirmDelete.value = true;
    };

    const toggleAlertState = (row: any) => {
      alertStateLoadingMap.value[row.uuid] = true;
      const alert: Alert = alerts.value.find(
        (alert) => alert.uuid === row.uuid
      ) as Alert;
      alertsService
        .toggleState(
          store.state.selectedOrganization.identifier,
          alert.stream_name,
          alert.name,
          !alert?.enabled,
          alert.stream_type
        )
        .then(() => {
          alert.enabled = !alert.enabled;
          alertsRows.value.forEach((alert) => {
            alert.uuid === row.uuid ? (alert.enabled = !alert.enabled) : null;
          });
        })
        .finally(() => {
          alertStateLoadingMap.value[row.uuid] = false;
        });
    };

    const routeTo = (name: string) => {
      router.push({
        name: name,
        query: {
          action: "add",
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    return {
      t,
      qTable,
      store,
      router,
      alerts,
      columns,
      formData,
      hideForm,
      confirmDelete,
      selectedDelete,
      getAlerts,
      pagination,
      resultTotal,
      refreshList,
      perPageOptions,
      selectedPerPage,
      addAlert,
      deleteAlert,
      isUpdated,
      showAddUpdateFn,
      showDeleteDialogFn,
      changePagination,
      maxRecordToReturn,
      showAddAlertDialog,
      changeMaxRecordToReturn,
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
      activeTab,
      destinations,
      verifyOrganizationStatus,
      folders,
      splitterModel,
      outlinedPause,
      outlinedPlayArrow,
      alertsRows,
      toggleAlertState,
      alertStateLoadingMap,
      templates,
      routeTo,
    };
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

.alerts-tabs {
  .q-tabs {
    &--vertical {
      margin: 1.5rem 1rem 0 0;

      .q-tab {
        justify-content: flex-start;
        padding: 0 1rem 0 1.25rem;
        border-radius: 0.5rem;
        margin-bottom: 0.5rem;

        &__content.tab_content {
          .q-tab {
            &__icon + &__label {
              padding-left: 0.875rem;
              font-weight: 600;
            }
          }
        }

        &--active {
          background-color: $accent;
        }
      }
    }
  }
}
</style>
