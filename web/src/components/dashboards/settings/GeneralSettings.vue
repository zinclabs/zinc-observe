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
  <div class="column full-height">
    <DashboardHeader :title="t('dashboard.generalSettingsTitle')" />
    <div>
      <q-form ref="addDashboardForm" @submit="onSubmit">
        <q-input
          v-model="dashboardData.title"
          :label="t('dashboard.name') + ' *'"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          :rules="[(val: any) => !!val.trim() || t('dashboard.nameRequired')]"
          data-test="dashboard-general-setting-name"
        />
        <span>&nbsp;</span>
        <q-input
          v-model="dashboardData.description"
          :label="t('dashboard.typeDesc')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          data-test="dashboard-general-setting-description"
        />
        <div v-if="dateTimeValue" data-test="dashboard-general-setting-datetime-picker">
          <label>Default Duration</label>
          <DateTimePickerDashboard
            v-show="store.state.printMode === false"
            ref="dateTimePicker"
            class="dashboard-icons q-my-sm"
            size="sm"
            :initialTimezone="initialTimezone"
            v-model="dateTimeValue"
            :auto-apply-dashboard="true"
          />
        </div>
        <q-toggle
          v-model="dashboardData.showDynamicFilters"
          label="Show Dynamic Filters"
          data-test="dashboard-general-setting-dynamic-filter"
        ></q-toggle>
        <div class="flex justify-center q-mt-lg">
          <q-btn
            ref="closeBtn"
            v-close-popup="true"
            class="q-mb-md text-bold"
            :label="t('dashboard.cancel')"
            text-color="light-text"
            padding="sm md"
            no-caps
            data-test="dashboard-general-setting-cancel-btn"
          />
          <q-btn
            :disable="dashboardData.title.trim() === ''"
            :label="t('dashboard.save')"
            class="q-mb-md text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            type="submit"
            no-caps
            :loading="saveDashboardApi.isLoading.value"
            data-test="dashboard-general-setting-save-btn"
          />
        </div>
      </q-form>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, onMounted, ref, watch, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { reactive } from "vue";
import { getDashboard, updateDashboard } from "@/utils/commons";
import { useRoute } from "vue-router";
import DashboardHeader from "./common/DashboardHeader.vue";
import { useLoading } from "@/composables/useLoading";
import DateTimePickerDashboard from "@/components/DateTimePickerDashboard.vue";
import useNotifications from "@/composables/useNotifications";

export default defineComponent({
  name: "GeneralSettings",
  components: {
    DashboardHeader,
    DateTimePickerDashboard,
  },
  emits: ["save"],
  setup(props, { emit }) {
    const store: any = useStore();
    const { t } = useI18n();
    const route = useRoute();
    const {
      showPositiveNotification,
      showErrorNotification,
      showConfictErrorNotificationWithRefreshBtn,
    } = useNotifications();

    const addDashboardForm: Ref<any> = ref(null);
    const closeBtn: Ref<any> = ref(null);
    // initial timezone, which will come from the route query
    const initialTimezone: any = ref(store.state.timezone ?? null);

    const dashboardData = reactive({
      title: "",
      description: "",
      showDynamicFilters: true,
      defaultDatetimeDuration: {
        startTime: null,
        endTime: null,
        relativeTimePeriod: "15m",
        type: "relative",
      },
    });

    let dateTimeValue: any = ref(null);
    const getDashboardData = async () => {
      const data = await getDashboard(
        store,
        route.query.dashboard,
        route.query.folder ?? "default"
      );

      dashboardData.title = data.title;
      dashboardData.description = data.description;
      dashboardData.showDynamicFilters =
        data.variables?.showDynamicFilters ?? false;

      dateTimeValue.value = {
        startTime: data?.defaultDatetimeDuration?.startTime,
        endTime: data?.defaultDatetimeDuration?.endTime,
        relativeTimePeriod: data?.defaultDatetimeDuration?.relativeTimePeriod,
        valueType: data?.defaultDatetimeDuration?.type,
      };
    };
    onMounted(async () => {
      await getDashboardData();
    });

    const saveDashboardApi = useLoading(async () => {
      try {
        // get the latest dashboard data and update the title and description
        const data = JSON.parse(
          JSON.stringify(
            await getDashboard(
              store,
              route.query.dashboard,
              route.query.folder ?? "default"
            )
          )
        );

        // update the values
        data.title = dashboardData.title;
        data.description = dashboardData.description;

        if (!data.variables) {
          data.variables = {
            list: [],
            showDynamicFilters: dashboardData.showDynamicFilters,
          };
        } else {
          data.variables.showDynamicFilters = dashboardData.showDynamicFilters;
        }

        data.defaultDatetimeDuration = {
          startTime: dateTimeValue?.value?.startTime,
          endTime: dateTimeValue?.value?.endTime,
          relativeTimePeriod: dateTimeValue?.value?.relativeTimePeriod,
          type: dateTimeValue?.value?.valueType,
        };

        // now lets save it
        await updateDashboard(
          store,
          store.state.selectedOrganization.identifier,
          route.query.dashboard,
          data,
          route?.query?.folder ?? "default"
        );

        showPositiveNotification("Dashboard updated successfully.");

        emit("save");
      } catch (error: any) {
        if (error?.response?.status === 409) {
          showConfictErrorNotificationWithRefreshBtn(
            error?.response?.data?.message ??
              error?.message ??
              "Dashboard updation failed"
          );
        } else {
          showErrorNotification(error?.message ?? "Dashboard updation failed", {
            timeout: 2000,
          });
        }
      }
    });

    const onSubmit = () => {
      addDashboardForm.value.validate().then((valid: any) => {
        if (!valid) {
          return false;
        }

        saveDashboardApi.execute().catch((err: any) => {
          showErrorNotification(
            JSON.stringify(
              err.response.data["error"] || "Dashboard creation failed."
            )
          );
        });
      });
    };

    return {
      t,
      dashboardData,
      addDashboardForm,
      store,
      saveDashboardApi,
      onSubmit,
      closeBtn,
      initialTimezone,
      dateTimeValue,
    };
  },
});
</script>

<style lang="scss" scoped>
.dashboard-icons {
  height: 30px;
}
</style>
