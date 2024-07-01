// useWebSocket.ts

import { onMounted, onUnmounted } from "vue";

type MessageHandler = (event: MessageEvent) => void;
type OpenHandler = (event: Event) => void;
type CloseHandler = (event: CloseEvent) => void;
type ErrorHandler = (event: Event) => void;

let socket: WebSocket | null = null;
let reconnectInterval: number;
let maxReconnectAttempts: number;
let reconnectAttempts = 0;
let socketUrl: string = "";

const messageHandlers: MessageHandler[] = [];
const openHandlers: OpenHandler[] = [];
const closeHandlers: CloseHandler[] = [];
const errorHandlers: ErrorHandler[] = [];

let isSocketActive: boolean = false;

const connect = (url: string, interval: number, maxAttempts: number) => {
  if (socket) return;

  socketUrl = url;

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
  messageHandlers.forEach((handler) => handler(event));
};

const onClose = (event: CloseEvent) => {
  closeHandlers.forEach((handler) => handler(event));
  if (isSocketActive && reconnectAttempts < maxReconnectAttempts) {
    setTimeout(() => {
      reconnectAttempts++;
      connect(socketUrl, reconnectInterval, maxReconnectAttempts);
    }, reconnectInterval);
  }
};

const onError = (event: Event) => {
  console.error("WebSocket error observed:", event);
  errorHandlers.forEach((handler) => handler(event));
};

const sendMessage = (message: string) => {
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

const openWebSocket = (
  url: string,
  interval: number = 5000,
  maxAttempts: number = 10
) => {
  if (!socket) {
    isSocketActive = true;
    connect(url, interval, maxAttempts);
  }
};

const closeWebSocket = () => {
  if (socket) {
    isSocketActive = false;
    socket.close();
    socket = null;
  }
};

const useWebSocket = () => {
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
    openWebSocket,
    closeWebSocket,
  };
};

export default useWebSocket;
