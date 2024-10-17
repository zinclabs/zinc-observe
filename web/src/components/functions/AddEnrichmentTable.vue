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
  <div class="q-mx-md q-my-md">
    <div class="row items-center no-wrap">
      <div class="col">
        <div v-if="isUpdating" class="text-h6">
          {{ t("function.updateEnrichmentTable") }}
        </div>
        <div v-else class="text-h6">{{ t("function.addEnrichmentTable") }}</div>
      </div>
    </div>

    <q-separator />
    <div>
      <q-form ref="addJSTransformForm" @submit="onSubmit">
        <div class="row">
          <q-input
            v-model="formData.name"
            :label="t('function.name')"
            color="input-border"
            bg-color="input-bg"
            class="col-12 q-py-md showLabelOnTop text-grey-8 text-bold"
            stack-label
            outlined
            filled
            dense
            v-bind:readonly="isUpdating"
            v-bind:disable="isUpdating"
            :rules="[(val: any) => !!val || 'Field is required!']"
            tabindex="0"
          />

          <q-file
            color="lime-11"
            filled
            v-model="formData.file"
            :label="t('function.uploadCSVFile')"
            bg-color="input-bg"
            class="col-12 q-py-md showLabelOnTop lookup-table-file-uploader"
            stack-label
            outlined
            dense
            :rules="[(val: any) => !!val || 'CSV File is required!']"
          >
            <template v-slot:prepend>
              <q-icon name="attachment" />
            </template>
          </q-file>
          <div v-if="isUpdating">
            <q-toggle
              class="col-12 q-py-md text-grey-8 text-bold"
              v-model="formData.append"
              :label="t('function.appendData')"
            />
          </div>
        </div>

        <pre class="q-py-md showLabelOnTop text-bold text-h7">{{
          compilationErr
        }}</pre>

        <div class="flex justify-center q-mt-lg">
          <q-btn
            v-close-popup="true"
            class="q-mb-md text-bold"
            :label="t('function.cancel')"
            text-color="light-text"
            padding="sm md"
            no-caps
            @click="$emit('cancel:hideform')"
          />
          <q-btn
            :label="t('function.save')"
            class="q-mb-md text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            type="submit"
            no-caps
          />
        </div>
      </q-form>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, computed } from "vue";
import jsTransformService from "../../services/jstransform";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import segment from "../../services/segment_analytics";

const defaultValue: any = () => {
  return {
    name: "",
    file: "",
    append: false,
  };
};

export default defineComponent({
  name: "AddEnrichmentTable",
  props: {
    modelValue: {
      type: Object,
      default: () => defaultValue(),
    },
    isUpdating: {
      type: Boolean,
      default: false,
    },
  },
  emits: ["update:list", "cancel:hideform"],
  setup(props, { emit }) {
    const store: any = useStore();
    const addJSTransformForm: any = ref(null);
    const disableColor: any = ref("");
    const formData: any = ref(defaultValue());
    const indexOptions = ref([]);
    const { t } = useI18n();
    const q = useQuasar();
    const editorRef: any = ref(null);
    let editorobj: any = null;
    const isFetchingStreams = ref(false);

    let compilationErr = ref("");

    const editorUpdate = (e: any) => {
      formData.value.function = e.target.value;
    };

    const onSubmit = () => {
      const dismiss = q.notify({
        spinner: true,
        message: "Please wait...",
        timeout: 2000,
      });
      let reqformData = new FormData();
      reqformData.append("file", formData.value.file);

      jsTransformService
        .create_enrichment_table(
          store.state.selectedOrganization.identifier,
          formData.value.name,
          reqformData,
          formData.value.append
        )
        .then((res) => {
          formData.value = { ...defaultValue() };
          emit("update:list");

          dismiss();
          q.notify({
            type: "positive",
            message: res.data.message,
          });
        })
        .catch((err) => {
          compilationErr.value = err.response.data["message"];
          q.notify({
            type: "negative",
            message:
              JSON.stringify(err.response.data["error"]) ||
              "Enrichment Table creation failed",
          });
          dismiss();
        });

      segment.track("Button Click", {
        button: "Save Enrichment Table",
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        function_name: formData.value.name,
        page: "Add/Update Enrichment Table",
      });
    };

    return {
      t,
      q,
      disableColor,
      formData,
      addJSTransformForm,
      store,
      compilationErr,
      indexOptions,
      editorRef,
      editorobj,
      editorUpdate,
      isFetchingStreams,
      onSubmit,
    };
  },
  created() {
    this.formData = { ...defaultValue(), ...this.modelValue };

    if (this.isUpdating) {
      this.disableColor = "grey-5";
      this.formData = this.modelValue;
      if (this.formData.append == undefined) this.formData.append = false;
    }
  },
});
</script>

<style scoped>
#editor {
  width: 100%;
  min-height: 15rem;
  padding-bottom: 14px;
  resize: both;
}
</style>
<style lang="scss">
.no-case .q-field__native span {
  text-transform: none !important;
}

.lookup-table-file-uploader {
  .q-field__label {
    left: -30px;
  }
}
</style>
