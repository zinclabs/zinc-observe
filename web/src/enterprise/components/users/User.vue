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
  <q-page class="q-pa-none" style="min-height: inherit">
    <q-table
      ref="qTable"
      :rows="orgMembers"
      :columns="columns"
      row-key="id"
      :pagination="pagination"
      :filter="filterQuery"
      :filter-method="filterData"
    >
      <template #no-data>
        <NoData></NoData>
      </template>
      <template #body-cell-role="props">
        <q-td
          :props="props"
          v-if="currentUserRole == 'admin' && !props.row.isLoggedinUser"
        >
          <q-select
            dense
            borderless
            v-model="props.row.role"
            :options="options"
            style="width: 70px"
            @update:model-value="updateUserRole(props.row)"
          />
        </q-td>
        <q-td :props="props" v-else>
          {{ props.row.role }}
        </q-td>
      </template>
      <template #top="scope">
        <div
          class="q-table__title full-width q-mb-md"
          data-test="user-title-text"
        >
          {{ t("user.header") }}
        </div>

        <div class="full-width row q-mb-xs items-start">
          <q-input
            v-model="filterQuery"
            filled
            dense
            class="col-4 q-pr-sm"
            :placeholder="t('user.search')"
          >
            <template #prepend>
              <q-icon name="search" />
            </template>
          </q-input>
          <div class="col-8 flex" v-if="currentUserRole == 'admin'">
            <q-separator vertical class="separator q-mr-sm" />

            <div
              class="row invite-user"
              style="width: calc(100% - 110px); display: inline-flex"
            >
              <q-input
                v-model="userEmail"
                borderless
                filled
                dense
                :placeholder="t('user.inviteByEmail')"
                style="width: calc(100% - 120px)"
                class="q-pr-sm"
              />
              
              <div class="flex justify-center">
                <q-select
                  dense
                  filled
                  borderless
                  v-model="selectedRole"
                  :options="options"
                  style="width: 120px"
                  class="q-pr-sm"
                />
              </div>
            </div>
            <q-btn
              v-if="currentUserRole == 'admin'"
              class="text-bold no-border"
              padding="sm 0"
              color="secondary"
              no-caps
              :label="t(`user.sendInvite`)"
              @click="inviteUser()"
              :disable="userEmail == ''"
              style="
                padding: 7px 9px;
                min-height: 0px;
                width: 100px;
                display: block;
                float: right;
                top: 1px;
              "
            />
            
            <label class="inputHint q-pl-md" style="display: block">{{
              t("user.inviteByEmailHint")
            }}</label>
          </div>
         
        </div>
        <QTablePagination
          :scope="scope"
          :pageTitle="t('user.header')"
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
          position="bottom"
          @update:changeRecordPerPage="changePagination"
        />
      </template>
    </q-table>
  </q-page>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar, type QTableProps, date } from "quasar";
import { useI18n } from "vue-i18n";
import config from "@/aws-exports";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import usersService from "@/services/users";
import NoData from "@/components/shared/grid/NoData.vue";
import { validateEmail } from "@/utils/zincutils";
import organizationsService from "@/services/organizations";
import segment from "@/services/segment_analytics";
import {
  getImageURL,
  verifyOrganizationStatus,
  maskText,
} from "@/utils/zincutils";

export default defineComponent({
  name: "UserPageCloud",
  components: { QTablePagination, NoData },
  emits: [
    "updated:fields",
    "deleted:fields",
    "updated:dates",
    "update:changeRecordPerPage",
    "update:maxRecordToReturn",
  ],
  setup(props, { emit }) {
    const store = useStore();
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const orgMembers: any = ref([]);
    const resultTotal = ref<number>(0);
    const orgData: any = ref(store.state.selectedOrganization);
    const qTable: any = ref(null);

    const columns: any = ref<QTableProps["columns"]>([
      {
        name: "#",
        label: "#",
        field: "#",
        align: "left",
      },
      {
        name: "email",
        field: "email",
        label: t("user.email"),
        align: "left",
        sortable: true,
      },
      {
        name: "first_name",
        field: "first_name",
        label: t("user.firstName"),
        align: "left",
        sortable: true,
      },
      {
        name: "last_name",
        field: "last_name",
        label: t("user.lastName"),
        align: "left",
        sortable: true,
      },
      {
        name: "role",
        field: "role",
        label: t("user.role"),
        align: "left",
        sortable: true,
      },
      {
        name: "status",
        field: "status",
        label: t("user.status"),
        align: "left",
        sortable: true,
      },
    ]);
    const userEmail: any = ref("");
    const options = ["admin", "member"];
    const selectedRole = ref(options[0]);
    const currentUserRole = ref("");

    const getOrgMembers = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading users...",
      });

      usersService
        .orgUsers(
          0,
          1000,
          "email",
          false,
          "",
          store.state.selectedOrganization.identifier
        )
        .then((res) => {
          resultTotal.value = res.data.data.length;

          let counter = 1;
          orgMembers.value = res.data.data.map((data: any) => {
            if (store.state.userInfo.email == data.email) {
              currentUserRole.value = data.role;
            }

            return {
              "#": counter <= 9 ? `0${counter++}` : counter++,
              email: maskText(data.email),
              first_name: data.first_name,
              last_name: data.last_name,
              role: data.role,
              member_created: date.formatDate(
                parseInt(data.member_created),
                "YYYY-MM-DDTHH:mm:ssZ"
              ),
              member_updated: date.formatDate(
                parseInt(data.member_updated),
                "YYYY-MM-DDTHH:mm:ssZ"
              ),
              org_member_id: data.org_member_id,
              isLoggedinUser: store.state.userInfo.email == data.email,
              status: data.status,
            };
          });

          dismiss();
        });
    };

    if (orgMembers.value.length == 0) {
      getOrgMembers();
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

    const inviteUser = () => {
      const emailArray = userEmail.value
        .split(";")
        .flatMap((email: any) => email.split(","))
        .filter((email: any) => email.trim().length > 0)
        .map((email: any) => email.trim().toLowerCase());
      const validationArray = emailArray.map((email: any) =>
        validateEmail(email)
      );

      if (!validationArray.includes(false)) {
        const dismiss = $q.notify({
          spinner: true,
          message: "Please wait...",
          timeout: 2000,
        });

        organizationsService
          .add_members(
            { member_lists: emailArray, role: selectedRole.value },
            store.state.selectedOrganization.identifier
          )
          .then((res: { data: any }) => {
            const data = res.data;

            if (data.data.invalid_members != null) {
              const message = `Error while member invitation: ${data.data.invalid_members.toString()}`;

              $q.notify({
                type: "negative",
                message: message,
                timeout: 15000,
              });
            } else {
              $q.notify({
                type: "positive",
                message: data.message,
                timeout: 5000,
              });
            }

            // this.$emit("updated");
            dismiss();
          })
          .catch((error) => {
            dismiss();
            $q.notify({
              type: "negative",
              message: error.message,
              timeout: 5000,
            });
          });

        userEmail.value = "";

        segment.track("Button Click", {
          button: "Invite User",
          user_org: store.state.selectedOrganization.identifier,
          user_id: store.state.userInfo.email,
          page: "Users",
        });
      } else {
        $q.notify({
          type: "negative",
          message: `Please enter correct email id.`,
        });
      }
    };
    const updateUserRole = (row: any) => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait...",
        timeout: 2000,
      });

      organizationsService
        .update_member_role(
          {
            id: parseInt(row.orgMemberId ? row.orgMemberId : row.org_member_id),
            role: row.role,
            email: row.email,
            organization_id: parseInt(store.state.selectedOrganization.id),
          },
          store.state.selectedOrganization.identifier
        )
        .then((res: { data: any }) => {
          if (res.data.error_members != null) {
            const message = `Error while updating organization member`;
            $q.notify({
              type: "negative",
              message: message,
              timeout: 15000,
            });
          } else {
            $q.notify({
              type: "positive",
              message: "Organization member updated successfully.",
              timeout: 3000,
            });
          }
          dismiss();
        })
        .catch((error) => {
          dismiss();
          console.log(error);
        });

      segment.track("Button Click", {
        button: "Update Role",
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        update_user: row.email,
        page: "Users",
      });
    };
    return {
      t,
      qTable,
      router,
      store,
      config,
      orgMembers,
      columns,
      orgData,
      getOrgMembers,
      pagination,
      resultTotal,
      perPageOptions,
      selectedPerPage,
      changePagination,
      maxRecordToReturn,
      changeMaxRecordToReturn,
      filterQuery: ref(""),
      filterData(rows: any, terms: any) {
        var filtered = [];
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (
            rows[i]["first_name"]?.toLowerCase().includes(terms) ||
            rows[i]["last_name"]?.toLowerCase().includes(terms) ||
            rows[i]["email"]?.toLowerCase().includes(terms) ||
            rows[i]["role"].toLowerCase().includes(terms)
          ) {
            filtered.push(rows[i]);
          }
        }
        return filtered;
      },
      userEmail,
      selectedRole,
      options,
      inviteUser,
      currentUserRole,
      updateUserRole,
      getImageURL,
      verifyOrganizationStatus,
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

.iconHoverBtn {
  cursor: pointer !important;
}

.confirmBody {
  padding: 11px 1.375rem 0;
  font-size: 0.875rem;
  text-align: center;
  font-weight: 700;

  .head {
    line-height: 2.125rem;
    margin-bottom: 0.5rem;
    color: $dark-page;
  }

  .para {
    color: $light-text;
  }
}

.confirmActions {
  justify-content: center;
  padding: 1.25rem 1.375rem 1.625rem;
  display: flex;

  .q-btn {
    font-size: 0.75rem;
    font-weight: 700;
  }
}

.non-selectable {
  cursor: default !important;
}

.invite-user {
  background: $input-bg;
  border-radius: 4px;

  .separator {
    width: 1px;
  }
}

.inputHint {
  font-size: 11px;
  color: $light-text;
}
</style>
