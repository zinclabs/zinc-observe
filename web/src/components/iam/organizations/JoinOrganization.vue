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
  <q-card class="full-height">
    <q-card-section class="q-px-md q-py-md">
      <div class="row items-center no-wrap">
        <div class="col">
          <div class="text-body1 text-bold">
            {{ t("organization.invite") }}
          </div>
          <div class="text-bold captionText">
            ({{ organizationData.name }}: ID#{{ organizationData.id }})
          </div>
        </div>
        <div class="col-auto">
          <q-btn
            v-close-popup="true"
            round
            flat
            icon="img:/src/assets/images/common/close_icon.svg"
            @click="router.replace({ name: 'organizations' })"
          />
        </div>
      </div>
    </q-card-section>
    <q-separator />

    <q-card-section class="q-w-md q-mx-lg">
      <ListAssociatedMembers
        :key="associatedMembers.length"
        :data="associatedMembers"
      ></ListAssociatedMembers>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";

import organizationsService from "@/services/organizations";
import ListAssociatedMembers from "./ListAssociatedMembers.vue";

const defaultValue: any = () => {
  return {
    id: "",
    name: "",
    role: "",
    identifier: "",
    member_lists: [],
  };
};

export default defineComponent({
  name: "ComponentAddUpdateUser",
  components: { ListAssociatedMembers },
  props: {
    modelValue: {
      type: Object,
      default: () => defaultValue(),
    },
  },
  emits: ["update:modelValue", "updated", "finish"],
  setup() {
    const store: any = useStore();
    const router = useRouter();
    const organizationData: any = ref(defaultValue());
    const associatedMembers: any = ref([]);
    const { t } = useI18n();
    const $q = useQuasar();

    return {
      t,
      organizationData,
      store,
      associatedMembers,
      router,
    };
  },
  created() {
    if (this.modelValue && this.modelValue.id) {
      this.organizationData = {
        id: this.modelValue.id,
        name: this.modelValue.name,
        role: this.modelValue.role,
        identifier: this.modelValue.identifier,
        member_lists: this.modelValue.member_lists,
      };
    }

    this.getAssociatedMembers(this.modelValue?.identifier);
  },
  methods: {
    getAssociatedMembers(identifier: string) {
      const dismiss = this.$q.notify({
        spinner: true,
        message: "Please wait while loading organizations...",
      });

      organizationsService
        .get_associated_members(identifier)
        .then((res) => {
          let counter = 1;
          this.associatedMembers = res.data.data.map(
            (data: { email: any; status: any }) => {
              return {
                "#": counter++,
                email: data.email,
                status: data.status,
                owner:
                  data.email == this.store.state.userInfo.email ? true : false,
              };
            }
          );

          dismiss();
        })
        .catch((error: any) => console.log(error));
    },
  },
});
</script>

<style lang="scss" scoped>
.captionText {
  font-size: 0.625rem;
}
</style>
