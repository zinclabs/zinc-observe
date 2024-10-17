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
  <q-card class="column full-height">
    <q-card-section class="q-px-md q-py-md">
      <div class="row items-center no-wrap">
        <div class="col">
          <div class="text-body1 text-bold text-dark">
            {{ t("user.editUser") }}
          </div>
          <!-- <div>({{ orgMemberData.first_name }}: {{ orgMemberData.email }})</div> -->
        </div>
        <div class="col-auto">
          <q-btn v-close-popup="true" round flat icon="cancel" />
        </div>
      </div>
    </q-card-section>
    <q-separator />
    <q-card-section class="q-w-md q-mx-lg">
      <q-form ref="updateUserForm" @submit.prevent="onSubmit">
        <q-input
          v-model="orgMemberData.first_name"
          :label="t('user.name')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          readonly
          filled
          dense
        />

        <!--
        <q-input
          v-model="orgMemberData.email"
          :label="t('user.email')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          readonly
          filled
          dense
        />
        -->

        <q-select
          v-model="orgMemberData.role"
          :label="t('user.role')"
          :options="roleOptions"
          color="input-border"
          bg-color="input-bg"
          class="q-pt-md q-pb-sm showLabelOnTop"
          stack-label
          outlined
          filled
          dense
        />

        <div class="flex justify-center q-mt-lg">
          <q-btn
            v-close-popup="true"
            class="q-mb-md text-bold no-border"
            :label="t('user.cancel')"
            text-color="light-text"
            padding="sm md"
            color="accent"
            no-caps
          />
          <q-btn
            :label="t('user.save')"
            class="q-mb-md text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            type="submit"
            no-caps
          />
        </div>
      </q-form>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import { getImageURL } from "@/utils/zincutils";

import organizationsService from "@/services/organizations";

const defaultValue: any = () => {
  return {
    org_member_id: "",
    role: "",
    first_name: "",
    email: "",
  };
};
let callOrgMember: any;

export default defineComponent({
  name: "ComponentUpdateUser",
  props: {
    modelValue: {
      type: Object,
      default: () => defaultValue(),
    },
  },
  emits: ["update:modelValue", "updated", "finish"],
  setup() {
    const store: any = useStore();
    const { t } = useI18n();
    const $q = useQuasar();
    const roleOptions = ["admin"];
    const orgMemberData: any = ref(defaultValue());
    const updateUserForm: any = ref(null);

    return {
      t,
      orgMemberData,
      store,
      roleOptions,
      updateUserForm,
      getImageURL,
    };
  },
  created() {
    if (this.modelValue) {
      this.orgMemberData = {
        org_member_id: this.modelValue.org_member_id,
        role: this.modelValue.role,
        first_name: this.modelValue.first_name,
        email: this.modelValue.email,
      };
    }
  },
  methods: {
    onSubmit() {
      const dismiss = this.$q.notify({
        spinner: true,
        message: "Please wait...",
        timeout: 2000,
      });

      this.updateUserForm.validate().then((valid: any) => {
        if (!valid) {
          return false;
        }

        callOrgMember = organizationsService.update_member_role(
          {
            id: parseInt(this.orgMemberData.org_member_id),
            role: this.orgMemberData.role,
            organization_id: parseInt(this.store.state.selectedOrganization.id),
          },
          this.store.state.selectedOrganization.identifier
        );

        callOrgMember.then((res: { data: any }) => {
          if (res.data.error_members != null) {
            const message = `Error while updating organization member`;

            this.$q.notify({
              type: "negative",
              message: message,
              timeout: 15000,
            });
          } else {
            this.$q.notify({
              type: "positive",
              message: "Organization member updated successfully.",
              timeout: 3000,
            });
          }

          this.$emit("updated", res.data);
          this.updateUserForm.resetValidation();
          dismiss();
        });
      });
    },
  },
});
</script>

<style lang="scss" scoped>
.q-menu {
  box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
  transform: translateY(0.5rem);
  border-radius: 8px;

  .q-virtual-scroll__content {
    padding: 0.5rem;

    .q-item {
      text-transform: capitalize;
      border-radius: 0.25rem;
      margin-bottom: 0.25rem;
      font-weight: 600;

      &--active {
        background-color: $selected-list-bg;
        color: $primary;
      }
    }
  }
}
</style>
