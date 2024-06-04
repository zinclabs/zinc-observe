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

<!-- eslint-disable vue/v-on-event-hyphenation -->
<!-- eslint-disable vue/attribute-hyphenation -->
<template>
  <div>
    <!-- flag to check if dashboardVariablesAndPanelsDataLoaded which is used while print mode-->
    <span
      v-if="isDashboardVariablesAndPanelsDataLoadedDebouncedValue"
      id="dashboardVariablesAndPanelsDataLoaded"
      style="display: none"
    >
    </span>
    <VariablesValueSelector
      :variablesConfig="dashboardData?.variables"
      :showDynamicFilters="dashboardData.variables?.showDynamicFilters"
      :selectedTimeDate="currentTimeObj"
      :initialVariableValues="initialVariableValues"
      @variablesData="variablesDataUpdated"
      ref="variablesValueSelectorRef"
    />
    <TabList
      v-if="showTabs && selectedTabId !== null"
      class="q-mt-sm"
      :dashboardData="dashboardData"
      :viewOnly="viewOnly"
      @refresh="refreshDashboard"
    />
    <slot name="before_panels" />
    <div class="displayDiv">
      <grid-layout
        ref="gridLayoutRef"
        v-if="panels.length > 0"
        :layout.sync="getDashboardLayout(panels)"
        :col-num="48"
        :row-height="30"
        :is-draggable="!viewOnly"
        :is-resizable="!viewOnly"
        :vertical-compact="true"
        :autoSize="true"
        :restore-on-drag="true"
        :use-css-transforms="false"
        :margin="[4, 4]"
      >
        <grid-item
          class="gridBackground"
          :class="store.state.theme == 'dark' ? 'dark' : ''"
          v-for="item in panels"
          :key="item.id"
          :x="getPanelLayout(item, 'x')"
          :y="getPanelLayout(item, 'y')"
          :w="getPanelLayout(item, 'w')"
          :h="getPanelLayout(item, 'h')"
          :i="getPanelLayout(item, 'i')"
          :minH="getMinimumHeight(item.type)"
          :minW="getMinimumWidth(item.type)"
          @resized="resizedEvent"
          @moved="movedEvent"
          drag-allow-from=".drag-allow"
        >
          <div style="height: 100%">
            <PanelContainer
              @onDeletePanel="onDeletePanel"
              @onViewPanel="onViewPanel"
              :viewOnly="viewOnly"
              :data="item"
              :dashboardId="dashboardData.id"
              :selectedTimeDate="currentTimeObj"
              :variablesData="variablesData"
              :width="getPanelLayout(item, 'w')"
              :height="getPanelLayout(item, 'h')"
              :forceLoad="forceLoad"
              @updated:data-zoom="$emit('updated:data-zoom', $event)"
              @onMovePanel="onMovePanel"
              @refresh="refreshDashboard"
              @update:initial-variable-values="updateInitialVariableValues"
            >
            </PanelContainer>
          </div>
        </grid-item>
      </grid-layout>
    </div>

    <!-- view panel dialog -->
    <q-dialog
      v-model="showViewPanel"
      :no-route-dismiss="true"
      full-height
      full-width
    >
      <q-card style="overflow: hidden">
        <ViewPanel
          :panelId="viewPanelId"
          :selectedDateForViewPanel="selectedDateForViewPanel"
          :initialVariableValues="variablesData"
          @close-panel="() => (showViewPanel = false)"
          :class="store.state.theme == 'dark' ? 'dark-mode' : 'bg-white'"
          @update:initial-variable-values="updateInitialVariableValues"
        />
      </q-card>
    </q-dialog>
    <div v-if="!panels.length">
      <!-- if data not available show nodata component -->
      <NoPanel @update:Panel="addPanelData" :view-only="viewOnly" />
    </div>
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import {
  computed,
  defineAsyncComponent,
  defineComponent,
  onActivated,
  provide,
  ref,
  watch,
} from "vue";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import { useI18n } from "vue-i18n";
import VueGridLayout from "vue3-grid-layout";
import { useRouter } from "vue-router";
import { reactive } from "vue";
import PanelContainer from "../../components/dashboards/PanelContainer.vue";
import { useRoute } from "vue-router";
import { updateDashboard } from "../../utils/commons";
import { useCustomDebouncer } from "../../utils/dashboard/useCustomDebouncer";
import NoPanel from "../../components/shared/grid/NoPanel.vue";
import VariablesValueSelector from "../../components/dashboards/VariablesValueSelector.vue";
import TabList from "@/components/dashboards/tabs/TabList.vue";
import { inject } from "vue";

const ViewPanel = defineAsyncComponent(() => {
  return import("@/components/dashboards/viewPanel/ViewPanel.vue");
});

export default defineComponent({
  name: "RenderDashboardCharts",
  emits: [
    "onDeletePanel",
    "onViewPanel",
    "variablesData",
    "updated:data-zoom",
    "refresh",
    "onMovePanel",
  ],
  props: {
    viewOnly: {},
    dashboardData: {},
    currentTimeObj: {},
    initialVariableValues: { value: {} },
    selectedDateForViewPanel: {},
    showTabs: {
      type: Boolean,
      default: false,
    },
    forceLoad: {
      type: Boolean,
      default: false,
      required: false,
    },
  },

  components: {
    GridLayout: VueGridLayout.GridLayout,
    GridItem: VueGridLayout.GridItem,
    PanelContainer,
    NoPanel,
    VariablesValueSelector,
    ViewPanel,
    TabList,
  },
  setup(props: any, { emit }) {
    const { t } = useI18n();
    const route = useRoute();
    const router = useRouter();
    const store = useStore();
    const $q = useQuasar();
    const gridLayoutRef = ref(null);
    const variablesValueSelectorRef = ref(null);

    const showViewPanel = ref(false);
    // holds the view panel id
    const viewPanelId = ref("");

    // inject selected tab, default will be default tab
    const selectedTabId = inject("selectedTabId", ref("default"));

    const panels: any = computed(() => {
      return selectedTabId.value !== null
        ? props.dashboardData?.tabs?.find(
            (it: any) => it.tabId === selectedTabId.value
          )?.panels ?? []
        : [];
    });

    const refreshDashboard = () => {
      emit("refresh");
    };

    const onMovePanel = (panelId: any, newTabId: any) => {
      emit("onMovePanel", panelId, newTabId);
    };

    // variables data
    const variablesData = reactive({});

    // ======= [START] dashboard PrintMode =======

    //reactive object for loading state of variablesData and panels
    const variablesAndPanelsDataLoadingState = reactive({
      variablesData: {},
      panels: {},
    });

    // provide variablesAndPanelsDataLoadingState to share data between components
    provide(
      "variablesAndPanelsDataLoadingState",
      variablesAndPanelsDataLoadingState
    );

    //computed property based on panels and variables loading state
    const isDashboardVariablesAndPanelsDataLoaded = computed(() => {
      // Get values of variablesData and panels
      const variablesDataValues = Object.values(
        variablesAndPanelsDataLoadingState.variablesData
      );
      const panelsValues = Object.values(
        variablesAndPanelsDataLoadingState.panels
      );

      // Check if every value in both variablesData and panels is false
      const isAllVariablesAndPanelsDataLoaded =
        variablesDataValues.every((value) => value === false) &&
        panelsValues.every((value) => value === false);
      return isAllVariablesAndPanelsDataLoaded;
    });

    // Create debouncer for isDashboardVariablesAndPanelsDataLoaded
    let {
      valueRef: isDashboardVariablesAndPanelsDataLoadedDebouncedValue,
      setImmediateValue,
      setDebounceValue,
    } = useCustomDebouncer(false, 3000);

    onActivated(() => {
      // set the initial value as false on component activated
      // also, this function will clear the settimeout if previously set
      setImmediateValue(false);
      window.dispatchEvent(new Event("resize"));
    });

    // Watch for changes in the computed property and update the debouncer accordingly
    watch(isDashboardVariablesAndPanelsDataLoaded, (newValue) => {
      // if value is false, then immediately set the value
      if (isDashboardVariablesAndPanelsDataLoaded.value === false) {
        setImmediateValue(newValue);
      } else if (store.state.printMode) {
        // if value is true, then debounce the value
        setDebounceValue(newValue);
      }
    });

    const variablesDataUpdated = (data: any) => {
      try {
        // update the variables data
        Object.assign(variablesData, data);

        // emit the variables data
        emit("variablesData", variablesData);

        // update the loading state
        if (variablesAndPanelsDataLoadingState) {
          variablesAndPanelsDataLoadingState.variablesData =
            variablesData?.values?.reduce(
              (obj: any, item: any) => ({
                ...obj,
                [item.name]: item.isLoading,
              }),
              {}
            );
        }
      } catch (error) {
        console.log(error);
      }
    };

    // ======= [END] dashboard PrintMode =======

    const hoveredSeriesState = ref({
      hoveredSeriesName: "",
      panelId: -1,
      dataIndex: -1,
      seriesIndex: -1,
      hoveredTime: null,
      setHoveredSeriesName: function (name: string) {
        hoveredSeriesState.value.hoveredSeriesName = name ?? "";
      },
      setIndex: function (
        dataIndex: number,
        seriesIndex: number,
        panelId: any,
        hoveredTime?: any
      ) {
        hoveredSeriesState.value.dataIndex = dataIndex ?? -1;
        hoveredSeriesState.value.seriesIndex = seriesIndex ?? -1;
        hoveredSeriesState.value.panelId = panelId ?? -1;
        hoveredSeriesState.value.hoveredTime = hoveredTime ?? null;
      },
    });

    // used provide and inject to share data between components
    // it is currently used in panelschemarendered, chartrenderer, convertpromqldata(via panelschemarenderer), and convertsqldata
    provide("hoveredSeriesState", hoveredSeriesState);

    // save the dashboard value
    const saveDashboard = async () => {
      try {
        await updateDashboard(
          store,
          store.state.selectedOrganization.identifier,
          props.dashboardData.dashboardId,
          props.dashboardData,
          route.query.folder ?? "default"
        );

        $q.notify({
          type: "positive",
          message: "Dashboard updated successfully",
          timeout: 5000,
        });
      } catch (error: any) {
        $q.notify({
          type: "negative",
          message: error?.message ?? "Dashboard update failed",
          timeout: 2000,
        });

        // refresh dashboard
        refreshDashboard();
      }
    };

    //add panel
    const addPanelData = () => {
      return router.push({
        path: "/dashboards/add_panel",
        query: {
          dashboard: route.query.dashboard,
          folder: route.query.folder ?? "default",
          tab: route.query.tab ?? props.dashboardData.panels[0]?.tabId,
        },
      });
    };

    const movedEvent = async (i, newX, newY) => {
      await saveDashboard();
    };

    const resizedEvent = async (i, newX, newY, newHPx, newWPx) => {
      window.dispatchEvent(new Event("resize"));
      await saveDashboard();
    };

    const getDashboardLayout: any = (panels: any) => {
      //map on each panels and return array of layouts
      return panels?.map((item: any) => item.layout) || [];
    };

    const getPanelLayout = (panelData, position) => {
      if (position == "x") {
        return panelData.layout?.x;
      } else if (position == "y") {
        return panelData?.layout?.y;
      } else if (position == "w") {
        return panelData?.layout?.w;
      } else if (position == "h") {
        return panelData?.layout?.h;
      } else if (position == "i") {
        return panelData?.layout?.i;
      }
      return 0;
    };

    const getMinimumHeight = (type) => {
      switch (type) {
        case "area":
        case "bar":
        case "h-bar":
        case "line":
        case "pie":
        case "scatter":
        case "table":
          return 4;

        default:
          break;
      }
    };

    const getMinimumWidth = (type) => {
      switch (type) {
        case "area":
        case "bar":
        case "h-bar":
        case "line":
        case "pie":
        case "scatter":
        case "table":
          return 3;

        default:
          break;
      }
    };

    const layoutUpdate = () => {
      if (gridLayoutRef.value) {
        gridLayoutRef.value.layoutUpdate();
      }
    };

    // update initial variable values using the variable value selector ref
    const updateInitialVariableValues = async (...args: any) => {
      // if view panel is open then close it
      showViewPanel.value = false;

      // first, refresh the dashboard
      refreshDashboard();

      // NOTE: after variables in variables feature, it works without changing the initial variable values
      // then, update the initial variable values
      await variablesValueSelectorRef.value.changeInitialVariableValues(
        ...args
      );
    };

    return {
      store,
      addPanelData,
      t,
      movedEvent,
      resizedEvent,
      getPanelLayout,
      getMinimumHeight,
      getMinimumWidth,
      variablesData,
      variablesDataUpdated,
      getDashboardLayout,
      gridLayoutRef,
      layoutUpdate,
      showViewPanel,
      viewPanelId,
      selectedTabId,
      panels,
      refreshDashboard,
      onMovePanel,
      variablesValueSelectorRef,
      updateInitialVariableValues,
      isDashboardVariablesAndPanelsDataLoadedDebouncedValue,
    };
  },
  methods: {
    onDeletePanel(panelId) {
      this.$emit("onDeletePanel", panelId);
    },
    onViewPanel(panelId) {
      this.viewPanelId = panelId;
      this.showViewPanel = true;
    },
  },
});
</script>

<style lang="scss" scoped>
.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}

.vue-grid-layout {
  transition: none;
}

.vue-grid-item {
  transition: none;
}

// .vue-grid-item:not(.vue-grid-placeholder) {
//   background: #ccc;
//   border: 1px solid black;
// }

.vue-grid-item {
  border: 1px solid black;
}

.vue-grid-item .resizing {
  opacity: 0.9;
}

.vue-grid-item .static {
  background: #cce;
}

.vue-grid-item .text {
  font-size: 24px;
  text-align: center;
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  right: 0;
  margin: auto;
  height: 100%;
  width: 100%;
}

.vue-grid-item .no-drag {
  height: 100%;
  width: 100%;
}

.vue-grid-item .minMax {
  font-size: 12px;
}

.vue-grid-item .add {
  cursor: pointer;
}

.vue-draggable-handle {
  position: absolute;
  width: 20px;
  height: 20px;
  top: 0;
  left: 0;
  background: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10'><circle cx='5' cy='5' r='5' fill='#999999'/></svg>")
    no-repeat;
  background-position: bottom right;
  padding: 0 8px 8px 0;
  background-repeat: no-repeat;
  background-origin: content-box;
  box-sizing: border-box;
  cursor: pointer;
}

.layoutJSON {
  background: #ddd;
  border: 1px solid black;
  margin-top: 10px;
  padding: 10px;
}

.eventsJSON {
  background: #ddd;
  border: 1px solid black;
  margin-top: 10px;
  padding: 10px;
  height: 100px;
  overflow-y: scroll;
}

.displayDiv {
  clear: both;
  // padding: 1.625em 0 0;
  // overflow: auto;
}

.gridBackground {
  background: #00000000 !important;
  border-radius: 4px;
  border-color: #c2c2c27a !important;
}

.gridBackground.dark {
  border-color: rgba(204, 204, 220, 0.12) !important;
}
</style>
