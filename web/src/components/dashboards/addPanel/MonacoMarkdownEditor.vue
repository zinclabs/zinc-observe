<!-- Copyright 2023 OpenObserve Inc.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
-->

<template>
  <div
    data-test="markdown-editor"
    class="markdown-editor"
    ref="editorRef"
    id="markdown-editor"
    style="width: 100% !important; height: 100% !important"
  ></div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onMounted,
  type Ref,
  onDeactivated,
  onUnmounted,
  onActivated,
  watch,
} from "vue";

import "monaco-editor/esm/vs/editor/editor.all.js";
import "monaco-editor/esm/vs/basic-languages/markdown/markdown.contribution.js";
import "monaco-editor/esm/vs/basic-languages/markdown/markdown.js";
import * as monaco from "monaco-editor/esm/vs/editor/editor.api";

import { useStore } from "vuex";
import { debounce } from "quasar";

export default defineComponent({
  props: {
    debounceTime: {
      type: Number,
      default: 500,
    },
    modelValue: {
      type: String,
      required: true,
    },
  },
  emits: ["update:modelValue"],
  setup(props: any, { emit }) {
    const store = useStore();
    const editorRef: any = ref();
    let editorObj: any = null;

    let provider: Ref<monaco.IDisposable | null> = ref(null);

    onMounted(async () => {
      monaco.editor.defineTheme("myCustomTheme", {
        base: "vs", // can also be vs-dark or hc-black
        inherit: true, // can also be false to completely replace the builtin rules
        rules: [{ token: "comment", background: "FFFFFF" }],
        colors: {
          "editor.foreground": "#000000",
          "editor.background": "#fafafa",
          "editorCursor.foreground": "#000000",
          "editor.lineHighlightBackground": "#FFFFFF",
          "editorLineNumber.foreground": "#000000",
          "editor.border": "#000000",
        },
      });

      const editorElement = document.getElementById("markdown-editor");

      if (!editorElement) {
        console.error("Markdown Editor element not found");
        return;
      }
      if (editorElement && editorElement?.hasChildNodes()) return;

      editorObj = monaco.editor.create(editorElement as HTMLElement, {
        value: props.modelValue,
        language: "markdown",
        theme: store.state.theme == "dark" ? "vs-dark" : "myCustomTheme",
        showFoldingControls: "never",
        wordWrap: "on",
        lineNumbers: "on",
        lineNumbersMinChars: 0,
        overviewRulerLanes: 0,
        fixedOverflowWidgets: false,
        overviewRulerBorder: false,
        lineDecorationsWidth: 3,
        hideCursorInOverviewRuler: true,
        renderLineHighlight: "none",
        glyphMargin: false,
        folding: false,
        scrollBeyondLastColumn: 0,
        scrollBeyondLastLine: false,
        smoothScrolling: true,
        mouseWheelScrollSensitivity: 1,
        fastScrollSensitivity: 1,
        scrollbar: { horizontal: "auto", vertical: "visible" },
        find: {
          addExtraSpaceOnTop: false,
          autoFindInSelection: "never",
          seedSearchStringFromSelection: "never",
        },
        minimap: { enabled: false },
        readOnly: false,
      });

      editorObj.onDidChangeModelContent(
        debounce((e: any) => {
          emit("update:modelValue", editorObj.getValue());
        }, props.debounceTime)
      );

      onActivated(async () => {
        provider.value?.dispose();
        editorObj.layout();
      });

      onDeactivated(() => {
        provider.value?.dispose();
      });

      onUnmounted(() => {
        provider.value?.dispose();
      });

      window.addEventListener("click", () => {
        editorObj.layout();
      });

      return {
        editorRef,
      };
    });

    watch(
      () => store.state.theme,
      () => {
        monaco.editor.setTheme(
          store.state.theme == "dark" ? "vs-dark" : "myCustomTheme"
        );
      }
    );
  },
});
</script>

<style lang="scss">
.markdown-editor {
  .monaco-editor,
  .monaco-editor .monaco-editor {
    padding: 0px 0px 0px 0px !important;
    width: 100%;
    height: 100%;

    .editor-widget .suggest-widget {
      z-index: 9999;
      display: flex !important;
      visibility: visible !important;
    }
  }
}
</style>
