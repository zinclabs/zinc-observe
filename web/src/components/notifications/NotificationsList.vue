<template>
  <div
    class="tw-mr-4 tw-w-96 tw-bg-white tw-shadow-lg tw-rounded-lg tw-overflow-hidden"
  >
    <div class="tw-flex tw-justify-between tw-items-center tw-p-2 tw-border-b">
      <p class="tw-text-lg tw-font-semibold">Notifications</p>

      <button @click="closeNotifications" class="tw-text-grey-500">
        <q-icon size="20px" :name="outlinedCancel" />
      </button>
    </div>
    <div class="tw-flex tw-justify-end tw-px-4 tw-pt-4">
      <button @click="markAllAsRead" class="tw-text-sm tw-text-blue-500">
        Mark all as read
      </button>
    </div>
    <div class="tw-p-4 tw-space-y-4">
      <div
        v-for="notification in notifications"
        :key="notification.id"
        class="tw-p-4 tw-rounded-lg tw-border tw-shadow-sm"
      >
        <div>
          <div class="tw-flex tw-items-center tw-justify-between">
            <p class="tw-font-bold">{{ notification.title }}</p>
            <button
              @click="removeNotification(notification.id)"
              class="tw-text-red-500"
            >
              <q-icon size="16px" :name="outlinedCancel" />
            </button>
          </div>
          <p class="tw-text-[12px] tw-text-gray-500">
            {{ notification.time }}
          </p>
        </div>
        <p class="tw-mt-2 tw-text-[14px]">{{ notification.message }}</p>

        <div class="tw-flex tw-justify-between tw-mt-2 tw-space-x-2">
          <button
            @click="expandNotification(notification.id)"
            class="tw-text-yellow-500"
          >
            Show Details
          </button>
          <button
            @click="expandNotification(notification.id)"
            class="tw-text-yellow-500"
          >
            Mark as read
          </button>
        </div>
      </div>
      <div class="tw-flex tw-justify-between tw-mt-2 tw-px-2">
        <button @click="expandNotification" class="tw-text-yellow-500">
          Show more
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import useNotifications from "@/composables/useNotifications";
import { Notification } from "@/types/notification";
import { outlinedCancel } from "@quasar/extras/material-icons-outlined";
import { useStore } from "vuex";

const {
  notifications,
  removeNotification,
  markAsRead,
  markAllAsRead,
  expandNotification,
} = useNotifications();
const notificationsToShow = ref(3); // Number of notifications to show initially
const store = useStore();

const visibleNotifications = computed(() =>
  notifications.value.slice(0, notificationsToShow.value)
);
const showMoreButton = computed(
  () => notifications.value.length > notificationsToShow.value
);

const showMore = () => {
  notificationsToShow.value += 3; // Show 3 more notifications on each click
};

const closeNotifications = () => {
  store.commit("setNotificationDrawer", false);
};
</script>

<style scoped>
/* Add your styles here if needed */
</style>
