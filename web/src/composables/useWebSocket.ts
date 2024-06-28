// useWebSocket.ts

import { onUnmounted } from "vue";

type MessageHandler = (event: MessageEvent) => void;
type OpenHandler = (event: Event) => void;
type CloseHandler = (event: CloseEvent) => void;
type ErrorHandler = (event: Event) => void;

let socket: WebSocket | null = null;
let reconnectInterval: number;
let maxReconnectAttempts: number;
let reconnectAttempts = 0;

const messageHandlers: MessageHandler[] = [];
const openHandlers: OpenHandler[] = [];
const closeHandlers: CloseHandler[] = [];
const errorHandlers: ErrorHandler[] = [];

const connect = (url: string, interval: number, maxAttempts: number) => {
  if (socket) return;

  reconnectInterval = interval;
  maxReconnectAttempts = maxAttempts;

  socket = new WebSocket(url);
  console.log("WebSocket is connecting now.", socket);
  socket.addEventListener("open", onOpen);
  socket.addEventListener("message", onMessage);
  socket.addEventListener("close", onClose);
  socket.addEventListener("error", onError);
};

const onOpen = (event: Event) => {
  console.log("WebSocket is open now.");
  reconnectAttempts = 0;
  openHandlers.forEach((handler) => handler(event));
};

const onMessage = (event: MessageEvent) => {
  // ws_types.rs refer for the MessageEvent type
  console.log("Message from server:", event.data);
  messageHandlers.forEach((handler) => handler(event));
};

const onClose = (event: CloseEvent) => {
  console.log("WebSocket is closed now.");
  closeHandlers.forEach((handler) => handler(event));
  if (reconnectAttempts < maxReconnectAttempts) {
    setTimeout(() => {
      reconnectAttempts++;
      connect(socket!.url, reconnectInterval, maxReconnectAttempts);
    }, reconnectInterval);
  }
};

const onError = (event: Event) => {
  console.error("WebSocket error observed:", event);
  errorHandlers.forEach((handler) => handler(event));
};

const sendMessage = (message: string) => {
  console.log("Sending message to server:", message);
  console.log("WebSocket ready state:", socket, socket?.readyState);
  if (socket && socket.readyState === WebSocket.OPEN) {
    socket.send(message);
  } else {
    console.error("WebSocket is not open. Ready state is:", socket?.readyState);
  }
};

const addMessageHandler = (handler: MessageHandler) => {
  messageHandlers.push(handler);
};

const removeMessageHandler = (handler: MessageHandler) => {
  const index = messageHandlers.indexOf(handler);
  if (index > -1) {
    messageHandlers.splice(index, 1);
  }
};

const addOpenHandler = (handler: OpenHandler) => {
  openHandlers.push(handler);
};

const removeOpenHandler = (handler: OpenHandler) => {
  const index = openHandlers.indexOf(handler);
  if (index > -1) {
    openHandlers.splice(index, 1);
  }
};

const addCloseHandler = (handler: CloseHandler) => {
  closeHandlers.push(handler);
};

const removeCloseHandler = (handler: CloseHandler) => {
  const index = closeHandlers.indexOf(handler);
  if (index > -1) {
    closeHandlers.splice(index, 1);
  }
};

const addErrorHandler = (handler: ErrorHandler) => {
  errorHandlers.push(handler);
};

const removeErrorHandler = (handler: ErrorHandler) => {
  const index = errorHandlers.indexOf(handler);
  if (index > -1) {
    errorHandlers.splice(index, 1);
  }
};

const useWebSocket = (
  url: string,
  interval: number = 5000,
  maxAttempts: number = 10
) => {
  if (!socket) {
    connect(url, interval, maxAttempts);
  }

  onUnmounted(() => {
    if (socket) {
      socket.close();
      socket = null;
    }
  });

  return {
    sendMessage,
    addMessageHandler,
    removeMessageHandler,
    addOpenHandler,
    removeOpenHandler,
    addCloseHandler,
    removeCloseHandler,
    addErrorHandler,
    removeErrorHandler,
  };
};

export default useWebSocket;
