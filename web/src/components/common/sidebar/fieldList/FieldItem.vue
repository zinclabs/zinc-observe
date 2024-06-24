<template>
  <div v-if="isExpandable">
    <div class="flex items-center full-width position-relative field-item">
      <div class="flex items-center">
        <template>
          <q-icon
            :name="expand.icon"
            :class="expand.class"
            :color="expand.color"
            @click="expandField"
          />
        </template>
        <template v-for="icon in icons" :key="icon.name">
          <q-icon :name="icon.name" :class="icon.class" :color="icon.color" />
        </template>
      </div>
      <div class="flex items-center">
        <template v-for="icon in icons" :key="icon.name">
          <q-icon :name="icon.name" :class="icon.class" :color="icon.color" />
        </template>
      </div>
      <div clas>{{ field.name }}</div>
      <div
        class="flex items-center absolute field-item-actions"
        style="right: 0"
      >
        <template v-for="action in actions" :key="action.name">
          <q-btn
            v-if="action.show"
            size="sm"
            dense
            :icon="action.icon"
            :class="action.class"
            :disable="action.disable"
          >
            <span style="font-size: 12px"> {{ action.name }}</span>
          </q-btn>
        </template>
      </div>
    </div>
    <BasicValuesFilter />
    <AdvancedValuesFilter
      v-if="fieldValues[field.name]"
      :key="data.stream.selectedStream.value"
      :row="field"
      v-model="fieldValues[field.name]"
      :filter="fieldValues[field.name]"
      @update:is-open="(isOpen) => handleFilterCreator(isOpen, field.name)"
      @update:selectedValues="
        (currValue, prevValue) =>
          updateQueryFilter(field.name, currValue, prevValue)
      "
      @update:search-keyword="getFieldValues(field.name)"
    />
  </div>

  <FieldList>
    <template #field-item>
      <FieldItem></FieldItem>
    </template>
  </FieldList>
</template>

<script setup lang="ts">
import useFieldList from "@/composables/useFieldList";
import AdvancedValuesFilter from "@/plugins/traces/fields-sidebar/AdvancedValuesFilter.vue";
import BasicValuesFilter from "@/plugins/traces/fields-sidebar/BasicValuesFilter.vue";
import { defineProps, watch, ref } from "vue";

interface Icon {
  name: string;
  color: string;
  class: string;
  show: boolean;
  disable?: boolean;
}
interface Action {
  name: string;
  icon?: string;
  class: string;
  show: boolean;
  disable?: boolean;
}
const props = defineProps({
  context: {
    type: String,
    required: true,
  },
  field: {
    type: Object,
    required: true,
    default: () => ({
      name: "Field Name",
      type: "Utf8",
      isInteresting: false,
      isFTS: false,
    }),
  },
});

const { data } = useFieldList();

const icons = ref<Icon[]>([]);

const actions = ref<Action[]>([]);

const expand = ref({
  icon: "expand_more",
  class: "",
  color: "",
});

const isExpandable = ref(false);

const isExpanded = ref(false);

// query type : sql, promql
//

const setDashboardField = () => {
  icons.value = [];
  actions.value = [];

  // Dashboard Icons
  icons.value.push({
    name: "drag_indicator",
    color: "grey-13",
    class: "q-mr-xs",
    show: props.data.showDragIndicator,
    disable: props.data.disabledDragIndicator,
  });

  icons.value.push({
    name:
      props.field.type == "Utf8"
        ? "text_fields"
        : props.field.type == "Int64"
        ? "tag"
        : "toggle_off",
    color: "grey-6",
    class: "q-mr-xs",
    show: true,
  });

  // Dashboard Actions
  if (!props.data.showActions) return;

  actions.value.push({
    name: props.data.chartType != "h-bar" ? "+X" : "+Y",
    class: "q-px-sm q-py-xs",
    show: true,
    disable: props.data.isAddXAxisNotAllowed,
  });

  actions.value.push({
    name: props.data.chartType != "h-bar" ? "+Y" : "+X",
    class: "q-px-sm q-py-xs",
    show: true,
    disable: props.data.isAddYAxisNotAllowed,
  });

  if (props.data.chartType === "heatmap") {
    actions.value.push({
      name: "+Z",
      class: "q-px-sm q-py-xs",
      show: true,
      disable: props.data.isAddZAxisNotAllowed,
    });
  }

  actions.value.push({
    name: "+F",
    class: "q-px-sm q-py-xs",
    show: true,
    disable: props.data.isAddFilterNotAllowed,
  });
};

const setLogsField = () => {};

const setVisualiseField = () => {};

watch(
  () => props.context,
  () => {
    if (props.context === "dashboard") {
      setDashboardField();
    }

    if (props.context === "logs") {
      setLogsField();
    }

    if (props.context === "visualise") {
      setVisualiseField();
    }
  },
  {
    immediate: true,
  }
);

const expandField = () => {};

const handleFilterCreator = () => {};

const updateQueryFilter = () => {};

const getFieldValues = () => {};
</script>

<style lang="scss" scoped>
.field-item {
  height: fit-content;
  height: 26px;
  transition: background-color 0.3s;
  &:hover {
    background-color: #f0f0f0;
    box-shadow: 0px 3px 6px -2px #d1d1d1;
  }
  .field-item-actions {
    z-index: 1;
    background-color: #ffffff;
  }
}
</style>
