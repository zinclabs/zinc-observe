import { getUUID, getWebSocketUrl } from "@/utils/zincutils";
import useWebSocket from "@/composables/useWebSocket";
import type { SearchRequestPayload } from "@/ts/interfaces";

const useSearchWebSocket = () => {
  const webSocket = useWebSocket();

  const fetchQueryDataWithWebSocket = (
    data: {
      queryReq: SearchRequestPayload;
      type: "search" | "histogram" | "pageCount";
      isPagination: boolean;
      traceId: string;
      org_id: string;
    },
    handlers: {
      open: (requestId: string, data: any, response: any) => void;
      message: (requestId: string, data: any, response: any) => void;
      close: (requestId: string, data: any, response: any) => void;
      error: (requestId: string, data: any, response: any) => void;
    },
  ) => {
    try {
      const requestId = getUUID();
      const url = getWebSocketUrl(requestId, data.org_id);

      webSocket.connect(requestId, url);

      // Gets called when socket connect is established
      webSocket.addOpenHandler(
        requestId,
        handlers.open.bind(null, requestId, data),
      );

      // When we receive message from BE/server
      webSocket.addMessageHandler(
        requestId,
        handlers.message.bind(null, requestId, data),
      );

      // On closing of ws, when search is completed Server closes the WS
      webSocket.addCloseHandler(
        requestId,
        handlers.close.bind(null, requestId, data),
      );

      webSocket.addErrorHandler(
        requestId,
        handlers.error.bind(null, requestId, data),
      );
      return requestId;
    } catch (error: any) {
      console.error(
        `Error in fetching search data: ${error instanceof Error ? error.message : String(error)}`,
      );
      return "";
    }
  };

  const sendSearchMessageBasedOnRequestId = (requestId: string, data: any) => {
    try {
      webSocket.sendMessage(requestId, JSON.stringify(data));
    } catch (error: any) {
      console.error(
        `Failed to send WebSocket message: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  };

  const cancelSearchQueryBasedOnRequestId = (
    requestId: string,
    trace_id: string,
  ) => {
    const socket = webSocket.getWebSocketBasedOnSocketId(requestId);
    // check state of socket
    if (socket && socket.readyState === WebSocket.OPEN) {
      webSocket.sendMessage(
        requestId,
        JSON.stringify({
          type: "cancel",
          content: {
            trace_id: trace_id,
          },
        }),
      );
    }
  };

  const closeSocketBasedOnRequestId = (requestId: string) => {
    try {
      webSocket.closeSocket(requestId);
    } catch (error: any) {
      console.error(`Failed to close socket ${requestId}:`, error);
    }
  };

  return {
    fetchQueryDataWithWebSocket,
    sendSearchMessageBasedOnRequestId,
    cancelSearchQueryBasedOnRequestId,
    closeSocketBasedOnRequestId,
  };
};

export default useSearchWebSocket;
