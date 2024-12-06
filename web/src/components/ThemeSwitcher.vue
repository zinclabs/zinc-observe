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
  <q-btn class="round-button" round flat dense :ripple="false" @click="toggleDarkMode">
    <q-icon :name="DarkModeIcon" size="25px" class="header-icon"></q-icon>
    <q-tooltip anchor="top middle" self="bottom middle">
      Switch to {{ darkMode ? "Light Mode" : "Dark Mode" }}
    </q-tooltip>
  </q-btn>
</template>

<script lang="ts">
import { ref, watch, onMounted, computed, defineComponent } from "vue";
import { useQuasar } from "quasar";
import { useStore } from "vuex";
import {
  outlinedDarkMode,
  outlinedLightMode,
} from "@quasar/extras/material-icons-outlined";

export default defineComponent({
  setup() {
    const store = useStore();
    const $q = useQuasar();
    const darkMode = ref(false);

    const DarkModeIcons = {
      light: outlinedLightMode,
      dark: outlinedDarkMode,
    };

    onMounted(() => {
      const savedTheme = localStorage.getItem("theme");
      if (savedTheme !== null) {
        darkMode.value = savedTheme == "dark";
      } else {
        // Default to light theme if no saved theme is found
        darkMode.value = false;
      }
      setTheme(darkMode.value ? "dark" : "light");
    });

    const DarkModeIcon = computed(() => {
      return darkMode.value ? DarkModeIcons.dark : DarkModeIcons.light;
    });

    watch(darkMode, () => {
      setTheme(darkMode.value ? "dark" : "light");
    });
    watch(
      () => store.state.theme,
      () => {}
    );

    const setTheme = (theme: any) => {
      localStorage.setItem("theme", theme);
      $q.dark.set(theme == "dark");
      store.dispatch("appTheme", theme);
    };

    const toggleDarkMode = () => {
      darkMode.value = !darkMode.value;
    };

    return {
      store,
      darkMode,
      DarkModeIcon,
      toggleDarkMode,
      outlinedDarkMode,
      outlinedLightMode,
    };
  },
});
</script>

<style>
.light-mode {
  background-color: #ffffff;
  color: #000000;
}

.dark-mode {
  background-color: #36383a;
  color: #ffffff;
}

.round-button {
  border: none;
  border-radius: 50%;
  box-shadow: none;
  transition: box-shadow 0.3s ease;
}

.round-button:hover {
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}
</style>
