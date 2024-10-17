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
  <div
    class="q-pa-none !tw-w-[300px]"
    :class="store.state.theme == 'dark' ? 'dark-mode' : 'bg-white'"
    style="min-height: inherit"
  >
    <div class="row items-center no-wrap">
      <div class="col">
        <div class="q-mx-md q-my-md text-h6">
          {{ t("panel.layout") }}
        </div>
      </div>
      <div class="col-auto">
        <q-btn
          v-close-popup="true"
          round
          flat
          :icon="'img:' + getImageURL('images/common/close_icon.svg')"
        />
      </div>
    </div>
    <q-separator></q-separator>
    <q-form @submit="savePanelLayout">
      <div class="q-mx-md">
        <div
          data-test="panel-layout-settings-height"
          class="o2-input tw-relative"
          style="padding-top: 12px"
        >
          <q-input
            v-model.number="updatedLayout.h"
            :label="t('dashboard.panelHeight') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            type="number"
            :rules="[
              (val: any) => {
                if (val === null || val === undefined || val === '') {
                  return t('common.required'); // If value is empty or null
                }
                return val > 0 ? true : t('common.valueMustBeGreaterThanZero'); // Ensure value is greater than 0
              },
            ]"
            style="min-width: 220px"
          />

          <div class="tw-text-[12px]">
            Approximately
            <span class="tw-font-bold">{{ getRowCount }}</span> table rows will
            be displayed
          </div>

          <q-icon
            name="info_outline"
            class="cursor-pointer q-ml-sm tw-absolute tw-top-[14px] tw-left-[94px]"
            size="16px"
          >
            <q-tooltip
              anchor="center end"
              self="center left"
              class="tw-text-[12px]"
            >
              1 unit = 30px
            </q-tooltip>
          </q-icon>
        </div>
      </div>
      <div class="flex justify-center q-mt-lg">
        <q-btn
          ref="closeBtn"
          v-close-popup="true"
          class="q-mb-md text-bold"
          :label="t('dashboard.cancel')"
          text-color="light-text"
          padding="sm md"
          no-caps
        />
        <q-btn
          :label="t('dashboard.save')"
          class="q-mb-md text-bold no-border q-ml-md"
          color="secondary"
          padding="sm xl"
          type="submit"
          no-caps
        />
      </div>
    </q-form>
  </div>
</template>

<script lang="ts">
import { computed, defineComponent, ref } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { getImageURL } from "../../utils/zincutils";

export default defineComponent({
  name: "PanelLayoutSettings",
  components: {},
  props: {
    layout: {
      type: Object,
      required: true,
    },
  },
  emits: ["save:layout"],
  setup(props, { emit }) {
    const store = useStore();
    const { t } = useI18n();
    const router = useRouter();

    const updatedLayout = ref({ ...props.layout });

    const savePanelLayout = () => {
      emit("save:layout", { ...updatedLayout.value });
    };

    const getRowCount = computed(() => {
      // 24 is the height of toolbar
      // 28.5 is the height of each row
      const count = Number(Math.ceil((updatedLayout.value.h * 30 - 24) / 28.5));

      if (count < 0) return 0;

      return count;
    });

    return {
      t,
      store,
      router,
      getImageURL,
      savePanelLayout,
      getRowCount,
      updatedLayout,
    };
  },
});
</script>

<style scoped lang="scss">
.dark-mode {
  background-color: $dark-page;
}
</style>
