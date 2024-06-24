<template>
  <div class="p-4">
    <input
      type="text"
      v-model="filter"
      placeholder="Search"
      class="mb-4 p-2 border border-gray-300 rounded w-full"
    />
    <ul class="list-none p-0">
      <li
        v-for="item in filteredItems"
        :key="item.name"
        class="mb-2 flex items-center"
      >
        <input
          v-if="isValueSelectable"
          v-model="isSelected"
          type="checkbox"
          class="mr-2"
        />
        <span>{{ item.name }}</span>
        <span class="ml-2 text-gray-500">{{ item.count }}</span>
        <template v-for="action in actions" :key="action.name">
          <q-btn
            v-if="action.show"
            size="sm"
            dense
            :icon="action.icon"
            :class="action.class"
            :disable="action.disable"
            round
            @click="handleAction(action)"
          >
            <span style="font-size: 12px"> {{ action.name }}</span>
          </q-btn>
        </template>
      </li>
    </ul>
  </div>
</template>

<script setup>
import { ref, computed, defineProps } from "vue";

const props = defineProps({
  items: {
    type: Array,
    required: true,
  },
  isValueSelectable: {
    type: Boolean,
    default: false,
  },
  actions: {
    type: Array,
    default: () => [],
  },
});

const actions = [
  {
    name: "add-value-to-query",
  }
]

const {}

const filter = ref("");

const isSelected = ref(false);

const filteredItems = computed(() => {
  return props.items.filter((item) =>
    item.name.toLowerCase().includes(filter.value.toLowerCase())
  );
});

const handleAction = (action) => {

};
</script>
