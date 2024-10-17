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
  <div>
    <div class="tags-title text-bold q-ml-xs">Tags</div>
    <div class="row items-center">
      <div class="q-mr-lg row items-center">
        <img
          :src="ip"
          alt="Chrome"
          class="col-6 q-mr-sm"
          style="height: 30px; width: 30px"
        />
        <div class="q-pl-xs">{{ error.ip }}</div>
      </div>
      <q-separator vertical />
      <div class="q-mx-lg items-center">
        <img
          :src="browserIcon"
          alt="Chrome"
          class="q-mr-md inline-block"
          style="height: auto; width: 30px"
        />
        <div class="inline-block">
          <div class="q-pl-xs">{{ error.user_agent_user_agent_family }}</div>
          <span class="q-pl-xs"> {{ getBrowserVersion }} </span>
        </div>
      </div>
      <q-separator vertical />
      <div class="q-mx-lg items-center">
        <img
          :src="osIcon"
          alt="Chrome"
          class="q-mr-md inline-block"
          style="height: auto; width: 30px"
        />
        <div class="inline-block">
          <div class="q-pl-xs">{{ error.user_agent_os_family }}</div>
          <div class="q-pl-xs row">{{ getOsVersion }}</div>
        </div>
      </div>
    </div>
    <div class="row items-center wrap q-mt-md">
      <template v-for="(value, tag) in getTags" :key="tag">
        <ErrorTag :tag="{ key: tag, value }" />
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import ErrorTag from "@/components/rum/errorTracking/view/ErrorTag.vue";
import chrome from "@/assets/images/rum/chrome.png";
import firefox from "@/assets/images/rum/firefox.png";
import safari from "@/assets/images/rum/safari.png";
import opera from "@/assets/images/rum/opera.png";
import edge from "@/assets/images/rum/edge.png";
import ip from "@/assets/images/rum/ip_ad.png";
import windows from "@/assets/images/rum/windows.png";
import mac from "@/assets/images/rum/mac.png";
import linux from "@/assets/images/rum/linux.png";

const props = defineProps({
  error: {
    type: Object,
    required: true,
  },
});

const osIcon = ref("");
const browserIcon = ref("");

onMounted(() => {
  osIcon.value = getOsIcon();
  browserIcon.value = getBrowserIcon();
});

const getBrowserIcon = () => {
  if (!props.error.user_agent_user_agent_family) return chrome;

  if (
    props.error.user_agent_user_agent_family?.toLowerCase().includes("chrome")
  ) {
    return chrome;
  } else if (
    props.error.user_agent_user_agent_family?.toLowerCase().includes("opera")
  ) {
    return opera;
  } else if (
    props.error.user_agent_user_agent_family?.toLowerCase().includes("firefox")
  ) {
    return firefox;
  } else if (
    props.error.user_agent_user_agent_family?.toLowerCase().includes("edge")
  ) {
    return edge;
  } else if (
    props.error.user_agent_user_agent_family?.toLowerCase().includes("safari")
  ) {
    return safari;
  }
  return chrome;
};

const getOsIcon = () => {
  if (props.error?.user_agent_os_family?.toLowerCase().includes("windows")) {
    return windows;
  } else if (props.error?.user_agent_os_family?.toLowerCase().includes("mac")) {
    return mac;
  } else if (
    props.error?.user_agent_os_family?.toLowerCase().includes("linux")
  ) {
    return linux;
  } else {
    return windows;
  }
};

const getOsVersion = computed(() => {
  let version = "Version ";

  if (!props.error.user_agent_os_major) return version + "Unknown";

  if (props.error.user_agent_os_major)
    version += props.error.user_agent_os_major;

  if (props.error.user_agent_os_minor)
    version += "." + props.error.user_agent_os_minor;

  if (props.error.user_agent_os_patch)
    version += "." + props.error.user_agent_os_patch;

  return version;
});

const getBrowserVersion = computed(() => {
  let version = "Version ";

  if (!props.error.user_agent_user_agent_major) return version + "Unknown";

  if (props.error.user_agent_user_agent_major)
    version += props.error.user_agent_user_agent_major;

  if (props.error.user_agent_user_agent_minor)
    version += "." + props.error.user_agent_user_agent_minor;

  if (props.error.user_agent_user_agent_patch)
    version += "." + props.error.user_agent_user_agent_patch;

  return version;
});

const getTags = computed(() => {
  return {
    ip: props.error.ip,
    url: props.error.view_url,
    handled: props.error.error_handling === "handled",
    location: getLocation(),
    service: props.error.service || "Unknown",
    source: props.error.source || "Unknown",
    device: getDevice(),
    browser: props.error.user_agent_user_agent_family,
    level: "error",
    sdk_version: props.error.sdk_version,
    user_email: props.error.usr_email || "Unknown User",
  };
});

const getDevice = () => {
  if (
    !props.error.user_agent_device_brand &&
    !props.error.user_agent_device_family
  )
    return "Unknown";

  if (!props.error.user_agent_device_brand)
    return props.error.user_agent_device_family;

  if (!props.error.user_agent_device_family)
    return props.error.user_agent_device_brand;

  return (
    props.error.user_agent_device_brand +
    " " +
    props.error.user_agent_device_family
  );
};

const getLocation = () => {
  if (!props.error.geo_info_country && !props.error.geo_info_city)
    return "Unknown";
  if (!props.error.geo_info_country) return props.error.geo_info_city;
  if (!props.error.geo_info_city) return props.error.geo_info_country;

  return props.error.geo_info_country + ", " + props.error.geo_info_city;
};
</script>

<style lang="scss" scoped>
.tags-title {
  font-size: 16px;
}

.tag-block {
  border: 1px solid #e6e7f0;
  border-radius: 4px;
}
</style>
