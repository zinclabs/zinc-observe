import { useQuasar } from "quasar";
import { computed } from "vue";
import { useStore } from "vuex";
import { Notification } from "@/ts/interfaces/notification";
import { getUUID } from "@/utils/zincutils";

const useNotifications = () => {
  const quasar = useQuasar();

  const store = useStore();

  const notifications = computed(() => store.state.notifications.notifications);

  const unreadNotifications = computed(() =>
    store.state.notifications.notifications.filter((n: Notification) => !n.read)
  );

  const addNotification = (notification: Notification) => {
    const id = getUUID().replace(/-/g, "");

    store.dispatch("addNotification", {
      ...notification,
      id,
      read: notification.read || false,
      expanded: notification.expanded || false,
      time: notification.time || Date.now(),
    });
  };

  const removeNotification = (id: string) => {
    store.dispatch("removeNotification", id);
  };

  const markAsRead = (id: string) => {
    store.dispatch("markAsRead", id);
  };

  const markAllAsRead = () => {
    store.dispatch("markAllAsRead");
  };

  const expandNotification = (id: string) => {
    store.dispatch("expandNotification", id);
  };

  const showErrorNotification = (message: string) => {
    return quasar.notify({
      type: "negative",
      message: message,
      timeout: 5000,
      actions: [
        {
          icon: "close",
          color: "white",
          handler: () => {
            /* ... */
          },
        },
      ],
    });
  };

  return {
    showErrorNotification,
    addNotification,
    expandNotification,
    markAllAsRead,
    markAsRead,
    removeNotification,
    notifications,
    unreadNotifications,
  };
};

export default useNotifications;
