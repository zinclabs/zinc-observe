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
    <div class="row items-center no-wrap q-mb-sm">
      <div v-if="backButton" class="col-auto">
        <q-btn
          class="q-mr-md"
          no-caps
          @click="onBackClicked"
          padding="xs"
          outline
          icon="arrow_back_ios_new"
        />
      </div>
      <div class="col">
        <div class="text-h6">
          {{ title }}
        </div>
      </div>
      <div class="col-auto">
        <slot name="right"></slot>
      </div>
    </div>
    <q-separator class="q-mb-sm" />
  </div>
</template>
  
<script lang="ts">
import { defineComponent } from "vue";

export default defineComponent({
  name: "DashboardHeader",
  components: {},
  props: {
    title: {
      type: String,
      default: "",
    },
    backButton: {
      type: Boolean,
      default: false,
    },
  },
  emits: ["back"],
  setup(props, { emit }) {
    const onBackClicked = () => {
      emit("back");
    };

    return {
      onBackClicked,
    };
  },
});
</script>
  
<style scoped lang="scss"></style>
  