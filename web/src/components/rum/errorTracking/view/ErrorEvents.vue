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
  <div class="q-mt-lg">
    <div class="tags-title text-bold q-mb-sm q-ml-xs">Events</div>
    <AppTable :columns="columns || []" :rows="error.events || []">
      <template v-slot:error-type="slotProps: any">
        <ErrorTypeIcons :column="slotProps.column.row" />
      </template>
      <template v-slot:description="slotProps: any">
        <ErrorEventDescription :column="slotProps.column.row" />
      </template>
    </AppTable>
  </div>
</template>

<script setup lang="ts">
import AppTable from "@/components/AppTable.vue";
import { ref } from "vue";
import ErrorEventDescription from "@/components/rum/errorTracking/view/ErrorEventDescription.vue";
import { date } from "quasar";
import ErrorTypeIcons from "./ErrorTypeIcons.vue";
import { useI18n } from "vue-i18n";

defineProps({
  error: {
    type: Object,
    required: true,
  },
});

const { t } = useI18n();

const columns = ref([
  {
    name: "type",
    label: t("rum.type"),
    align: "left",
    sortable: true,
    style: (row: any) =>
      row["type"] === "error" ? "border-bottom: 1px solid red" : "",
    classes: "error-type",
    slot: true,
    slotName: "error-type",
  },
  {
    name: "category",
    field: (row: any) => getErrorCategory(row),
    prop: (row: any) => getErrorCategory(row),
    label: t("rum.category"),
    align: "left",
    sortable: true,
    style: (row: any) =>
      row["type"] === "error" ? "border-bottom: 1px solid red" : "",
  },
  {
    name: "description",
    label: t("rum.description"),
    align: "left",
    sortable: true,
    style: (row: any) =>
      row["type"] === "error" ? "border-bottom: 1px solid red" : "",
    classes: "description-column",
    slot: true,
    slotName: "description",
  },
  {
    name: "level",
    field: (row: any) => (row["type"] === "error" ? "error" : "info"),
    prop: (row: any) => (row["type"] === "error" ? "error" : "info"),
    label: t("rum.level"),
    align: "left",
    style: (row: any) =>
      row["type"] === "error" ? "color: red; border-bottom: 1px solid red" : "",
    classes: "error-level",
  },
  {
    name: "timestamp",
    field: (row: any) => getFormattedDate(row["_timestamp"] / 1000),
    prop: (row: any) => getFormattedDate(row["_timestamp"] / 1000),
    label: t("rum.timestamp"),
    align: "left",
    style: (row: any) =>
      row["type"] === "error" ? "border-bottom: 1px solid red" : "",
    sortable: true,
  },
]);

const getErrorCategory = (row: any) => {
  if (row["type"] === "error") return row["error_type"] || "Error";
  else if (row["type"] === "resource") return row["resource_type"];
  else if (row["type"] === "view")
    return row["view_loading_type"] === "route_change"
      ? "Navigation"
      : "Reload";
  else if (row["type"] === "action") return row["action_type"];
  else return row["type"];
};

const getFormattedDate = (timestamp: number) =>
  date.formatDate(Math.floor(timestamp), "MMM DD, YYYY HH:mm:ss Z");
</script>

<style scoped>
.tags-title {
  font-size: 16px;
}
</style>

<style>
.description-column {
  max-width: 60vw;
}

.error-level {
  border: 1px solid #e0e0e0;
}
</style>
