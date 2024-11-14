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

let messageHandlers: MessageHandler[] = [];
let openHandlers: OpenHandler[] = [];
let closeHandlers: CloseHandler[] = [];
let errorHandlers: ErrorHandler[] = [];

let pingInterval: any = null;
const pingTimeout: any = 5000;

const connect = (url: string, interval: number, maxAttempts: number) => {
  if (socket) return;

  reconnectInterval = interval;
  maxReconnectAttempts = maxAttempts;

  socket = new WebSocket(url);
  socket.addEventListener("open", onOpen);
  socket.addEventListener("message", onMessage);
  socket.addEventListener("close", onClose);
  socket.addEventListener("error", onError);
};

const onOpen = (event: Event) => {
  reconnectAttempts = 0;
  openHandlers.forEach((handler) => handler(event));

  pingInterval = setInterval(() => {
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify({ type: "ping", message: "ping" }));
    }
  }, pingTimeout);
};

const onMessage = (event: MessageEvent) => {
  // ws_types.rs refer for the MessageEvent type
  messageHandlers.forEach((handler) => handler(JSON.parse(event.data)));
};

const onClose = (event: CloseEvent) => {
  socket?.close();
  socket = null;
  pingInterval && clearInterval(pingInterval);
  closeHandlers.forEach((handler) => handler(event));
  openHandlers = [];
  errorHandlers = [];
  messageHandlers = [];
  closeHandlers = [];
  // if (reconnectAttempts < maxReconnectAttempts) {
  //   setTimeout(() => {
  //     reconnectAttempts++;
  //     connect(socket!.url, reconnectInterval, maxReconnectAttempts);
  //   }, reconnectInterval);
  // }
};

const onError = (event: Event) => {
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

const useWebSocket = (
  url?: string,
  interval: number = 5000,
  maxAttempts: number = 10,
) => {
  if (!socket && url) {
    connect(url, interval, maxAttempts);
  }

  onUnmounted(() => {
    if (socket) {
      socket.close();
      socket = null;
    }

    pingInterval && clearInterval(pingInterval);
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
    connect,
  };
};

export default useWebSocket;
