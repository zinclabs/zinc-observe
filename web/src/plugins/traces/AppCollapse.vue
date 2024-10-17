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
    <div class="flex items-start">
      <slot name="header" />
    </div>
    <div v-if="isOpen">
      <slot name="content"></slot>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";

export default defineComponent({
  name: "AppCollapse",
  props: {
    open: {
      type: Boolean,
      default: () => false,
    },
    headerStyle: {
      type: Object,
      default: () => {},
    },
  },
  setup(props, { emit }) {
    const isOpen: any = ref(props.open);
    function updateCollapse() {
      isOpen.value = !isOpen.value;
    }
    return {
      isOpen,
      updateCollapse,
    };
  },
});
</script>

<style scoped></style>
