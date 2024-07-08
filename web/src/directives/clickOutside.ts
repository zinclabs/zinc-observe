// src/directives/clickOutside.ts
import { DirectiveBinding } from "vue";

export default {
  beforeMount(
    el: HTMLElement & { clickOutsideEvent?: (event: Event) => void },
    binding: DirectiveBinding
  ) {
    el.clickOutsideEvent = function (event: Event) {
      console.log(binding);
      // Check if the click was outside the element
      if (!(el === event.target || el.contains(event.target as Node))) {
        // Call the method provided in the binding value
        binding.value(event);
      }
    };
    document.body.addEventListener("click", el.clickOutsideEvent);
  },
  unmounted(el: HTMLElement & { clickOutsideEvent?: (event: Event) => void }) {
    document.body.removeEventListener(
      "click",
      el.clickOutsideEvent as EventListener
    );
  },
};
