<template>
  <div>
    <div
      v-if="
        !(
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery && dashboardPanelData.data.queryType == 'sql'
        )
      "
      style="display: flex; flex-direction: row"
      class="q-pl-md"
    >
      <div class="layout-name">{{ t("panel.filters") }}</div>
      <span class="layout-separator">:</span>
      <div
        class="axis-container droppable scroll row"
        data-test="dashboard-filter-layout"
      >
        <Group
          v-if="topLevelGroup"
          :group="topLevelGroup"
          :group-nested-index="0"
          :group-index="0"
          :schema-options="schemaOptions"
          :dashboard-variables-filter-items="dashboardVariablesFilterItems(0)"
          :load-filter-item="loadFilterItem"
          :dashboard-panel-data="dashboardPanelData"
          @add-condition="addConditionToGroup"
          @add-group="addGroupToGroup"
          @remove-group="() => removeGroup(0)"
          @logical-operator-change="handleLogicalOperatorChange"
        />
      </div>
      <div></div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, computed, inject } from "vue";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import { useI18n } from "vue-i18n";
import Group from "./Group.vue";
import AddCondition from "./AddCondition.vue";

export default defineComponent({
  name: "DashboardFiltersOption",
  components: {
    Group,
    AddCondition,
  },
  props: ["dashboardData"],

  setup(props) {
    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "dashboard",
    );

    const {
      dashboardPanelData,
      removeFilterItem,
      loadFilterItem,
      selectedStreamFieldsBasedOnUserDefinedSchema,
    } = useDashboardPanelData(dashboardPanelDataPageKey);

    const { t } = useI18n();
    const showAddMenu = ref(false);

    const topLevelGroup = computed(() => {
      return dashboardPanelData?.data?.queries?.[
        dashboardPanelData?.layout?.currentQueryIndex || 0
      ]?.fields?.filter;
    });

    const addFilter = (filterType: string) => {
      showAddMenu.value = false;
      const currentQuery =
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ];

      if (filterType === "condition") {
        const defaultCondition = {
          type: "list",
          column: `${schemaOptions.value[0]?.value}`,
          filterType: "condition",
          operator: null,
          value: null,
          logicalOperator: "AND",
          values: [],
        };
        currentQuery.fields?.filter.conditions.push(defaultCondition);
      } else if (filterType === "group") {
        const defaultGroup = {
          conditions: [
            {
              type: "list",
              column: `${schemaOptions.value[0]?.value}`,
              filterType: "condition",
              operator: null,
              value: null,
              logicalOperator: "AND",
              values: [],
            },
          ],
          filterType: "group",
          logicalOperator: "AND",
        };
        currentQuery.fields?.filter.conditions.push(defaultGroup);
      }
    };

    const addConditionToGroup = (group: any) => {
      group.conditions.push({
        type: "list",
        column: `${schemaOptions.value[0]?.value}`,
        filterType: "condition",
        operator: null,
        value: null,
        logicalOperator: "AND",
        values: [],
      });
    };

    const addGroupToGroup = (group: any) => {
      group.conditions.push({
        conditions: [
          {
            type: "list",
            column: `${schemaOptions.value[0]?.value}`,
            filterType: "condition",
            operator: null,
            value: null,
            logicalOperator: "AND",
            values: [],
          },
        ],
        filterType: "group",
        logicalOperator: "AND",
      });
    };

    const removeGroup = (index: number) => {
      const currentQuery =
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ];
      currentQuery.fields?.filter?.splice(index, 1);
    };

    const handleLogicalOperatorChange = (
      index: number,
      newOperator: string,
    ) => {
      const currentQuery =
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ];
      const item = currentQuery.fields?.filter.conditions[index];

      if (item) {
        if (item.filterType === "condition") {
          item.logicalOperator = newOperator;
        } else if (item.filterType === "group") {
          updateGroupLogicalOperators(item, newOperator);
        }
      }
    };

    const updateGroupLogicalOperators = (group: any, newOperator: string) => {
      group.logicalOperator = newOperator;
      group.conditions.forEach((condition: any) => {
        if (condition.filterType === "group") {
          updateGroupLogicalOperators(condition, newOperator);
        } else {
          condition.logicalOperator = newOperator;
        }
      });
    };

    const dashboardVariablesFilterItems = (index: number) =>
      (props.dashboardData?.variables?.list ?? []).map((it: any) => {
        let value;
        const operator =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields?.filter?.conditions?.[index]?.operator || null;

        if (operator === "Contains" || operator === "Not Contains") {
          value = it.multiSelect
            ? "(" + "$" + "{" + it.name + "}" + ")"
            : "$" + it.name;
        } else {
          value = it.multiSelect
            ? "(" + "$" + "{" + it.name + "}" + ")"
            : "$" + it.name;
        }

        return {
          label: it.name,
          value: value,
        };
      });

    const schemaOptions = computed(() =>
      selectedStreamFieldsBasedOnUserDefinedSchema?.value?.map(
        (field: any) => ({
          label: field.name,
          value: field.name,
        }),
      ),
    );

    return {
      t,
      showAddMenu,
      dashboardPanelData,
      removeFilterItem,
      loadFilterItem,
      addFilter,
      addConditionToGroup,
      addGroupToGroup,
      removeGroup,
      handleLogicalOperatorChange,
      dashboardVariablesFilterItems,
      schemaOptions,
      topLevelGroup,
    };
  },
});
</script>

<style lang="scss" scoped>
.layout-name {
  font-size: 14px;
  white-space: nowrap;
  min-width: 130px;
  display: flex;
  align-items: center;
}

.layout-separator {
  display: flex;
  align-items: center;
  margin-left: 2px;
  margin-right: 2px;
}

.axis-container {
  margin: 5px;
}
</style>
