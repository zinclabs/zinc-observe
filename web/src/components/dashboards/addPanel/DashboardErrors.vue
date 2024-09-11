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
  <div v-if="props.errors.errors.length" :data-test="`dashboard-error`">
    <q-separator />
    <div>
      <q-bar class="row q-pa-sm expand-bar">
        <div style="flex: 1" @click="onDropDownClick">
          <q-icon
            flat
            :name="!showErrors ? 'arrow_right' : 'arrow_drop_down'"
            text-color="black"
            class="q-mr-sm"
          />
          <span class="text-subtitle2 text-weight-bold" style="color: red"
            >Errors ({{ props.errors.errors.length }})</span
          >
          <q-space />
        </div>
      </q-bar>
    </div>
    <div
      class="row"
      :style="!showErrors ? 'height: 0px;' : 'height: auto;'"
      style="overflow: hidden"
    >
      <div class="col">
        <div data-test="dashboard-error">
          <ul class="tw-list-disc tw-list-inside tw-px-3">
            <li
              v-for="(item, index) in props.errors.errors"
              :key="index"
              style="color: red"
              class="tw-py-1"
            >
              {{ item }}
            </li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

export default defineComponent({
  name: "DashboardErrorsComponent",
  props: ["errors"],

  setup(props, { emit }) {
    const showErrors = ref(false);
    const { t } = useI18n();

    const onDropDownClick = () => {
      showErrors.value = !showErrors.value;
    };

    watch([showErrors, props.errors], () => {
      resizeChartEvent();
    });

    watch(props.errors, () => {
      if (props.errors.errors.length > 0) {
        showErrors.value = true;
      }
    });

    const resizeChartEvent = () => {
      window.dispatchEvent(new Event("resize"));
    };

    return {
      props,
      t,
      onDropDownClick,
      showErrors,
    };
  },
});
</script>

<style lang="scss" scoped>
.expand-bar {
  overflow: hidden;
  cursor: pointer;

  &:hover {
    background-color: #eaeaeaa5;
  }
}
</style>