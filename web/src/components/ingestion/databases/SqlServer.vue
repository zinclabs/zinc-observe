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
  <div class="q-ma-md">
    <div class="tw-text-[16px]">
      <div class="tw-font-bold tw-pt-6 tw-pb-2">
        Check further documentation at:
        <br />
        <a :href='docURL' target="_blank" class="cursor-pointer">{{docURL}}</a>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, type Ref } from "vue";
import config from "../../../aws-exports";
import { useStore } from "vuex";
import { getImageURL } from "../../../utils/zincutils";
import CopyContent from "@/components/CopyContent.vue";

export default defineComponent({
  name: "AWSConfig",
  props: {
    currOrgIdentifier: {
      type: String,
    },
    currUserEmail: {
      type: String,
    },
  },
  components: { CopyContent },
  setup(props) {
    const docURL = `https://short.openobserve.ai/database/sql-server`;

    
    return {
      config,
      docURL,
      getImageURL,
    };
  },
});
</script>
