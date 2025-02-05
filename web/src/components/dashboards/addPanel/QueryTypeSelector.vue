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
  <div class="q-py-sm">
    <div class="row button-group">
      <div v-if="dashboardPanelData.data.type != 'custom_chart'">
        <button
          data-test="dashboard-auto"
          :class="selectedButtonType === 'auto' ? 'selected' : ''"
          class="button button-left"
          @click="onUpdateButton('auto', $event)"
          :style="{
            backgroundColor:
            store.state.theme == 'dark' ? '#bfbebef5' : '#f0eaea',
          }"
        >
          {{ t("panel.auto") }}
        </button>
      </div>
      <div>
        <button
          data-test="dashboard-promQL"
          class="button"
          :style="{
            backgroundColor:
            store.state.theme == 'dark' ? '#bfbebef5' : '#f0eaea',
          }"
          :class="selectedButtonType === 'promql' ? 'selected' : ''"
          v-show="
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.stream_type == 'metrics'
          "
          @click="onUpdateButton('promql', $event)"
        >
          {{ t("panel.promQL") }}
        </button>
      </div>
      <div>
        <button
          data-test="dashboard-customSql"
          :class="selectedButtonType === 'custom-sql' ? 'selected' : ''"
          :style="{
            backgroundColor:
            store.state.theme == 'dark' ? '#bfbebef5' : '#f0eaea',
          }"
          class="button button-right"
          @click="onUpdateButton('custom-sql', $event)"
        >
          {{ t("panel.customSql") }}
        </button>
      </div>
    </div>
    <ConfirmDialog
      title="Change Query Mode"
      message="Are you sure you want to change the query mode? The data saved for X-Axis, Y-Axis and Filters will be wiped off."
      @update:ok="changeToggle()"
      @update:cancel="confirmQueryModeChangeDialog = false"
      v-model="confirmQueryModeChangeDialog"
    />
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  watch,
  onActivated,
  onMounted,
  nextTick,
  inject,
} from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import ConfirmDialog from "../../ConfirmDialog.vue";
import { useStore } from "vuex";

export default defineComponent({
  name: "QueryTypeSelector",
  component: { ConfirmDialog },
  props: [],
  emits: [],
  setup() {
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const store = useStore();
    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "dashboard"
    );
    const {
      dashboardPanelData,
      removeXYFilters,
      updateXYFieldsForCustomQueryMode,
    } = useDashboardPanelData(dashboardPanelDataPageKey);
    const confirmQueryModeChangeDialog = ref(false);

    // this is the value of the current button
    const selectedButtonType = ref("auto");

    // this holds the temporary value of the button while user confirms the choice on popup
    const popupSelectedButtonType = ref("auto");

    const ignoreSelectedButtonTypeUpdate = ref(false);

    const initializeSelectedButtonType = async () => {
      // set a variable to ignore updates for selectedButtonType

      ignoreSelectedButtonTypeUpdate.value = true;
      if (dashboardPanelData.data.type == "custom_chart") {
        selectedButtonType.value = "custom-sql";
        ignoreSelectedButtonTypeUpdate.value = false;
        return;
      } else if (
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery == false &&
        dashboardPanelData.data.queryType == "sql"
      ) {
        selectedButtonType.value = "auto";
      } else if (
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery == true &&
        dashboardPanelData.data.queryType == "sql"
      ) {
        selectedButtonType.value = "custom-sql";
      } else if (
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery == true &&
        dashboardPanelData.data.queryType == "promql"
      ) {
        selectedButtonType.value = "promql";
      } else {
        selectedButtonType.value = "auto";
        // if the query type is not present, set the value to "sql"
        if (
          !dashboardPanelData.data.queryType ||
          dashboardPanelData.data.queryType == ""
        ) {
          dashboardPanelData.data.queryType = "sql";
        }
      }
      await nextTick();

      // set a variable to allow watcher updates for selectedButtonType
      ignoreSelectedButtonTypeUpdate.value = false;
    };

    onMounted(async () => {
      initializeSelectedButtonType();
    });

    // onActivated(async () => {
    //     initializeSelectedButtonType()
    // })

    // when the data is loaded, initialize the selectedButtonType
    watch(
      () => [
        dashboardPanelData.data.queryType,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery,
      ],
      () => {
        initializeSelectedButtonType();
      }
    );

    const onUpdateButton = (selectedQueryType: any, event?: any) => {
      event.stopPropagation();
      if (selectedQueryType != selectedButtonType.value) {
        // some exceptions
        // If user is switching from auto to custom, promql to auto, or propmql to custom-sql no need for the popup,
        // else show the popup
        if (
          (selectedButtonType.value == "auto" &&
            selectedQueryType == "custom-sql") ||
          (selectedButtonType.value == "promql" &&
            selectedQueryType == "auto") ||
          (selectedButtonType.value == "promql" &&
            selectedQueryType == "custom-sql")
        ) {
          // act like you confirmed without opening the popup
          popupSelectedButtonType.value = selectedQueryType;
          changeToggle();
          dashboardPanelData.layout.showQueryBar = true;
        } else {
          popupSelectedButtonType.value = selectedQueryType;
          dashboardPanelData.data.queries[0].query != ""
            ? (confirmQueryModeChangeDialog.value = true)
            : changeToggle();
        }
      }
    };
    const changeToggle = async () => {
      selectedButtonType.value = popupSelectedButtonType.value;
      await nextTick(); // let the watchers execute first
      removeXYFilters();
      updateXYFieldsForCustomQueryMode();

      // empty the errors
      dashboardPanelData.meta.errors.queryErrors = [];

      if (selectedButtonType.value == "promql") {
        dashboardPanelData.layout.currentQueryIndex = 0;
        dashboardPanelData.data.queries = dashboardPanelData.data.queries.slice(
          0,
          1
        );
      }
      dashboardPanelData.layout.showQueryBar = true;
    };

    watch(
      () =>
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream_type,
      () => {
        if (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream_type != "metrics" &&
          selectedButtonType.value == "promql"
        ) {
          selectedButtonType.value = "auto";
        }
      }
    );

    watch(selectedButtonType, () => {
      window.dispatchEvent(new Event("resize"));
      if (!ignoreSelectedButtonTypeUpdate.value) {
        if (selectedButtonType.value == "auto") {
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery = false;
          dashboardPanelData.data.queryType = "sql";
        } else if (selectedButtonType.value == "custom-sql") {
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery = true;
          dashboardPanelData.data.queryType = "sql";
        } else if (selectedButtonType.value == "promql") {
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery = true;
          dashboardPanelData.data.queryType = "promql";

          // set some defaults for the promql query
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].query = "";
          dashboardPanelData.data.type = "line";
        } else {
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery = false;
          dashboardPanelData.data.queryType = "sql";
        }
      }
    });

    return {
      t,
      router,
      dashboardPanelData,
      onUpdateButton,
      changeToggle,
      confirmQueryModeChangeDialog,
      selectedButtonType,
      store,
    };
  },
  components: { ConfirmDialog },
});
</script>

<style lang="scss" scoped>
.selected {
  background-color: var(--q-primary) !important;
  font-weight: bold;
  color: white;
}

.button-group {
  border: 1px solid gray !important;
  border-radius: 9px;
}

.button {
  display: block;
  cursor: pointer;
  // background-color: #f0eaea;
  border: none;
  font-size: 14px;
  padding: 3px 10px;
}

.button-left {
  border-top-left-radius: 8px;
  border-bottom-left-radius: 8px;
}

.button-right {
  border-top-right-radius: 8px;
  border-bottom-right-radius: 8px;
}
</style>
