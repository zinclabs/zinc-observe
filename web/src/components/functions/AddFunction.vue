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
  <div class="add-functions-section tw-pl-4 tw-py-0 tw-pr-0">
    <div class="add-function-actions tw-pb-2 tw-pt-1">
      <FunctionsToolbar
        v-model:name="formData.name"
        ref="functionsToolbarRef"
        :disable-name="beingUpdated"
        @test="onTestFunction"
        @save="onSubmit"
        @back="closeAddFunction"
        @cancel="cancelAddFunction"
        class="tw-pr-4"
      />
      <q-separator />
    </div>

    <div
      class="tw-flex tw-h-[calc(100vh-112px)] tw-overflow-auto tw-pr-2 tw-pb-4"
    >
      <q-splitter
        v-model="splitterModel"
        :limits="[30, Infinity]"
        class="tw-overflow-hidden tw-w-full"
        reverse
      >
        <template v-slot:before>
          <div class="tw-pr-2">
            <q-form
              id="addFunctionForm"
              ref="addJSTransformForm"
              @submit="onSubmit"
            >
              <div class="add-function-name-input q-pb-sm o2-input">
                <FullViewContainer
                  name="function"
                  v-model:is-expanded="expandState.functions"
                  :label="t('function.jsfunction') + '*'"
                  class="tw-mt-1"
                />
                <div
                  v-show="expandState.functions"
                  class="tw-border-[1px] tw-border-gray-200"
                >
                  <query-editor
                    data-test="logs-vrl-function-editor"
                    ref="editorRef"
                    editor-id="add-function-editor"
                    class="monaco-editor"
                    v-model:query="formData.function"
                    language="vrl"
                  />
                </div>
                <div class="text-subtitle2">
                  <div v-if="vrlFunctionError">
                    <FullViewContainer
                      name="function"
                      v-model:is-expanded="expandState.functionError"
                      :label="t('function.errorDetails')"
                      labelClass="tw-text-red-600"
                    />
                    <div
                      v-if="expandState.functionError"
                      class="q-px-sm q-pb-sm"
                      :class="
                        store.state.theme === 'dark'
                          ? 'bg-grey-10'
                          : 'bg-grey-2'
                      "
                    >
                      <pre class="q-my-none" style="white-space: pre-wrap">{{
                        vrlFunctionError
                      }}</pre>
                    </div>
                  </div>
                </div>
              </div>
            </q-form>
          </div>
        </template>
        <template v-slot:after>
          <div
            class="q-px-md q-pt-sm q-pb-md tw-rounded-md tw-border-1 tw-border-gray-900 tw-h-max q-ml-sm"
            :class="
              store.state.theme === 'dark' ? 'tw-bg-gray-700' : 'tw-bg-zinc-100'
            "
          >
            <TestFunction
              ref="testFunctionRef"
              :vrlFunction="formData"
              @function-error="handleFunctionError"
            />
          </div>
        </template>
      </q-splitter>
    </div>
  </div>
  <confirm-dialog
    :title="confirmDialogMeta.title"
    :message="confirmDialogMeta.message"
    @update:ok="confirmDialogMeta.onConfirm()"
    @update:cancel="resetConfirmDialog"
    v-model="confirmDialogMeta.show"
  />
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onMounted,
  computed,
  watch,
  onUnmounted,
} from "vue";

import jsTransformService from "../../services/jstransform";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import segment from "../../services/segment_analytics";
import QueryEditor from "@/components/QueryEditor.vue";
import TestFunction from "@/components/functions/TestFunction.vue";
import FunctionsToolbar from "@/components/functions/FunctionsToolbar.vue";
import FullViewContainer from "@/components/functions/FullViewContainer.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

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
  components: {
    QueryEditor,
    FunctionsToolbar,
    FullViewContainer,
    TestFunction,
    ConfirmDialog,
  },
  emits: ["update:list", "cancel:hideform"],
  setup(props, { emit }) {
    const store: any = useStore();
    // let beingUpdated: boolean = false;
    const addJSTransformForm: any = ref(null);
    const disableColor: any = ref("");
    const formData: any = ref({
      name: "",
      function: "",
      params: "row",
      transType: "0",
    });
    const indexOptions = ref([]);
    const { t } = useI18n();
    const $q = useQuasar();
    const editorRef: any = ref(null);
    let editorobj: any = null;
    const streams: any = ref({});
    const isFetchingStreams = ref(false);
    const testFunctionRef = ref<typeof TestFunction>();
    const functionsToolbarRef = ref<typeof FunctionsToolbar>();
    const splitterModel = ref(50);
    const confirmDialogMeta = ref({
      title: "",
      message: "",
      show: false,
      onConfirm: () => {},
      data: null,
    });

    const expandState = ref({
      functions: true,
      functionError: false,
    });

    const vrlFunctionError = ref("");

    let compilationErr = ref("");

    const beingUpdated = computed(() => props.isUpdated);

    const streamTypes = ["logs", "metrics", "traces"];

    const isFunctionDataChanged = ref(false);

    watch(
      () => formData.value.name + formData.value.function,
      () => {
        isFunctionDataChanged.value = true;
      },
    );

    const beforeUnloadHandler = (e: any) => {
      //check is data updated or not
      if (isFunctionDataChanged.value) {
        // Display a confirmation message
        const confirmMessage = t("dashboard.unsavedMessage"); // Some browsers require a return statement to display the message
        e.returnValue = confirmMessage;
        return confirmMessage;
      }
      return;
    };

    const editorUpdate = (e: any) => {
      formData.value.function = e.target.value;
    };
    const prefixCode = ref("");
    const suffixCode = ref("");

    const isValidParam = () => {
      const methodPattern = /^[A-Za-z0-9]+(?:,[A-Za-z0-9]+)*$/g;
      return methodPattern.test(formData.value.params) || "Invalid params.";
    };

    const isValidMethodName = () => {
      const methodPattern = /^[$A-Z_][0-9A-Z_$]*$/i;
      return (
        methodPattern.test(formData.value.name) || "Invalid Function name."
      );
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

      formData.value.function = `${prefixCode.value}
    ${formData.value.function}
    ${suffixCode.value}`;
    };

    const isValidFnName = () => {
      return formData.value.name.trim().length > 0;
    };

    const onSubmit = () => {
      if (!functionsToolbarRef.value) return;

      functionsToolbarRef.value.addFunctionForm
        .validate()
        .then((valid: any) => {
          if (!valid) {
            return false;
          }

          const dismiss = $q.notify({
            spinner: true,
            message: "Please wait...",
            timeout: 2000,
          });

          if (!beingUpdated.value) {
            formData.value.transType = parseInt(formData.value.transType);
            //trans type is lua remove params from form
            if (formData.value.transType == 1) {
              formData.value.params = "";
            }

            callTransform = jsTransformService.create(
              store.state.selectedOrganization.identifier,
              formData.value,
            );
          } else {
            formData.value.transType = parseInt(formData.value.transType);
            //trans type is lua remove params from form
            if (formData.value.transType == 1) {
              formData.value.params = "";
            }

            callTransform = jsTransformService.update(
              store.state.selectedOrganization.identifier,
              formData.value,
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
                message: res.data.message || "Function saved successfully",
              });
            })
            .catch((err) => {
              compilationErr.value = err.response.data["message"];
              $q.notify({
                type: "negative",
                message:
                  JSON.stringify(err.response.data?.error) ||
                  err.response.data?.message ||
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

    const onTestFunction = () => {
      if (testFunctionRef.value) testFunctionRef.value.testFunction();
    };

    const handleFunctionError = (err: string) => {
      vrlFunctionError.value = err;
    };

    const closeAddFunction = () => {
      if (isFunctionDataChanged.value) {
        confirmDialogMeta.value.show = true;
        confirmDialogMeta.value.title = t("common.usavedTitle");
        confirmDialogMeta.value.message = t("common.unsavedMessage");
        confirmDialogMeta.value.onConfirm = () => {
          emit("cancel:hideform");
          resetConfirmDialog();
        };
      } else {
        emit("cancel:hideform");
      }
    };

    const cancelAddFunction = () => {
      if (isFunctionDataChanged.value) {
        confirmDialogMeta.value.show = true;
        confirmDialogMeta.value.title = t("common.cancelTitle");
        confirmDialogMeta.value.message = t("common.cancelMessage");
        confirmDialogMeta.value.onConfirm = () => {
          emit("cancel:hideform");
          resetConfirmDialog();
        };
      } else {
        emit("cancel:hideform");
      }
    };

    const resetConfirmDialog = () => {
      confirmDialogMeta.value.show = false;
      confirmDialogMeta.value.title = "";
      confirmDialogMeta.value.message = "";
      confirmDialogMeta.value.onConfirm = () => {};
      confirmDialogMeta.value.data = null;
    };

    return {
      t,
      $q,
      emit,
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
      editorUpdate,
      updateEditorContent,
      streamTypes,
      isFetchingStreams,
      isValidParam,
      isValidMethodName,
      onSubmit,
      expandState,
      testFunctionRef,
      onTestFunction,
      handleFunctionError,
      vrlFunctionError,
      functionsToolbarRef,
      splitterModel,
      closeAddFunction,
      confirmDialogMeta,
      resetConfirmDialog,
      cancelAddFunction,
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

<style scoped lang="scss">
.monaco-editor {
  width: 100%;
  height: calc(100vh - 160px);
  border-radius: 5px;
}

.add-function-name-input {
  :deep(.q-field--dense .q-field__control) {
    height: 36px;
    min-height: auto;
    border-radius: 3px;

    .q-field__control-container {
      height: 32px;

      .q-field__native {
        height: 32px !important;
      }
    }

    .q-field__marginal {
      height: 32px;
      min-height: auto;
    }
  }

  :deep(.q-field__bottom) {
    padding-top: 4px !important;
    min-height: auto;
  }
}

.function-stream-select-input {
  :deep(.q-field--auto-height .q-field__control) {
    height: 32px;
    min-height: auto;

    .q-field__control-container {
      height: 32px;

      .q-field__native {
        min-height: 32px !important;
        height: 32px !important;
      }
    }

    .q-field__marginal {
      height: 32px;
      min-height: auto;
    }
  }
}

.functions-duration-input {
  :deep(.date-time-button) {
    width: 100%;
  }
}
</style>
<style>
.no-case .q-field__native span {
  text-transform: none !important;
}
</style>
