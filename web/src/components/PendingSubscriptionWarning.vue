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
  <div class="row col-11 justify-center q-mt-xl">
    <q-icon
      name="warning"
      size="80px"
      color="warning row col-8 justify-center q-mt-lg"
    /><br />
    <h5>
      <b
        ><u><span class="text-red">Warning:</span></u></b
      >
      The content of this page cannot be viewed because the selected
      organization's is on pending subscription state. <br /><br /><q-btn
        color="primary"
        @click="routerPush"
        >Click here to proceed with subscription</q-btn
      >
    </h5>
  </div>
</template>

<script lang="ts">
import { useRouter } from "vue-router";
// @ts-nocheck
import { defineComponent } from "vue";
import { useStore } from "vuex";

export default defineComponent({
  name: "PendingSubscriptionWarning",
  setup() {
    const router = useRouter();
    const store = useStore();

    const routerPush = () => {
      router.push({
        name: "plans",
        query: { org_identifier: store.state.selectedOrganization.identifier },
      });
    };

    return {
      router,
      routerPush,
    };
  },
});
</script>
