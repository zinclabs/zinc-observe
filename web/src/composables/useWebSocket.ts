// useWebSocket.ts

import { onUnmounted } from "vue";

type MessageHandler = (event: MessageEvent) => void;
type OpenHandler = (event: Event) => void;
type CloseHandler = (event: CloseEvent) => void;
type ErrorHandler = (event: Event) => void;

// Store WebSocket instances and their handlers by socketId
const sockets: Record<string, WebSocket> = {};
const messageHandlers: Record<string, MessageHandler[]> = {};
const openHandlers: Record<string, OpenHandler[]> = {};
const closeHandlers: Record<string, CloseHandler[]> = {};
const errorHandlers: Record<string, ErrorHandler[]> = {};

const pingIntervals: Record<string, any> = {};
const pingTimeout: number = 5000;

const connect = (
  socketId: string,
  url: string,
  interval: number = 5000,
  maxAttempts: number = 10,
) => {
  if (sockets[socketId]) return; // Prevent duplicate connections

  const createSocket = () => {
    const socket = new WebSocket(url);
    sockets[socketId] = socket;

    socket.addEventListener("open", (event) => onOpen(socketId, event));
    socket.addEventListener("message", (event) => onMessage(socketId, event));
    socket.addEventListener("close", (event) =>
      onClose(socketId, event, interval, maxAttempts, createSocket),
    );
  };

  createSocket();
};

const onOpen = (socketId: string, event: Event) => {
  openHandlers[socketId]?.forEach((handler) => handler(event));

  // Start ping interval
  // pingIntervals[socketId] = setInterval(() => {
  //   if (sockets[socketId]?.readyState === WebSocket.OPEN) {
  //     sockets[socketId].send(JSON.stringify({ type: "ping", message: "ping" }));
  //   }
  // }, pingTimeout);
};

const onMessage = (socketId: string, event: MessageEvent) => {
  const data = JSON.parse(event.data);

  if (data.type === "error") {
    errorHandlers[socketId]?.forEach((handler) => handler(data));
    return;
  }

  messageHandlers[socketId]?.forEach((handler) => handler(data));
};

const onClose = (
  socketId: string,
  event: CloseEvent,
  interval: number,
  maxAttempts: number,
  reconnect: () => void,
) => {
  clearInterval(pingIntervals[socketId]);
  delete pingIntervals[socketId];
  delete sockets[socketId];

  closeHandlers[socketId]?.forEach((handler) => handler(event));

  // Reconnect, if socket got closed unexpectedly
  // if (reconnectAttempts < maxAttempts) {
  //   setTimeout(() => {
  //     reconnectAttempts++;
  //     reconnect();
  //   }, interval);
  // }
};

const onError = (socketId: string, event: Event) => {
  errorHandlers[socketId]?.forEach((handler) => handler(event));
};

// Message and handler functions
const sendMessage = (socketId: string, message: string) => {
  const socket = sockets[socketId];
  if (socket && socket.readyState === WebSocket.OPEN) {
    socket.send(message);
  } else {
    console.error(
      `WebSocket ${socketId} is not open. Ready state is:`,
      socket?.readyState,
    );
  }
};

const addHandler = (
  handlersMap: Record<string, any[]>,
  socketId: string,
  handler: any,
) => {
  if (!handlersMap[socketId]) handlersMap[socketId] = [];
  handlersMap[socketId].push(handler);
};

const removeHandler = (
  handlersMap: Record<string, any[]>,
  socketId: string,
  handler: any,
) => {
  const handlers = handlersMap[socketId];
  if (handlers) {
    const index = handlers.indexOf(handler);
    if (index > -1) handlers.splice(index, 1);
  }
};

// Composable
const useWebSocket = () => {
  onUnmounted(() => {
    for (const socketId in sockets) {
      const socket = sockets[socketId];
      if (socket) {
        socket.close();
        clearInterval(pingIntervals[socketId]);
        delete sockets[socketId];
        delete pingIntervals[socketId];
      }
    }
  });

  const getWebSocketBasedOnSocketId = (socketId: string) => {
    return sockets[socketId];
  };

  return {
    connect,
    sendMessage,
    getWebSocketBasedOnSocketId,
    addMessageHandler: (socketId: string, handler: MessageHandler) =>
      addHandler(messageHandlers, socketId, handler),
    removeMessageHandler: (socketId: string, handler: MessageHandler) =>
      removeHandler(messageHandlers, socketId, handler),
    addOpenHandler: (socketId: string, handler: OpenHandler) =>
      addHandler(openHandlers, socketId, handler),
    removeOpenHandler: (socketId: string, handler: OpenHandler) =>
      removeHandler(openHandlers, socketId, handler),
    addCloseHandler: (socketId: string, handler: CloseHandler) =>
      addHandler(closeHandlers, socketId, handler),
    removeCloseHandler: (socketId: string, handler: CloseHandler) =>
      removeHandler(closeHandlers, socketId, handler),
    addErrorHandler: (socketId: string, handler: ErrorHandler) =>
      addHandler(errorHandlers, socketId, handler),
    removeErrorHandler: (socketId: string, handler: ErrorHandler) =>
      removeHandler(errorHandlers, socketId, handler),
  };
};

export default useWebSocket;
