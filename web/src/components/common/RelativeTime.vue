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
  <span :title="formattedExactTime">{{ relativeTime }}</span>
</template>

<script>
import { timestampToTimezoneDate } from "@/utils/zincutils";
import { ref, onMounted, onBeforeUnmount, watch, computed } from "vue";
import { useStore } from "vuex";

export default {
  props: {
    timestamp: {
      type: Number,
      required: false, // Make the timestamp prop optional
      default: null,
    },
    fullTimePrefix: {
      type: String,
      required: false,
      default: "",
    },
  },
  setup(props, { root }) {
    const store = useStore();

    const relativeTime = ref("");
    let intervalId = null;

    const getBestUnit = (diffInSeconds) => {
      if (diffInSeconds < 60) return { value: diffInSeconds, unit: "second" };
      if (diffInSeconds < 3600)
        return { value: Math.floor(diffInSeconds / 60), unit: "minute" };
      if (diffInSeconds < 86400)
        return { value: Math.floor(diffInSeconds / 3600), unit: "hour" };
      if (diffInSeconds < 2592000)
        return { value: Math.floor(diffInSeconds / 86400), unit: "day" };
      if (diffInSeconds < 31536000)
        return { value: Math.floor(diffInSeconds / 2592000), unit: "month" };
      return { value: Math.floor(diffInSeconds / 31536000), unit: "year" };
    };

    const updateRelativeTime = () => {
      if (!props.timestamp) {
        relativeTime.value = "";
        return;
      }

      const now = Date.now();
      const diffInSeconds = Math.floor((now - props.timestamp) / 1000);

      const rtf = new Intl.RelativeTimeFormat("en", {
        numeric: "auto",
        style: "narrow",
      });
      const { value, unit } = getBestUnit(diffInSeconds);

      relativeTime.value = rtf.format(-value, unit);
    };

    const formattedExactTime = computed(() => {
      if (!props.timestamp) return "";

      return `${props.fullTimePrefix} ${timestampToTimezoneDate(
        props.timestamp,
        store.state.timezone,
        "yyyy-MM-dd HH:mm:ss.SSS"
      )} ${store.state.timezone}`;
    });

    onMounted(() => {
      updateRelativeTime();
      intervalId = setInterval(updateRelativeTime, 60000); // Update every minute
    });

    watch(
      () => props.timestamp,
      () => {
        updateRelativeTime(); // Update immediately if the timestamp prop changes
      }
    );

    onBeforeUnmount(() => {
      clearInterval(intervalId); // Clean up interval on unmount
    });

    return {
      relativeTime,
      formattedExactTime,
    };
  },
};
</script>
