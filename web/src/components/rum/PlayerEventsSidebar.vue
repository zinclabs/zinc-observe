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
  <div class="events-container relative-position">
    <AppTabs :tabs="tabs" v-model:active-tab="activeTab" />
    <template v-if="activeTab === 'tags'">
      <div class="row q-pa-sm event-metadata">
        <div class="col-12 row">
          <div class="col-12 q-pb-sm text-caption">
            <q-icon name="mail" size="16px" class="q-pr-xs" />
            {{ sessionDetails.user_email || "Unknown User" }}
          </div>
          <div class="col-12 q-mb-sm text-caption ellipsis q-pr-xs">
            <q-icon name="schedule" size="16px" class="q-pr-xs" />
            {{ sessionDetails.date }}
          </div>
          <div class="col-12 q-mb-sm text-caption ellipsis q-pr-xs">
            <q-icon name="settings" size="16px" class="q-pr-xs" />
            {{ sessionDetails.browser }}, {{ sessionDetails.os }}
          </div>
          <div class="col-12 q-mb-sm text-caption ellipsis">
            <q-icon name="language" size="16px" class="q-pr-xs" />
            {{ sessionDetails.ip }}
          </div>
          <div class="col-12 q-mb-sm text-caption ellipsis">
            <q-icon name="location_on" size="16px" class="q-pr-xs" />
            {{ sessionDetails.city }}, {{ sessionDetails.country }}
          </div>
        </div>
      </div>
    </template>
    <template v-else>
      <div class="flex items-center justify-between col-12 q-pt-sm">
        <div class="q-pr-xs" style="width: 60%">
          <q-input
            v-model="searchEvent"
            size="xs"
            filled
            borderless
            dense
            clearable
            debounce="1"
            placeholder="Search Event"
            @update:model-value="searchEvents"
          />
        </div>
        <div class="q-pl-xs event-type-selector" style="width: 40%">
          <q-select
            v-model="selectedEventTypes"
            :options="eventOptions"
            behavior="menu"
            multiple
            filled
            borderless
            dense
            emit-value
            size="xs"
            @update:model-value="searchEvents(searchEvent)"
          />
        </div>
      </div>
      <q-separator class="q-mt-sm" />
      <div class="events-list">
        <template
          v-for="(filteredEvent, index) in filteredEvents"
          :key="filteredEvent.id + '-' + index"
        >
          <div
            class="q-mt-xs q-px-sm event-container q-py-sm cursor-pointer rounded-borders"
            @click="handleEventClick(filteredEvent)"
          >
            <div class="ellipsis">
              <div class="q-mr-md inline">{{ filteredEvent.displayTime }}</div>
              <div
                class="q-mr-md inline event-type q-px-xs"
                style="border-radius: 4px"
                :class="filteredEvent.type === 'error' ? 'bg-red-3' : ''"
              >
                {{ filteredEvent.type }}
              </div>
              <div class="inline" :title="filteredEvent.name">
                {{ filteredEvent.name }}
              </div>
            </div>
          </div>
        </template>
      </div>
    </template>
  </div>
</template>

<script lang="ts" setup>
import { ref, watch } from "vue";
import AppTabs from "../common/AppTabs.vue";

const props = defineProps({
  events: {
    type: Array,
    required: true,
  },
  sessionDetails: {
    type: Object,
    required: true,
  },
});

const activeTab = ref<string>("breadcrumbs");
const tabs = [
  {
    label: "Breadcrumbs",
    value: "breadcrumbs",
    style: { width: "fit-content", padding: "8px 10px", "margin-right": "4px" },
  },
  {
    label: "Tags",
    value: "tags",
    style: { width: "fit-content", padding: "8px 10px" },
  },
];

const emit = defineEmits(["event-emitted"]);

const filteredEvents = ref<any[]>([]);

watch(
  () => props.events,
  () => {
    filteredEvents.value = [...props.events];
  },
  { immediate: true, deep: true }
);

const selectedEventTypes = ref<string[]>(["error", "action", "view"]);
const searchEvent = ref<string>("");

const eventOptions = [
  { label: "Error", value: "error" },
  { label: "Action", value: "action" },
  { label: "View", value: "view" },
];

const searchEvents = (value: string | number | null) => {
  if (value === null) {
    value = "";
  }
  const _value = value.toString();
  filteredEvents.value = props.events.filter((event: any) => {
    return (
      selectedEventTypes.value.includes(event.type) &&
      (event.type + " " + event?.name)
        .toLowerCase()
        .includes(_value.toString().toLowerCase())
    );
  });
};

const handleEventClick = (event: any) => {
  emit("event-emitted", "event-click", event);
};
</script>

<style scoped lang="scss">
.inline {
  display: inline;
}

.events-container {
  width: calc(100% - 1px);
  height: calc(100vh - 57px);
  overflow: hidden;
  padding-right: 8px;
  padding-left: 8px;
}

.events-list {
  height: calc(100vh - 207px);
  overflow-x: hidden;
  overflow-y: auto;
}

.event-container:hover {
  background-color: #ededed;
  color: black;
}

.event-type {
  text-transform: capitalize;
}

.event-metadata {
  position: sticky;
  top: 0;
}
</style>

<style lang="scss">
.event-type-selector {
  .q-field__control {
    .q-field__native {
      span {
        text-overflow: ellipsis;
        white-space: nowrap;
        overflow: hidden;
      }
    }
  }
}
</style>
