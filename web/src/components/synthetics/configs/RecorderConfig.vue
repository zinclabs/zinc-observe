<template>
  <div>
    <query-editor
      editor-id="synthetics-test-recording-editor"
      v-model:query="testScript"
      language="javascript"
      class="synthetics-test-recording-editor"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import QueryEditor from "@/components/synthetics/QueryEditor.vue";

const testScript = ref("");
</script>

<style lang="scss" scoped>
.synthetics-test-recording-editor {
  height: 60vh;
  width: 100%;
}
</style>
