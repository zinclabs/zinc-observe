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
  <div v-if="memberData.length == 0" class="q-pt-md text-center">
    No data available.
  </div>
  <div v-else class="q-pt-md round-box" id="data-list">
    <q-toolbar class="bordered-toolbar-title">
      <q-toolbar-title>Member List</q-toolbar-title>
    </q-toolbar>
    <q-list bordered separator style="overflow: auto; max-height: calc(800vh)">
      <q-item
        v-for="member in memberData"
        :key="member.email"
        v-ripple
        bordered
        clickable
      >
        <q-item-section>{{ member.email }}</q-item-section>
        <q-item-section side
          ><q-item-label caption> {{ member.status }}</q-item-label>
        </q-item-section>
      </q-item>
    </q-list>
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent } from "vue";

export default defineComponent({
  name: "ListAssociatedMembers",
  props: {
    // eslint-disable-next-line vue/require-default-prop
    data: {
      type: Array,
    },
  },
  setup() {
    return {};
  },
  data() {
    return {
      memberData: this.data,
    };
  },
});
</script>

<style lang="scss">
.round-box {
  border-radius: 2px;
  & > :first-of-type {
    border-radius: 2px 2px 0 0;
  }
  & > :last-of-type {
    border-radius: 0 0 2px 2px;
  }

  .bordered-toolbar-title {
    border: 1px solid $input-border;
    border-bottom: none;

    .q-toolbar__title {
      font-size: 0.75rem;
      font-weight: bold;
    }
  }
}
</style>
