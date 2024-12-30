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
  <q-page class="q-pa-none" style="min-height: inherit">
    <div class="row items-center no-wrap q-mx-md q-my-sm">
      <div class="flex items-center">
        <div
          class="flex justify-center items-center q-mr-md cursor-pointer"
          style="
            border: 1.5px solid;
            border-radius: 50%;
            width: 22px;
            height: 22px;
          "
          title="Go Back"
          @click="$emit('cancel:hideform')"
        >
          <q-icon name="arrow_back_ios_new" size="14px" />
        </div>
        <div class="col" data-test="add-template-title">
          <div v-if="isUpdatingCipherKey" class="text-h6">
            {{ t("cipherKey.update") }}
          </div>
          <div v-else class="text-h6">
            {{ t("cipherKey.add") }}
          </div>
        </div>
      </div>
    </div>
    <q-separator />
    <q-form
      class="create-cipher-form"
      ref="addCipherKeyFormRef"
      @submit="onSubmit"
    >
      <div class="row">
        <div class="col-4 o2-input flex q-mx-md q-mt-md">
          <q-input
            data-test="add-cipher-key-name-input"
            v-model="formData.name"
            :label="t('cipherKey.name') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop full-width"
            stack-label
            outlined
            filled
            dense
            v-bind:readonly="isUpdatingCipherKey"
            v-bind:disable="isUpdatingCipherKey"
            :rules="[
              (val: any) =>
                !!val
                  ? isValidResourceName(val) ||
                    `Characters like :, ?, /, #, and spaces are not allowed.`
                  : 'Name is required',
            ]"
            tabindex="0"
          />
        </div>
        <div class="col-4 o2-input flex q-mx-md q-mt-md">
          <q-select
            data-test="add-cipher-key-type-input"
            v-model="formData.key.store.type"
            :label="t('cipherKey.type') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop full-width"
            stack-label
            outlined
            filled
            dense
            :options="cipherKeyTypes"
            option-value="value"
            option-label="label"
            map-options
            emit-value
            :rules="[(val: any) => !!val || 'Type is required']"
            tabindex="0"
          />
        </div>
      </div>

      <q-stepper
        v-model="step"
        vertical
        color="primary"
        animated
        class="q-mx-md q-pa-none"
        header-nav
      >
        <q-step
          data-test="cipher-key-key-store-detils-step"
          :name="1"
          :title="
            t('cipherKey.step1') +
            ' (Type: ' +
            getTypeLabel(formData.key.store.type) +
            ')'
          "
          icon="addition"
          :done="step > 1"
        >
          <div>
            <add-openobserve-type
              v-if="formData.key.store.type == 'openobserve'"
              v-model:formData="formData"
            />
            <add-akeyless-type
              v-else-if="formData.key.store.type == 'akeyless'"
              v-model:formData="formData"
            />
          </div>
          <q-stepper-navigation>
            <q-btn
              data-test="add-report-step1-continue-btn"
              @click="step = 2"
              color="secondary"
              label="Continue"
              no-caps
            />
          </q-stepper-navigation>
        </q-step>

        <q-step
          data-test="cipher-key-encryption-mechanism-step"
          :name="2"
          :title="t('cipherKey.step2')"
          icon="addition"
          :done="step > 2"
        >
          <add-encryption-mechanism v-model:formData="formData" />
          <q-stepper-navigation>
            <q-btn
              data-test="add-cipher-key-step2-back-btn"
              flat
              @click="step = 1"
              color="primary"
              :label="t('common.back')"
              class="q-ml-sm"
              no-caps
            />
          </q-stepper-navigation>
        </q-step>
      </q-stepper>
      <div class="flex justify-start q-px-md q-py-sm full-width">
        <q-btn
          data-test="add-cipher-key-cancel-btn"
          class="text-bold"
          :label="t('common.cancel')"
          text-color="light-text"
          padding="sm md"
          no-caps
          @click="openCancelDialog"
        />
        <q-btn
          data-test="add-cipher-key-save-btn"
          :label="t('common.save')"
          class="text-bold no-border q-ml-md"
          color="secondary"
          padding="sm xl"
          no-caps
          @click="onSubmit"
        />
      </div>
    </q-form>
    <ConfirmDialog
      v-model="dialog.show"
      :title="dialog.title"
      :message="dialog.message"
      @update:ok="dialog.okCallback"
      @update:cancel="dialog.show = false"
    />
  </q-page>
</template>
<script lang="ts" setup>
import { ref, onBeforeMount, onActivated, defineEmits, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useStore } from "vuex";
import { isValidResourceName } from "@/utils/zincutils";
import AddOpenobserveType from "@/components/cipherkeys/AddOpenobserveType.vue";
import AddAkeylessType from "@/components/cipherkeys/AddAkeylessType.vue";
import AddEncryptionMechanism from "@/components/cipherkeys/AddEncryptionMechanism.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

const emit = defineEmits(["cancel:hideform"]);
const { t } = useI18n();
const router = useRouter();
const store = useStore();
const addCipherKeyFormRef: any = ref();
const isUpdatingCipherKey: any = ref(false);
const step = ref(1);
const formData: any = ref({
  name: "",
  key: {
    store: {
      type: "openobserve",
      akeyless: {
        base_url: "",
        access_id: "",
        auth: {
          type: "access_key",
          access_key: "",
          ldap: {
            username: "",
            password: "",
          },
        },
        store: {
          type: "static_secret",
          static_secret: "",
          dfc: {
            name: "",
            iv: "",
            encrypted_data: "",
          },
        },
      },
      local: "",
    },
    mechanism: {
      type: "simple",
      simple_algorithm: "aes-256-siv",
    },
  },
});
const cipherKeyTypes = computed(() => [
  { label: "OpenObserve", value: "openobserve" },
  { label: "Akeyless", value: "akeyless" },
]);

const originalData: Ref<string> = ref("");

const dialog = ref({
  show: false,
  title: "",
  message: "",
  okCallback: () => {},
});

onActivated(() => setupTemplateData());
onBeforeMount(() => {
  setupTemplateData();
});

const getTypeLabel = (type: string) => {
  return cipherKeyTypes.value.find((t) => t.value === type)?.label;
};

const setupTemplateData = () => {
  // Format the data for the template editor
  // const params = router.currentRoute.value.query;
  // if (props.template) {
  //   isUpdatingCipherKey.value = true;
  //   formData.value.name = props.template.name;
  //   formData.value.body = props.template.body;
  //   formData.value.type = props.template.type;
  //   formData.value.title = props.template.title;
  // }
  // if (params.type) {
  //   formData.value.type = params.type as "email" | "http";
  // }
};

const onSubmit = () => {
  // if (isUpdatingCipherKey.value) {
  //   updateTemplate();
  // } else {
  //   addTemplate();
  // }
};

const openCancelDialog = () => {
  if (originalData.value === JSON.stringify(formData.value)) {
    goToCipherList();
    return;
  }
  dialog.value.show = true;
  dialog.value.title = "Discard Changes";
  dialog.value.message = "Are you sure you want to cancel changes?";
  dialog.value.okCallback = goToCipherList;
};

const goToCipherList = () => {
  emit("cancel:hideform");
};
</script>
