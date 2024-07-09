<template>
  <div
    class="tw-mr-4 tw-w-96 tw-bg-white tw-shadow-lg tw-rounded-lg tw-overflow-hidden"
    v-click-outside="closeNotifications"
  >
    <div class="tw-flex tw-justify-between tw-items-center tw-p-2 tw-border-b">
      <p class="tw-text-lg tw-font-semibold">Notifications</p>

      <button @click="closeNotifications" class="tw-text-grey-500">
        <q-icon size="20px" :name="outlinedCancel" />
      </button>
    </div>
    <div class="tw-flex tw-justify-between tw-px-4 tw-py-2">
      <div class="notifications-tabs">
        <AppTabs
          :show="true"
          v-model:active-tab="activeTab"
          :tabs="[
            {
              label: 'All',
              value: 'all',
            },
            {
              label: 'Unread',
              value: 'unread',
            },
          ]"
          class="tw-text-sm"
        />
      </div>
      <button
        :disabled="!visibleNotifications.length"
        @click="markAllAsRead"
        class="tw-text-sm tw-text-primary"
      >
        Mark all as read
      </button>
    </div>
    <div
      class="notifications-list tw-pb-0 tw-px-3 tw-space-y-3 tw-overflow-y-auto"
    >
      <template v-if="!visibleNotifications.length">
        <p class="tw-text-center tw-text-gray-500 tw-mt-4 tw-text-[14px]">
          No notifications to show
        </p>
      </template>
      <template v-else>
        <div
          v-for="notification in visibleNotifications"
          :key="notification.id"
          class="tw-px-3 tw-py-2 tw-rounded-lg tw-border tw-shadow-sm"
        >
          <div>
            <div class="tw-flex tw-items-center tw-justify-between">
              <p class="tw-font-bold">{{ notification.title }}</p>

              <div class="tw-flex tw-items-center">
                <div
                  v-if="!notification.read"
                  class="tw-mr-2 tw-text-[11px] tw-w-[8px] tw-h-[8px] tw-bg-primary tw-rounded-full"
                />
                <button
                  @click.stop="removeNotification(notification.id)"
                  class="tw-text-gray-500"
                >
                  <q-icon size="16px" :name="outlinedCancel" />
                </button>
              </div>
            </div>
            <p class="tw-text-[12px] tw-text-gray-500">
              {{
                timestampToTimezoneDate(
                  notification.time,
                  useLocalTimezone(),
                  "dd MMMM    hh:mm a"
                )
              }}
            </p>
          </div>
          <p class="tw-mt-1 tw-text-[13px] tw-text-gray-700">
            {{ notification.message }}
          </p>

          <div class="tw-flex tw-justify-between tw-mt-1 tw-space-x-2">
            <button
              @click="expandNotification(notification.id)"
              class="tw-text-gray-600"
            >
              Show Details
            </button>
            <button
              @click="markAsRead(notification.id)"
              class="tw-text-gray-600"
            >
              Mark as read
            </button>
          </div>
        </div>
      </template>
    </div>
    <div class="tw-flex tw-justify-between tw-py-3 tw-px-3 tw-pl-4">
      <button
        :disabled="notificationsToShow >= totalNotifications"
        @click="showMore"
        class="tw-text-primary"
        L
      >
        Show more
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import useNotifications from "@/composables/useNotifications";
import { Notification } from "@/types/notification";
import { outlinedCancel } from "@quasar/extras/material-icons-outlined";
import { useStore } from "vuex";
import AppTabs from "@/components/common/AppTabs.vue";
import { timestampToTimezoneDate, useLocalTimezone } from "@/utils/zincutils";

const {
  notifications,
  removeNotification,
  markAsRead,
  markAllAsRead,
  expandNotification,
  unreadNotifications,
} = useNotifications();
const notificationsToShow = ref(3); // Number of notifications to show initially
const store = useStore();
const activeTab = ref("all");

const visibleNotifications = computed(() => {
  if (activeTab.value === "unread") {
    return unreadNotifications.value.slice(0, notificationsToShow.value);
  }

  return notifications.value.slice(0, notificationsToShow.value);
});

const totalNotifications = computed(() => {
  if (activeTab.value === "unread") {
    return unreadNotifications.value.length;
  }

  return notifications.value.length;
});

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

<style scoped lang="scss">
/* Add your styles here if needed */
.notifications-list {
  max-height: calc(83vh - 81px);
}
</style>
<style lang="scss">
.notifications-tabs {
  .rum-tabs:first-child .rum-tab {
    width: 34px !important;
  }

  .rum-tabs div:nth-child(2) {
    .rum-tab {
      width: 56px !important;
    }
  }

  .rum-tab {
    width: 60px !important;
    font-size: 12px !important;
    padding: 0px 4px !important;
    border: none !important;
    border-radius: 2px !important;

    &.active {
      background: $primary !important;
      color: #ffffff !important;
    }
  }
}
</style>
