import { ref } from "vue";

import useEventBus from "@/composables/useEventBus";

const useFieldList = () => {
  const { on, off, emit } = useEventBus();

  const fields = ref([]);

  const data: any = ref({});

  const fieldValues = ref({});

  return {
    fields,
    data,
    fieldValues,
    addFieldListEvent: on,
    removeFieldListEvent: off,
    emitFieldListEvent: emit,
  };
};

export default useFieldList;
