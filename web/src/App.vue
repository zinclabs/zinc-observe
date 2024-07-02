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

<template>
  <router-view></router-view>
  <app-notifications />
</template>

<script lang="ts">
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import AppNotifications from "@/components/notifications/AppNotifications.vue";

export default {
  components: {
    AppNotifications,
  },
  setup() {
    // if (
    //   config.isCloud == "false" &&
    //   window.location.origin != "http://localhost:8081"
    // ) {
    //   const pos = window.location.pathname.indexOf("/web/");
    //   const path =
    //     window.location.origin == "http://localhost:8081"
    //       ? "/"
    //       : pos > -1
    //       ? window.location.pathname.slice(0, pos + 5)
    //       : window.location.pathname;
    //   store.dispatch("endpoint", path);
    // }
    const router = useRouter();
    const creds = localStorage.getItem("creds");
    if (creds) {
      // const credsInfo = JSON.parse(creds);
      // store.dispatch("login", credsInfo);
      router.push("/logs");
    }
  },
};
</script>
