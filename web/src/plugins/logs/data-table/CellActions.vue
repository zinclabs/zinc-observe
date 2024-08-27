<template>
  <div
    class="field_overlay tw-absolute tw-right-0 tw-top-[50%] table-cell-actions tw-translate-y-[-50%]"
    :class="backgroundClass"
    :title="row[column.id]"
    :data-test="`log-add-data-from-column-${row[column.id]}`"
  >
    <q-btn
      class="q-mr-xs"
      size="6px"
      @click.prevent.stop="copyLogToClipboard(row[column.id])"
      title="Copy"
      round
      icon="content_copy"
    />
    <q-btn
      class="q-mr-xs"
      size="6px"
      @click.prevent.stop="addSearchTerm(`${column.id}='${row[column.id]}'`)"
      :data-test="`log-details-include-field-${row[column.id]}`"
      title="Include Term"
      round
    >
      <q-icon color="currentColor">
        <EqualIcon></EqualIcon>
      </q-icon>
    </q-btn>
    <q-btn
      size="6px"
      @click.prevent.stop="addSearchTerm(`${column.id}!='${row[column.id]}'`)"
      title="Exclude Term"
      :data-test="`log-details-exclude-field-${row[column.id]}`"
      round
    >
      <q-icon color="currentColor">
        <NotEqualIcon></NotEqualIcon>
      </q-icon>
    </q-btn>
  </div>
</template>

<script setup lang="ts">
import { defineProps, defineEmits, computed } from "vue";
import { useStore } from "vuex";
import EqualIcon from "@/components/icons/EqualIcon.vue";
import NotEqualIcon from "@/components/icons/NotEqualIcon.vue";

defineProps({
  column: {
    type: Object,
    required: true,
  },
  row: {
    type: Object,
    required: true,
  },
});

const store = useStore();

const emit = defineEmits(["copy", "addSearchTerm", "addFieldToTable"]);

const copyLogToClipboard = (value: any) => {
  emit("copy", value);
};
const addSearchTerm = (value: string) => {
  emit("addSearchTerm", value);
};

const backgroundClass = computed(() =>
  store.state.theme === "dark" ? "tw-bg-black" : "tw-bg-white",
);
</script>
