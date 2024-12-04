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
  <div :style="{ height: 'calc(100vh - 57px)', overflow: 'hidden' }">
    <template v-if="isLoading.length">
      <div
        class="q-pb-lg flex items-center justify-center text-center"
        style="height: calc(100vh - 190px)"
      >
        <div>
          <q-spinner-hourglass
            color="primary"
            size="40px"
            style="margin: 0 auto; display: block"
          />
          <div class="text-center full-width">
            Hold on tight, we're loading RUM data.
          </div>
        </div>
      </div>
    </template>
    <template v-else-if="isRumEnabled || isSessionReplayEnabled">
      <AppTabs
        :show="showTabs"
        :tabs="tabs"
        v-model:active-tab="activeTab"
        @update:active-tab="changeTab"
      />
      <router-view v-slot="{ Component }">
        <template v-if="$route.meta.keepAlive">
          <keep-alive>
            <component
              :is="Component"
              :isRumEnabled="isRumEnabled"
              :isSessionReplayEnabled="isSessionReplayEnabled"
            />
          </keep-alive>
        </template>
        <template v-else>
          <component
            :is="Component"
            :isRumEnabled="isRumEnabled"
            :isSessionReplayEnabled="isSessionReplayEnabled"
          />
        </template>
      </router-view>
    </template>
    <template v-else>
      <div class="q-pa-lg enable-rum">
        <div class="q-pb-lg">
          <div class="text-left text-h6 text-bold q-pb-md">
            {{ t("rum.aboutRUMTitle") }}
          </div>
          <div class="text-subtitle1">
            {{ t("rum.aboutRUMMessage") }}
          </div>
          <div>
            <div></div>
          </div>
        </div>
        <q-btn
          class="bg-secondary rounded text-white"
          no-caps
          :title="t('rum.getStartedTitle')"
          @click="getStarted"
        >
          {{ t("rum.getStartedLabel") }}
          <q-icon name="arrow_forward" size="20px" class="q-ml-xs" />
        </q-btn>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onActivated, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useStore } from "vuex";
import useSession from "@/composables/useSessionReplay";
import useErrorTracking from "@/composables/useErrorTracking";
import usePerformance from "@/composables/rum/usePerformance";

import { b64EncodeUnicode } from "@/utils/zincutils";
import { useI18n } from "vue-i18n";
import useRum from "@/composables/rum/useRum";
import useStreams from "@/composables/useStreams";
import AppTabs from "@/components/common/AppTabs.vue";

const route = useRoute();
const router = useRouter();
const store = useStore();
const showTabs = computed(() => {
  const routes = [
    "Sessions",
    "ErrorTracking",
    "RumPerformance",
    "rumPerformanceSummary",
    "rumPerformanceWebVitals",
    "rumPerformanceErrors",
    "rumPerformanceApis",
  ];
  return routes.includes(router.currentRoute.value.name?.toString() || "");
});

const { t } = useI18n();
const isLoading = ref<boolean[]>([]);
const { sessionState } = useSession();
const { errorTrackingState } = useErrorTracking();
const { performanceState } = usePerformance();
const { rumState } = useRum();
const { getStream, getStreams } = useStreams();

const activeTab = ref<string>("performance");
const tabs = [
  {
    label: t("rum.performance"),
    value: "performance",
  },
  {
    label: t("rum.sessions"),
    value: "sessions",
  },
  {
    label: t("rum.errorTracking"),
    value: "error_tracking",
  },
];

const isRumEnabled = ref<boolean>(false);
const isSessionReplayEnabled = ref<boolean>(false);

const routeName = computed(() => router.currentRoute.value.name);

onMounted(async () => {
  isLoading.value.push(true);

  await checkIfRumEnabled();

  isLoading.value.pop();

  if (!isRumEnabled.value && !isSessionReplayEnabled.value) return;

  await getSchema();

  const routeNameMapping: { [key: string]: string } = {
    SessionViewer: "sessions",
    ErrorTracking: "error_tracking",
    RumPerformance: "performance",
    ErrorViewer: "error_tracking",
    Sessions: "sessions",
    rumPerformanceSummary: "performance",
  };

  if (routeNameMapping[routeName.value?.toString() || "placeholder"]) {
    activeTab.value =
      routeNameMapping[
        router.currentRoute.value.name?.toString() || "placeholder"
      ];
  } else {
    activeTab.value = "performance";
  }

  // This is temporary fix, as we have kept sessionViewer keep-alive as false.
  // So on routing to sessionViewer, this hook is called triggered and it routes to Session page again
  const ignoreRoutes = ["SessionViewer", "ErrorViewer"];

  if (!ignoreRoutes.includes(routeName.value as string))
    changeTab(activeTab.value);
});

onActivated(async () => {
  await checkIfRumEnabled();
});

watch(
  () => routeName.value,
  () => updateTabOnRouteChange()
);

const updateTabOnRouteChange = () => {
  const routeNameMapping: { [key: string]: string } = {
    SessionViewer: "sessions",
    ErrorTracking: "error_tracking",
    RumPerformance: "performance",
    Sessions: "sessions",
    rumPerformanceSummary: "performance",
    rumPerformanceWebVitals: "performance",
    rumPerformanceErrors: "performance",
    rumPerformanceApis: "performance",
  };
  const tab =
    routeNameMapping[
      router.currentRoute.value.name?.toString() || "placeholder"
    ];
  if (tab !== activeTab.value && tab !== undefined) {
    activeTab.value = tab;
  }
};

const checkIfRumEnabled = async () => {
  await nextTick();
  return new Promise(async (resolve) => {
    getStream("_rumdata", "logs", false)
      .then((response: any) => {
        if (response) isRumEnabled.value = true;
      })
      .finally(() => {
        resolve(true);
      });

    getStream("_sessionreplay", "logs", false)
      .then((response: any) => {
        if (response) isSessionReplayEnabled.value = true;
      })
      .finally(() => {
        resolve(true);
      });
  });
};

const getQueryParams = (dateTime: any, editorValue: string) => {
  const query: any = {};

  if (dateTime.valueType == "relative") {
    query["period"] = dateTime.relativeTimePeriod;
  } else {
    query["from"] = dateTime.startTime;
    query["to"] = dateTime.endTime;
  }

  if (editorValue) query["query"] = b64EncodeUnicode(editorValue);

  query["org_identifier"] = store.state.selectedOrganization.identifier;
  return query;
};

const changeTab = (tab: string) => {
  if (tab === "performance") {
    router.push({
      name: "rumPerformanceSummary",
      query: getQueryParams(performanceState.data.datetime, ""),
    });
    return;
  }

  if (tab === "error_tracking") {
    router.push({
      name: "ErrorTracking",
      query: getQueryParams(
        performanceState.data.datetime,
        errorTrackingState.data.editorValue
      ),
    });
    return;
  }

  if (tab === "sessions") {
    router.push({
      name: "Sessions",
      query: getQueryParams(
        performanceState.data.datetime,
        sessionState.data.editorValue
      ),
    });
    return;
  }
};

const getStarted = () => {
  router.push({
    name: "frontendMonitoring",
    query: { org_identifier: store.state.selectedOrganization.identifier },
  });
};

const getSchema = async () => {
  return new Promise((resolve) => {
    getSessionReplayFields().finally(() => {
      getRumDataFields().finally(() => {
        resolve(true);
      });
    });
  });
};

const getSessionReplayFields = () => {
  isLoading.value.push(true);
  return new Promise((resolve) => {
    getStream("_sessionreplay", "logs", true)
      .then((stream) => {
        performanceState.data.streams["_sessionreplay"] = {
          schema: {},
          name: "_sessionreplay",
        };
        stream.schema.forEach((field: any) => {
          performanceState.data.streams["_sessionreplay"]["schema"][
            field.name
          ] = field;
        });
      })
      .finally(() => {
        resolve(true);
        isLoading.value.pop();
      });
  });
};

const getRumDataFields = () => {
  isLoading.value.push(true);
  return new Promise((resolve) => {
    getStream("_rumdata", "logs", true)
      .then((stream) => {
        performanceState.data.streams["_rumdata"] = {
          schema: {},
          name: "_rumdata",
        };
        stream.schema.forEach((field: any) => {
          performanceState.data.streams["_rumdata"]["schema"][field.name] =
            field;
        });
      })
      .finally(() => {
        resolve(true);
        isLoading.value.pop();
      });
  });
};
</script>

<style scoped lang="scss">
.rum-tabs {
  border-bottom: 1px solid #e0e0e0;
  .rum-tab {
    border-bottom: 2px solid transparent;
    width: 140px;
  }
  .active {
    border-bottom: 2px solid $primary;
  }
}

.enable-rum {
  max-width: 1024px;
}
</style>
