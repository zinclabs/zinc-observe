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
  <div class="logs-search-bar-component" id="searchBarComponent">
    <div class="row">
      <div class="float-right col q-mb-xs">
        <q-toggle
          data-test="logs-search-bar-show-histogram-toggle-btn"
          v-model="searchObj.meta.showHistogram"
          :label="t('search.showHistogramLabel')"
        />
        <q-toggle
          data-test="logs-search-bar-sql-mode-toggle-btn"
          v-model="searchObj.meta.sqlMode"
          :label="t('search.sqlModeLabel')"
        />
        <q-btn
          data-test="logs-search-bar-reset-filters-btn"
          label="Reset Filters"
          no-caps
          size="sm"
          icon="restart_alt"
          class="q-pr-sm q-pl-xs reset-filters q-ml-xs"
          @click="resetFilters"
        />
        <syntax-guide
          data-test="logs-search-bar-sql-mode-toggle-btn"
          :sqlmode="searchObj.meta.sqlMode"
        ></syntax-guide>
        <q-btn-group class="q-ml-xs no-outline q-pa-none no-border">
          <q-btn-dropdown
            data-test="logs-search-saved-views-btn"
            v-model="savedViewDropdownModel"
            size="12px"
            icon="save"
            icon-right="saved_search"
            :title="t('search.savedViewsLabel')"
            @click="fnSavedView"
            @show="loadSavedView"
            split
            class="no-outline saved-views-dropdown no-border"
          >
            <q-list
              :style="
                localSavedViews.length > 0 ? 'width: 500px' : 'width: 250px'
              "
              data-test="logs-search-saved-view-list"
            >
              <q-item style="padding: 0px 0px 0px 0px">
                <q-item-section
                  class="column"
                  style="width: 60%; border-right: 1px solid lightgray"
                >
                  <q-table
                    data-test="log-search-saved-view-list-fields-table"
                    :visible-columns="['view_name']"
                    :rows="searchObj.data.savedViews"
                    :row-key="(row) => 'saved_view_' + row.view_id"
                    :filter="searchObj.data.savedViewFilterFields"
                    :filter-method="filterSavedViewFn"
                    :pagination="{ rowsPerPage }"
                    hide-header
                    :wrap-cells="searchObj.meta.resultGrid.wrapCells"
                    class="saved-view-table full-height"
                    id="savedViewList"
                    :rows-per-page-options="[]"
                    :hide-bottom="
                      searchObj.data.savedViews.length <= rowsPerPage ||
                      searchObj.data.savedViews.length == 0
                    "
                  >
                    <template #top-right>
                      <div class="full-width">
                        <q-input
                          data-test="log-search-saved-view-field-search-input"
                          v-model="searchObj.data.savedViewFilterFields"
                          data-cy="index-field-search-input"
                          filled
                          borderless
                          dense
                          clearable
                          debounce="1"
                          :placeholder="t('search.searchSavedView')"
                        >
                          <template #prepend>
                            <q-icon name="search" />
                          </template>
                        </q-input>
                      </div>
                      <div
                        v-if="searchObj.loadingSavedView == true"
                        class="full-width float-left"
                      >
                        <div class="text-subtitle2 text-weight-bold float-left">
                          <q-spinner-hourglass size="20px" />
                          {{ t("confirmDialog.loading") }}
                        </div>
                      </div>
                      <q-tr>
                        <q-td
                          v-if="
                            searchObj.data.savedViews.length == 0 &&
                            searchObj.loadingSavedView == false
                          "
                        >
                          <q-item-label class="q-pl-sm q-pt-sm">{{
                            t("search.savedViewsNotFound")
                          }}</q-item-label>
                        </q-td>
                      </q-tr>
                    </template>
                    <template v-slot:body-cell-view_name="props">
                      <q-td :props="props" class="field_list">
                        <q-item
                          class="q-pa-sm saved-view-item"
                          clickable
                          v-close-popup
                        >
                          <q-item-section
                            @click.stop="applySavedView(props.row)"
                            v-close-popup
                            :title="props.row.view_name"
                          >
                            <q-item-label
                              class="ellipsis"
                              style="max-width: 188px"
                              >{{ props.row.view_name }}</q-item-label
                            >
                          </q-item-section>
                          <q-item-section
                            :data-test="`logs-search-bar-favorite-${props.row.view_name}-saved-view-btn`"
                            side
                            @click.stop="
                              handleFavoriteSavedView(
                                props.row,
                                favoriteViews.includes(props.row.view_id)
                              )
                            "
                          >
                            <q-icon
                              :name="
                                favoriteViews.includes(props.row.view_id)
                                  ? 'favorite'
                                  : 'favorite_border'
                              "
                              color="grey"
                              size="xs"
                            />
                          </q-item-section>
                          <q-item-section
                            :data-test="`logs-search-bar-delete-${props.row.view_name}-saved-view-btn`"
                            side
                            @click.stop="handleDeleteSavedView(props.row)"
                          >
                            <q-icon name="delete" color="grey" size="xs" />
                          </q-item-section>
                        </q-item> </q-td
                    ></template>
                  </q-table>
                </q-item-section>

                <q-item-section
                  class="column"
                  style="width: 40%; margin-left: 0px"
                  v-if="localSavedViews.length > 0"
                >
                  <q-table
                    data-test="log-search-saved-view-favorite-list-fields-table"
                    :visible-columns="['view_name']"
                    :rows="localSavedViews"
                    :row-key="(row) => 'favorite_saved_view_' + row.view_name"
                    hide-header
                    hide-bottom
                    :wrap-cells="searchObj.meta.resultGrid.wrapCells"
                    class="saved-view-table full-height"
                    id="savedViewFavoriteList"
                    :rows-per-page-options="[0]"
                  >
                    <template #top-right>
                      <q-item style="padding: 0px"
                        ><q-item-label
                          header
                          class="q-pa-sm text-bold favorite-label"
                          >Favorite Views</q-item-label
                        ></q-item
                      >
                      <q-separator horizontal inset></q-separator>
                    </template>
                    <template v-slot:body-cell-view_name="props">
                      <q-td :props="props" class="field_list q-pa-xs">
                        <q-item
                          class="q-pa-sm saved-view-item"
                          clickable
                          v-close-popup
                        >
                          <q-item-section
                            @click.stop="applySavedView(props.row)"
                            v-close-popup
                          >
                            <q-item-label
                              class="ellipsis"
                              style="max-width: 185px"
                              >{{ props.row.view_name }}</q-item-label
                            >
                          </q-item-section>
                          <q-item-section
                            :data-test="`logs-search-bar-favorite-${props.row.view_name}-saved-view-btn`"
                            side
                            @click.stop="
                              handleFavoriteSavedView(
                                props.row,
                                favoriteViews.includes(props.row.view_id)
                              )
                            "
                          >
                            <q-icon
                              :name="
                                favoriteViews.includes(props.row.view_id)
                                  ? 'favorite'
                                  : 'favorite_border'
                              "
                              color="grey"
                              size="xs"
                            />
                          </q-item-section>
                        </q-item> </q-td
                    ></template>
                  </q-table>
                </q-item-section>
              </q-item>
            </q-list>
          </q-btn-dropdown>
        </q-btn-group>
        <q-toggle
          data-test="logs-search-bar-quick-mode-toggle-btn"
          v-model="searchObj.meta.quickMode"
          :label="t('search.quickModeLabel')"
          @click="handleQuickMode"
        />
      </div>
      <div class="float-right col-auto q-mb-xs">
        <q-toggle
          data-test="logs-search-bar-wrap-table-content-toggle-btn"
          v-if="searchObj.meta.flagWrapContent"
          v-model="searchObj.meta.toggleSourceWrap"
          icon="wrap_text"
          :title="t('search.messageWrapContent')"
          class="float-left"
          size="32px"
        />
        <q-toggle
          data-test="logs-search-bar-show-query-toggle-btn"
          v-model="searchObj.meta.toggleFunction"
          :icon="'img:' + getImageURL('images/common/function.svg')"
          title="Toggle Function Editor"
          class="float-left"
          size="32px"
        />
        <q-btn-group
          class="no-outline q-pa-none no-border float-left q-mr-xs"
          :disable="!searchObj.meta.toggleFunction"
        >
          <q-btn-dropdown
            data-test="logs-search-bar-function-dropdown"
            v-model="functionModel"
            auto-close
            size="12px"
            icon="save"
            :icon-right="'img:' + getImageURL('images/common/function.svg')"
            :title="t('search.functionPlaceholder')"
            split
            class="no-outline saved-views-dropdown no-border btn-function"
            @click="fnSavedFunctionDialog"
          >
            <q-list data-test="logs-search-saved-function-list">
              <q-item-label header class="q-pa-sm">{{
                t("search.functionPlaceholder")
              }}</q-item-label>
              <q-separator inset></q-separator>

              <div v-if="functionOptions.length">
                <q-item
                  class="q-pa-sm saved-view-item"
                  clickable
                  v-for="(item, i) in functionOptions"
                  :key="'saved-view-' + i"
                  v-close-popup
                >
                  <q-item-section
                    @click.stop="populateFunctionImplementation(item, true)"
                    v-close-popup
                  >
                    <q-item-label>{{ item.name }}</q-item-label>
                  </q-item-section>
                </q-item>
              </div>
              <div v-else>
                <q-item>
                  <q-item-section>
                    <q-item-label>{{
                      t("search.savedFunctionNotFound")
                    }}</q-item-label>
                  </q-item-section>
                </q-item>
              </div>
            </q-list>
          </q-btn-dropdown>
        </q-btn-group>
        <q-btn-group class="no-outline q-pa-none no-border">
          <q-btn-dropdown
            data-test="logs-search-bar-reset-function-btn"
            class="q-mr-xs download-logs-btn q-px-xs"
            size="sm"
            icon="download"
            :title="t('search.exportLogs')"
          >
            <q-list>
              <q-item
                class="q-pa-sm saved-view-item"
                clickable
                v-close-popup
                v-bind:disable="
                  searchObj.data.queryResults &&
                  searchObj.data.queryResults.hasOwnProperty('hits') &&
                  !searchObj.data.queryResults.hits.length
                "
              >
                <q-item-section
                  @click.stop="downloadLogs(searchObj.data.queryResults.hits)"
                  v-close-popup
                >
                  <q-item-label>{{ t("search.downloadTable") }}</q-item-label>
                </q-item-section>
              </q-item>
              <q-separator />
              <q-item class="q-pa-sm saved-view-item" clickable v-close-popup>
                <q-item-section
                  @click.stop="toggleCustomDownloadDialog"
                  v-close-popup
                >
                  <q-item-label>{{ t("search.customRange") }}</q-item-label>
                </q-item-section>
              </q-item>
            </q-list>
          </q-btn-dropdown>
        </q-btn-group>
        <q-btn
          data-test="logs-search-bar-share-link-btn"
          class="q-mr-xs download-logs-btn q-px-sm"
          size="sm"
          icon="share"
          :title="t('search.shareLink')"
          @click="shareLink"
        ></q-btn>
        <div class="float-left">
          <date-time
            ref="dateTimeRef"
            auto-apply
            :default-type="searchObj.data.datetime.type"
            :default-absolute-time="{
              startTime: searchObj.data.datetime.startTime,
              endTime: searchObj.data.datetime.endTime,
            }"
            :default-relative-time="searchObj.data.datetime.relativeTimePeriod"
            data-test="logs-search-bar-date-time-dropdown"
            @on:date-change="updateDateTime"
            @on:timezone-change="updateTimezone"
          />
        </div>
        <div class="search-time float-left q-mr-xs">
          <div class="flex">
            <auto-refresh-interval
              class="q-mr-xs q-px-none logs-auto-refresh-interval"
              v-model="searchObj.meta.refreshInterval"
              @update:model-value="onRefreshIntervalUpdate"
            />
            <q-btn-group
              class="no-outline q-pa-none no-border q-mr-xs"
              v-if="
                config.isEnterprise == 'true' &&
                Object.keys(store.state.regionInfo).length > 0 &&
                store.state.zoConfig.super_cluster_enabled
              "
            >
              <q-btn-dropdown
                data-test="logs-search-bar-region-btn"
                class="region-dropdown-btn q-px-xs"
                :title="t('search.regionTitle')"
                label="Region"
              >
                <q-input
                  ref="reginFilterRef"
                  filled
                  flat
                  dense
                  v-model="regionFilter"
                  :label="t('search.regionFilterMsg')"
                >
                  <template v-slot:append>
                    <q-icon
                      v-if="regionFilter !== ''"
                      name="clear"
                      class="cursor-pointer"
                      @click="resetRegionFilter"
                    />
                  </template>
                </q-input>
                <q-tree
                  class="col-12 col-sm-6"
                  :nodes="store.state.regionInfo"
                  node-key="label"
                  :filter="regionFilter"
                  :filter-method="regionFilterMethod"
                  tick-strategy="leaf"
                  v-model:ticked="searchObj.meta.clusters"
                />
              </q-btn-dropdown>
            </q-btn-group>

            <q-btn
              data-test="logs-search-bar-refresh-btn"
              data-cy="search-bar-refresh-button"
              dense
              flat
              :title="t('search.runQuery')"
              class="q-pa-none search-button"
              @click="handleRunQuery"
              :disable="
                searchObj.loading == 'true' ||
                (searchObj.data.hasOwnProperty('streamResults') &&
                  searchObj.data.streamResults?.list?.length == 0)
              "
              >{{ t("search.runQuery") }}</q-btn
            >
          </div>
        </div>
      </div>
    </div>
    <div class="row query-editor-container" v-show="searchObj.meta.showQuery">
      <div class="col" style="border-top: 1px solid #dbdbdb; height: 100%">
        <q-splitter
          class="logs-search-splitter"
          no-scroll
          v-model="searchObj.config.fnSplitterModel"
          :limits="searchObj.config.fnSplitterLimit"
          style="width: 100%; height: 100%"
        >
          <template #before>
            <query-editor
              data-test="logs-search-bar-query-editor"
              editor-id="logsQueryEditor"
              ref="queryEditorRef"
              class="monaco-editor"
              v-model:query="searchObj.data.query"
              :keywords="autoCompleteKeywords"
              :suggestions="autoCompleteSuggestions"
              @update:query="updateQueryValue"
              @run-query="handleRunQuery"
              :class="
                searchObj.data.editorValue == '' &&
                searchObj.meta.queryEditorPlaceholderFlag
                  ? 'empty-query'
                  : ''
              "
              @focus="searchObj.meta.queryEditorPlaceholderFlag = false"
              @blur="searchObj.meta.queryEditorPlaceholderFlag = true"
            />
          </template>
          <template #after>
            <div
              data-test="logs-vrl-function-editor"
              v-show="searchObj.meta.toggleFunction"
              style="width: 100%; height: 100%"
            >
              <query-editor
                data-test="logs-vrl-function-editor"
                ref="fnEditorRef"
                editor-id="fnEditor"
                class="monaco-editor"
                v-model:query="searchObj.data.tempFunctionContent"
                :class="
                  searchObj.data.tempFunctionContent == '' &&
                  searchObj.meta.functionEditorPlaceholderFlag
                    ? 'empty-function'
                    : ''
                "
                language="ruby"
                @focus="searchObj.meta.functionEditorPlaceholderFlag = false"
                @blur="searchObj.meta.functionEditorPlaceholderFlag = true"
              />
            </div>
          </template>
        </q-splitter>
      </div>
    </div>

    <q-dialog ref="confirmDialog" v-model="confirmDialogVisible">
      <q-card>
        <q-card-section>
          {{ confirmMessage }}
        </q-card-section>

        <q-card-actions align="right">
          <q-btn
            data-test="logs-search-bar-confirm-dialog-cancel-btn"
            :label="t('confirmDialog.cancel')"
            color="primary"
            @click="cancelConfirmDialog"
          />
          <q-btn
            data-test="logs-search-bar-confirm-dialog-ok-btn"
            :label="t('confirmDialog.ok')"
            color="positive"
            @click="confirmDialogOK"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <q-dialog
      ref="confirmSavedViewDialog"
      v-model="confirmSavedViewDialogVisible"
    >
      <q-card>
        <q-card-section>
          {{ confirmMessageSavedView }}
        </q-card-section>

        <q-card-actions align="right">
          <q-btn
            data-test="logs-search-bar-confirm-dialog-cancel-btn"
            :label="t('confirmDialog.cancel')"
            color="primary"
            @click="cancelConfirmDialog"
          />
          <q-btn
            data-test="logs-search-bar-confirm-dialog-ok-btn"
            :label="t('confirmDialog.ok')"
            color="positive"
            @click="confirmDialogOK"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <q-dialog v-model="customDownloadDialog">
      <q-card>
        <q-card-section>
          {{ t("search.customDownloadMessage") }}
        </q-card-section>

        <q-card-section>
          <q-input
            type="number"
            data-test="custom-download-initial-number-input"
            v-model="downloadCustomInitialNumber"
            :label="t('search.initialNumber')"
            default-value="0"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            tabindex="0"
            min="0"
          />
          <q-select
            data-test="custom-download-range-select"
            v-model="downloadCustomRange"
            :options="downloadCustomRangeOptions"
            :label="t('search.range')"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm showLabelOnTop"
            stack-label
            outlined
            filled
            dense
          />
        </q-card-section>

        <q-card-actions align="right">
          <q-btn
            unelevated
            no-caps
            class="q-mr-sm text-bold"
            data-test="logs-search-bar-confirm-dialog-cancel-btn"
            :label="t('confirmDialog.cancel')"
            color="secondary"
            v-close-popup
          />
          <q-btn
            unelevated
            no-caps
            class="q-mr-sm text-bold"
            data-test="logs-search-bar-confirm-dialog-ok-btn"
            :label="t('search.btnDownload')"
            color="primary"
            @click="downloadRangeData"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <q-dialog v-model="store.state.savedViewDialog">
      <q-card style="width: 700px; max-width: 80vw">
        <q-card-section>
          <div class="text-h6">{{ t("search.savedViewsLabel") }}</div>
        </q-card-section>

        <q-card-section class="q-pt-none">
          <span>Update</span>
          <q-toggle
            data-test="saved-view-action-toggle"
            v-bind:disable="searchObj.data.savedViews.length == 0"
            name="saved_view_action"
            v-model="isSavedViewAction"
            true-value="create"
            false-value="update"
            label=""
            @change="savedViewName = ''"
          />
          <span>Create</span>
          <div v-if="isSavedViewAction == 'create'">
            <q-input
              data-test="add-alert-name-input"
              v-model="savedViewName"
              :label="t('search.savedViewName')"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[
                (val) => !!val.trim() || 'This field is required',
                (val) =>
                  /^[-A-Za-z0-9 /@/_]+$/.test(val) ||
                  'Input must be alphanumeric',
              ]"
              tabindex="0"
            />
          </div>
          <div v-else>
            <q-select
              data-test="saved-view-name-select"
              v-model="savedViewSelectedName"
              :options="searchObj.data.savedViews"
              option-label="view_name"
              option-value="view_id"
              :label="t('search.savedViewName')"
              :popup-content-style="{ textTransform: 'capitalize' }"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[(val: any) => !!val || 'Field is required!']"
            />
          </div>
        </q-card-section>

        <q-card-actions align="right" class="bg-white text-teal">
          <q-btn
            data-test="saved-view-dialog-cancel-btn"
            unelevated
            no-caps
            class="q-mr-sm text-bold"
            :label="t('confirmDialog.cancel')"
            color="secondary"
            v-close-popup
          />
          <q-btn
            data-test="saved-view-dialog-save-btn"
            v-if="!saveViewLoader"
            unelevated
            no-caps
            :label="t('confirmDialog.ok')"
            color="primary"
            class="text-bold"
            @click="handleSavedView"
          />
          <q-btn
            data-test="saved-view-dialog-loading-btn"
            v-if="saveViewLoader"
            unelevated
            no-caps
            :label="t('confirmDialog.loading')"
            color="primary"
            class="text-bold"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <q-dialog v-model="store.state.savedFunctionDialog">
      <q-card style="width: 700px; max-width: 80vw">
        <q-card-section>
          <div class="text-h6">{{ t("search.functionPlaceholder") }}</div>
        </q-card-section>

        <q-card-section class="q-pt-none">
          <span>Update</span>
          <q-toggle
            data-test="saved-function-action-toggle"
            v-bind:disable="functionOptions.length == 0"
            name="saved_function_action"
            v-model="isSavedFunctionAction"
            true-value="create"
            false-value="update"
            label=""
            @change="savedFunctionName = ''"
          />
          <span>Create</span>
          <div v-if="isSavedFunctionAction == 'create'">
            <q-input
              data-test="saved-function-name-input"
              v-model="savedFunctionName"
              :label="t('search.saveFunctionName')"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[
                (val) => !!val.trim() || 'This field is required',
                (val) =>
                  /^[-A-Za-z0-9/_]+$/.test(val) || 'Input must be alphanumeric',
              ]"
              tabindex="0"
            />
          </div>
          <div v-else>
            <q-select
              data-test="saved-function-name-select"
              v-model="savedFunctionSelectedName"
              :options="functionOptions"
              option-label="name"
              option-value="name"
              :label="t('search.saveFunctionName')"
              placeholder="Select Function Name"
              :popup-content-style="{ textTransform: 'capitalize' }"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[(val: any) => !!val || 'Field is required!']"
            />
          </div>
        </q-card-section>

        <q-card-actions align="right" class="bg-white text-teal">
          <q-btn
            data-test="saved-function-dialog-cancel-btn"
            unelevated
            no-caps
            class="q-mr-sm text-bold"
            :label="t('confirmDialog.cancel')"
            color="secondary"
            v-close-popup
          />
          <q-btn
            data-test="saved-view-dialog-save-btn"
            v-if="!saveFunctionLoader"
            unelevated
            no-caps
            :label="t('confirmDialog.ok')"
            color="primary"
            class="text-bold"
            @click="saveFunction"
          />
          <q-btn
            data-test="saved-function-dialog-loading-btn"
            v-if="saveFunctionLoader"
            unelevated
            no-caps
            :label="t('confirmDialog.loading')"
            color="primary"
            class="text-bold"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <ConfirmDialog
      title="Delete Saved View"
      message="Are you sure you want to delete saved view?"
      @update:ok="confirmDeleteSavedViews"
      @update:cancel="confirmDelete = false"
      v-model="confirmDelete"
    />
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import {
  defineComponent,
  ref,
  onMounted,
  nextTick,
  watch,
  toRaw,
  onActivated,
  onUnmounted,
  onDeactivated,
  defineAsyncComponent,
  onBeforeMount,
} from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useStore } from "vuex";
import { useQuasar, copyToClipboard } from "quasar";

import DateTime from "@/components/DateTime.vue";
import useLogs from "@/composables/useLogs";
import SyntaxGuide from "./SyntaxGuide.vue";
import jsTransformService from "@/services/jstransform";
import searchService from "@/services/search";

import segment from "@/services/segment_analytics";
import config from "@/aws-exports";

import AutoRefreshInterval from "@/components/AutoRefreshInterval.vue";
import useSqlSuggestions from "@/composables/useSuggestions";
import {
  mergeDeep,
  b64DecodeUnicode,
  getImageURL,
  useLocalInterestingFields,
  useLocalSavedView,
} from "@/utils/zincutils";
import savedviewsService from "@/services/saved_views";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import { cloneDeep } from "lodash-es";

const defaultValue: any = () => {
  return {
    name: "",
    function: "",
    params: "row",
    transType: "0",
  };
};

export default defineComponent({
  name: "ComponentSearchSearchBar",
  components: {
    DateTime,
    QueryEditor: defineAsyncComponent(
      () => import("@/components/QueryEditor.vue")
    ),
    SyntaxGuide,
    AutoRefreshInterval,
    ConfirmDialog,
  },
  emits: [
    "searchdata",
    "onChangeInterval",
    "onChangeTimezone",
    "handleQuickModeChange",
  ],
  methods: {
    searchData() {
      if (this.searchObj.loading == false) {
        // this.searchObj.runQuery = true;
        this.$emit("searchdata");
      }
    },
    changeFunctionName(value) {
      // alert(value)
      console.log(value);
    },
    createNewValue(inputValue, doneFn) {
      // Call the doneFn with the new value
      doneFn(inputValue);
    },
    updateSelectedValue() {
      // Update the selected value with the newly created value
      if (
        this.functionModel &&
        !this.functionOptions.includes(this.functionModel)
      ) {
        this.functionOptions.push(this.functionModel);
      }
    },
    handleDeleteSavedView(item: any) {
      this.savedViewDropdownModel = false;
      this.deleteViewID = item.view_id;
      this.confirmDelete = true;
    },
    confirmDeleteSavedViews() {
      this.deleteSavedViews();
    },
    toggleCustomDownloadDialog() {
      this.customDownloadDialog = true;
    },
    downloadRangeData() {
      if (
        this.downloadCustomInitialNumber < 0 ||
        this.downloadCustomInitialNumber == ""
      ) {
        this.$q.notify({
          message: "Initial number must be positive number.",
          color: "negative",
          position: "bottom",
          timeout: 2000,
        });
        return;
      }
      // const queryReq = this.buildSearch();
      // console.log(this.searchObj.data.customDownloadQueryObj)
      this.searchObj.data.customDownloadQueryObj.query.from = parseInt(
        this.downloadCustomInitialNumber
      );
      this.searchObj.data.customDownloadQueryObj.query.size =
        this.downloadCustomRange;
      searchService
        .search({
          org_identifier: this.searchObj.organizationIdetifier,
          query: this.searchObj.data.customDownloadQueryObj,
          page_type: this.searchObj.data.stream.streamType,
        }, "UI")
        .then((res) => {
          this.customDownloadDialog = false;
          if (res.data.hits.length > 0) {
            this.downloadLogs(res.data.hits);
          } else {
            this.$q.notify({
              message: "No data found to download.",
              color: "positive",
              position: "top",
              timeout: 2000,
            });
          }
        })
        .catch((err) => {
          this.$q.notify({
            message: err.message,
            color: "negative",
            position: "top",
            timeout: 2000,
          });
        });
    },
  },
  props: {
    fieldValues: {
      type: Object,
      default: () => ({}),
    },
  },
  setup(props, { emit }) {
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const store = useStore();
    const rowsPerPage = ref(10);
    const regionFilter = ref();
    const regionFilterRef = ref(null);

    const {
      searchObj,
      refreshData,
      handleRunQuery,
      updatedLocalLogFilterField,
      getSavedViews,
      getQueryData,
      getStreams,
      updateUrlQueryParams,
      generateURLQuery,
      buildSearch,
      resetStreamData,
      loadStreamLists,
      fnParsedSQL,
      onStreamChange,
      moveItemsToTop,
    } = useLogs();
    const queryEditorRef = ref(null);

    const formData: any = ref(defaultValue());
    const functionOptions = ref(searchObj.data.transforms);

    const functionModel: string = ref(null);
    const fnEditorRef: any = ref(null);

    const isSavedFunctionAction: string = ref("create");
    const savedFunctionName: string = ref("");
    const savedFunctionSelectedName: string = ref("");
    const saveFunctionLoader = ref(false);

    const confirmDialogVisible: boolean = ref(false);
    const confirmSavedViewDialogVisible: boolean = ref(false);
    let confirmCallback;
    let streamName = "";

    let parser: any;
    const dateTimeRef = ref(null);
    const saveViewLoader = ref(false);
    const favoriteViews = ref([]);

    const localSavedViews = ref([]);
    let savedViews = useLocalSavedView();
    if (savedViews.value != null) {
      const favoriteValues = [];
      Object.values(savedViews.value).forEach((view) => {
        if (view.org_id === store.state.selectedOrganization.identifier) {
          favoriteViews.value.push(view.view_id);
          favoriteValues.push(view);
        }
      });

      localSavedViews.value.push(...favoriteValues);
    }

    const {
      autoCompleteData,
      autoCompleteKeywords,
      autoCompleteSuggestions,
      getSuggestions,
      updateFieldKeywords,
      updateFunctionKeywords,
    } = useSqlSuggestions();

    const refreshTimeChange = (item) => {
      searchObj.meta.refreshInterval = Number(item.value);
    };

    const isSavedViewAction = ref("create");
    const savedViewName = ref("");
    const savedViewSelectedName = ref("");
    const confirmDelete = ref(false);
    const deleteViewID = ref("");
    const savedViewDropdownModel = ref(false);

    watch(
      () => searchObj.data.stream.selectedStreamFields,
      (fields) => {
        if (fields != undefined && fields.length) updateFieldKeywords(fields);
      },
      { immediate: true, deep: true }
    );

    watch(
      () => searchObj.data.stream.functions,
      (funs) => {
        if (funs.length) updateFunctionKeywords(funs);
      },
      { immediate: true, deep: true }
    );

    onBeforeMount(async () => {
      await importSqlParser();
    });

    const importSqlParser = async () => {
      const useSqlParser: any = await import("@/composables/useParser");
      const { sqlParser }: any = useSqlParser.default();
      parser = await sqlParser();
    };

    const updateAutoComplete = (value) => {
      autoCompleteData.value.query = value;
      autoCompleteData.value.cursorIndex =
        queryEditorRef.value.getCursorIndex();
      autoCompleteData.value.fieldValues = props.fieldValues;
      autoCompleteData.value.popup.open =
        queryEditorRef.value.triggerAutoComplete;
      getSuggestions();
    };

    function getColumnNames(data) {
      const columnNames = [];
      data.forEach((item) => {
        if (item.expr && item.expr.column) {
          columnNames.push(item.expr.column);
        } else if (item.expr && item.expr.args && item.expr.args.expr) {
          if (item.expr.args.expr.column) {
            columnNames.push(item.expr.args.expr.column);
          } else if (item.expr.args.expr.value) {
            columnNames.push(item.expr.args.expr.value);
          }
        }
      });
      return columnNames;
    }

    const updateQueryValue = (value: string) => {
      if (searchObj.meta.quickMode == true) {
        const parsedSQL = fnParsedSQL();

        if (
          parsedSQL != undefined &&
          parsedSQL.hasOwnProperty("from") &&
          parsedSQL?.from.length > 0 &&
          parsedSQL?.from[0].table !==
            searchObj.data.stream.selectedStream.value
        ) {
          searchObj.data.stream.selectedStream = {
            label: parsedSQL.from[0].table,
            value: parsedSQL.from[0].table,
          };
          searchObj.data.stream.selectedStreamFields = [];
          onStreamChange(value);
        }
        // if (
        //   parsedSQL.hasOwnProperty("columns") &&
        //   parsedSQL?.columns.length > 0
        // ) {
        //   const columnNames = getColumnNames(parsedSQL?.columns);
        //   searchObj.data.stream.interestingFieldList = [];
        //   for (const [index, col] of columnNames.entries()) {
        //     if (
        //       !searchObj.data.stream.interestingFieldList.includes(col) &&
        //       col != "*"
        //     ) {
        //       // searchObj.data.stream.interestingFieldList.push(col);
        //       for (const stream of searchObj.data.streamResults.list) {
        //         if (stream.value == col) {
        //           searchObj.data.stream.interestingFieldList.push(col);
        //         }
        //       }
        //     }
        //   }
        if (parsedSQL?.columns?.length > 0) {
          const columnNames = getColumnNames(parsedSQL?.columns);
          searchObj.data.stream.interestingFieldList = [];
          for (const col of columnNames) {
            if (
              !searchObj.data.stream.interestingFieldList.includes(col) &&
              col != "*"
            ) {
              // searchObj.data.stream.interestingFieldList.push(col);
              for (const stream of searchObj.data.stream.selectedStreamFields) {
                if (stream.name == col) {
                  searchObj.data.stream.interestingFieldList.push(col);
                  const localInterestingFields: any =
                    useLocalInterestingFields();
                  let localFields: any = {};
                  if (localInterestingFields.value != null) {
                    localFields = localInterestingFields.value;
                  }
                  localFields[
                    searchObj.organizationIdetifier +
                      "_" +
                      searchObj.data.stream.selectedStream.value
                  ] = searchObj.data.stream.interestingFieldList;
                  useLocalInterestingFields(localFields);
                }
              }
            }
          }

          for (const item of searchObj.data.stream.selectedStreamFields) {
            if (
              searchObj.data.stream.interestingFieldList.includes(item.name)
            ) {
              item.isInterestingField = true;
            } else {
              item.isInterestingField = false;
            }
          }
        }
      }

      searchObj.data.editorValue = value;

      updateAutoComplete(value);
      try {
        if (searchObj.meta.sqlMode == true) {
          searchObj.data.parsedQuery = parser.astify(value);
          if (searchObj.data.parsedQuery?.from?.length > 0) {
            if (
              searchObj.data.parsedQuery.from[0].table !==
                searchObj.data.stream.selectedStream.length > 0 &&
              searchObj.data.parsedQuery.from[0].table !== streamName
            ) {
              let streamFound = false;
              streamName = searchObj.data.parsedQuery.from[0].table;
              searchObj.data.streamResults.list.forEach((stream) => {
                if (stream.name == searchObj.data.parsedQuery.from[0].table) {
                  streamFound = true;
                  let itemObj = {
                    label: stream.name,
                    value: stream.name,
                  };
                  searchObj.data.stream.selectedStream = itemObj;
                  stream.schema.forEach((field) => {
                    searchObj.data.stream.selectedStreamFields.push({
                      name: field.name,
                    });
                  });
                }
              });
              if (streamFound == false) {
                searchObj.data.stream.selectedStream = { label: "", value: "" };
                searchObj.data.stream.selectedStreamFields = [];
                $q.notify({
                  message: "Stream not found",
                  color: "negative",
                  position: "top",
                  timeout: 2000,
                });
              }
            }
          }
        }
      } catch (e) {
        console.log("Logs: Error while updating query value");
      }
    };

    const updateDateTime = async (value: object) => {
      searchObj.data.datetime = {
        startTime: value.startTime,
        endTime: value.endTime,
        relativeTimePeriod: value.relativeTimePeriod
          ? value.relativeTimePeriod
          : searchObj.data.datetime.relativeTimePeriod,
        type: value.relativeTimePeriod ? "relative" : "absolute",
        selectedDate: value?.selectedDate,
        selectedTime: value?.selectedTime,
      };

      await nextTick();
      await nextTick();
      await nextTick();
      await nextTick();

      if (
        searchObj.loading == false &&
        store.state.zoConfig.query_on_stream_selection == false
      ) {
        searchObj.loading = true;
        searchObj.runQuery = true;
      }

      if (config.isCloud == "true" && value.userChangedValue) {
        segment.track("Button Click", {
          button: "Date Change",
          tab: value.tab,
          value: value,
          //user_org: this.store.state.selectedOrganization.identifier,
          //user_id: this.store.state.userInfo.email,
          stream_name: searchObj.data.stream.selectedStream.join(","),
          page: "Search Logs",
        });
      }

      if (
        value.valueType === "relative" &&
        store.state.zoConfig.query_on_stream_selection == false
      ) {
        emit("searchdata");
      }
    };

    const updateTimezone = () => {
      if (store.state.zoConfig.query_on_stream_selection == false) {
        emit("onChangeTimezone");
      }
    };

    const udpateQuery = () => {
      if (queryEditorRef.value?.setValue)
        queryEditorRef.value.setValue(searchObj.data.query);
    };

    const jsonToCsv = (jsonData) => {
      const replacer = (key, value) => (value === null ? "" : value);
      const header = Object.keys(jsonData[0]);
      let csv = header.join(",") + "\r\n";

      for (let i = 0; i < jsonData.length; i++) {
        const row = header
          .map((fieldName) => JSON.stringify(jsonData[i][fieldName], replacer))
          .join(",");
        csv += row + "\r\n";
      }

      return csv;
    };

    const downloadLogs = (data) => {
      const filename = "logs-data.csv";
      const dataobj = jsonToCsv(data);
      const file = new File([dataobj], filename, {
        type: "text/csv",
      });
      const url = URL.createObjectURL(file);
      const link = document.createElement("a");
      link.href = url;
      link.download = filename;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
    };

    onMounted(async () => {
      if (
        router.currentRoute.value.query.functionContent ||
        searchObj.data.tempFunctionContent
      ) {
        searchObj.meta.toggleFunction = true;
        const fnContent = router.currentRoute.value.query.functionContent
          ? b64DecodeUnicode(router.currentRoute.value.query.functionContent)
          : searchObj.data.tempFunctionContent;
        fnEditorRef.value.setValue(fnContent);
        fnEditorRef.value.resetEditorLayout();
        searchObj.config.fnSplitterModel = 60;
      }
    });

    onUnmounted(() => {
      window.removeEventListener("click", () => {
        fnEditorRef.value.resetEditorLayout();
      });
    });

    onActivated(() => {
      udpateQuery();

      if (
        router.currentRoute.value.query.functionContent ||
        searchObj.data.tempFunctionContent
      ) {
        searchObj.meta.toggleFunction = true;
        const fnContent = router.currentRoute.value.query.functionContent
          ? b64DecodeUnicode(router.currentRoute.value.query.functionContent)
          : searchObj.data.tempFunctionContent;
        fnEditorRef.value.setValue(fnContent);
        fnEditorRef.value.resetEditorLayout();
        searchObj.config.fnSplitterModel = 60;
        window.removeEventListener("click", () => {
          fnEditorRef.value.resetEditorLayout();
        });
      }
      fnEditorRef.value.resetEditorLayout();
    });

    onDeactivated(() => {
      window.removeEventListener("click", () => {
        fnEditorRef.value.resetEditorLayout();
      });
    });

    const saveFunction = () => {
      saveFunctionLoader.value = true;
      let callTransform: Promise<{ data: any }>;
      const content = searchObj.data.tempFunctionContent;
      let fnName = "";
      if (isSavedFunctionAction.value == "create") {
        fnName = savedFunctionName.value;
      } else {
        fnName = savedFunctionSelectedName.value.name;
      }

      if (content.trim() == "") {
        $q.notify({
          type: "warning",
          message:
            "The function field must contain a value and cannot be left empty.",
        });
        saveFunctionLoader.value = false;
        return;
      }

      const pattern = /^[a-zA-Z][a-zA-Z0-9_]*$/;
      if (!pattern.test(fnName)) {
        $q.notify({
          type: "negative",
          message: "Function name is not valid.",
        });
        saveFunctionLoader.value = false;
        return;
      }

      formData.value.params = "row";
      formData.value.function = content;
      formData.value.transType = 0;
      formData.value.name = fnName;
      searchObj.data.tempFunctionContent = content;

      // const result = functionOptions.value.find((obj) => obj.name === fnName);
      if (isSavedFunctionAction.value == "create") {
        callTransform = jsTransformService.create(
          store.state.selectedOrganization.identifier,
          formData.value
        );

        callTransform
          .then((res: { data: any }) => {
            $q.notify({
              type: "positive",
              message: res.data.message,
            });

            functionModel.value = {
              name: formData.value.name,
              function: formData.value.function,
            };
            functionOptions.value.push({
              name: formData.value.name,
              function: formData.value.function,
              transType: 0,
              params: "row",
            });
            store.dispatch("setSavedFunctionDialog", false);
            isSavedFunctionAction.value = "create";
            savedFunctionName.value = "";
            saveFunctionLoader.value = false;
            savedFunctionSelectedName.value = "";
          })
          .catch((err) => {
            saveFunctionLoader.value = false;
            $q.notify({
              type: "negative",
              message:
                JSON.stringify(err.response.data["message"]) ||
                "Function creation failed",
              timeout: 5000,
            });
          });
      } else {
        saveFunctionLoader.value = false;
        showConfirmDialog(() => {
          saveFunctionLoader.value = true;
          callTransform = jsTransformService.update(
            store.state.selectedOrganization.identifier,
            formData.value
          );

          callTransform
            .then((res: { data: any }) => {
              $q.notify({
                type: "positive",
                message: "Function updated successfully.",
              });

              const transformIndex = searchObj.data.transforms.findIndex(
                (obj) => obj.name === formData.value.name
              );
              if (transformIndex !== -1) {
                searchObj.data.transforms[transformIndex].name =
                  formData.value.name;
                searchObj.data.transforms[transformIndex].function =
                  formData.value.function;
              }

              functionOptions.value = searchObj.data.transforms;
              store.dispatch("setSavedFunctionDialog", false);
              isSavedFunctionAction.value = "create";
              savedFunctionName.value = "";
              saveFunctionLoader.value = false;
              savedFunctionSelectedName.value = "";
            })
            .catch((err) => {
              saveFunctionLoader.value = false;
              $q.notify({
                type: "negative",
                message:
                  JSON.stringify(err.response.data["message"]) ||
                  "Function updation failed",
                timeout: 5000,
              });
            });
        });
      }
    };

    const resetFunctionContent = () => {
      fnEditorRef.value.setValue("");
      store.dispatch("setSavedFunctionDialog", false);
      isSavedFunctionAction.value = "create";
      savedFunctionName.value = "";
      saveFunctionLoader.value = false;
      savedFunctionSelectedName.value = "";
    };

    const resetEditorLayout = () => {
      setTimeout(() => {
        queryEditorRef.value.resetEditorLayout();
        console.log("resetEditorLayout", fnEditorRef.value);
        fnEditorRef.value.resetEditorLayout();
      }, 100);
    };

    const populateFunctionImplementation = (fnValue, flag = false) => {
      if (flag) {
        $q.notify({
          type: "positive",
          message: `${fnValue.name} function applied successfully.`,
          timeout: 3000,
        });
      }
      searchObj.meta.toggleFunction = true;
      searchObj.config.fnSplitterModel = 60;
      fnEditorRef.value.setValue(fnValue.function);
      searchObj.data.tempFunctionName = fnValue.name;
      searchObj.data.tempFunctionContent = fnValue.function;
    };

    const fnSavedFunctionDialog = () => {
      const content = searchObj.data.tempFunctionContent;
      if (content == "") {
        $q.notify({
          type: "negative",
          message: "No function definition found.",
        });
        return;
      }
      store.dispatch("setSavedFunctionDialog", true);
      isSavedFunctionAction.value = "create";
      savedFunctionName.value = "";
      saveFunctionLoader.value = false;
      savedFunctionSelectedName.value = "";
    };

    const showConfirmDialog = (callback) => {
      confirmDialogVisible.value = true;
      confirmCallback = callback;
    };

    const showSavedViewConfirmDialog = (callback) => {
      confirmSavedViewDialogVisible.value = true;
      confirmCallback = callback;
    };

    const cancelConfirmDialog = () => {
      confirmSavedViewDialogVisible.value = false;
      confirmDialogVisible.value = false;
      confirmCallback = null;
    };

    const confirmDialogOK = () => {
      if (confirmCallback) {
        confirmCallback();
      }
      confirmDialogVisible.value = false;
      confirmCallback = null;
    };

    const filterFn = (val, update) => {
      update(() => {
        if (val === "") {
          functionOptions.value = searchObj.data.transforms;
        } else {
          const needle = val.toLowerCase();
          functionOptions.value = searchObj.data.transforms.filter(
            (v) => v.name?.toLowerCase().indexOf(needle) > -1
          );
        }
      });
    };

    const onRefreshIntervalUpdate = () => {
      emit("onChangeInterval");
    };

    const fnSavedView = () => {
      if (!searchObj.data.stream.selectedStream.value) {
        $q.notify({
          type: "negative",
          message: "No stream available to save view.",
        });
        return;
      }
      store.dispatch("setSavedViewDialog", true);
      isSavedViewAction.value = "create";
      savedViewName.value = "";
      saveViewLoader.value = false;
      savedViewSelectedName.value = "";
      savedViewDropdownModel.value = false;
    };

    const applySavedView = (item) => {
      searchObj.shouldIgnoreWatcher = true;
      searchObj.meta.sqlMode = false;
      savedviewsService
        .getViewDetail(
          store.state.selectedOrganization.identifier,
          item.view_id
        )
        .then(async (res) => {
          if (res.status == 200) {
            store.dispatch("setSavedViewFlag", true);
            const extractedObj = res.data.data;

            if (extractedObj.data?.timezone) {
              store.dispatch("setTimezone", extractedObj.data.timezone);
            }

            if (!extractedObj.data.stream.hasOwnProperty("streamType")) {
              extractedObj.data.stream.streamType = "logs";
            }

            delete searchObj.data.queryResults.aggs;

            if (
              searchObj.data.stream.streamType ==
              extractedObj.data.stream.streamType
            ) {
              // if (
              //   extractedObj.data.stream.selectedStream.value !=
              //   searchObj.data.stream.selectedStream.value
              // ) {
              //   extractedObj.data.stream.streamLists =
              //     searchObj.data.stream.streamLists;
              // }
              delete extractedObj.data.stream.streamLists;
              delete searchObj.data.stream.selectedStream;
              delete searchObj.meta.regions;
              if (extractedObj.meta.hasOwnProperty("regions")) {
                searchObj.meta["regions"] = extractedObj.meta.regions;
              } else {
                searchObj.meta["regions"] = [];
              }

              if (extractedObj.meta.hasOwnProperty("clusters")) {
                searchObj.meta["clusters"] = extractedObj.meta.clusters;
              } else {
                searchObj.meta["clusters"] = [];
              }
              extractedObj.data.transforms = searchObj.data.transforms;
              extractedObj.data.stream.functions =
                searchObj.data.stream.functions;
              extractedObj.data.histogram = {
                xData: [],
                yData: [],
                chartParams: {},
              };
              extractedObj.data.savedViews = searchObj.data.savedViews;
              extractedObj.data.queryResults = [];
              extractedObj.meta.scrollInfo = {};
              searchObj.value = mergeDeep(searchObj, extractedObj);
              searchObj.shouldIgnoreWatcher = true;
              await nextTick();
              if (extractedObj.data.tempFunctionContent != "") {
                populateFunctionImplementation(
                  {
                    name: "",
                    function: searchObj.data.tempFunctionContent,
                  },
                  false
                );
                searchObj.data.tempFunctionContent =
                  extractedObj.data.tempFunctionContent;
                searchObj.meta.functionEditorPlaceholderFlag = false;
              } else {
                populateFunctionImplementation(
                  {
                    name: "",
                    function: "",
                  },
                  false
                );
                searchObj.data.tempFunctionContent = "";
                searchObj.meta.functionEditorPlaceholderFlag = true;
              }
              dateTimeRef.value.setSavedDate(searchObj.data.datetime);
              if (searchObj.meta.refreshInterval != "0") {
                onRefreshIntervalUpdate();
              } else {
                clearInterval(store.state.refreshIntervalID);
              }
              await updatedLocalLogFilterField();
              await getStreams("logs", true);
            } else {
              // ----- Here we are explicitly handling stream change -----
              resetStreamData();
              searchObj.data.stream.streamType =
                extractedObj.data.stream.streamType;

              delete searchObj.meta.regions;
              if (extractedObj.meta.hasOwnProperty("regions")) {
                searchObj.meta["regions"] = extractedObj.meta.regions;
              } else {
                searchObj.meta["regions"] = [];
              }
              // Here copying selected stream object, as in loadStreamLists() we are setting selected stream object to empty object
              // After loading stream list, we are setting selected stream object to copied object
              const selectedStream = cloneDeep(
                extractedObj.data.stream.selectedStream
              );

              extractedObj.data.transforms = searchObj.data.transforms;
              extractedObj.data.histogram = {
                xData: [],
                yData: [],
                chartParams: {},
              };
              extractedObj.data.savedViews = searchObj.data.savedViews;
              extractedObj.data.queryResults = [];
              extractedObj.meta.scrollInfo = {};

              searchObj.value = mergeDeep(searchObj, extractedObj);
              searchObj.data.streamResults = {};

              const streamData = await getStreams(
                searchObj.data.stream.streamType,
                true
              );
              searchObj.data.streamResults = streamData;
              await loadStreamLists();
              searchObj.data.stream.selectedStream = selectedStream;
              // searchObj.value = mergeDeep(searchObj, extractedObj);

              await nextTick();
              if (extractedObj.data.tempFunctionContent != "") {
                populateFunctionImplementation(
                  {
                    name: "",
                    function: searchObj.data.tempFunctionContent,
                  },
                  false
                );
                searchObj.data.tempFunctionContent =
                  extractedObj.data.tempFunctionContent;
                searchObj.meta.functionEditorPlaceholderFlag = false;
              } else {
                populateFunctionImplementation(
                  {
                    name: "",
                    function: "",
                  },
                  false
                );
                searchObj.data.tempFunctionContent = "";
                searchObj.meta.functionEditorPlaceholderFlag = true;
              }
              dateTimeRef.value.setSavedDate(searchObj.data.datetime);
              if (searchObj.meta.refreshInterval != "0") {
                onRefreshIntervalUpdate();
              } else {
                clearInterval(store.state.refreshIntervalID);
              }
              await updatedLocalLogFilterField();
            }
            $q.notify({
              message: `${item.view_name} view applied successfully.`,
              color: "positive",
              position: "bottom",
              timeout: 1000,
            });
            setTimeout(async () => {
              try {
                searchObj.loading = true;
                await getQueryData();
                store.dispatch("setSavedViewFlag", false);
                updateUrlQueryParams();
                searchObj.shouldIgnoreWatcher = false;
              } catch (e) {
                searchObj.shouldIgnoreWatcher = false;
                console.log(e);
              }
            }, 1000);

            // } else {
            //   searchObj.value = mergeDeep(searchObj, extractedObj);
            //   await nextTick();
            //   updatedLocalLogFilterField();
            //   handleRunQuery();
            // }
          } else {
            searchObj.shouldIgnoreWatcher = false;
            store.dispatch("setSavedViewFlag", false);
            $q.notify({
              message: `Error while applying saved view. ${res.data.error_detail}`,
              color: "negative",
              position: "bottom",
              timeout: 1000,
            });
          }
        })
        .catch((err) => {
          searchObj.shouldIgnoreWatcher = false;
          store.dispatch("setSavedViewFlag", false);
          $q.notify({
            message: `Error while applying saved view.`,
            color: "negative",
            position: "bottom",
            timeout: 1000,
          });
          console.log(err);
        });
      // const extractedObj = JSON.parse(b64DecodeUnicode(item.data));
      // searchObj.value = mergeDeep(searchObj, extractedObj);
      // await nextTick();
      // updatedLocalLogFilterField();
      // handleRunQuery();
    };

    const handleSavedView = () => {
      if (isSavedViewAction.value == "create") {
        if (
          savedViewName.value == "" ||
          /^[A-Za-z0-9 \-\_]+$/.test(savedViewName.value) == false
        ) {
          $q.notify({
            message: `Please provide valid view name.`,
            color: "negative",
            position: "bottom",
            timeout: 1000,
          });
        } else {
          saveViewLoader.value = true;
          createSavedViews(savedViewName.value);
        }
      } else {
        if (savedViewSelectedName.value.view_id) {
          saveViewLoader.value = false;
          showSavedViewConfirmDialog(() => {
            saveViewLoader.value = true;
            updateSavedViews(
              savedViewSelectedName.value.view_id,
              savedViewSelectedName.value.view_name
            );
          });
        } else {
          $q.notify({
            message: `Please select saved view to update.`,
            color: "negative",
            position: "bottom",
            timeout: 1000,
          });
        }
      }
    };

    const deleteSavedViews = async () => {
      try {
        savedviewsService
          .delete(
            store.state.selectedOrganization.identifier,
            deleteViewID.value
          )
          .then((res) => {
            if (res.status == 200) {
              $q.notify({
                message: `View deleted successfully.`,
                color: "positive",
                position: "bottom",
                timeout: 1000,
              });
              getSavedViews();
            } else {
              $q.notify({
                message: `Error while deleting saved view. ${res.data.error_detail}`,
                color: "negative",
                position: "bottom",
                timeout: 1000,
              });
            }
          })
          .catch((err) => {
            $q.notify({
              message: `Error while deleting saved view.`,
              color: "negative",
              position: "bottom",
              timeout: 1000,
            });
            console.log(err);
          });
      } catch (e: any) {
        console.log("Error while getting saved views", e);
      }
    };

    const getSearchObj = () => {
      try {
        delete searchObj.meta.scrollInfo;
        delete searchObj?.value;
        let savedSearchObj = toRaw(searchObj);
        savedSearchObj = JSON.parse(JSON.stringify(savedSearchObj));

        delete savedSearchObj.data.queryResults;
        delete savedSearchObj.data.histogram;
        delete savedSearchObj.data.sortedQueryResults;
        delete savedSearchObj.data.stream.streamLists;
        delete savedSearchObj.data.stream.functions;
        delete savedSearchObj.data.streamResults;
        delete savedSearchObj.data.savedViews;
        delete savedSearchObj.data.transforms;

        savedSearchObj.data.timezone = store.state.timezone;
        delete savedSearchObj.value;

        return savedSearchObj;
        // return b64EncodeUnicode(JSON.stringify(savedSearchObj));
      } catch (e) {
        console.log("Error while encoding search obj", e);
      }
    };

    const createSavedViews = (viewName: string) => {
      try {
        if (viewName.trim() == "") {
          $q.notify({
            message: `Please provide valid view name.`,
            color: "negative",
            position: "bottom",
            timeout: 1000,
          });
          saveViewLoader.value = false;
          return;
        }

        const viewObj: any = {
          data: getSearchObj(),
          view_name: viewName,
        };

        savedviewsService
          .post(store.state.selectedOrganization.identifier, viewObj)
          .then((res) => {
            if (res.status == 200) {
              store.dispatch("setSavedViewDialog", false);
              if (searchObj.data.hasOwnProperty("savedViews") == false) {
                searchObj.data.savedViews = [];
              }
              searchObj.data.savedViews.push({
                org_id: res.data.org_id,
                payload: viewObj.data,
                view_id: res.data.view_id,
                view_name: viewName,
              });
              $q.notify({
                message: `View created successfully.`,
                color: "positive",
                position: "bottom",
                timeout: 1000,
              });
              getSavedViews();
              isSavedViewAction.value = "create";
              savedViewName.value = "";
              saveViewLoader.value = false;
            } else {
              saveViewLoader.value = false;
              $q.notify({
                message: `Error while creating saved view. ${res.data.error_detail}`,
                color: "negative",
                position: "bottom",
                timeout: 1000,
              });
            }
          })
          .catch((err) => {
            saveViewLoader.value = false;
            $q.notify({
              message: `Error while creating saved view.`,
              color: "negative",
              position: "bottom",
              timeout: 1000,
            });
            console.log(err);
          });
      } catch (e: any) {
        isSavedViewAction.value = "create";
        savedViewName.value = "";
        saveViewLoader.value = false;
        $q.notify({
          message: `Error while saving view: ${e}`,
          color: "negative",
          position: "bottom",
          timeout: 1000,
        });
        console.log("Error while saving view", e);
      }
    };

    const updateSavedViews = (viewID: string, viewName: string) => {
      try {
        const viewObj: any = {
          data: getSearchObj(),
          view_name: viewName,
        };

        savedviewsService
          .put(store.state.selectedOrganization.identifier, viewID, viewObj)
          .then((res) => {
            if (res.status == 200) {
              store.dispatch("setSavedViewDialog", false);
              //update the payload and view_name in savedViews object based on id
              searchObj.data.savedViews.forEach(
                (item: { view_id: string }, index: string | number) => {
                  if (item.view_id == viewID) {
                    searchObj.data.savedViews[index].payload = viewObj.data;
                    searchObj.data.savedViews[index].view_name = viewName;
                  }
                }
              );

              $q.notify({
                message: `View updated successfully.`,
                color: "positive",
                position: "bottom",
                timeout: 1000,
              });
              isSavedViewAction.value = "create";
              savedViewSelectedName.value = "";
              saveViewLoader.value = false;
              confirmSavedViewDialogVisible.value = false;
            } else {
              saveViewLoader.value = false;
              $q.notify({
                message: `Error while updating saved view. ${res.data.error_detail}`,
                color: "negative",
                position: "bottom",
                timeout: 1000,
              });
            }
          })
          .catch((err) => {
            saveViewLoader.value = false;
            $q.notify({
              message: `Error while updating saved view.`,
              color: "negative",
              position: "bottom",
              timeout: 1000,
            });
            console.log(err);
          });
      } catch (e: any) {
        isSavedViewAction.value = "create";
        savedViewSelectedName.value = "";
        saveViewLoader.value = false;
        $q.notify({
          message: `Error while saving view: ${e}`,
          color: "negative",
          position: "bottom",
          timeout: 1000,
        });
        console.log("Error while saving view", e);
      }
    };

    const shareLink = () => {
      const queryObj = generateURLQuery(true);
      const queryString = Object.entries(queryObj)
        .map(
          ([key, value]) =>
            `${encodeURIComponent(key)}=${encodeURIComponent(value)}`
        )
        .join("&");

      let shareURL = window.location.origin + window.location.pathname;

      if (queryString != "") {
        shareURL += "?" + queryString;
      }

      copyToClipboard(shareURL)
        .then(() => {
          $q.notify({
            type: "positive",
            message: "Link Copied Successfully!",
            timeout: 5000,
          });
        })
        .catch(() => {
          $q.notify({
            type: "negative",
            message: "Error while copy link.",
            timeout: 5000,
          });
        });
    };

    const resetFilters = () => {
      if (searchObj.meta.sqlMode == true) {
        searchObj.data.query = `SELECT [FIELD_LIST] FROM "${searchObj.data.stream.selectedStream.value}" ORDER BY ${store.state.zoConfig.timestamp_column} DESC`;
        if (
          searchObj.data.stream.interestingFieldList.length > 0 &&
          searchObj.meta.quickMode
        ) {
          searchObj.data.query = searchObj.data.query.replace(
            "[FIELD_LIST]",
            searchObj.data.stream.interestingFieldList.join(",")
          );
        } else {
          searchObj.data.query = searchObj.data.query.replace(
            "[FIELD_LIST]",
            "*"
          );
        }
      } else {
        searchObj.data.query = "";
      }
      searchObj.data.editorValue = "";
      queryEditorRef.value.setValue(searchObj.data.query);
      if (store.state.zoConfig.query_on_stream_selection == false) {
        handleRunQuery();
      }
    };

    const customDownloadDialog = ref(false);
    const downloadCustomInitialNumber = ref(0);
    const downloadCustomRange = ref(100);
    const downloadCustomRangeOptions = ref([100, 500, 1000, 5000, 10000]);

    const loadSavedView = () => {
      if (searchObj.data.savedViews.length == 0) {
        getSavedViews();
      }
    };

    const handleFavoriteSavedView = (row: any, flag: boolean) => {
      let localSavedView: any = {};
      let savedViews = useLocalSavedView();

      if (savedViews.value != null) {
        localSavedView = savedViews.value;
      }

      Object.keys(localSavedView).forEach((item, key) => {
        if (item == row.view_id) {
          if (flag) {
            delete localSavedView[item];
            useLocalSavedView(localSavedView);
            const index = favoriteViews.value.indexOf(row.view_id);
            if (index > -1) {
              favoriteViews.value.splice(index, 1);
            }

            let favoriteViewsList = localSavedViews.value;
            if (favoriteViewsList.length > 0) {
              favoriteViewsList = favoriteViewsList.filter(
                (item) => item.view_id != row.view_id
              );
              // for (const [key, item] of favoriteViewsList.entries()) {
              //   console.log(item, key);
              //   if (item.view_id == row.view_id) {
              //     delete favoriteViewsList[key];
              //   }
              // }
              console.log(favoriteViewsList);
              localSavedViews.value = favoriteViewsList;
            }
          }
        }
      });

      if (!flag) {
        if (favoriteViews.value.length >= 10) {
          $q.notify({
            message: "You can only save 10 views.",
            color: "info",
            position: "bottom",
            timeout: 2000,
          });
          return;
        }
        localSavedView[row.view_id] = JSON.parse(JSON.stringify(row));
        favoriteViews.value.push(row.view_id);
        localSavedViews.value.push(row);

        // moveItemsToTop(localSavedView, favoriteViews.value);

        useLocalSavedView(localSavedView);
        $q.notify({
          message: "View added to favorites.",
          color: "positive",
          position: "bottom",
          timeout: 2000,
        });
      } else {
        // alert(favoriteViews.value.length)
        // moveItemsToTop(localSavedView, favoriteViews.value);
        $q.notify({
          message: "View removed from favorites.",
          color: "positive",
          position: "bottom",
          timeout: 2000,
        });
      }
    };

    const filterSavedViewFn = (rows: any, terms: any) => {
      var filtered = [];
      if (terms != "") {
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (rows[i]["view_name"].toLowerCase().includes(terms)) {
            filtered.push(rows[i]);
          }
        }
      }
      return filtered;
    };

    const handleRegionsSelection = (item, isSelected) => {
      if (isSelected) {
        const index = searchObj.meta.regions.indexOf(item);
        if (index > -1) {
          searchObj.meta.regions.splice(index, 1);
        }
      } else {
        searchObj.meta.regions.push(item);
      }
    };

    const handleQuickMode = () => {
      emit("handleQuickModeChange");
    };

    const regionFilterMethod = (node, filter) => {
      const filt = filter.toLowerCase();
      return node.label && node.label.toLowerCase().indexOf(filt) > -1;
    };

    const resetRegionFilter = () => {
      regionFilter.value = "";
    };

    return {
      t,
      store,
      router,
      fnEditorRef,
      searchObj,
      queryEditorRef,
      confirmDialogVisible,
      confirmCallback,
      refreshTimes: searchObj.config.refreshTimes,
      refreshTimeChange,
      updateQueryValue,
      updateDateTime,
      showConfirmDialog,
      showSavedViewConfirmDialog,
      cancelConfirmDialog,
      confirmDialogOK,
      udpateQuery,
      downloadLogs,
      saveFunction,
      resetFunctionContent,
      resetEditorLayout,
      populateFunctionImplementation,
      functionModel,
      functionOptions,
      filterFn,
      refreshData,
      handleRunQuery,
      autoCompleteKeywords,
      autoCompleteSuggestions,
      onRefreshIntervalUpdate,
      updateTimezone,
      dateTimeRef,
      fnSavedView,
      applySavedView,
      isSavedViewAction,
      savedViewName,
      savedViewSelectedName,
      handleSavedView,
      deleteSavedViews,
      deleteViewID,
      confirmDelete,
      saveViewLoader,
      savedViewDropdownModel,
      fnSavedFunctionDialog,
      isSavedFunctionAction,
      savedFunctionName,
      savedFunctionSelectedName,
      saveFunctionLoader,
      shareLink,
      getImageURL,
      resetFilters,
      customDownloadDialog,
      downloadCustomInitialNumber,
      downloadCustomRange,
      downloadCustomRangeOptions,
      buildSearch,
      confirmSavedViewDialogVisible,
      rowsPerPage,
      handleFavoriteSavedView,
      favoriteViews,
      localSavedViews,
      loadSavedView,
      filterSavedViewFn,
      config,
      handleRegionsSelection,
      handleQuickMode,
      regionFilterMethod,
      regionFilterRef,
      regionFilter,
      resetRegionFilter,
    };
  },
  computed: {
    addSearchTerm() {
      return this.searchObj.data.stream.addToFilter;
    },
    toggleFunction() {
      return this.searchObj.meta.toggleFunction;
    },
    confirmMessage() {
      return "Are you sure you want to update the function?";
    },
    confirmMessageSavedView() {
      return "Are you sure you want to update the saved view?";
    },
    resetFunction() {
      return this.searchObj.data.tempFunctionName;
    },
    resetFunctionDefination() {
      return this.searchObj.data.tempFunctionContent;
    },
  },
  watch: {
    addSearchTerm() {
      if (this.searchObj.data.stream.addToFilter != "") {
        let currentQuery = this.searchObj.data.editorValue.split("|");
        let filter = this.searchObj.data.stream.addToFilter;

        const isFilterValueNull = filter.split(/=|!=/)[1] === "'null'";

        if (isFilterValueNull) {
          filter = filter
            .replace(/=|!=/, (match) => {
              return match === "=" ? " is " : " is not ";
            })
            .replace(/'null'/, "null");
        }

        if (currentQuery.length > 1) {
          if (currentQuery[1].trim() != "") {
            currentQuery[1] += " and " + filter;
          } else {
            currentQuery[1] = filter;
          }
          this.searchObj.data.query = currentQuery.join("| ");
        } else {
          // if (currentQuery != "") {
          //   if (
          //     this.searchObj.meta.sqlMode == true &&
          //     currentQuery.toString().toLowerCase().indexOf("where") == -1
          //   ) {
          //     currentQuery += " where " + filter;
          //   } else {
          //     currentQuery += " and " + filter;
          //   }
          // } else {
          //   if (this.searchObj.meta.sqlMode == true) {
          //     currentQuery = "where " + filter;
          //   } else {
          //     currentQuery = filter;
          //   }
          // }

          if (this.searchObj.meta.sqlMode == true) {
            // if query contains order by clause or limit clause then add where clause before that
            // if query contains where clause then add filter after that with and operator and keep order by or limit after that
            // if query does not contain where clause then add where clause before filter
            let query = currentQuery[0].toLowerCase();
            if (query.includes("where")) {
              if (query.includes("order by")) {
                const [beforeOrderBy, afterOrderBy] = query.split("order by");
                query =
                  beforeOrderBy.trim() +
                  " AND " +
                  filter +
                  " order by" +
                  afterOrderBy;
              } else if (query.includes("limit")) {
                const [beforeLimit, afterLimit] = query.split("limit");
                query =
                  beforeLimit.trim() + " AND " + filter + " limit" + afterLimit;
              } else {
                query = query + " AND " + filter;
              }
            } else {
              if (query.includes("order by")) {
                const [beforeOrderBy, afterOrderBy] = query.split("order by");
                query =
                  beforeOrderBy.trim() +
                  " where " +
                  filter +
                  " order by" +
                  afterOrderBy;
              } else if (query.includes("limit")) {
                const [beforeLimit, afterLimit] = query.split("limit");
                query =
                  beforeLimit.trim() +
                  " where " +
                  filter +
                  " limit" +
                  afterLimit;
              } else {
                query = query + " where " + filter;
              }
            }
            currentQuery[0] = query;
          } else {
            currentQuery[0].length == 0
              ? (currentQuery[0] = filter)
              : (currentQuery[0] += " and " + filter);
          }

          this.searchObj.data.query = currentQuery[0];
        }
        this.searchObj.data.stream.addToFilter = "";
        if (this.queryEditorRef?.setValue)
          this.queryEditorRef.setValue(this.searchObj.data.query);
      }
    },
    toggleFunction(newVal) {
      if (newVal == false) {
        this.searchObj.config.fnSplitterModel = 99.5;
        this.resetFunctionContent();
      } else {
        this.searchObj.config.fnSplitterModel = 60;
      }
      this.resetEditorLayout();
    },
    resetFunction(newVal) {
      if (newVal == "" && store.state.savedViewFlag == false) {
        this.resetFunctionContent();
      }
    },
    resetFunctionDefination(newVal) {
      if (newVal == "") this.resetFunctionContent();
    },
  },
});
</script>

<style lang="scss">
#logsQueryEditor,
#fnEditor {
  height: 100% !important;
}
#fnEditor {
  width: 100%;
  border-radius: 5px;
  border: 0px solid #dbdbdb;
  overflow: hidden;
}

.q-field--standard .q-field__control:before,
.q-field--standard .q-field__control:focus:before,
.q-field--standard .q-field__control:hover:before {
  border: 0px !important;
  border-color: none;
  transition: none;
}

.logs-search-bar-component > .row:nth-child(2) {
  height: 100%; /* or any other height you want to set */
}

.empty-query .monaco-editor-background {
  background-image: url("../../assets/images/common/query-editor.png");
  background-repeat: no-repeat;
  background-size: 115px;
}

.empty-function .monaco-editor-background {
  background-image: url("../../assets/images/common/vrl-function.png");
  background-repeat: no-repeat;
  background-size: 170px;
}

.logs-search-bar-component {
  padding-bottom: 1px;
  height: 100%;
  overflow: visible;

  .function-dropdown {
    width: 205px;
    padding-bottom: 0px;
    border: 1px solid #dbdbdb;
    border-radius: 5px;
    cursor: pointer;

    .q-field__input {
      cursor: pointer;
      font-weight: 600;
      font-size: 12px;
    }
    .q-field__native,
    .q-field__control {
      min-height: 29px;
      height: 29px;
      padding: 0px 0px 0px 4px;
    }

    .q-field__marginal {
      height: 30px;
    }
  }

  .q-toggle__inner {
    font-size: 30px;
  }

  .q-toggle__label {
    font-size: 12px;
  }

  .casesensitive-btn {
    padding: 8px;
    margin-left: -6px;
    background-color: #d5d5d5;
    border-radius: 0px 3px 3px 0px;
  }

  .search-field .q-field {
    &__control {
      border-radius: 3px 0px 0px 3px !important;
    }

    &__native {
      font-weight: 600;
    }
  }

  .search-time {
    // width: 120px;
    .q-btn-group {
      border-radius: 3px;

      .q-btn {
        min-height: auto;
      }
    }
  }

  .search-dropdown {
    padding: 0px;

    .block {
      color: $dark-page;
      font-weight: 600;
      font-size: 12px;
    }

    .q-btn-dropdown__arrow-container {
      color: $light-text2;
    }
  }

  .refresh-rate-dropdown-container {
    width: 220px;

    * .q-btn {
      font-size: 12px !important;
      padding-left: 8px;
      padding-right: 8px;
    }
  }

  .flex-start {
    justify-content: flex-start;
    align-items: flex-start;
    display: flex;
  }

  .resultsOverChart {
    margin-bottom: 0.75rem;
    font-size: 0.875rem;
    color: $dark-page;
    font-weight: 700;
  }

  .ddlWrapper {
    position: relative;
    z-index: 10;

    .listWrapper {
      box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
      transition: height 0.25s ease;
      height: calc(100vh - 146px);
      background-color: white;
      position: absolute;
      top: 2.75rem;
      width: 100%;
      left: 0;

      &:empty {
        height: 0;
      }

      &,
      .q-list {
        border-radius: 3px;
      }
    }
  }

  .fields_autocomplete {
    max-height: 250px;
  }

  .search-button {
    min-width: 70px;
    line-height: 29px;
    font-weight: bold;
    text-transform: initial;
    font-size: 11px;
    color: white;

    .q-btn__content {
      background: $secondary;
      border-radius: 3px 3px 3px 3px;
      padding: 0px 5px;

      .q-icon {
        font-size: 15px;
        color: #ffffff;
      }
    }
  }

  .download-logs-btn {
    height: 30px;
  }
}

.query-editor-container {
  height: calc(100% - 30px) !important;
}

.logs-auto-refresh-interval {
  .q-btn {
    min-height: 30px;
    max-height: 30px;
    padding: 0 4px;
  }
}

.saved-views-dropdown {
  border-radius: 4px;
  button {
    padding: 4px 5px;
  }
}

.savedview-dropdown {
  width: 215px;
  display: inline-block;
  border: 1px solid #dbdbdb;

  .q-field__input {
    cursor: pointer;
    font-weight: 600;
    font-size: 12px;
  }
  .q-field__native,
  .q-field__control {
    min-height: 29px !important;
    height: 29px;
    padding: 0px 0px 0px 4px;
  }

  .q-field__marginal {
    height: 30px;
  }
}

.saved-view-item {
  padding: 4px 5px !important;
}

.body--dark {
  .btn-function {
    filter: brightness(100);
  }
}

.q-pagination__middle > .q-btn {
  min-width: 30px !important;
  max-width: 30px !important;
}
</style>
<style lang="scss">
.saved-view-table {
  td {
    padding: 0;
    height: 25px !important;
    min-height: 25px !important;
  }

  .q-table__control {
    margin: 0px !important;
    width: 100% !important;
    text-align: right;
  }

  .q-table__bottom {
    padding: 0px !important;
    min-height: 35px;

    .q-table__control {
      padding: 0px 10px !important;
    }
  }

  .q-table__top {
    padding: 0px !important;
    margin: 0px !important;
    left: 0px;
    width: 100%;

    .q-table__separator {
      display: none;
    }

    .q-table__control {
      padding: 0px !important;
    }
  }

  .q-field--filled .q-field__control {
    padding: 0px 5px !important;
  }

  .saved-view-item {
    padding: 4px 5px 4px 10px !important;
  }

  .q-item__section--main ~ .q-item__section--side {
    padding-left: 5px !important;
  }
}
.logs-search-bar-component {
  .q-item {
    padding: 0px !important;
  }

  .q-focus-helper:hover {
    background: transparent !important;
  }
}

.favorite-label {
  line-height: 24px !important;
  font-weight: bold !important;
}

.region-dropdown-btn {
  text-transform: capitalize;
  font-weight: 600;
  font-size: 12px;
  padding-left: 8px;
  height: 30px;
  padding-top: 3px;

  .q-btn-dropdown__arrow {
    margin-left: 0px !important;
  }
}

.download-logs-btn {
  .q-btn-dropdown__arrow {
    margin-left: 0px !important;
  }
}

.region-dropdown-list {
  min-width: 150px;

  .q-item__section {
    display: inline-block;
  }

  .q-item__label {
    margin-left: 20px;
    text-transform: capitalize;
    margin-top: 2px;
  }
}
</style>
