import { ref } from "vue";
import useNotifications from "@/composables/useNotifications";
import queryService from "../../services/search";
import { useStore } from "vuex";

/**
 * Provides a composable to cancel running queries.
 *
 * @returns {{
 *   traceIdRef: Ref<any[]>,
 *   searchRequestTraceIds: (data: any) => void,
 *   cancelQuery: () => void,
 * }}
 */
const useCancelQuery = () => {
  const { showPositiveNotification, showErrorNotification } =
    useNotifications();
  const traceIdRef: any = ref([]);
  const store = useStore();

  /**
   * Sets the trace IDs of the running queries to cancel.
   *
   * @param {any} data trace IDs of the running queries
   */
  const searchRequestTraceIds = (data: any) => {
    traceIdRef.value = Array.isArray(data) ? data : [data];
  };

  /**
   * Cancels the running queries with trace IDs in `traceIdRef.value`.
   */
  const cancelQuery = () => {
    window.dispatchEvent(new Event("cancelQuery"));

    const traceIdArray = Array.isArray(traceIdRef.value)
      ? traceIdRef.value
      : [];

    if (traceIdArray.length === 0) {
      return;
    }

    const tracesIdsCopy = [...traceIdArray];

    queryService
      .delete_running_queries(
        store.state.selectedOrganization.identifier,
        traceIdArray,
      )
      .then((res) => {
        const isCancelled = res.data.some((item: any) => item.is_success);

        if (isCancelled) {
          showPositiveNotification("Running query canceled successfully", {
            timeout: 3000,
          });
        }
      })
      .catch((error) => {
        console.error("delete running queries error", error);
        showErrorNotification(
          error.response?.data?.message || "Failed to cancel running query",
          { timeout: 3000 },
        );
      })
      .finally(() => {
        traceIdRef.value = traceIdRef.value.filter(
          (id: any) => !tracesIdsCopy.includes(id),
        );
      });
  };

  return {
    traceIdRef,
    searchRequestTraceIds,
    cancelQuery,
  };
};


export default useCancelQuery;
