<template>
  <div class="q-py-xs flex justify-start q-px-md log-detail-actions">
    <app-tabs
      class="logs-json-preview-tabs q-mr-sm"
      style="border: 1px solid #8a8a8a; border-radius: 4px; overflow: hidden"
      data-test="logs-json-preview-tabs"
      :tabs="tabs"
      v-model:active-tab="activeTab"
      @update:active-tab="handleTabChange"
    />

    <q-btn
      :label="t('common.copyToClipboard')"
      dense
      size="sm"
      no-caps
      class="q-px-sm copy-log-btn q-mr-sm"
      icon="content_copy"
      @click="copyLogToClipboard"
    />

    <div
      v-if="showViewTraceBtn"
      class="o2-input flex items-center logs-trace-selector"
    >
      <q-select
        data-test="log-search-index-list-select-stream"
        v-model="searchObj.meta.selectedTraceStream"
        :label="
          searchObj.meta.selectedTraceStream ? '' : t('search.selectIndex')
        "
        :options="filteredTracesStreamOptions"
        data-cy="stream-selection"
        input-debounce="0"
        behavior="menu"
        filled
        size="xs"
        borderless
        dense
        fill-input
        :title="searchObj.meta.selectedTraceStream"
      >
        <template #no-option>
          <div class="o2-input log-stream-search-input">
            <q-input
              data-test="alert-list-search-input"
              v-model="streamSearchValue"
              borderless
              filled
              debounce="500"
              autofocus
              dense
              size="xs"
              @update:model-value="filterStreamFn"
              class="q-ml-auto q-mb-xs no-border q-pa-xs"
              :placeholder="t('search.searchStream')"
            >
              <template #prepend>
                <q-icon name="search" class="cursor-pointer" />
              </template>
            </q-input>
          </div>
          <q-item>
            <q-item-section> {{ t("search.noResult") }}</q-item-section>
          </q-item>
        </template>
        <template #before-options>
          <div class="o2-input log-stream-search-input">
            <q-input
              data-test="alert-list-search-input"
              v-model="streamSearchValue"
              borderless
              debounce="500"
              filled
              dense
              size="xs"
              autofocus
              @update:model-value="filterStreamFn"
              class="q-ml-auto q-mb-xs no-border q-pa-xs"
              :placeholder="t('search.searchStream')"
            >
              <template #prepend>
                <q-icon name="search" class="cursor-pointer" />
              </template>
            </q-input>
          </div>
        </template>
      </q-select>
      <q-btn
        data-test="trace-view-logs-btn"
        v-close-popup="true"
        class="text-bold traces-view-logs-btn q-px-sm view-trace-btn"
        :label="t('search.viewTrace')"
        padding="sm sm"
        size="xs"
        no-caps
        dense
        :icon="outlinedAccountTree"
        @click="redirectToTraces"
      />
    </div>
  </div>
  <div v-show="activeTab === 'unflattened'" class="q-pl-md">
    <query-editor
      v-model:query="nestedJson"
      ref="queryEditorRef"
      editor-id="logs-json-preview-unflattened-json-editor"
      class="monaco-editor"
      language="json"
    />
  </div>
  <div v-show="activeTab !== 'unflattened'" class="q-pl-md">
    {
    <div
      class="log_json_content"
      v-for="(key, index) in Object.keys(value)"
      :key="key"
    >
      <q-btn-dropdown
        data-test="log-details-include-exclude-field-btn"
        size="0.5rem"
        flat
        outlined
        filled
        dense
        class="q-ml-sm pointer"
        :name="'img:' + getImageURL('images/common/add_icon.svg')"
      >
        <q-list>
          <q-item
            clickable
            v-close-popup
            v-if="
              searchObj.data.stream.selectedStreamFields.some((item: any) =>
                item.name === key ? item.isSchemaField : '',
              ) && multiStreamFields.includes(key)
            "
          >
            <q-item-section>
              <q-item-label
                data-test="log-details-include-field-btn"
                @click.stop="addSearchTerm(`${key}='${value[key]}'`)"
                v-close-popup
                ><q-btn
                  title="Add to search query"
                  size="6px"
                  round
                  class="q-mr-sm pointer"
                >
                  <q-icon color="currentColor">
                    <EqualIcon></EqualIcon>
                  </q-icon> </q-btn
                >{{ t("common.includeSearchTerm") }}</q-item-label
              >
            </q-item-section>
          </q-item>

          <q-item
            clickable
            v-close-popup
            v-if="
              searchObj.data.stream.selectedStreamFields.some((item: any) =>
                item.name === key ? item.isSchemaField : '',
              ) && multiStreamFields.includes(key)
            "
          >
            <q-item-section>
              <q-item-label
                data-test="log-details-exclude-field-btn"
                @click.stop="addSearchTerm(`${key}!='${value[key]}'`)"
                v-close-popup
                ><q-btn
                  title="Add to search query"
                  size="6px"
                  round
                  class="q-mr-sm pointer"
                >
                  <q-icon color="currentColor">
                    <NotEqualIcon></NotEqualIcon>
                  </q-icon> </q-btn
                >{{ t("common.excludeSearchTerm") }}</q-item-label
              >
            </q-item-section>
          </q-item>
          <q-item clickable v-close-popup>
            <q-item-section>
              <q-item-label
                data-test="log-details-add-field-btn"
                @click.stop="addFieldToTable(key)"
                v-close-popup
                ><q-btn
                  title="Add field to table"
                  icon="visibility"
                  size="6px"
                  round
                  class="q-mr-sm pointer"
                ></q-btn
                >{{ t("common.addFieldToTable") }}</q-item-label
              >
            </q-item-section>
          </q-item>
        </q-list>
      </q-btn-dropdown>

      <span class="q-pl-xs">
        <span
          :class="store.state.theme === 'dark' ? 'text-red-5' : 'text-red-10'"
          >{{ key }}:</span
        ><span class="q-pl-xs"
          ><template v-if="index < Object.keys(value).length - 1"
            >{{ value[key] }},</template
          >
          <template v-else>
            {{ value[key] }}
          </template>
        </span>
      </span>
    </div>
    }
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onBeforeMount,
  computed,
  onMounted,
  watch,
  nextTick,
} from "vue";
import { getImageURL } from "@/utils/zincutils";
import { useStore } from "vuex";
import EqualIcon from "@/components/icons/EqualIcon.vue";
import NotEqualIcon from "@/components/icons/NotEqualIcon.vue";
import { useI18n } from "vue-i18n";
import useLogs from "../../composables/useLogs";
import { outlinedAccountTree } from "@quasar/extras/material-icons-outlined";
import { useRouter } from "vue-router";
import useStreams from "@/composables/useStreams";
import AppTabs from "@/components/common/AppTabs.vue";
import QueryEditor from "@/components/QueryEditor.vue";

export default {
  name: "JsonPreview",
  props: {
    value: {
      type: Object,
      required: true,
      default: () => ({}),
    },
    showCopyButton: {
      type: Boolean,
      default: true,
    },
  },
  components: { NotEqualIcon, EqualIcon, AppTabs, QueryEditor },
  emits: ["copy", "addSearchTerm", "addFieldToTable", "view-trace"],
  setup(props: any, { emit }: any) {
    const { t } = useI18n();
    const store = useStore();

    const streamSearchValue = ref<string>("");

    const { getStreams } = useStreams();

    const filteredTracesStreamOptions = ref([]);

    const tracesStreams = ref([]);

    const activeTab = ref("flattened");

    const queryEditorRef = ref<any>();

    const nestedJson = ref("");

    const tabs = [
      {
        value: "flattened",
        label: t("search.flattened"),
      },
      {
        value: "unflattened",
        label: t("search.unflattened"),
      },
    ];

    const copyLogToClipboard = () => {
      emit(
        "copy",
        activeTab.value === "unflattened"
          ? JSON.parse(nestedJson.value)
          : props.value,
      );
    };
    const addSearchTerm = (value: string) => {
      emit("addSearchTerm", value);
    };
    const addFieldToTable = (value: string) => {
      emit("addFieldToTable", value);
    };
    const { searchObj } = useLogs();
    let multiStreamFields: any = ref([]);

    onBeforeMount(() => {
      searchObj.data.stream.selectedStreamFields.forEach((item: any) => {
        if (
          item.streams.length == searchObj.data.stream.selectedStream.length
        ) {
          multiStreamFields.value.push(item.name);
        }
      });
      getTracesStreams();
    });

    onMounted(() => {
      nestedJson.value = getNestedJson();
    });

    watch(
      () => props.value,
      () => {
        nestedJson.value = getNestedJson();
      },
    );

    const getTracesStreams = async () => {
      await getStreams("traces", false)
        .then((res: any) => {
          tracesStreams.value = res.list.map((option: any) => option.name);
          filteredTracesStreamOptions.value = JSON.parse(
            JSON.stringify(tracesStreams.value),
          );

          console.log("tracesStreams", tracesStreams.value);

          if (!searchObj.meta.selectedTraceStream.length)
            searchObj.meta.selectedTraceStream = tracesStreams.value[0];
        })
        .catch(() => Promise.reject())
        .finally(() => {});
    };

    const filterStreamFn = (val: any = "") => {
      filteredTracesStreamOptions.value = tracesStreams.value.filter(
        (stream: any) => {
          return stream.toLowerCase().indexOf(val.toLowerCase()) > -1;
        },
      );
    };

    const redirectToTraces = () => {
      emit("view-trace");
    };

    const showViewTraceBtn = computed(() => {
      return (
        !store.state.hiddenMenus.has("traces") && // Check if traces menu is hidden
        filteredTracesStreamOptions.value.length && // Check if traces streams are available
        props.value[
          store.state.organizationData?.organizationSettings
            ?.trace_id_field_name
        ] // Check if trace_id_field_name is available in the log fields
      );
    });

    const getNestedJson = () => {
      const result = {};

      Object.keys(props.value).forEach((key) => {
        let keys = key.split("_");

        // If any field starts with _
        let keyWithPrefix = "";
        for (let i = 0; i < keys.length; i++) {
          if (keys[i] === "") {
            keyWithPrefix += "_";
          } else {
            if (keyWithPrefix.length) {
              keyWithPrefix += keys[i];
              keys[i] = keyWithPrefix;
              keyWithPrefix = "";
            }
          }
        }

        keys = keys.filter((k) => k !== "");

        type NestedObject = {
          [key: string]: NestedObject | any;
        };

        // { [key: string]: string }
        keys.reduce((acc: NestedObject, k: string, index: number) => {
          if (index === keys.length - 1) {
            acc[k] = props.value[key];
          } else {
            if (!(k in acc)) {
              acc[k] = {};
            }
          }
          return acc[k];
        }, result);
      });

      return JSON.stringify(result);
    };

    const handleTabChange = async () => {
      if (activeTab.value === "unflattened") {
        await nextTick();
        queryEditorRef.value.formatDocument();
      }
    };

    return {
      t,
      copyLogToClipboard,
      getImageURL,
      addSearchTerm,
      addFieldToTable,
      outlinedAccountTree,
      store,
      searchObj,
      multiStreamFields,
      redirectToTraces,
      filteredTracesStreamOptions,
      filterStreamFn,
      streamSearchValue,
      showViewTraceBtn,
      tabs,
      activeTab,
      nestedJson,
      handleTabChange,
      queryEditorRef,
    };
  },
};
</script>

<style lang="scss" scoped>
.log_json_content {
  white-space: pre-wrap;
  font-family: monospace;
  font-size: 12px;
}
.monaco-editor {
  width: calc(100% - 16px) !important;
  height: calc(100vh - 250px) !important;
}
</style>

<style lang="scss">
.logs-trace-selector {
  .q-select {
    .q-field__control {
      min-height: 30px !important;
      height: 30px !important;
      padding: 0px 8px !important;
      width: 220px !important;

      .q-field,
      .q-field__native {
        span {
          display: inline-block;
          width: 180px;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          text-align: left;
        }
      }

      .q-field__append {
        height: 27px !important;
      }
    }
  }

  .q-btn {
    height: 30px !important;
    padding: 2px 8px !important;
    margin-left: -1px;

    .q-btn__content {
      span {
        font-size: 11px;
      }
    }
  }
}

.logs-json-preview-tabs {
  height: fit-content;
  .rum-tab {
    width: fit-content !important;
    padding: 2px 12px !important;
    border: none !important;
    font-size: 12px !important;

    &.active {
      background: #5960b2;
      color: #ffffff !important;
    }
  }
}
</style>
