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
  <div>
    <img
      :alt="column.category"
      :src="typeIcons"
      style="width: 22px; height: 22px"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import error from "@/assets/images/rum/events/error.png";
import navigation from "@/assets/images/rum/events/navigation.png";
import user from "@/assets/images/rum/events/user.png";
import xhr from "@/assets/images/rum/events/xhr.png";

const props = defineProps({
  column: {
    type: Object,
    required: true,
  },
});

const typeIcons = computed(() => {
  if (props.column.category.toLowerCase() === "error") {
    return error;
  } else if (props.column.category.toLowerCase() === "xhr") {
    return xhr;
  } else if (props.column.category.toLowerCase() === "navigation") {
    return navigation;
  } else if (props.column.category.toLowerCase() === "click") {
    return user;
  } else if (props.column.category.toLowerCase() === "reload") {
    return navigation;
  }
  return error;
});
</script>

<style scoped></style>
