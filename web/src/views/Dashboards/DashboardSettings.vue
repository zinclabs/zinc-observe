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
    class="q-pa-none"
    :class="store.state.theme == 'dark' ? 'dark-mode' : 'bg-white'"
    style="min-height: inherit"
  >
    <div class="row items-center no-wrap">
      <div class="col">
        <div class="q-mx-md q-my-md text-h6">
          {{ t("dashboard.setting") }}
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
    <q-splitter
      v-model="splitterModel"
      unit="px"
      style="height: calc(100vh - 65px)"
      disable
    >
      <template v-slot:before>
        <div class="functions-tabs" style="width: 100%">
          <q-tabs
            v-model="activeTab"
            indicator-color="transparent"
            inline-label
            vertical
          >
            <q-tab
              name="generalSettings"
              icon="settings"
              :label="t('dashboard.generalSettings')"
            />
            <q-tab
              name="variableSettings"
              icon="data_array"
              :label="t('dashboard.variableSettings')"
            />
            <q-tab
              name="tabSettings"
              icon="tab"
              :label="t('dashboard.tabSettings')"
            />
          </q-tabs>
        </div>
      </template>
      <template v-slot:after>
        <div class="q-mx-md q-my-sm scroll" style="width: 50vw">
          <q-tab-panels
            v-model="activeTab"
            animated
            swipeable
            vertical
            transition-prev="fade"
            transition-next="fade"
          >
            <q-tab-panel name="generalSettings">
              <GeneralSettings @save="refreshRequired" />
            </q-tab-panel>

            <q-tab-panel name="variableSettings">
              <VariableSettings @save="refreshRequired" />
            </q-tab-panel>

            <q-tab-panel name="tabSettings">
              <TabsSettings @refresh="refreshRequired" />
            </q-tab-panel>
          </q-tab-panels>
        </div>
      </template>
    </q-splitter>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, onActivated, onBeforeMount } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import GeneralSettings from "../../components/dashboards/settings/GeneralSettings.vue";
import VariableSettings from "../../components/dashboards/settings/VariableSettings.vue";
import TabsSettings from "../../components/dashboards/settings/TabsSettings.vue";
import { getImageURL } from "../../utils/zincutils";

export default defineComponent({
  name: "AppSettings",
  components: {
    VariableSettings,
    GeneralSettings,
    TabsSettings,
  },
  emits: ["refresh"],
  setup(props, { emit }) {
    const store = useStore();
    const { t } = useI18n();
    const router = useRouter();
    const activeTab: any = ref("generalSettings");
    const templates = ref([]);
    const splitterModel = ref(220);

    const refreshRequired = () => {
      emit("refresh");
    };

    return {
      t,
      store,
      router,
      splitterModel,
      activeTab,
      templates,
      getImageURL,
      refreshRequired,
    };
  },
});
</script>

<style scoped lang="scss">
.dark-mode {
  background-color: $dark-page;
}
.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}
.functions-tabs {
  .q-tabs {
    &--vertical {
      margin: 20px 16px 0 16px;
      .q-tab {
        justify-content: flex-start;
        padding: 0 1rem 0 1.25rem;
        border-radius: 0.5rem;
        margin-bottom: 0.5rem;
        // color: $dark;
        text-transform: capitalize;
        &__content.tab_content {
          .q-tab {
            &__icon + &__label {
              padding-left: 0.875rem;
              font-weight: 600;
            }
          }
        }
        &--active {
          background-color: $accent;
          color: $dark;
        }
      }
    }
  }
}
</style>
