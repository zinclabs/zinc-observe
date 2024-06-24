<template>
  <q-expansion-item
    class="field-expansion-item"
    dense
    switch-toggle-side
    :label="row.name"
    v-model="filterData.isOpen"
    expand-icon-class="field-expansion-icon"
    expand-icon="expand_more"
    @before-show="(event: any) => openFilterCreator()"
    @before-hide="(event: any) => closeFilterCreator()"
  >
    <template v-slot:header>
      <div class="flex content-center ellipsis full-width" :title="row.name">
        <div
          class="field_label ellipsis"
          style="width: calc(100% - 28px); font-size: 14px"
        >
          {{ row.name }}
        </div>
      </div>
    </template>
    <q-card>
      <q-card-section class="q-pl-md q-pr-xs q-py-xs">
        <div class="q-mr-sm q-ml-sm">
          <input
            v-model="filterData.searchKeyword"
            class="full-width"
            :disabled="filterData.isLoading"
            placeholder="Search"
            @input="onSearchValue()"
          />
        </div>
        <div class="filter-values-container q-mt-sm">
          <div
            v-show="!filterData.values?.length && !filterData.isLoading"
            class="q-py-xs text-grey-9 text-center"
          >
            No values found
          </div>
          <div v-for="value in (filterData.values as any[])" :key="value.key">
            <q-list dense>
              <q-item tag="label" class="q-pr-none">
                <div
                  class="flex row wrap justify-between items-center"
                  style="width: calc(100%)"
                >
                  <q-checkbox
                    size="xs"
                    v-model="filterData.selectedValues"
                    :val="value.key.toString()"
                    class="filter-check-box cursor-pointer"
                  />
                  <div
                    :title="value.key"
                    class="ellipsis q-pr-xs"
                    style="width: calc(100% - 74px)"
                  >
                    {{ value.key }}
                  </div>
                  <div
                    :title="value.count"
                    class="ellipsis text-right q-pr-sm"
                    style="width: 50px"
                  >
                    {{ value.count }}
                  </div>
                </div>
              </q-item>
            </q-list>
          </div>
          <div
            v-show="filterData.isLoading"
            class="q-pl-md q-mb-xs q-mt-md"
            style="height: 60px; position: relative"
          >
            <q-inner-loading
              size="xs"
              :showing="filterData.isLoading"
              label="Fetching values..."
              label-style="font-size: 1.1em"
            />
          </div>
          <div
            v-show="filterData.values.length === filterData.size"
            class="text-right flex items-center justify-end q-pt-xs"
          >
            <div
              style="width: fit-content"
              class="flex items-center cursor-pointer"
              @click="fetchMoreValues()"
            >
              <div style="width: fit-content" class="show-more-btn">
                Show more
              </div>
            </div>
          </div>
        </div>
      </q-card-section>
    </q-card>
  </q-expansion-item>
</template>

<script lang="ts" setup>
import { ref, defineEmits, computed, watch } from "vue";
import { debounce } from "quasar";

const props = defineProps({
  modelValue: {
    type: Object,
    default: () => ({}),
  },
  row: {
    type: Object,
    default: () => null,
  },
  values: {
    type: Array,
    default: () => [],
  },
  selectedValues: {
    type: Array,
    default: () => [],
  },
  searchKeyword: {
    type: String,
    default: "",
  },
});

const valuesSize = ref(4);

const filterData = computed({
  get: () => props.modelValue,
  set: (value) => {
    console.log("value", value);
    emits("update:modelValue", value);
  },
});

const emits = defineEmits([
  "update:modelValue",
  "update:searchKeyword",
  "update:isOpen",
  "update:selectedValues",
]);

watch(
  () => filterData.value.selectedValues,
  (values, oldValues) => {
    emits("update:selectedValues", values, oldValues);
  }
);

const onSearchValue = () => {
  debouncedOpenFilterCreator();
};

const debouncedOpenFilterCreator = debounce(() => {
  emits("update:searchKeyword", filterData.value.searchKeyword);
}, 400);

const fetchMoreValues = () => {
  valuesSize.value = valuesSize.value * 2;
  openFilterCreator();
};

const closeFilterCreator = () => {
  emits("update:isOpen", false);
};

const openFilterCreator = () => {
  emits("update:isOpen", true, props.row.name);
};
</script>

<style lang="scss" scoped>
.show-more-btn {
  &:hover {
    color: $primary;
  }
}
</style>
<style lang="scss">
.filter-check-box {
  .q-checkbox__inner {
    font-size: 24px !important;
  }
}
.q-expansion-item {
  .q-item {
    display: flex;
    align-items: center;
    padding: 0;
    height: 32px !important;
    min-height: 32px !important;
  }
  .q-item__section--avatar {
    min-width: 12px;
    max-width: 12px;
    margin-right: 8px;
  }

  .filter-values-container {
    .q-item {
      padding-left: 4px;

      .q-focus-helper {
        background: none !important;
      }
    }
  }
  .q-item-type {
    &:hover {
      .field_overlay {
        visibility: visible;

        .q-icon {
          opacity: 1;
        }
      }
    }
  }
  .field-expansion-icon {
    margin-right: 4px !important;
    .q-icon {
      font-size: 18px;
      color: #808080;
    }
  }
}
</style>
