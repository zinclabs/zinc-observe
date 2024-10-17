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
  <div
    class="column index-menu"
    :class="store.state.theme == 'dark' ? 'theme-dark' : 'theme-light'"
  >
    <q-select
      data-test="log-search-index-list-select-stream"
      v-model="selectedMetric"
      :label="selectedMetric ? '' : t('search.selectIndex')"
      :options="streamOptions"
      data-cy="index-dropdown"
      input-debounce="0"
      behavior="menu"
      filled
      borderless
      dense
      use-input
      hide-selected
      fill-input
      class="metric-select-input"
      @filter="filterMetrics"
      @update:model-value="onMetricChange"
    >
      <template v-if="selectedMetric?.type" v-slot:prepend>
        <q-icon
          :title="selectedMetric?.type"
          size="xs"
          :name="metricsIconMapping[selectedMetric?.type || '']"
        />
      </template>
      <template v-slot:option="scope">
        <q-item
          :class="
            store.state.theme === 'dark' &&
            selectedMetric?.value !== scope.opt.value
              ? 'text-white'
              : ''
          "
          v-bind="scope.itemProps"
        >
          <q-item-section
            :title="scope?.opt?.type"
            class="metric-explore-metric-icon"
            avatar
          >
            <q-icon
              size="xs"
              :name="metricsIconMapping[scope?.opt?.type] || ''"
            />
          </q-item-section>
          <q-item-section>
            <q-item-label>{{ scope.opt.label }}</q-item-label>
          </q-item-section>
        </q-item>
      </template>
      <template #no-option>
        <q-item>
          <q-item-section> {{ t("search.noResult") }}</q-item-section>
        </q-item>
      </template>
    </q-select>
    <div class="metric-list">
      <div class="metrics-label-table q-mt-xs">
        <q-table
          data-test="log-search-index-list-fields-table"
          :visible-columns="['name']"
          :rows="filteredMetricLabels"
          :row-key="(row: any) => row.name"
          :filter="searchMetricLabel"
          :filter-method="filterMetricLabels"
          :pagination="{ rowsPerPage: 10000 }"
          hide-header
          hide-bottom
          class="field-table"
          id="fieldList"
        >
          <template #body-cell-name="props">
            <q-tr :props="props">
              <q-td :props="props" class="field_list">
                <!-- TODO OK : Repeated code make separate component to display field  -->
                <template
                  v-if="
                    props.row.name === store.state.zoConfig.timestamp_column
                  "
                >
                  <div class="field_label ellipsis q-pl-lg">
                    {{ props.row.name }}
                  </div>
                </template>
                <template v-else>
                  <q-expansion-item
                    dense
                    switch-toggle-side
                    :label="props.row.name"
                    expand-icon-class="field-expansion-icon"
                    expand-icon="expand_more"
                    expanded-icon="expand_less"
                    @before-show="(event: any) => openFilterCreator(event, props.row)"
                  >
                    <template v-slot:header>
                      <div
                        class="flex content-center ellipsis"
                        :title="props.row.name"
                      >
                        <div class="field_label ellipsis">
                          {{ props.row.name }}
                        </div>
                        <div class="field_overlay">
                          <q-btn
                            :data-test="`metrics-list-add-${props.row.name}-label-btn`"
                            :icon="outlinedAdd"
                            size="0.4rem"
                            class="q-mr-none"
                            @click.stop="
                              addValueToEditor(props.row.name, '', '=')
                            "
                            round
                          />
                        </div>
                      </div>
                    </template>
                    <q-card>
                      <q-card-section class="q-pl-md q-pr-xs q-py-xs">
                        <div class="filter-values-container">
                          <div
                            v-show="
                              metricLabelValues[props.row.name]?.isLoading
                            "
                            class="q-pl-md q-py-xs"
                            style="height: 60px"
                          >
                            <q-inner-loading
                              size="xs"
                              :showing="
                                metricLabelValues[props.row.name]?.isLoading
                              "
                              label="Fetching values..."
                              label-style="font-size: 1.1em"
                            />
                          </div>
                          <div
                            v-show="
                              !metricLabelValues[props.row.name]?.values
                                ?.length &&
                              !metricLabelValues[props.row.name]?.isLoading
                            "
                            class="q-pl-md q-py-xs text-subtitle2"
                          >
                            No values found
                          </div>
                          <div
                            v-for="value in metricLabelValues[props.row.name]
                              ?.values || []"
                            :key="value.key + value.count"
                          >
                            <q-list dense>
                              <q-item tag="label" class="q-pr-none">
                                <div
                                  class="flex row wrap justify-between"
                                  style="width: calc(100% - 46px)"
                                  :class="
                                    store.state.theme === 'dark'
                                      ? 'text-grey-4'
                                      : 'text-grey-8'
                                  "
                                >
                                  <div
                                    :title="value.key"
                                    class="ellipsis q-pr-xs"
                                    style="width: calc(100% - 50px)"
                                  >
                                    {{ value.key }}
                                  </div>
                                  <div
                                    :title="value.count?.toString()"
                                    class="ellipsis text-right q-pr-sm"
                                    style="width: 50px"
                                  >
                                    {{ value.count }}
                                  </div>
                                </div>
                                <div
                                  class="flex row"
                                  :class="
                                    store.state.theme === 'dark'
                                      ? 'text-white'
                                      : 'text-black'
                                  "
                                >
                                  <q-btn
                                    class="q-mr-xs"
                                    size="6px"
                                    title="Include Term"
                                    round
                                    @click="
                                      addValueToEditor(
                                        props.row.name,
                                        value.key,
                                        '='
                                      )
                                    "
                                  >
                                    <q-icon>
                                      <EqualIcon></EqualIcon>
                                    </q-icon>
                                  </q-btn>
                                  <q-btn
                                    class="q-mr-xs"
                                    size="6px"
                                    title="Include Term"
                                    round
                                    @click="
                                      addValueToEditor(
                                        props.row.name,
                                        value.key,
                                        '!='
                                      )
                                    "
                                  >
                                    <q-icon>
                                      <NotEqualIcon></NotEqualIcon>
                                    </q-icon>
                                  </q-btn>
                                </div>
                              </q-item>
                            </q-list>
                          </div>
                        </div>
                      </q-card-section>
                    </q-card>
                  </q-expansion-item>
                </template>
              </q-td>
            </q-tr>
          </template>
          <template #top-right>
            <q-input
              data-test="log-search-index-list-field-search-input"
              v-model="searchMetricLabel"
              data-cy="index-field-search-input"
              filled
              borderless
              dense
              clearable
              debounce="1"
              :placeholder="t('search.searchField')"
            >
              <template #prepend>
                <q-icon name="search" />
              </template>
            </q-input>
          </template>
        </q-table>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  type Ref,
  watch,
  onMounted,
  computed,
} from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import { useRouter } from "vue-router";
import useMetrics from "../../composables/useMetrics";
import { formatLargeNumber, getImageURL } from "../../utils/zincutils";
import stream from "@/services/stream";
import { outlinedAdd } from "@quasar/extras/material-icons-outlined";
import EqualIcon from "@/components/icons/EqualIcon.vue";
import metricService from "@/services/metrics";
import NotEqualIcon from "@/components/icons/NotEqualIcon.vue";
import usePromqlSuggestions from "@/composables/usePromqlSuggestions";
import searchService from "@/services/search";
import useStreams from "@/composables/useStreams";

export default defineComponent({
  name: "MetricsList",
  emits: ["update:change-metric", "select-label", "update:modelValue"],
  components: { EqualIcon, NotEqualIcon },
  props: ["modelValue", "metricsList"],
  setup(props, { emit }) {
    const store = useStore();
    const router = useRouter();
    const { t } = useI18n();
    const quasar = useQuasar();
    const { searchObj } = useMetrics();
    const streamOptions: any = ref(props.metricsList || []);
    const selectedMetricLabels = ref([]);
    const searchMetricLabel = ref("");
    const filteredMetricLabels = ref([]);
    const metricLabelValues: Ref<{
      [key: string]: {
        isLoading: boolean;
        values: {
          key: string;
          count: number | string;
        }[];
      };
    }> = ref({});
    const metricsIconMapping: any = {
      summary: "description",
      gauge: "speed",
      histogram: "bar_chart",
      counter: "pin",
    };
    const { parsePromQlQuery } = usePromqlSuggestions();
    const { getStream } = useStreams();

    const selectedMetric = computed({
      get() {
        return props.modelValue;
      },
      set(value) {
        emit("update:modelValue", value);
      },
    });
    watch(
      () => props.metricsList,
      () => {
        streamOptions.value = props.metricsList;
      },
      { deep: true }
    );
    onMounted(() => {
      if (!streamOptions.value.length) streamOptions.value = props.metricsList;
    });
    const filterMetrics = (val: string, update: any) => {
      update(() => {
        streamOptions.value = props.metricsList;
        const needle = val.toLowerCase();
        streamOptions.value = streamOptions.value.filter(
          (v: any) => v.label.toLowerCase().indexOf(needle) > -1
        );
      });
    };
    const updateMetricLabels = async () => {
      const streamData = await getStream(
        selectedMetric.value?.value || "",
        "metrics",
        true
      );

      selectedMetricLabels.value = streamData.schema;

      if (Array.isArray(selectedMetricLabels.value))
        filteredMetricLabels.value = [...selectedMetricLabels.value];
    };
    watch(
      () => selectedMetric.value,
      (metric) => {
        if (metric?.value) updateMetricLabels();
      },
      { immediate: true, deep: true }
    );
    const filterMetricLabels = (rows: any, terms: any) => {
      var filtered = [];
      if (terms != "") {
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (rows[i]["name"].toLowerCase().includes(terms)) {
            filtered.push(rows[i]);
          }
        }
      }
      return filtered;
    };

    const getMetricsFieldValues = (name: string) => {
      const startISOTimestamp: any = searchObj.data.datetime.startTime;
      const endISOTimestamp: any = searchObj.data.datetime.endTime;
      metricLabelValues.value[name] = {
        isLoading: true,
        values: [],
      };
      try {
        stream
          .fieldValues({
            org_identifier: store.state.selectedOrganization.identifier,
            stream_name: selectedMetric.value?.value,
            start_time: startISOTimestamp,
            end_time: endISOTimestamp,
            fields: [name],
            type: "metrics",
            size: 10,
          })
          .then((res: any) => {
            if (res.data.hits.length) {
              metricLabelValues.value[name]["values"] = res.data.hits
                .find((field: any) => field.field === name)
                .values.map((value: any) => {
                  return {
                    key: value.zo_sql_key ? value.zo_sql_key : "null",
                    count: formatLargeNumber(value.zo_sql_num),
                  };
                });
            }
          })
          .finally(() => {
            metricLabelValues.value[name]["isLoading"] = false;
          });
      } catch (err) {
        quasar.notify({
          type: "negative",
          message: "Error while fetching field values",
        });
      }
    };

    const getFilteredMetricValues = (name: string) => {
      const parsedQuery: any = parsePromQlQuery(searchObj.data.query);
      const metricName = parsedQuery?.metricName || "";
      const labels = parsedQuery?.label?.labels || {};
      if (metricName) labels["__name__"] = metricName;

      const formattedLabels = Object.keys(labels).map((key) => {
        return `${key}="${labels[key]}"`;
      });

      searchService
        .get_promql_series({
          org_identifier: store.state.selectedOrganization.identifier,
          start_time: Number(searchObj.data.datetime.startTime),
          end_time: Number(searchObj.data.datetime.endTime),
          labels: `{${formattedLabels.join(",")}}`,
        })
        .then((res) => {
          if (res.data.data.length) {
            const valuesSet = new Map();
            res.data.data.forEach((label: any) => {
              if (!label[name]) return;
              if (valuesSet.has(label[name])) {
                valuesSet.set(label[name], valuesSet.get(label[name]) + 1);
              } else {
                valuesSet.set(label[name], 1);
              }
            });
            metricLabelValues.value[name]["values"] = [];
            valuesSet.forEach((value, key) => {
              metricLabelValues.value[name]["values"].push({
                key: key ? key : "null",
                count: value,
              });
            });
          }
        })
        .catch((err) => console.log(err))
        .finally(() => {
          metricLabelValues.value[name].isLoading = false;
        });
    };

    const openFilterCreator = (event: any, { name }: any) => {
      metricLabelValues.value[name] = {
        isLoading: true,
        values: [],
      };
      metricService
        .formatPromqlQuery({
          org_identifier: store.state.selectedOrganization.identifier,
          query: searchObj.data.query,
        })
        .then(() => {
          getFilteredMetricValues(name);
        })
        .catch(() => {
          getMetricsFieldValues(name);
        });
    };

    const onMetricChange = () => {
      updateMetricLabels();
      emit("update:change-metric", selectedMetric.value);
    };
    const setSelectedMetricType = (option: any) => {
      searchObj.data.metrics.selectedMetricType = option.type;
    };
    const addLabelToEditor = (label: string) => {
      emit("select-label", label);
    };
    const addValueToEditor = (
      label: string,
      value: string,
      operator: string
    ) => {
      addLabelToEditor(`${label}${operator}"${value}"`);
    };
    return {
      quasar,
      t,
      store,
      router,
      searchObj,
      streamOptions,
      getImageURL,
      filterMetrics,
      filteredMetricLabels,
      searchMetricLabel,
      filterMetricLabels,
      openFilterCreator,
      metricLabelValues,
      onMetricChange,
      metricsIconMapping,
      setSelectedMetricType,
      outlinedAdd,
      addLabelToEditor,
      addValueToEditor,
      selectedMetric,
    };
  },
});
</script>

<style lang="scss" scoped>
.q-menu {
  box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
  transform: translateY(0.5rem);
  border-radius: 0px;

  .q-virtual-scroll__content {
    padding: 0.5rem;
  }
}
.index-menu {
  width: 100%;

  .metric-list {
    height: calc(100vh - 98px);
    width: 100%;
    overflow-x: hidden;
    overflow-y: auto;
    .metric-container {
      width: 100%;

      &:hover {
        background-color: color-mix(in srgb, currentColor 15%, transparent);
      }

      &.selected {
        background-color: rgba(89, 96, 178, 0.3);
      }
    }
  }
}

.q-item {
  color: $dark-page;
  min-height: 1.3rem;
  padding: 5px 10px;

  &__label {
    font-size: 0.75rem;
  }

  &.q-manual-focusable--focused > .q-focus-helper {
    background: none !important;
    opacity: 0.3 !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &--active {
    background-color: $selected-list-bg !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &:hover,
  &--active {
    color: $primary;
  }
}
.q-field--dense .q-field__before,
.q-field--dense .q-field__prepend {
  padding: 0px 0px 0px 0px;
  height: auto;
  line-height: auto;
}
.q-field__native,
.q-field__input {
  padding: 0px 0px 0px 0px;
}

.q-field--dense .q-field__label {
  top: 5px;
}
.q-field--dense .q-field__control,
.q-field--dense .q-field__marginal {
  height: 34px;
}
</style>

<style lang="scss" scoped>
.q-menu {
  box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
  transform: translateY(0.5rem);
  border-radius: 0px;

  .q-virtual-scroll__content {
    padding: 0.5rem;
  }
}

.index-menu {
  width: 100%;

  .q-field {
    &__control {
      height: 35px;
      padding: 0px 5px;
      min-height: auto !important;

      &-container {
        padding-top: 0px !important;
      }
    }

    &__native :first-of-type {
      padding-top: 0.25rem;
    }
  }

  .metrics-label-table {
    width: 100%;
    // border: 1px solid rgba(0, 0, 0, 0.02);

    .q-table {
      display: table;
      table-layout: fixed !important;
    }

    tr {
      margin-bottom: 1px;
    }

    tbody,
    tr,
    td {
      width: 100%;
      display: block;
      height: fit-content;
      overflow: hidden;
    }

    .q-table__control,
    label.q-field {
      width: 100%;
    }

    .q-table thead tr,
    .q-table tbody td {
      height: auto;
    }

    .q-table__top {
      border-bottom: unset;
    }
  }

  .field-table {
    width: 100%;
  }

  .field_list {
    padding: 0px;
    margin-bottom: 0.125rem;
    position: relative;
    overflow: visible;
    cursor: default;

    .field_label {
      pointer-events: none;
      font-size: 0.825rem;
      position: relative;
      display: inline;
      z-index: 2;
      left: 0;
      // text-transform: capitalize;
    }

    .field-container {
      height: 25px;
    }

    .field_overlay {
      position: absolute;
      height: 100%;
      right: 0;
      top: 0;
      z-index: 5;
      background-color: #e8e8e8;
      padding: 0 6px;
      visibility: hidden;
      display: flex;
      align-items: center;

      .q-icon {
        cursor: pointer;
        opacity: 0;
        margin: 0 1px;
      }
    }

    &.selected {
      .field_overlay {
        background-color: rgba(89, 96, 178, 0.3);

        .field_icons {
          opacity: 0;
        }
      }
    }

    &:hover {
      .field-container {
        background-color: #e8e8e8;
      }
    }
  }
}

.q-item {
  color: $dark-page;
  min-height: 1.3rem;
  padding: 5px 10px;

  &__label {
    font-size: 0.75rem;
  }

  &.q-manual-focusable--focused > .q-focus-helper {
    background: none !important;
    opacity: 0.3 !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &--active {
    background-color: $selected-list-bg !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &:hover,
  &--active {
    color: $primary;
  }
}

.q-field--dense .q-field__before,
.q-field--dense .q-field__prepend {
  padding: 0px 0px 0px 0px;
  height: auto;
  line-height: auto;
}

.q-field__native,
.q-field__input {
  padding: 0px 0px 0px 0px;
}

.q-field--dense .q-field__label {
  top: 5px;
}

.q-field--dense .q-field__control,
.q-field--dense .q-field__marginal {
  height: 34px;
}
</style>

<style lang="scss">
.metrics-label-table {
  .q-table {
    width: 100%;
    table-layout: fixed;

    .q-expansion-item {
      .q-item {
        display: flex;
        align-items: center;
        padding: 0;
        height: 25px !important;
        min-height: 25px !important;
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
        img {
          width: 12px;
          height: 12px;
        }
      }
    }

    .field-container {
      &:hover {
        .field_overlay {
          visibility: visible;

          .q-icon {
            opacity: 1;
          }
        }
      }
    }

    .field_list {
      &.selected {
        .q-expansion-item {
          background-color: rgba(89, 96, 178, 0.3);
        }

        .field_overlay {
          background-color: #ffffff;
        }
      }
    }
  }
}

.theme-dark {
  .field_list {
    &:hover {
      box-shadow: 0px 4px 15px rgb(255, 255, 255, 0.1);

      .field_overlay {
        background-color: #3f4143;
        opacity: 1;
      }
    }
  }
}

.theme-light {
  .field_list {
    &:hover {
      box-shadow: 0px 4px 15px rgba(0, 0, 0, 0.17);

      .field_overlay {
        background-color: #e8e8e8;
        opacity: 1;
      }
    }
  }
}
</style>

<style lang="scss">
.metric-explore-metric-icon {
  min-width: 28px !important;
  padding-right: 8px !important;
}
</style>
