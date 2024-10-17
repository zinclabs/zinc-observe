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
  <div class="flex column justify-center q-pa-xs" style="font-size: 1.5rem">
    <q-img
      :src="getImageURL('images/common/clipboard_icon.svg')"
      style="width: 230px; margin: 5vh auto 2rem"
    />
    <div class="flex justify-center q-mb-md">
      <span>Start by adding your first dashboard panel</span>
    </div>
    <div class="flex justify-center" v-if="!viewOnly">
      <q-btn
        :label="t('panel.add')"
        stack
        padding="md"
        outline
        icon="insert_drive_file"
        class=""
        @click="$emit('update:Panel')"
        data-test="dashboard-if-no-panel-add-panel-btn"
      >
      </q-btn>
    </div>
  </div>
</template>
<script lang="ts">
import { defineComponent } from "vue";
import { useI18n } from "vue-i18n";
import { getImageURL } from "../../../utils/zincutils";

export default defineComponent({
  name: "NoPanel",
  props: ["Panel", "viewOnly"],
  emits: ["update:Panel"],

  setup() {
    const { t } = useI18n();

    return { t, getImageURL };
  },
});
</script>
