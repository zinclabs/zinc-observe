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

<template>
  <div>
    <div class="add-function-header row items-center no-wrap">
      <div class="col">
        <div v-if="beingUpdated" class="text-h6">
          {{ t("function.updateTitle") }}
        </div>
        <div v-else class="text-h6">{{ t("function.addTitle") }}</div>
      </div>
    </div>

    <q-separator />
    <div>
      <q-form id="addFunctionForm" ref="addJSTransformForm" @submit="onSubmit">
        <div
          class="add-function-name-input row q-pb-sm q-pt-md q-col-gutter-md"
        >
          <q-input
            v-model="formData.name"
            :label="t('function.name')"
            color="input-border"
            bg-color="input-bg"
            class="col-4 q-py-md showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            v-bind:readonly="beingUpdated"
            v-bind:disable="beingUpdated"
            :rules="[(val: any) => !!val || 'Field is required!', isValidMethodName,]"
            tabindex="0"
          />
        </div>

        <div v-if="store.state.zoConfig.lua_fn_enabled" class="q-gutter-sm">
          <q-radio
            v-bind:readonly="beingUpdated"
            v-bind:disable="beingUpdated"
            v-model="formData.transType"
            :checked="formData.transType === '0'"
            val="0"
            :label="t('function.vrl')"
            class="q-ml-none"
            @update:model-value="updateEditorContent"
          />
          <q-radio
            v-bind:readonly="beingUpdated"
            v-bind:disable="beingUpdated"
            v-model="formData.transType"
            :checked="formData.transType === '1'"
            val="1"
            :label="t('function.lua')"
            class="q-ml-none"
            @update:model-value="updateEditorContent"
          />
        </div>

        <q-input
          v-if="formData.transType === '0'"
          v-model="formData.params"
          :label="t('function.params')"
          :placeholder="t('function.paramsHint')"
          color="input-border"
          bg-color="input-bg"
          class="col-4 q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          v-bind:readonly="beingUpdated"
          v-bind:disable="beingUpdated"
          :rules="[(val: any) => !!val || 'Field is required!', isValidParam,]"
          tabindex="0"
        />

        <div class="q-py-md showLabelOnTop text-bold text-h7">Function:</div>
        <div
          ref="editorRef"
          id="editor"
          :label="t('function.jsfunction')"
          stack-label
          style="border: 1px solid #dbdbdb; border-radius: 5px"
          @keyup="editorUpdate"
          class="showLabelOnTop"
          resize
        ></div>

        <!-- <q-input v-if="formData.ingest" v-model="formData.order" :label="t('function.order')" color="input-border"
                                                                                    bg-color="input-bg" class="q-py-md showLabelOnTop" stack-label outlined filled dense type="number" min="1" /> -->
        <pre class="q-py-md showLabelOnTop text-bold text-h7">{{
          compilationErr
        }}</pre>
        <div class="add-function-actions flex justify-center q-mt-lg">
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
import { defineComponent, ref, onMounted, computed, watch } from "vue";
import "monaco-editor/esm/vs/editor/editor.all.js";
import * as monaco from "monaco-editor/esm/vs/editor/editor.api";

import jsTransformService from "../../services/jstransform";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import segment from "../../services/segment_analytics";

const defaultValue: any = () => {
  return {
    name: "",
    function: "",
    params: "row",
    transType: "0",
  };
};

let callTransform: Promise<{ data: any }>;

export default defineComponent({
  name: "ComponentAddUpdateFunction",
  props: {
    modelValue: {
      type: Object,
      default: () => defaultValue(),
    },
    isUpdated: {
      type: Boolean,
      default: false,
    },
  },
  emits: ["update:list", "cancel:hideform"],
  setup(props, { emit }) {
    const store: any = useStore();
    // let beingUpdated: boolean = false;
    const addJSTransformForm: any = ref(null);
    const disableColor: any = ref("");
    const formData: any = ref(defaultValue());
    const indexOptions = ref([]);
    const { t } = useI18n();
    const $q = useQuasar();
    const editorRef: any = ref(null);
    let editorobj: any = null;
    const streams: any = ref({});
    const isFetchingStreams = ref(false);

    let compilationErr = ref("");

    const beingUpdated = computed(() => props.isUpdated);

    const streamTypes = ["logs", "metrics"];

    const editorUpdate = (e: any) => {
      formData.value.function = e.target.value;
    };
    const editorData = ref("");
    const prefixCode = ref("");
    const suffixCode = ref("");

    onMounted(async () => {
      monaco.editor.defineTheme("myCustomTheme", {
        base: "vs", // can also be vs-dark or hc-black
        inherit: true, // can also be false to completely replace the builtin rules
        rules: [
          {
            token: "comment",
            foreground: "ffa500",
            fontStyle: "italic underline",
          },
          { token: "comment.js", foreground: "008800", fontStyle: "bold" },
          { token: "comment.css", foreground: "0000ff" }, // will inherit fontStyle from `comment` above
        ],
        colors: {
          "editor.foreground": "#000000",
        },
      });
      editorobj = monaco.editor.create(editorRef.value, {
        value: ``,
        language: "vrl",
        minimap: {
          enabled: false,
        },
        theme: store.state.theme == "dark" ? "vs-dark" : "myCustomTheme",
      });

      editorobj.onKeyUp((e: any) => {
        if (editorobj.getValue() != "") {
          editorData.value = editorobj.getValue();
          formData.value.function = editorobj.getValue();
        }
      });

      editorobj.setValue(formData.value.function);
    });

    watch(
      () => store.state.theme,
      () => {
        monaco.editor.setTheme(
          store.state.theme == "dark" ? "vs-dark" : "myCustomTheme"
        );
      }
    );

    const isValidParam = () => {
      const methodPattern = /^[A-Za-z0-9]+(?:,[A-Za-z0-9]+)*$/g;
      return methodPattern.test(formData.value.params) || "Invalid params.";
    };

    const isValidMethodName = () => {
      const methodPattern = /^[$A-Z_][0-9A-Z_$]*$/i;
      return methodPattern.test(formData.value.name) || "Invalid method name.";
    };
    const updateEditorContent = () => {
      if (formData.value.transType == "1") {
        prefixCode.value = `function(row)`;
        suffixCode.value = `
end`;
      } else {
        prefixCode.value = ``;
        suffixCode.value = ``;
      }

      const someCode = `${prefixCode.value}
    ${editorData.value}
    ${suffixCode.value}`;
      editorobj.setValue(someCode);
      formData.value.function = editorobj.getValue();
    };

    const onSubmit = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait...",
        timeout: 2000,
      });

      formData.value.function = editorobj.getValue();
      addJSTransformForm.value.validate().then((valid: any) => {
        if (!valid) {
          return false;
        }

        if (!beingUpdated.value) {
          formData.value.transType = parseInt(formData.value.transType);
          //trans type is lua remove params from form
          if (formData.value.transType == 1) {
            formData.value.params = "";
          }

          callTransform = jsTransformService.create(
            store.state.selectedOrganization.identifier,
            formData.value
          );
        } else {
          formData.value.transType = parseInt(formData.value.transType);
          //trans type is lua remove params from form
          if (formData.value.transType == 1) {
            formData.value.params = "";
          }

          callTransform = jsTransformService.update(
            store.state.selectedOrganization.identifier,
            formData.value
          );
        }

        callTransform
          .then((res: { data: any }) => {
            const data = res.data;
            const _formData: any = { ...formData.value };
            formData.value = { ...defaultValue() };

            emit("update:list", _formData);
            addJSTransformForm.value.resetValidation();
            dismiss();
            $q.notify({
              type: "positive",
              message: res.data.message,
            });
          })
          .catch((err) => {
            compilationErr.value = err.response.data["message"];
            $q.notify({
              type: "negative",
              message:
                JSON.stringify(err.response.data["error"]) ||
                "Function creation failed",
            });
            dismiss();
          });

        segment.track("Button Click", {
          button: "Save Function",
          user_org: store.state.selectedOrganization.identifier,
          user_id: store.state.userInfo.email,
          function_name: formData.value.name,
          page: "Add/Update Function",
        });
      });
    };

    return {
      t,
      $q,
      disableColor,
      beingUpdated,
      formData,
      addJSTransformForm,
      store,
      compilationErr,
      indexOptions,
      editorRef,
      editorobj,
      prefixCode,
      suffixCode,
      editorData,
      editorUpdate,
      updateEditorContent,
      streamTypes,
      isFetchingStreams,
      isValidParam,
      isValidMethodName,
      onSubmit,
    };
  },
  created() {
    this.formData = { ...defaultValue(), ...this.modelValue };
    this.beingUpdated = this.isUpdated;

    if (
      this.modelValue &&
      this.modelValue.name != undefined &&
      this.modelValue.name != ""
    ) {
      this.beingUpdated = true;
      this.disableColor = "grey-5";
      this.formData = this.modelValue;
    }
  },
});
</script>

<style scoped>
#editor {
  width: 100%;
  min-height: 15rem;
  /* padding-bottom: 14px; */
  resize: both;
}
</style>
<style>
.no-case .q-field__native span {
  text-transform: none !important;
}
</style>
