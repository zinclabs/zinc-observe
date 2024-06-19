// useEventBus.js
// Use event bus only for generic components to avoid tight coupling
// Do not use event bus for parent-child communication

// useEventBus.js
import { reactive } from "vue";

export function useEventBus() {
  const events = reactive<{ [key: string]: Function }>({});

  function on(event: string, listener: Function) {
    events[event] = listener; // Override any existing listener for the event
  }

  function off(event: string) {
    delete events[event];
  }

  function emit(event: string, ...args: any[]) {
    if (!events[event]) return;

    events[event](...args); // Call the single listener
  }

  return {
    on,
    off,
    emit,
  };
}
