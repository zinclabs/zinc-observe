<template>
  <div
    v-show="store.state.notifications.isOpen"
    class="tw-fixed tw-top-[57px] tw-right-0 tw-bg-white tw-z-10 tw-max-h-[85%]"
  >
    <notifications-list />
  </div>
  Notifications
</template>

<script setup lang="ts">
import NotificationsList from "@/components/notifications/NotificationsList.vue";
import { useStore } from "vuex";
const store = useStore();
</script>

<style scoped lang="scss"></style>
