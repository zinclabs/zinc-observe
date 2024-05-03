<template>
  <div class="flex items-center o2-input">
    <div>In case of failure retry test</div>
    <div data-test="request-retry-count-input" class="q-mr-sm q-mt-sm q-mx-sm">
      <q-input
        v-model="retry.count"
        color="input-border"
        bg-color="input-bg"
        class="q-mb-sm"
        type="number"
        outlined
        filled
        dense
        tabindex="0"
        style="width: 80px"
      />
    </div>
    <div>times with</div>
    <div data-test="request-retry-delay-input" class="q-mr-sm q-mt-sm q-mx-sm">
      <q-input
        v-model="retry.delay"
        color="input-border"
        bg-color="input-bg"
        class="q-mb-sm"
        type="number"
        outlined
        filled
        dense
        tabindex="0"
        style="width: 150px"
      />
    </div>
    <div>ms of wait between failures.</div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";

const props = defineProps({
  modelValue: {
    type: Object,
    required: true,
    default: () => ({}),
  },
});

const emit = defineEmits(["update:modelValue"]);

const retry = computed({
  get: () => {
    return props.modelValue;
  },
  set: (value) => {
    emit("update:modelValue", value);
  },
});
</script>

<style scoped></style>
