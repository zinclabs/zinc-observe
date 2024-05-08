<template>
  <div class="request-config o2-input">
    <div class="flex items-center">
      <div class="request-url">
        <q-input
          v-model="request.url"
          color="input-border"
          bg-color="input-bg"
          class="showLabelOnTop q-mb-sm full-width"
          placeholder="https://example.com"
          stack-label
          outlined
          filled
          dense
          tabindex="0"
        />
      </div>
      <div class="request-submit-btn">
        <q-btn
          data-test="add-alert-submit-btn"
          :label="t('synthetics.openUrl')"
          class="text-bold no-border"
          color="secondary"
          padding="sm xl"
          type="submit"
          no-caps
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

const props = defineProps({
  modelValue: {
    type: Object,
    required: true,
    default: () => ({}),
  },
});

const emit = defineEmits(["update:modelValue"]);

const { t } = useI18n();

const request = computed({
  get: () => props.modelValue,
  set: (val) => {
    console.log(val);
    emit("update:modelValue", { ...props.modelValue });
  },
});
</script>

<style scoped lang="scss">
.request-url {
  width: 500px;
}
</style>
