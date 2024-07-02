<template>
  <div
    class="fixed top-0 right-0 mt-4 mr-4 w-96 bg-white shadow-lg rounded-lg overflow-hidden"
  >
    <div class="flex justify-between items-center p-4 bg-gray-100 border-b">
      <h2 class="text-lg font-semibold">Notifications</h2>
      <button @click="markAllAsRead" class="text-sm text-blue-500">
        Mark all as read
      </button>
    </div>
    <div class="p-4 space-y-4">
      <div
        v-for="notification in visibleNotifications"
        :key="notification.id"
        class="p-4 rounded-lg border shadow-sm"
      >
        <div class="flex items-center justify-between">
          <div>
            <h3 class="font-bold">{{ notification.title }}</h3>
            <p class="text-sm text-gray-500">{{ notification.time }}</p>
          </div>
          <button
            @click="removeNotification(notification.id)"
            class="text-red-500"
          >
            ✖
          </button>
        </div>
        <p class="mt-2">{{ notification.message }}</p>
        <div v-if="notification.expanded" class="mt-2 text-sm text-gray-600">
          {{ notification.details }}
        </div>
        <div class="flex justify-end mt-2 space-x-2">
          <button
            @click="markAsRead(notification.id)"
            :class="{
              'text-gray-500': notification.read,
              'text-green-500': !notification.read,
            }"
          >
            ✔
          </button>
          <button
            @click="expandNotification(notification.id)"
            class="text-yellow-500"
          >
            🔍
          </button>
        </div>
      </div>
      <button
        v-if="showMoreButton"
        @click="showMore"
        class="w-full p-2 text-center text-blue-500"
      >
        Show more
      </button>
    </div>
  </div>
</template>

<script>
import { ref, computed } from "vue";
import { useNotifications } from "@/composables/useNotifications";

export default {
  setup() {
    const {
      notifications,
      removeNotification,
      markAsRead,
      markAllAsRead,
      expandNotification,
    } = useNotifications();
    const notificationsToShow = ref(3); // Number of notifications to show initially

    const visibleNotifications = computed(() =>
      notifications.value.slice(0, notificationsToShow.value)
    );
    const showMoreButton = computed(
      () => notifications.value.length > notificationsToShow.value
    );

    const showMore = () => {
      notificationsToShow.value += 3; // Show 3 more notifications on each click
    };

    return {
      notifications: visibleNotifications,
      showMoreButton,
      removeNotification,
      markAsRead,
      markAllAsRead,
      expandNotification,
      showMore,
    };
  },
};
</script>

<style scoped>
/* Add your styles here if needed */
</style>
