import { onMounted, onUnmounted } from "vue";

type IntersectionObserverCallback = (element: Element) => void;

function useIntersectionObserver(callback: IntersectionObserverCallback) {
  let observer: IntersectionObserver | undefined;

  const observe = (element: Element) => {
    if (observer) {
      observer.observe(element);
    }
  };

  const unobserve = (element: Element) => {
    if (observer) {
      observer.unobserve(element);
    }
  };

  onMounted(() => {
    observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          callback(entry.target);
        }
      });
    });
  });

  onUnmounted(() => {
    if (observer) {
      observer.disconnect();
    }
  });

  return {
    observe,
    unobserve,
  };
}

export default useIntersectionObserver;
