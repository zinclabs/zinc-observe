<template>
  <div class="full-width q-mb-md q-px-md span-details-container">
    <div
      class="flex justify-between items-center full-width"
      style="border-bottom: 1px solid #e9e9e9"
    >
      <div style="font-size: 18px">
        {{ span.operationName }}
      </div>
      <div class="flex items-center">
        <div style="border-right: 1px solid #cccccc; font-size: 14px">
          <q-btn
            class="q-mx-sm view-span-logs-btn"
            size="10px"
            icon="search"
            dense
            padding="xs sm"
            no-caps
            :title="t('traces.viewLogs')"
            @click.stop="viewSpanLogs"
          >
            View Logs</q-btn
          >
        </div>
        <div
          class="q-px-sm"
          style="border-right: 1px solid #cccccc; font-size: 14px"
        >
          <span class="text-grey-7">Service: </span>
          <span>{{ span.serviceName }}</span>
        </div>
        <div
          class="q-px-sm"
          style="border-right: 1px solid #cccccc; font-size: 14px"
        >
          <span class="text-grey-7">Duration: </span>
          <span>{{ getDuration }}</span>
        </div>
        <div class="q-pl-sm" style="font-size: 14px">
          <span class="text-grey-7">Start Time: </span>
          <span>{{ getStartTime }}</span>
        </div>
      </div>
    </div>
    <div class="q-mt-sm">
      <div>
        <div
          class="flex items-center no-wrap cursor-pointer"
          @click="toggleTags"
        >
          <q-icon
            name="expand_more"
            :class="!areTagsExpanded ? 'rotate-270' : ''"
            size="14px"
            class="cursor-pointer text-grey-7"
          />
          <div class="cursor-pointer text-bold">Tags</div>
          <div
            v-if="!areTagsExpanded"
            class="q-ml-sm"
            style="
              white-space: nowrap;
              overflow: hidden;
              text-overflow: ellipsis;
            "
          >
            <template v-for="(val, key) in tags" :key="key">
              <span class="text-grey-8">{{ key }}</span>
              <span class="text-grey-8">: </span>
              <span class="">{{ val }}</span>
              <span class="q-mx-xs bg-grey-5" style="padding-left: 1px"></span>
            </template>
          </div>
        </div>
        <div
          v-show="areTagsExpanded"
          class="q-px-md flex justify-start items-center"
        >
          <table class="q-my-sm">
            <tbody>
              <template v-for="(val, key) in tags" :key="key">
                <tr>
                  <td
                    class="q-py-xs q-px-sm"
                    :class="
                      store.state.theme === 'dark'
                        ? 'text-red-5'
                        : 'text-red-10'
                    "
                  >
                    {{ key }}
                  </td>
                  <td class="q-py-xs q-px-sm">
                    {{ val }}
                  </td>
                </tr>
              </template>
            </tbody>
          </table>
        </div>
      </div>
      <div>
        <div
          class="flex items-center no-wrap cursor-pointer"
          @click="toggleProcess"
        >
          <q-icon
            name="expand_more"
            :class="!areProcessExpananded ? 'rotate-270' : ''"
            size="14px"
            class="cursor-pointer text-grey-7"
          />
          <div class="cursor-pointer text-bold">Process</div>
          <div
            v-if="!areProcessExpananded"
            class="q-ml-sm"
            style="
              white-space: nowrap;
              overflow: hidden;
              text-overflow: ellipsis;
            "
          >
            <template v-for="(val, key) in processes" :key="key">
              <span class="text-grey-8">{{ key }}</span>
              <span class="text-grey-8">: </span>
              <span class="">{{ val }}</span>
              <span class="q-mx-xs bg-grey-5" style="padding-left: 1px"></span>
            </template>
          </div>
        </div>
        <div v-show="areProcessExpananded" class="q-px-md">
          <table class="q-my-sm">
            <tbody>
              <template v-for="(val, key) in processes" :key="key">
                <tr>
                  <td
                    class="q-py-xs q-px-sm"
                    :class="
                      store.state.theme === 'dark'
                        ? 'text-red-5'
                        : 'text-red-10'
                    "
                  >
                    {{ key }}
                  </td>
                  <td class="q-py-xs q-px-sm">
                    {{ val }}
                  </td>
                </tr>
              </template>
            </tbody>
          </table>
        </div>
      </div>
      <div v-if="events.length">
        <div
          class="flex items-center no-wrap cursor-pointer"
          @click="toggleEvents"
        >
          <q-icon
            name="expand_more"
            :class="!areEventsExpananded ? 'rotate-270' : ''"
            size="14px"
            class="cursor-pointer text-grey-7"
          />
          <div class="cursor-pointer text-bold">Events</div>
          <div
            class="q-ml-sm text-grey-9"
            style="
              white-space: nowrap;
              overflow: hidden;
              text-overflow: ellipsis;
              font-size: 12px;
            "
          >
            {{ events.length }}
          </div>
        </div>
        <div v-show="areEventsExpananded" class="q-px-md q-my-sm">
          <q-virtual-scroll
            type="table"
            ref="searchTableRef"
            style="max-height: 100%"
            :items="events"
          >
            <template v-slot:before>
              <thead class="thead-sticky text-left">
                <tr>
                  <th
                    v-for="(col, index) in eventColumns"
                    :key="'result_' + index"
                    class="table-header"
                    :data-test="`trace-events-table-th-${col.label}`"
                  >
                    {{ col.label }}
                  </th>
                </tr>
              </thead>
            </template>

            <template v-slot="{ item: row, index }">
              <q-tr
                :data-test="`trace-event-detail-${
                  row[store.state.zoConfig.timestamp_column]
                }`"
                :key="'expand_' + index"
                @click="expandEvent(index)"
                style="cursor: pointer"
                class="pointer"
              >
                <q-td
                  v-for="column in eventColumns"
                  :key="index + '-' + column.name"
                  class="field_list"
                  style="cursor: pointer"
                >
                  <div class="flex row items-center no-wrap">
                    <q-btn
                      v-if="column.name === '@timestamp'"
                      :icon="
                        expandedEvents[index.toString()]
                          ? 'expand_more'
                          : 'chevron_right'
                      "
                      dense
                      size="xs"
                      flat
                      class="q-mr-xs"
                      @click.stop="expandEvent(index)"
                    ></q-btn>
                    {{ column.prop(row) }}
                  </div>
                </q-td>
              </q-tr>
              <q-tr v-if="expandedEvents[index.toString()]">
                <td colspan="2">
                  <pre class="log_json_content">{{ row }}</pre>
                </td>
              </q-tr>
            </template>
          </q-virtual-scroll>
          <div
            class="full-width text-center q-pt-lg text-bold"
            v-if="!events.length"
          >
            No events present for this span
          </div>
        </div>
      </div>
      <div v-if="getExceptionEvents.length">
        <div
          class="flex items-center no-wrap cursor-pointer"
          @click="toggleExceptions"
        >
          <q-icon
            name="expand_more"
            :class="!isExceptionExpanded ? 'rotate-270' : ''"
            size="14px"
            class="cursor-pointer text-grey-7"
          />
          <div class="cursor-pointer text-bold">Exceptions</div>
          <div
            class="q-ml-sm text-grey-9"
            style="
              white-space: nowrap;
              overflow: hidden;
              text-overflow: ellipsis;
              font-size: 12px;
            "
          >
            {{ getExceptionEvents.length }}
          </div>
        </div>
        <div v-show="isExceptionExpanded" class="q-px-md q-my-sm">
          <q-virtual-scroll
            type="table"
            ref="searchTableRef"
            style="max-height: 100%"
            :items="getExceptionEvents"
          >
            <template v-slot:before>
              <thead class="thead-sticky text-left">
                <tr>
                  <th
                    v-for="(col, index) in exceptionEventColumns"
                    :key="'result_' + index"
                    class="table-header"
                    :data-test="`trace-events-table-th-${col.label}`"
                  >
                    {{ col.label }}
                  </th>
                </tr>
              </thead>
            </template>

            <template v-slot="{ item: row, index }">
              <q-tr
                :data-test="`trace-event-detail-${
                  row[store.state.zoConfig.timestamp_column]
                }`"
                :key="'expand_' + index"
                @click="expandEvent(index)"
                style="cursor: pointer"
                class="pointer"
              >
                <q-td
                  v-for="column in exceptionEventColumns"
                  :key="index + '-' + column.name"
                  class="field_list"
                  style="cursor: pointer"
                >
                  <div class="flex row items-center no-wrap">
                    <q-btn
                      v-if="column.name === '@timestamp'"
                      :icon="
                        expandedEvents[index.toString()]
                          ? 'expand_more'
                          : 'chevron_right'
                      "
                      dense
                      size="xs"
                      flat
                      class="q-mr-xs"
                      @click.stop="expandEvent(index)"
                    ></q-btn>
                    {{ column.prop(row) }}
                  </div>
                </q-td>
              </q-tr>
              <q-tr v-if="expandedEvents[index.toString()]">
                <td colspan="2" style="font-size: 12px; font-family: monospace">
                  <div class="q-pl-sm">
                    <div>
                      <span>Type: </span>
                      <span>"{{ row["exception.type"] }}"</span>
                    </div>

                    <div class="q-mt-xs">
                      <span>Message: </span>
                      <span>"{{ row["exception.message"] }}"</span>
                    </div>

                    <div class="q-mt-xs">
                      <span>Escaped: </span>
                      <span>"{{ row["exception.escaped"] }}"</span>
                    </div>

                    <div class="q-mt-xs">
                      <span>Stacktrace: </span>
                      <div
                        class="q-px-sm q-mt-xs"
                        style="border: 1px solid #c1c1c1; border-radius: 4px"
                      >
                        <pre
                          style="font-size: 12px; text-wrap: wrap"
                          class="q-mt-xs"
                          >{{ row["exception.stacktrace"] }}</pre
                        >
                      </div>
                    </div>
                  </div>
                </td>
              </q-tr>
            </template>
          </q-virtual-scroll>
          <div
            class="full-width text-center q-pt-lg text-bold"
            v-if="!getExceptionEvents.length"
          >
            No events present for this span
          </div>
        </div>
      </div>
      <div class="text-right flex items-center justify-end">
        <span class="text-grey-7 q-mr-xs">Span Id: </span
        ><span class="">{{ span.spanId }}</span>
        <q-icon
          class="q-ml-xs text-grey-8 cursor-pointer trace-copy-icon"
          size="12px"
          name="content_copy"
          title="Copy"
          @click="copySpanId"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Ref } from "vue";
import { watch } from "vue";
import { computed } from "vue";
import { ref } from "vue";
import { useStore } from "vuex";
import { formatTimeWithSuffix } from "@/utils/zincutils";
import { date, useQuasar } from "quasar";
import { copyToClipboard } from "quasar";
import { useI18n } from "vue-i18n";

const props = defineProps({
  span: {
    type: Object,
    default: () => ({}),
  },
  spanData: {
    type: Object,
    default: () => ({}),
  },
  baseTracePosition: {
    type: Object,
    default: () => null,
  },
});

const emit = defineEmits(["view-logs"]);

const store = useStore();

const { t } = useI18n();

const getDuration = computed(() => formatTimeWithSuffix(props.span.durationUs));

const getStartTime = computed(() => {
  return props.span.startTimeMs - props.baseTracePosition.startTimeMs + "ms";
});

const $q = useQuasar();

const span_details = new Set([
  "span_id",
  "trace_id",
  "operation_name",
  store.state.zoConfig.timestamp_column,
  "start_time",
  "end_time",
  "duration",
  "busy_ns",
  "idle_ns",
  "events",
]);

const tags: Ref<{ [key: string]: string }> = ref({});
const processes: Ref<{ [key: string]: string }> = ref({});

const events: Ref<any[]> = ref([]);

const eventColumns = ref([
  {
    name: "@timestamp",
    field: (row: any) =>
      date.formatDate(
        Math.floor(row[store.state.zoConfig.timestamp_column] / 1000000),
        "MMM DD, YYYY HH:mm:ss.SSS Z"
      ),
    prop: (row: any) =>
      date.formatDate(
        Math.floor(row[store.state.zoConfig.timestamp_column] / 1000000),
        "MMM DD, YYYY HH:mm:ss.SSS Z"
      ),
    label: "Timestamp",
    align: "left",
    sortable: true,
  },
  {
    name: "source",
    field: (row: any) => JSON.stringify(row),
    prop: (row: any) => JSON.stringify(row),
    label: "source",
    align: "left",
    sortable: true,
  },
]);

const exceptionEventColumns = ref([
  {
    name: "@timestamp",
    field: (row: any) =>
      date.formatDate(
        Math.floor(row[store.state.zoConfig.timestamp_column] / 1000000),
        "MMM DD, YYYY HH:mm:ss.SSS Z"
      ),
    prop: (row: any) =>
      date.formatDate(
        Math.floor(row[store.state.zoConfig.timestamp_column] / 1000000),
        "MMM DD, YYYY HH:mm:ss.SSS Z"
      ),
    label: "Timestamp",
    align: "left",
    sortable: true,
  },
  {
    name: "type",
    field: (row: any) => row["exception.type"],
    prop: (row: any) => row["exception.type"],
    label: "Type",
    align: "left",
    sortable: true,
  },
]);

const setSpanEvents = () => {
  if (events.value) events.value = [];

  events.value = JSON.parse(props.spanData.events).map((event: any) => event);
};

const getExceptionEvents = computed(() => {
  return events.value.filter((event: any) => event.name === "exception");
});

watch(
  () => props.spanData,
  () => {
    Object.keys(props.spanData).forEach((key: string) => {
      if (!span_details.has(key)) {
        tags.value[key] = props.spanData[key];
      }
    });

    processes.value["service_name"] = props.spanData["service_name"];
    processes.value["service_service_instance"] =
      props.spanData["service_service_instance"];
    processes.value["service_service_version"] =
      props.spanData["service_service_version"];

    setSpanEvents();
  },
  {
    deep: true,
    immediate: true,
  }
);

const areTagsExpanded = ref(false);

const expandedEvents: any = ref({});

const areProcessExpananded = ref(false);

const areEventsExpananded = ref(false);

const isExceptionExpanded = ref(false);

const toggleProcess = () => {
  areProcessExpananded.value = !areProcessExpananded.value;
};

const toggleTags = () => {
  areTagsExpanded.value = !areTagsExpanded.value;
};

const toggleEvents = () => {
  areEventsExpananded.value = !areEventsExpananded.value;
};

const toggleExceptions = () => {
  isExceptionExpanded.value = !isExceptionExpanded.value;
};

const expandEvent = (index: number) => {
  if (expandedEvents.value[index.toString()])
    delete expandedEvents.value[index.toString()];
  else expandedEvents.value[index.toString()] = true;
};

const viewSpanLogs = () => {
  emit("view-logs", props.span.spanId);
};

const copySpanId = () => {
  $q.notify({
    type: "positive",
    message: "Span ID copied to clipboard",
    timeout: 2000,
  });
  copyToClipboard(props.span.spanId);
};
</script>

<style scoped lang="scss">
.span-details-container {
  table {
    border-collapse: collapse;
    width: 100%;
    /* Other styling properties */
  }

  th,
  td {
    border: 1px solid #f0f0f0;
    text-align: left;
    padding: 4px 8px !important;
    font-size: 13px;
    /* Other styling properties */
  }
}

.attr-text {
  font-size: 12px;
  font-family: monospace;
}
.table-header {
  // text-transform: capitalize;

  .table-head-chip {
    background-color: $accent;
    padding: 0px;

    .q-chip__content {
      margin-right: 0.5rem;
      font-size: 0.75rem;
      color: $dark;
    }

    .q-chip__icon--remove {
      height: 1rem;
      width: 1rem;
      opacity: 1;
      margin: 0;

      &:hover {
        opacity: 0.7;
      }
    }

    .q-table th.sortable {
      cursor: pointer;
      text-transform: capitalize;
      font-weight: bold;
    }
  }

  &.isClosable {
    padding-right: 26px;
    position: relative;

    .q-table-col-close {
      transform: translateX(26px);
      position: absolute;
      margin-top: 2px;
      color: grey;
      transition: transform 0.3s cubic-bezier(0.25, 0.8, 0.5, 1);
    }
  }

  .q-table th.sortable {
    cursor: pointer;
    text-transform: capitalize;
    font-weight: bold;
  }

  .log_json_content {
    white-space: pre-wrap;
  }
}
.q-table__top {
  padding-left: 0;
  padding-top: 0;
}

.q-table thead tr,
.q-table tbody td,
.q-table th,
.q-table td {
  height: 25px;
  padding: 0px 5px;
  font-size: 0.75rem;
}

.q-table__bottom {
  width: 100%;
}

.q-table__bottom {
  min-height: 40px;
  padding-top: 0;
  padding-bottom: 0;
}

.q-td {
  overflow: hidden;
  min-width: 100px;

  .expanded {
    margin: 0;
    white-space: pre-wrap;
    word-wrap: break-word;
    word-break: break-all;
  }
}

.thead-sticky tr > *,
.tfoot-sticky tr > * {
  position: sticky;
  opacity: 1;
  z-index: 1;
  background: #f5f5f5;
}

.q-table--dark .thead-sticky tr > *,
.q-table--dark .tfoot-sticky tr > * {
  background: #565656;
}
.thead-sticky tr:last-child > * {
  top: 0;
}

.tfoot-sticky tr:first-child > * {
  bottom: 0;
}

.field_list {
  padding: 0px;
  margin-bottom: 0.125rem;
  position: relative;
  overflow: visible;
  cursor: default;
  font-size: 12px;
  font-family: monospace;

  .field_overlay {
    position: absolute;
    height: 100%;
    right: 0;
    top: 0;
    background-color: #ffffff;
    border-radius: 6px;
    padding: 0 6px;
    visibility: hidden;
    display: flex;
    align-items: center;
    transition: all 0.3s linear;

    .q-icon {
      cursor: pointer;
      opacity: 0;
      transition: all 0.3s linear;
      margin: 0 1px;
    }
  }

  &:hover {
    .field_overlay {
      visibility: visible;

      .q-icon {
        opacity: 1;
      }
    }
  }
}
.span_details_tab-panels {
  height: calc(100% - 102px);
  overflow-y: auto;
  overflow-x: hidden;
}

.hearder_bg {
  border-top: 1px solid $border-color;
  background-color: color-mix(in srgb, currentColor 5%, transparent);
}
</style>
<style lang="scss">
.tags-expander {
  .q-item {
    width: fit-content;
    height: fit-content !important;
    min-height: fit-content !important;
    padding: 0 4px !important;
    border-radius: 4px;
    background-color: $primary;
  }

  .q-item__section {
    padding-right: 0;
  }

  .q-icon {
    font-size: 16px;
    color: #ffffff;
  }
}
</style>
