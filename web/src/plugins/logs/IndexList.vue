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
  <div
    class="column logs-index-menu full-height"
    :class="store.state.theme == 'dark' ? 'theme-dark' : 'theme-light'"
  >
    <div>
      <q-select
        data-test="log-search-index-list-select-stream"
        v-model="searchObj.data.stream.selectedStream"
        :options="streamOptions"
        data-cy="index-dropdown"
        input-debounce="0"
        behavior="menu"
        filled
        borderless
        dense
        use-input
        fill-input
        multiple
        emit-value
        map-options
        @filter="filterStreamFn"
        @update:model-value="handleMultiStreamSelection"
        :title="searchObj.data.stream.selectedStream.join(',')"
      >
        <template #no-option>
          <q-item>
            <q-item-section> {{ t("search.noResult") }}</q-item-section>
          </q-item>
        </template>
        <template v-slot:option="{ itemProps, opt, selected, toggleOption }">
          <q-item style="cursor: pointer">
            <q-item-section @click="handleSingleStreamSelect(opt)">
              <q-item-label v-html="opt.label" />
            </q-item-section>
            <q-item-section side v-if="streamOptions.length > 1">
              <q-toggle
                :data-test="`log-search-index-list-stream-toggle-${opt.label}`"
                :model-value="selected"
                size="20px"
                @update:model-value="toggleOption(opt.value)"
              />
            </q-item-section>
          </q-item>
        </template>
      </q-select>
    </div>
    <div class="index-table q-mt-xs">
      <q-table
        data-test="log-search-index-list-fields-table"
        v-model="sortedStreamFields"
        :visible-columns="['name']"
        :rows="searchObj.data.stream.selectedStreamFields"
        :row-key="(row) => searchObj.data.stream.selectedStream[0] + row.name"
        :filter="searchObj.data.stream.filterField"
        :filter-method="filterFieldFn"
        v-model:pagination="pagination"
        hide-header
        :wrap-cells="searchObj.meta.resultGrid.wrapCells"
        class="field-table full-height"
        id="fieldList"
        :rows-per-page-options="[]"
        :hide-bottom="
          (!store.state.zoConfig.user_defined_schemas_enabled ||
          !searchObj.meta.hasUserDefinedSchemas) &&
          searchObj.data.stream.selectedStreamFields != undefined &&
          (searchObj.data.stream.selectedStreamFields.length <=
            pagination.rowsPerPage ||
            searchObj.data.stream.selectedStreamFields.length == 0)
        "
      >
        <template #body-cell-name="props">
          <q-tr
            :props="props"
            v-if="props.row.label"
            @click="
              searchObj.data.stream.expandGroupRows[props.row.group] =
                !searchObj.data.stream.expandGroupRows[props.row.group]
            "
            class="cursor-pointer text-bold"
          >
            <q-td
              class="field_list bg-grey-3"
              style="line-height: 28px; padding-left: 10px"
            >
              {{ props.row.name }}
              <q-icon
                :name="
                  searchObj.data.stream.expandGroupRows[props.row.group]
                    ? 'expand_less'
                    : 'expand_more'
                "
                size="25px"
                class="float-right"
              ></q-icon>
            </q-td>
          </q-tr>
          <q-tr
            :props="props"
            v-else
            v-show="searchObj.data.stream.expandGroupRows[props.row.group]"
          >
            <q-td
              :props="props"
              class="field_list"
              :class="
                searchObj.data.stream.selectedFields.includes(props.row.name)
                  ? 'selected'
                  : ''
              "
            >
              <!-- TODO OK : Repeated code make seperate component to display field  -->
              <div
                v-if="
                  props.row.ftsKey ||
                  !props.row.isSchemaField ||
                  !props.row.showValues
                "
                class="field-container flex content-center ellipsis q-pl-lg full-width"
                :title="props.row.name"
              >
                <div
                  class="field_label full-width"
                  :data-test="`logs-field-list-item-${props.row.name}`"
                >
                  <div
                    class="ellipsis"
                    style="max-width: 90% !important; display: inline-block"
                  >
                    {{ props.row.name }}
                  </div>
                  <span class="float-right">
                    <q-icon
                      :data-test="`log-search-index-list-interesting-${props.row.name}-field-btn`"
                      v-if="searchObj.meta.quickMode"
                      :name="
                        props.row.isInterestingField ? 'info' : 'info_outline'
                      "
                      :class="
                        store.state.theme === 'dark' ? '' : 'light-dimmed'
                      "
                      style="margin-right: 0.375rem"
                      size="1.1rem"
                      :title="
                        props.row.isInterestingField
                          ? 'Remove from interesting fields'
                          : 'Add to interesting fields'
                      "
                    />
                  </span>
                </div>
                <div class="field_overlay">
                  <q-btn
                    v-if="props.row.isSchemaField"
                    :icon="outlinedAdd"
                    :data-test="`log-search-index-list-filter-${props.row.name}-field-btn`"
                    style="margin-right: 0.375rem"
                    size="0.4rem"
                    class="q-mr-sm"
                    @click.stop="addToFilter(`${props.row.name}=''`)"
                    round
                  />
                  <q-icon
                    :data-test="`log-search-index-list-add-${props.row.name}-field-btn`"
                    v-if="
                      !searchObj.data.stream.selectedFields.includes(
                        props.row.name
                      )
                    "
                    :name="outlinedVisibility"
                    style="margin-right: 0.375rem"
                    size="1.1rem"
                    title="Add field to table"
                    @click.stop="clickFieldFn(props.row, props.pageIndex)"
                  />
                  <q-icon
                    :data-test="`log-search-index-list-remove-${props.row.name}-field-btn`"
                    v-if="
                      searchObj.data.stream.selectedFields.includes(
                        props.row.name
                      )
                    "
                    :name="outlinedVisibilityOff"
                    style="margin-right: 0.375rem"
                    size="1.1rem"
                    title="Remove field from table"
                    @click.stop="clickFieldFn(props.row, props.pageIndex)"
                  />
                  <q-icon
                    :data-test="`log-search-index-list-interesting-${props.row.name}-field-btn`"
                    v-if="searchObj.meta.quickMode"
                    :name="
                      props.row.isInterestingField ? 'info' : 'info_outline'
                    "
                    size="1.1rem"
                    :title="
                      props.row.isInterestingField
                        ? 'Remove from interesting fields'
                        : 'Add to interesting fields'
                    "
                    @click.stop="
                      addToInterestingFieldList(
                        props.row,
                        props.row.isInterestingField
                      )
                    "
                  />
                </div>
              </div>
              <q-expansion-item
                v-else
                dense
                switch-toggle-side
                :label="props.row.name"
                expand-icon-class="field-expansion-icon"
                expand-icon="
                  expand_more
                "
                expanded-icon="
                  expand_less
                "
                @before-show="(event: any) => openFilterCreator(event, props.row)"
              >
                <template v-slot:header>
                  <div
                    class="flex content-center ellipsis full-width"
                    :title="props.row.name"
                    :data-test="`log-search-expand-${props.row.name}-field-btn`"
                  >
                    <div
                      class="field_label full-width"
                      :data-test="`logs-field-list-item-${props.row.name}`"
                    >
                      <div
                        class="ellipsis"
                        style="max-width: 90% !important; display: inline-block"
                      >
                        {{ props.row.name }}
                      </div>
                      <span class="float-right">
                        <q-icon
                          :data-test="`log-search-index-list-interesting-${props.row.name}-field-btn`"
                          v-if="searchObj.meta.quickMode"
                          :name="
                            props.row.isInterestingField
                              ? 'info'
                              : 'info_outline'
                          "
                          :class="
                            store.state.theme === 'dark' ? '' : 'light-dimmed'
                          "
                          style="margin-right: 0.375rem"
                          size="1.1rem"
                          :title="
                            props.row.isInterestingField
                              ? 'Remove from interesting fields'
                              : 'Add to interesting fields'
                          "
                        />
                      </span>
                    </div>
                    <div class="field_overlay">
                      <q-btn
                        v-if="props.row.isSchemaField"
                        :data-test="`log-search-index-list-filter-${props.row.name}-field-btn`"
                        :icon="outlinedAdd"
                        style="margin-right: 0.375rem"
                        size="0.4rem"
                        class="q-mr-sm"
                        @click.stop="addToFilter(`${props.row.name}=''`)"
                        round
                      />
                      <q-icon
                        :data-test="`log-search-index-list-add-${props.row.name}-field-btn`"
                        v-if="
                          !searchObj.data.stream.selectedFields.includes(
                            props.row.name
                          )
                        "
                        :name="outlinedVisibility"
                        style="margin-right: 0.375rem"
                        size="1.1rem"
                        title="Add field to table"
                        @click.stop="clickFieldFn(props.row, props.pageIndex)"
                      />
                      <q-icon
                        :data-test="`log-search-index-list-remove-${props.row.name}-field-btn`"
                        v-if="
                          searchObj.data.stream.selectedFields.includes(
                            props.row.name
                          )
                        "
                        :name="outlinedVisibilityOff"
                        style="margin-right: 0.375rem"
                        title="Remove field from table"
                        size="1.1rem"
                        @click.stop="clickFieldFn(props.row, props.pageIndex)"
                      />
                      <q-icon
                        :data-test="`log-search-index-list-interesting-${props.row.name}-field-btn`"
                        v-if="searchObj.meta.quickMode"
                        :name="
                          props.row.isInterestingField ? 'info' : 'info_outline'
                        "
                        size="1.1rem"
                        :title="
                          props.row.isInterestingField
                            ? 'Remove from interesting fields'
                            : 'Add to interesting fields'
                        "
                        @click.stop="
                          addToInterestingFieldList(
                            props.row,
                            props.row.isInterestingField
                          )
                        "
                      />
                    </div>
                  </div>
                </template>
                <q-card>
                  <q-card-section class="q-pl-md q-pr-xs q-py-xs">
                    <div class="filter-values-container">
                      <div
                        v-show="fieldValues[props.row.name]?.isLoading"
                        class="q-pl-md q-py-xs"
                        style="height: 60px"
                      >
                        <q-inner-loading
                          size="xs"
                          :showing="fieldValues[props.row.name]?.isLoading"
                          label="Fetching values..."
                          label-style="font-size: 1.1em"
                        />
                      </div>
                      <div
                        v-show="
                          !fieldValues[props.row.name]?.values?.length &&
                          !fieldValues[props.row.name]?.isLoading
                        "
                        class="q-pl-md q-py-xs text-subtitle2"
                      >
                        {{
                          fieldValues[props.row.name]?.errMsg ||
                          "No values found"
                        }}
                      </div>
                      <div
                        v-for="value in fieldValues[props.row.name]?.values ||
                        []"
                        :key="value.key"
                      >
                        <q-list dense>
                          <q-item
                            tag="label"
                            class="q-pr-none"
                            :data-test="`logs-search-subfield-add-${props.row.name}-${value.key}`"
                          >
                            <div
                              class="flex row wrap justify-between"
                              :style="
                                searchObj.data.stream.selectedStream.length ==
                                props.row.streams.length
                                  ? 'width: calc(100% - 42px)'
                                  : 'width: calc(100% - 0px)'
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
                                :title="value.count.toString()"
                                class="ellipsis text-right q-pr-sm"
                                style="display: contents"
                                :style="
                                  searchObj.data.stream.selectedStream.length ==
                                  props.row.streams.length
                                    ? 'width: 50px'
                                    : ''
                                "
                              >
                                {{ formatLargeNumber(value.count) }}
                              </div>
                            </div>
                            <div
                              v-if="
                                searchObj.data.stream.selectedStream.length ==
                                props.row.streams.length
                              "
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
                                @click="
                                  addSearchTerm(
                                    `${props.row.name}='${value.key}'`
                                  )
                                "
                                title="Include Term"
                                round
                                :data-test="`log-search-subfield-list-equal-${props.row.name}-field-btn`"
                              >
                                <q-icon>
                                  <EqualIcon></EqualIcon>
                                </q-icon>
                              </q-btn>
                              <q-btn
                                size="6px"
                                @click="
                                  addSearchTerm(
                                    `${props.row.name}!='${value.key}'`
                                  )
                                "
                                title="Exclude Term"
                                round
                                :data-test="`log-search-subfield-list-not-equal-${props.row.name}-field-btn`"
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
            </q-td>
          </q-tr>
        </template>
        <template #top-right>
          <q-input
            data-test="log-search-index-list-field-search-input"
            v-model="searchObj.data.stream.filterField"
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
          <q-tr v-if="searchObj.loadingStream == true">
            <q-td colspan="100%"
class="text-bold" style="opacity: 0.7">
              <div class="text-subtitle2 text-weight-bold">
                <q-spinner-hourglass size="20px" />
                {{ t("confirmDialog.loading") }}
              </div>
            </q-td>
          </q-tr>
        </template>
        <template v-slot:pagination="scope">
          <div
            v-if="
              store.state.zoConfig.user_defined_schemas_enabled &&
              searchObj.meta.hasUserDefinedSchemas
            "
          >
            <q-btn-toggle
              no-caps
              v-model="searchObj.meta.useUserDefinedSchemas"
              data-test="logs-page-field-list-user-defined-schema-toggle"
              class="schema-field-toggle q-mr-xs"
              toggle-color="primary"
              bordered
              size="8px"
              color="white"
              text-color="primary"
              @update:model-value="toggleSchema"
              :options="userDefinedSchemaBtnGroupOption"
            >
              <template v-slot:user_defined_slot>
                <q-icon name="person"></q-icon>
                <q-icon name="schema"></q-icon>
                <q-tooltip
                  data-test="logs-page-fields-list-user-defined-fields-warning-tooltip"
                  anchor="center right"
                  self="center left"
                  max-width="300px"
                  class="text-body2"
                >
                  <span class="text-bold" color="white">{{
                    t("search.userDefinedSchemaLabel")
                  }}</span>
                </q-tooltip>
              </template>
              <template v-slot:all_fields_slot>
                <q-icon name="schema"></q-icon>
                <q-tooltip
                  data-test="logs-page-fields-list-all-fields-warning-tooltip"
                  anchor="center right"
                  self="center left"
                  max-width="300px"
                  class="text-body2"
                >
                  <span class="text-bold" color="white">{{
                    t("search.allFieldsLabel")
                  }}</span>
                  <q-separator color="white" class="q-mt-xs q-mb-xs" />
                  {{ t("search.allFieldsWarningMsg") }}
                </q-tooltip>
              </template>
            </q-btn-toggle>
          </div>
          <div class="q-ml-xs text-right col" v-if="scope.pagesNumber > 1">
            <q-tooltip
              data-test="logs-page-fields-list-pagination-tooltip"
              anchor="center right"
              self="center left"
              max-width="300px"
              class="text-body2"
            >
              Total Fields:
              {{ searchObj.data.stream.selectedStreamFields.length }}
            </q-tooltip>
            <q-btn
              data-test="logs-page-fields-list-pagination-firstpage-button"
              v-if="scope.pagesNumber > 2"
              icon="skip_previous"
              color="grey-8"
              round
              dense
              flat
              :disable="scope.isFirstPage"
              @click="scope.firstPage"
            />

            <q-btn
              data-test="logs-page-fields-list-pagination-previouspage-button"
              icon="fast_rewind"
              color="grey-8"
              round
              dense
              flat
              :disable="scope.isFirstPage"
              @click="scope.prevPage"
            />

            <q-btn
              round
              data-test="logs-page-fields-list-pagination-messsage-button"
              dense
              flat
              class="text text-caption text-regular"
              >{{ scope.pagination.page }}/{{ scope.pagesNumber }}</q-btn
            >

            <q-btn
              data-test="logs-page-fields-list-pagination-nextpage-button"
              icon="fast_forward"
              color="grey-8"
              round
              dense
              flat
              :disable="scope.isLastPage"
              @click="scope.nextPage"
            />

            <q-btn
              data-test="logs-page-fields-list-pagination-lastpage-button"
              v-if="scope.pagesNumber > 2"
              icon="skip_next"
              color="grey-8"
              round
              dense
              flat
              :disable="scope.isLastPage"
              @click="scope.lastPage"
            />
          </div>
        </template>
      </q-table>
    </div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  type Ref,
  watch,
  computed,
  onBeforeMount,
} from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import { useRouter } from "vue-router";
import useLogs from "../../composables/useLogs";
import {
  b64EncodeUnicode,
  getImageURL,
  convertTimeFromMicroToMilli,
  formatLargeNumber,
  useLocalInterestingFields,
} from "../../utils/zincutils";
import streamService from "../../services/stream";
import {
  outlinedAdd,
  outlinedVisibility,
  outlinedVisibilityOff,
} from "@quasar/extras/material-icons-outlined";
import EqualIcon from "@/components/icons/EqualIcon.vue";
import NotEqualIcon from "@/components/icons/NotEqualIcon.vue";
import { getConsumableRelativeTime } from "@/utils/date";
import { cloneDeep } from "lodash-es";

interface Filter {
  fieldName: string;
  selectedValues: string[];
  selectedOperator: string;
}
export default defineComponent({
  name: "ComponentSearchIndexSelect",
  components: { EqualIcon, NotEqualIcon },
  emits: ["setInterestingFieldInSQLQuery"],
  methods: {
    handleMultiStreamSelection() {
      if (this.searchObj.meta.sqlMode) {
        this.searchObj.meta.sqlMode = false;
      }
      this.onStreamChange("");
    },
    handleSingleStreamSelect(opt: any) {
      this.searchObj.data.stream.selectedStream = [opt.value];
      this.onStreamChange("");
    },
  },
  setup(props, { emit }) {
    const store = useStore();
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const {
      searchObj,
      updatedLocalLogFilterField,
      handleQueryData,
      onStreamChange,
      filterHitsColumns,
      extractFields,
      validateFilterForMultiStream,
    } = useLogs();
    const userDefinedSchemaBtnGroupOption = [
      {
        label: "",
        value: "user_defined_schema",
        slot: "user_defined_slot",
      },
      {
        label: "",
        value: "all_fields",
        slot: "all_fields_slot",
      },
    ];
    const streamOptions: any = ref(searchObj.data.stream.streamLists);
    const fieldValues: Ref<{
      [key: string | number]: {
        isLoading: boolean;
        values: { key: string; count: number }[];
        errMsg?: string;
      };
    }> = ref({});
    let parser: any;

    const streamTypes = [
      { label: t("search.logs"), value: "logs" },
      { label: t("search.enrichmentTables"), value: "enrichment_tables" },
    ];

    const filterStreamFn = (val: string, update: any) => {
      update(() => {
        streamOptions.value = searchObj.data.stream.streamLists;
        const needle = val.toLowerCase();
        streamOptions.value = streamOptions.value.filter(
          (v: any) => v.label.toLowerCase().indexOf(needle) > -1
        );
      });
    };

    onBeforeMount(async () => {
      await importSqlParser();
    });

    const importSqlParser = async () => {
      const useSqlParser: any = await import("@/composables/useParser");
      const { sqlParser }: any = useSqlParser.default();
      parser = await sqlParser();
    };

    watch(
      () => {
        searchObj.data.stream.streamLists.length;
        store.state.organizationData.streams;
      },
      () => {
        streamOptions.value =
          searchObj.data.stream.streamLists ||
          store.state.organizationData.streams;
      }
    );

    const filterFieldFn = (rows: any, terms: any) => {
      var filtered = [];
      var includedFields: any = [];
      if (terms != "") {
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (
            rows[i]["name"].toLowerCase().includes(terms) &&
            includedFields.indexOf(rows[i]["name"]) == -1
          ) {
            filtered.push(rows[i]);
            includedFields.push(rows[i]["name"]);
          }
        }
      }
      return filtered;
    };

    const addToFilter = (field: any) => {
      searchObj.data.stream.addToFilter = field;
    };

    function clickFieldFn(row: { name: never }, pageIndex: number) {
      if (searchObj.data.stream.selectedFields.includes(row.name)) {
        searchObj.data.stream.selectedFields =
          searchObj.data.stream.selectedFields.filter(
            (v: any) => v !== row.name
          );
      } else {
        searchObj.data.stream.selectedFields.push(row.name);
      }
      searchObj.organizationIdetifier =
        store.state.selectedOrganization.identifier;
      updatedLocalLogFilterField();
      filterHitsColumns();
    }

    const openFilterCreator = (
      event: any,
      { name, ftsKey, isSchemaField, streams }: any
    ) => {
      if (ftsKey) {
        event.stopPropagation();
        event.preventDefault();
        return;
      }

      let timestamps: any =
        searchObj.data.datetime.type === "relative"
          ? getConsumableRelativeTime(
              searchObj.data.datetime.relativeTimePeriod
            )
          : cloneDeep(searchObj.data.datetime);

      if (searchObj.data.stream.streamType === "enrichment_tables") {
        const stream = searchObj.data.streamResults.list.find((stream: any) =>
          searchObj.data.stream.selectedStream.includes(stream.name)
        );
        if (stream.stats) {
          timestamps = {
            startTime:
              new Date(
                convertTimeFromMicroToMilli(
                  stream.stats.doc_time_min - 300000000
                )
              ).getTime() * 1000,
            endTime:
              new Date(
                convertTimeFromMicroToMilli(
                  stream.stats.doc_time_max + 300000000
                )
              ).getTime() * 1000,
          };
        }
      }

      const startISOTimestamp: number = timestamps?.startTime || 0;
      const endISOTimestamp: number = timestamps?.endTime || 0;

      fieldValues.value[name] = {
        isLoading: true,
        values: [],
        errMsg: "",
      };
      try {
        let query_context = "";
        let query = searchObj.data.query;
        let whereClause = "";
        searchObj.data.filterErrMsg = "";
        searchObj.data.missingStreamMessage = "";
        searchObj.data.stream.missingStreamMultiStreamFilter = [];
        if (searchObj.meta.sqlMode && query.trim().length) {
          const parsedSQL: any = parser.astify(query);
          //hack add time stamp column to parsedSQL if not already added
          query_context = parser.sqlify(parsedSQL).replace(/`/g, '"') || "";
        } else if (query.trim().length) {
          let parseQuery = query.split("|");
          let queryFunctions = "";
          let whereClause = "";
          if (parseQuery.length > 1) {
            queryFunctions = "," + parseQuery[0].trim();
            whereClause = parseQuery[1].trim();
          } else {
            whereClause = parseQuery[0].trim();
          }

          query_context = `SELECT *${queryFunctions} FROM [INDEX_NAME] [WHERE_CLAUSE]`;

          if (whereClause.trim() != "") {
            whereClause = whereClause
              .replace(/=(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " =")
              .replace(/>(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " >")
              .replace(/<(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " <");

            whereClause = whereClause
              .replace(/!=(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " !=")
              .replace(/! =(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " !=")
              .replace(/< =(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " <=")
              .replace(/> =(?=(?:[^"']*"[^"']*"')*[^"']*$)/g, " >=");

            const parsedSQL = whereClause.split(" ");
            searchObj.data.stream.selectedStreamFields.forEach((field: any) => {
              parsedSQL.forEach((node: any, index: any) => {
                if (node == field.name) {
                  node = node.replaceAll('"', "");
                  parsedSQL[index] = '"' + node + '"';
                }
              });
            });

            whereClause = parsedSQL.join(" ");

            query_context = query_context.replace(
              "[WHERE_CLAUSE]",
              " WHERE " + whereClause
            );
          } else {
            query_context = query_context.replace("[WHERE_CLAUSE]", "");
          }
          query_context = b64EncodeUnicode(query_context) || "";
        }

        let query_fn = "";
        if (
          searchObj.data.tempFunctionContent != "" &&
          searchObj.meta.toggleFunction
        ) {
          query_fn = b64EncodeUnicode(searchObj.data.tempFunctionContent) || "";
        }
        fieldValues.value[name] = {
          isLoading: true,
          values: [],
          errMsg: "",
        };
        if (whereClause.trim() != "") {
          // validateFilterForMultiStream function called to get missingStreamMultiStreamFilter
          const validationFlag = validateFilterForMultiStream();
          if (!validationFlag) {
            fieldValues.value[name]["isLoading"] = false;
            fieldValues.value[name]["errMsg"] =
              "Filter is not valid for selected streams.";
            return;
          }
          if (searchObj.data.stream.missingStreamMultiStreamFilter.length > 0) {
            streams = searchObj.data.stream.selectedStream.filter(
              (streams: any) =>
                !searchObj.data.stream.missingStreamMultiStreamFilter.includes(
                  streams
                )
            );
          }
        }
        let countTotal = streams.length;
        streams.forEach(async (selectedStream: string) => {
          await streamService
            .fieldValues({
              org_identifier: store.state.selectedOrganization.identifier,
              stream_name: selectedStream,
              start_time: startISOTimestamp,
              end_time: endISOTimestamp,
              fields: [name],
              size: 10,
              query_context:
                b64EncodeUnicode(
                  query_context.replace("[INDEX_NAME]", selectedStream)
                ) || "",
              query_fn: query_fn,
              type: searchObj.data.stream.streamType,
              regions: searchObj.meta.hasOwnProperty("regions")
                ? searchObj.meta.regions.join(",")
                : "",
            })
            .then((res: any) => {
              countTotal--;
              if (res.data.hits.length) {
                res.data.hits.forEach((item: any) => {
                  item.values.forEach((subItem: any) => {
                    if (fieldValues.value[name]["values"].length) {
                      let index = fieldValues.value[name]["values"].findIndex(
                        (value: any) => value.key == subItem.zo_sql_key
                      );
                      if (index != -1) {
                        fieldValues.value[name]["values"][index].count =
                          parseInt(subItem.zo_sql_num) +
                          fieldValues.value[name]["values"][index].count;
                      } else {
                        fieldValues.value[name]["values"].push({
                          key: subItem.zo_sql_key,
                          count: subItem.zo_sql_num,
                        });
                      }
                    } else {
                      fieldValues.value[name]["values"].push({
                        key: subItem.zo_sql_key,
                        count: subItem.zo_sql_num,
                      });
                    }
                  });
                });
                if (fieldValues.value[name]["values"].length > 10) {
                  fieldValues.value[name]["values"].sort(
                    (a, b) => b.count - a.count
                  ); // Sort the array based on count in descending order
                  fieldValues.value[name]["values"].slice(0, 10); // Return the first 10 elements
                }
              }
            })
            .finally(() => {
              if (countTotal == 0) fieldValues.value[name]["isLoading"] = false;
            });
        });
      } catch (err) {
        console.log(err);
        $q.notify({
          type: "negative",
          message: "Error while fetching field values",
        });
      }
    };

    const addSearchTerm = (term: string) => {
      // searchObj.meta.showDetailTab = false;
      searchObj.data.stream.addToFilter = term;
    };

    // const onStreamChange = () => {
    //   alert("onStreamChange")
    //   const query = searchObj.meta.sqlMode
    //     ? `SELECT * FROM "${searchObj.data.stream.selectedStream.value}"`
    //     : "";

    //   searchObj.data.editorValue = query;
    //   searchObj.data.query = query;

    //   handleQueryData();
    // };

    const addToInterestingFieldList = (
      field: any,
      isInterestingField: boolean
    ) => {
      if (isInterestingField) {
        const index = searchObj.data.stream.interestingFieldList.indexOf(
          field.name
        );
        if (index > -1) {
          // only splice array when item is found
          searchObj.data.stream.interestingFieldList.splice(index, 1); // 2nd parameter means remove one item only

          field.isInterestingField = !isInterestingField;
          const localInterestingFields: any = useLocalInterestingFields();
          let localStreamFields: any = {};
          if (localInterestingFields.value != null) {
            localStreamFields = localInterestingFields.value;
          }
          if (field.streams.length > 0) {
            let localFieldIndex = -1;
            for (const selectedStream of field.streams) {
              localFieldIndex = localStreamFields[
                searchObj.organizationIdetifier + "_" + selectedStream
              ].indexOf(field.name);
              if (localFieldIndex > -1) {
                localStreamFields[
                  searchObj.organizationIdetifier + "_" + selectedStream
                ].splice(localFieldIndex, 1);
              }
            }
          }
          useLocalInterestingFields(localStreamFields);
        }
      } else {
        const index = searchObj.data.stream.interestingFieldList.indexOf(
          field.name
        );
        if (index == -1 && field.name != "*") {
          searchObj.data.stream.interestingFieldList.push(field.name);
          const localInterestingFields: any = useLocalInterestingFields();
          field.isInterestingField = !isInterestingField;
          let localStreamFields: any = {};
          if (localInterestingFields.value != null) {
            localStreamFields = localInterestingFields.value;
          }
          if (field.streams.length > 0) {
            for (const selectedStream of field.streams) {
              if(selectedStream != undefined) {
                if (
                  localStreamFields[
                    searchObj.organizationIdetifier + "_" + selectedStream
                  ] == undefined
                ) {
                  localStreamFields[
                    searchObj.organizationIdetifier + "_" + selectedStream
                  ] = [];
                }

                if (
                  localStreamFields[
                    searchObj.organizationIdetifier + "_" + selectedStream
                  ].indexOf(field.name) == -1
                ) {
                  localStreamFields[
                    searchObj.organizationIdetifier + "_" + selectedStream
                  ].push(field.name);
                }
              }
            }
          }
          useLocalInterestingFields(localStreamFields);
        }
      }

      field.isInterestingField = !isInterestingField;

      const localInterestingFields: any = useLocalInterestingFields();
      let localFields: any = {};
      if (localInterestingFields.value != null) {
        localFields = localInterestingFields.value;
        localFields[
          searchObj.organizationIdetifier +
            "_" +
            searchObj.data.stream.selectedStream.value
        ] = searchObj.data.stream.interestingFieldList;
        useLocalInterestingFields(localFields);
      }

      emit("setInterestingFieldInSQLQuery", field, isInterestingField);
    };

    const pagination = ref({
      page: 1,
      rowsPerPage: 25,
    });

    const toggleSchema = async () => {
      searchObj.loadingStream = true;
      setTimeout(async () => {
        await extractFields();
        searchObj.loadingStream = false;
      }, 0);
    };

    const sortedStreamFields = () => {
      return searchObj.data.stream.selectedStreamFields.sort(
        (a: any, b: any) => a.group - b.group
      );
    };

    return {
      t,
      store,
      router,
      searchObj,
      streamOptions,
      filterFieldFn,
      addToFilter,
      clickFieldFn,
      getImageURL,
      filterStreamFn,
      openFilterCreator,
      addSearchTerm,
      fieldValues,
      streamTypes,
      outlinedAdd,
      outlinedVisibilityOff,
      outlinedVisibility,
      handleQueryData,
      onStreamChange,
      addToInterestingFieldList,
      extractFields,
      userDefinedSchemaBtnGroupOption,
      pagination,
      toggleSchema,
      pagesNumber: computed(() => {
        return Math.ceil(
          searchObj.data.stream.selectedStreamFields.length /
            pagination.value.rowsPerPage
        );
      }),
      formatLargeNumber,
      sortedStreamFields,
    };
  },
});
</script>

<style lang="scss">
$streamSelectorHeight: 44px;

.logs-index-menu {
  width: 100%;

  .q-menu {
    box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
    transform: translateY(0.5rem);
    border-radius: 0px;

    .q-virtual-scroll__content {
      padding: 0.5rem;
    }
  }

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

  .index-table {
    width: 100%;
    height: calc(100% - $streamSelectorHeight);
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

    > .q-table__bottom {
      padding: 0px !important;
    }
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
      height: 20px;
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
        // background-color: #ffffff;
      }
    }
  }

  &.theme-dark {
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

  &.theme-light {
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

  .q-item {
    min-height: 1.3rem;
    padding: 5px 10px;

    &__label {
      font-size: 0.75rem;
    }

    &.q-manual-focusable--focused > .q-focus-helper {
      background: currentColor !important;
      opacity: 0.3 !important;
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
}
</style>

<style lang="scss">
.index-table {
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

        .q-icon {
          font-size: 18px;
          color: #808080;
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
          // background-color: #ffffff;
        }
      }
    }
  }

  .schema-field-toggle {
    border: 1px solid light-grey;
    border-radius: 5px;
    line-height: 10px;
  }

  .q-table__bottom {
    padding: 0px !important;
  }

  .pagination-field-count {
    line-height: 32px;
    font-weight: 700;
    font-size: 13px;
  }
}
</style>

<style lang="scss">
.field-table {
  .q-table__bottom {
    padding: 5px !important;
  }

  .schema-field-toggle .q-btn {
    padding: 5px !important;
  }
}

.q-field--auto-height.q-field--dense .q-field__control,
.q-field--auto-height.q-field--dense .q-field__native,
.q-field--auto-height.q-field--dense .q-field__native span {
  text-overflow: ellipsis !important;
  overflow: hidden !important;
  white-space: nowrap !important;
  max-height: 40px !important;
  
  span, .q-field__input {
    margin-top: -12px;
  }
}
</style>
