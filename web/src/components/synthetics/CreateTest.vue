<template>
  <div
    class="full-height"
    :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
  >
    <q-form @submit="savePipeline">
      <div class="flex justify-between items-center q-px-md q-py-sm">
        <div data-test="add-role-section-title" style="font-size: 18px">
          {{ t("synthetics.addTest") }}
        </div>
        <q-icon
          data-test="add-role-close-dialog-btn"
          name="cancel"
          class="cursor-pointer"
          size="20px"
          :v-close-popup="true"
        />
      </div>

      <div class="full-width bg-grey-4" style="height: 1px" />

      <div class="q-px-md">
        <div
          data-test="add-alert-name-input"
          class="alert-name-input o2-input"
          style="padding-top: 12px"
        >
          <q-input
            v-model="formData.name"
            :label="t('alerts.name') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            v-bind:readonly="isUpdating"
            v-bind:disable="isUpdating"
            :rules="[
              (val, rules) =>
                !!val
                  ? isValidName ||
                    `Use alphanumeric and '+=,.@-_' characters only, without spaces.`
                  : t('common.nameRequired'),
            ]"
            tabindex="0"
            style="min-width: 480px"
          />
        </div>
        <div
          data-test="add-alert-stream-type-select"
          class="alert-stream-type o2-input q-mr-sm q-mb-sm"
          style="padding-top: 0"
        >
          <q-select
            v-model="formData.type"
            :options="streamTypes"
            :label="t('synthetics.testType') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm showLabelOnTop no-case"
            emit-value
            map-options
            stack-label
            outlined
            filled
            dense
            v-bind:readonly="isUpdating"
            v-bind:disable="isUpdating"
            :rules="[(val: any) => !!val || 'Field is required!']"
            style="min-width: 220px"
          />
        </div>
      </div>

      <div class="flex justify-start q-mt-lg q-px-md">
        <q-btn
          data-test="add-alert-cancel-btn"
          :v-close-popup="true"
          class="q-mb-md text-bold"
          :label="t('alerts.cancel')"
          text-color="light-text"
          padding="sm md"
          no-caps
        />
        <q-btn
          data-test="add-alert-submit-btn"
          :label="t('alerts.save')"
          class="q-mb-md text-bold no-border q-ml-md"
          color="secondary"
          padding="sm xl"
          no-caps
          type="submit"
        />
      </div>
    </q-form>
  </div>
</template>

<script lang="ts" setup>
import { ref, computed, type Ref, defineEmits } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";

const props = defineProps({
  isUpdating: {
    type: Boolean,
    required: false,
    default: false,
  },
});

const emit = defineEmits(["save"]);

const store = useStore();

const formData = ref({
  name: "",
  type: "",
});

const { t } = useI18n();

const streamTypes = ref([
  { label: "Api Test", value: "api_test" },
  { label: "Browser Test", value: "browser_test" },
]);

const isValidName = computed(() => {
  const roleNameRegex = /^[a-zA-Z0-9+=,.@_-]+$/;
  // Check if the role name is valid
  return roleNameRegex.test(formData.value.name);
});

const savePipeline = () => {
  emit("save", formData.value);
};
</script>

<style lang="scss" scoped></style>
