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
  <div class="sidebar" :class="{ open: isOpen }">
    <div v-if="!isOpen" class="sidebar-header-collapsed" @click="toggleSidebar">
      <!-- <div class="collapsed-icon">+</div> -->
      <q-icon
        name="expand_all"
        class="collapsed-icon rotate-90"
        data-test="dashboard-sidebar"
      />
      <div class="collapsed-title">{{ title }}</div>
    </div>
    <div v-else class="sidebar-header-expanded">
      <div class="expanded-title">{{ title }}</div>
      <q-btn
        square
        icon="unfold_less"
        class="collapse-button rotate-90"
        @click="toggleSidebar"
        data-test="dashboard-sidebar-collapse-btn"
      />
    </div>
    <q-separator style="margin-top: -1px" />
    <div class="sidebar-content scroll" v-if="isOpen">
      <slot></slot>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, watch } from "vue";

export default defineComponent({
  props: {
    title: {
      type: String,
      required: true,
    },
    modelValue: {
      type: Boolean,
      required: true,
    },
  },
  emits: ["update:modelValue"],
  setup(props, { emit }) {
    const isOpen = ref(props.modelValue);

    const toggleSidebar = () => {
      isOpen.value = !isOpen.value;
      emit("update:modelValue", isOpen.value);
    };

    watch(
      () => props.modelValue,
      (value) => {
        isOpen.value = value;
      }
    );

    return {
      isOpen,
      toggleSidebar,
    };
  },
});
</script>

<style scoped>
.sidebar {
  position: relative;
  width: 50px;
  height: 100%;
}

.sidebar.open {
  width: 300px;
}

.sidebar-header-collapsed {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-start;
  width: 50px;
  height: 100%;
  overflow-y: auto;
  cursor: pointer;
}

.sidebar-header-expanded {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 60px;
  padding: 0 10px;
}

.collapsed-icon {
  margin-top: 10px;
  font-size: 20px;
}

.collapsed-title {
  writing-mode: vertical-rl;
  text-orientation: mixed;
  font-weight: bold;
}

.expanded-title {
  font-weight: bold;
}

.collapse-button {
  height: 30px;
  width: 30px;
  padding: 0px 0px;
}

.sidebar-content {
  padding: 0px 10px;
  height: calc(100vh - 176px);
  overflow-y: auto;
}
</style>
