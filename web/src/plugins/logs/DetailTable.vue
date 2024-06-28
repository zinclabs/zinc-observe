<!-- Copyright 2023 Zinc Labs Inc.

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
  <q-card
    class="column full-height no-wrap searchdetaildialog"
    data-test="dialog-box"
  >
    <q-card-section class="q-pa-md q-pb-md">
      <div class="row items-center no-wrap">
        <div class="col">
          <div class="text-body1 text-bold" data-test="log-detail-title-text">
            {{ t("search.rowDetail") }}
          </div>
        </div>
        <div class="col-auto">
          <q-btn
            v-close-popup="true"
            round
            flat
            icon="cancel"
            data-test="close-dialog"
          />
        </div>
      </div>
    </q-card-section>
    <q-separator />
    <div class="row justify-between">
      <div class="col-10">
        <q-tabs v-model="tab" shrink align="left">
          <q-tab
            data-test="log-detail-json-tab"
            name="json"
            :label="t('common.json')"
          />
          <q-tab
            data-test="log-detail-table-tab"
            name="table"
            :label="t('common.table')"
          />
        </q-tabs>
      </div>
      <div
        v-show="tab === 'table'"
        class="col-2 flex justify-end align-center q-pr-md"
      >
        <q-toggle
          data-test="log-detail-wrap-values-toggle-btn"
          v-model="shouldWrapValues"
          :label="t('common.wrap')"
          color="primary"
          size="xs"
          @update:model-value="toggleWrapLogDetails"
        />
      </div>
    </div>

    <q-separator />

    <q-tab-panels
      data-test="log-detail-tab-container"
      class="tab-panels-container"
      v-model="tab"
      animated
    >
      <q-tab-panel name="json" class="q-pa-none">
        <q-card-section
          data-test="log-detail-json-content"
          class="q-pa-none q-mb-lg"
        >
          <json-preview
            :value="rowData"
            show-copy-button
            @copy="copyContentToClipboard"
            @add-field-to-table="addFieldToTable"
            @add-search-term="toggleIncludeSearchTerm"
          />
        </q-card-section>
      </q-tab-panel>
      <q-tab-panel name="table" class="q-pa-none">
        <q-card-section
          class="q-pa-none q-mb-lg"
          data-test="log-detail-table-content"
        >
          <div
            v-if="rowData.length == 0"
            class="q-pt-md"
            style="max-width: 350px"
          >
            No data available.
          </div>
          <div v-else class="indexDetailsContainer">
            <q-list
              separator
              style="height: calc(100vh - 290)"
              class="q-px-none q-py-none"
            >
              <q-item class="list-head">
                <q-item-section class="text-bold col-3">
                  {{ t("search.sourceName") }}
                </q-item-section>
                <q-item-section class="text-bold col-9">
                  {{ t("search.sourceValue") }}
                </q-item-section>
              </q-item>

              <q-item
                v-for="(key, value) in rowData"
                :key="'field_' + value"
                class="list-item"
              >
                <q-item-section
                  :data-test="`log-detail-${value}-key`"
                  class="col-3 text-weight-regular text-red-10"
                  >{{ value }}</q-item-section
                >
                <q-item-section
                  class="col-9"
                  :class="!shouldWrapValues ? 'ellipsis' : ''"
                  style="display: inline;relative"
                >
                  <q-btn-dropdown
                    :data-test="`log-details-include-exclude-field-btn-${value}`"
                    size="6px"
                    outlined
                    filled
                    dense
                    class="q-mr-sm pointer"
                    name="'img:' + getImageURL('images/common/add_icon.svg')"
                  >
                    <q-list data-test="field-list-modal">
                      <q-item
                        clickable
                        v-close-popup="true"
                        v-if="searchObj.data.stream.selectedStreamFields.some(
                                (item: any) =>
                                  item.name === value
                                    ? item.isSchemaField
                                    : ''
                              )
                            "
                      >
                        <q-item-section>
                          <q-item-label
                            data-test="log-details-include-field-btn"
                            @click="
                              toggleIncludeSearchTerm(`${value}='${key}'`)
                            "
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
                        v-close-popup="true"
                        v-if="searchObj.data.stream.selectedStreamFields.some(
                                (item: any) =>
                                  item.name === value
                                    ? item.isSchemaField
                                    : ''
                              )
                            "
                      >
                        <q-item-section>
                          <q-item-label
                            data-test="log-details-exclude-field-btn"
                            @click="
                              toggleExcludeSearchTerm(`${value}!='${key}'`)
                            "
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

                      <q-item
                        v-if="
                          !searchObj.data.stream.selectedFields.includes(
                            value.toString()
                          )
                        "
                        clickable
                        v-close-popup="true"
                      >
                        <q-item-section>
                          <q-item-label
                            data-test="log-details-include-field-btn"
                            @click="addFieldToTable(value.toString())"
                            ><q-btn
                              title="Add to table"
                              size="6px"
                              round
                              class="q-mr-sm pointer"
                            >
                              <q-icon
                                color="currentColor"
                                name="visibility" /></q-btn
                            >{{ t("common.addFieldToTable") }}</q-item-label
                          >
                        </q-item-section>
                      </q-item>
                      <q-item v-else clickable v-close-popup="true">
                        <q-item-section>
                          <q-item-label
                            data-test="log-details-include-field-btn"
                            @click="addFieldToTable(value.toString())"
                            ><q-btn
                              title="Remove from table "
                              size="6px"
                              round
                              class="q-mr-sm pointer"
                            >
                              <q-icon
                                color="currentColor"
                                name="visibility_off" /></q-btn
                            >{{
                              t("common.removeFieldFromTable")
                            }}</q-item-label
                          >
                        </q-item-section>
                      </q-item>
                    </q-list>
                  </q-btn-dropdown>

                  <pre
                    :data-test="`log-detail-${value}-value`"
                    class="table-pre"
                    :style="{
                      'white-space': !shouldWrapValues ? 'nowrap' : 'pre-wrap',
                    }"
                    >{{ key }}</pre
                  >
                </q-item-section>
              </q-item>
            </q-list>
          </div>
        </q-card-section>
      </q-tab-panel>
    </q-tab-panels>

    <q-separator />
    <q-card-section class="q-pa-md q-pb-md">
      <div class="row items-center no-wrap justify-between">
        <div class="col-2">
          <q-btn
            data-test="log-detail-previous-detail-btn"
            class="text-bold"
            text-color="light-text"
            no-caps
            :disabled="currentIndex <= 0"
            outline
            @click="$emit('showPrevDetail', false, true)"
            icon="navigate_before"
            :label="t('common.previous')"
          />
        </div>
        <div
          v-show="streamType !== 'enrichment_tables' && searchObj.data.stream.selectedStream.length <= 1"
          class="col-8 row justify-center align-center q-gutter-sm"
        >
          <div style="line-height: 40px; font-weight: bold">
            {{ t("common.noOfRecords") }}
          </div>
          <div class="" style="minwidth: 150px">
            <q-select
              v-model="selectedRelativeValue"
              :options="recordSizeOptions"
              dense
              filled
            ></q-select>
          </div>
          <div class="">
            <q-btn
              data-test="logs-detail-table-search-around-btn"
              class="text-bold"
              text-color="light-text"
              no-caps
              outline
              :label="t('common.searchAround')"
              @click="searchTimeBoxed(rowData, Number(selectedRelativeValue))"
              padding="sm md"
            />
          </div>
        </div>
        <div class="col-2 items-end">
          <q-btn
            data-test="log-detail-next-detail-btn"
            class="text-bold"
            text-color="light-text"
            no-caps
            :disabled="currentIndex >= totalLength - 1"
            outline
            @click="$emit('showNextDetail', true, false)"
            icon-right="navigate_next"
            :label="t('common.next')"
          />
        </div>
      </div>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { defineComponent, ref, onBeforeMount } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useStore } from "vuex";
import { getImageURL } from "../../utils/zincutils";
import useLogs from "@/composables/useLogs";
import EqualIcon from "@/components/icons/EqualIcon.vue";
import NotEqualIcon from "@/components/icons/NotEqualIcon.vue";
import { copyToClipboard, useQuasar } from "quasar";
import JsonPreview from "./JsonPreview.vue";

const defaultValue: any = () => {
  return {
    data: {},
  };
};

export default defineComponent({
  name: "SearchDetail",
  components: { EqualIcon, NotEqualIcon, JsonPreview },
  emits: [
    "showPrevDetail",
    "showNextDetail",
    "add:searchterm",
    "remove:searchterm",
    "search:timeboxed",
    "add:table",
  ],
  props: {
    modelValue: {
      type: Object,
      default: () => defaultValue(),
    },
    currentIndex: {
      type: Number,
      required: true,
    },
    totalLength: {
      type: Number,
      required: true,
    },
    streamType: {
      type: String,
      default: "logs",
    },
  },
  methods: {
    toggleIncludeSearchTerm(term: string) {
      // if (flag == false) {
      this.$emit("add:searchterm", term);
      // } else {
      //   this.$emit("remove:searchterm", term);
      // }
    },
    toggleExcludeSearchTerm(term: string) {
      // if (flag == false) {
      this.$emit("add:searchterm", term);
      // } else {
      //   this.$emit("remove:searchterm", term);
      // }
    },
    searchTimeBoxed(rowData: any, size: number) {
      this.$emit("search:timeboxed", {
        key: rowData[this.store.state.zoConfig.timestamp_column],
        size: size,
      });
    },
  },
  setup(props, { emit }) {
    const { t } = useI18n();
    const rowData: any = ref({});
    const router = useRouter();
    const store = useStore();
    const tab = ref("json");
    const selectedRelativeValue = ref("10");
    const recordSizeOptions: any = ref([10, 20, 50, 100, 200, 500, 1000]);
    const shouldWrapValues: any = ref(true);
    const { searchObj } = useLogs();
    const $q = useQuasar();
    let multiStreamFields: any = ref([]);

    onBeforeMount(() => {
      if (window.localStorage.getItem("wrap-log-details") === null) {
        window.localStorage.setItem("wrap-log-details", "true");
      }
      shouldWrapValues.value =
        window.localStorage.getItem("wrap-log-details") === "true";

      searchObj.data.stream.selectedStreamFields.forEach((item: any) => {
        if (
          item.streams.length == searchObj.data.stream.selectedStream.length
        ) {
          multiStreamFields.value.push(item.name);
        }
      });
    });

    const toggleWrapLogDetails = () => {
      window.localStorage.setItem(
        "wrap-log-details",
        shouldWrapValues.value ? "true" : "false"
      );
    };

    const flattenJSONObject = (obj: any, param: string) => {
      let newObj: any = {};
      for (let key in obj) {
        if (typeof obj[key] === "object") {
          // let childJSON = JSON.stringify(obj[key]);
          // let unflatten = flattenJSONObject(obj[key], key + ".");
          newObj = {
            ...newObj,
            ...flattenJSONObject(obj[key], param + key + "."),
          };
        } else {
          newObj[param + key] = obj[key];
        }
      }
      return newObj;
    };

    const copyContentToClipboard = (log: any) => {
      copyToClipboard(JSON.stringify(log)).then(() =>
        $q.notify({
          type: "positive",
          message: "Content Copied Successfully!",
          timeout: 1000,
        })
      );
    };

    const addFieldToTable = (value: string) => {
      emit("add:table", value);
    };

    return {
      t,
      store,
      router,
      rowData,
      tab,
      flattenJSONObject,
      selectedRelativeValue,
      recordSizeOptions,
      getImageURL,
      shouldWrapValues,
      toggleWrapLogDetails,
      copyContentToClipboard,
      addFieldToTable,
      searchObj,
      multiStreamFields,
    };
  },
  async created() {
    const newObj = await this.flattenJSONObject(this.modelValue, "");
    this.rowData = newObj;
  },
});
</script>

<style lang="scss" scoped>
.searchdetaildialog {
  width: 70vw;
}

.q-item__section {
  word-break: break-all;
}

.indexDetailsContainer .q-list .q-item {
  height: auto;
  min-height: fit-content;
  padding: 2px 8px;
}

.indexDetailsContainer .q-list .q-item .q-item__section {
  padding: 2px 8px !important;
  font-size: 12px;
  font-family: monospace;
}

.indexDetailsContainer {
  .log_json_content {
    white-space: pre-wrap;
    font-family: monospace;
    font-size: 12px;
  }
}

.q-icon {
  cursor: pointer;
}

.table-pre {
  white-space: pre-wrap;
  word-wrap: break-word;
  display: inline;
  font-weight: normal;
  font-family: monospace;
}

.json-pre {
  height: calc(100vh - 290px);
  white-space: pre-wrap;
  word-wrap: break-word;
}

.tab-panels-container {
  height: calc(100vh - 198px);
}
</style>
<style lang="scss">
.searchdetaildialog {
  .copy-log-btn {
    .q-btn .q-icon {
      font-size: 12px !important;
    }
  }
}
</style>
