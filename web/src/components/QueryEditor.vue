<!-- Copyright 2023 Zinc Labs Inc.

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
    data-test="query-editor"
    class="logs-query-editor"
    ref="editorRef"
    :id="editorId"
  />
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onMounted,
  nextTick,
  type Ref,
  onDeactivated,
  onUnmounted,
  onActivated,
  watch,
  computed,
} from "vue";

import "monaco-editor/esm/vs/editor/editor.all.js";
import * as monaco from "monaco-editor/esm/vs/editor/editor.api";
import { vrlLanguageDefinition } from "@/utils/query/vrlLanguageDefinition";

import { useStore } from "vuex";
import { debounce } from "quasar";
import useLogs from "@/composables/useLogs";

export default defineComponent({
  props: {
    editorId: {
      type: String,
      default: "editor",
    },
    query: {
      type: String,
      default: "",
    },
    showAutoComplete: {
      type: Boolean,
      default: true,
    },
    keywords: {
      type: Array,
      default: () => [],
    },
    suggestions: {
      type: Array,
      default: () => [],
    },
    debounceTime: {
      type: Number,
      default: 500,
    },
    readOnly: {
      type: Boolean,
      default: false,
    },
    language: {
      type: String,
      default: "sql",
    },
  },
  emits: ["update-query", "run-query", "update:query", "focus", "blur"],
  setup(props, { emit }) {
    const store = useStore();
    const editorRef: any = ref();
    let editorObj: any = null;
    const { searchObj } = useLogs();

    let provider: Ref<monaco.IDisposable | null> = ref(null);

    const CompletionKind: any = {
      Keyword: monaco.languages.CompletionItemKind.Keyword,
      Operator: monaco.languages.CompletionItemKind.Operator,
      Text: monaco.languages.CompletionItemKind.Text,
      Value: monaco.languages.CompletionItemKind.Value,
      Method: monaco.languages.CompletionItemKind.Method,
      Function: monaco.languages.CompletionItemKind.Function,
      Constructor: monaco.languages.CompletionItemKind.Constructor,
      Field: monaco.languages.CompletionItemKind.Field,
      Variable: monaco.languages.CompletionItemKind.Variable,
    };
    const insertTextRules: any = {
      InsertAsSnippet:
        monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
      KeepWhitespace:
        monaco.languages.CompletionItemInsertTextRule.KeepWhitespace,
      None: monaco.languages.CompletionItemInsertTextRule.None,
    };

    const createDependencyProposals = (range: any) => {
      return props.keywords.map((keyword: any) => {
        const itemObj: any = {
          ...keyword,
          label: keyword["label"],
          kind: CompletionKind[keyword["kind"]],
          insertText: keyword["insertText"],
          range: range,
        };
        if (insertTextRules[keyword["insertTextRule"]]) {
          itemObj["insertTextRules"] =
            insertTextRules[keyword["insertTextRule"]];
        }
        return itemObj;
      });
    };

    const setupEditor = async () => {
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

      registerAutoCompleteProvider();

      let editorElement = document.getElementById(props.editorId);
      let retryCount = 0;
      const maxRetries = 5;

      // Retry mechanism to ensure the editor element is found
      while (!editorElement && retryCount < maxRetries) {
        await new Promise((resolve) => setTimeout(resolve, 100)); // Wait for 100ms
        editorElement = document.getElementById(props.editorId);
        retryCount++;
      }

      if (!editorElement) {
        console.error("Query Editor element not found after retries");
        return;
      }

      if (editorElement && editorElement?.hasChildNodes()) return;

      editorObj = monaco.editor.create(editorElement as HTMLElement, {
        value: props.query,
        language: props.language,
        theme: store.state.theme == "dark" ? "vs-dark" : "myCustomTheme",
        showFoldingControls: enableCodeFolding.value ? "always" : "never",
        folding: enableCodeFolding.value,
        wordWrap: "on",
        automaticLayout: true,
        lineNumbers: "on",
        lineNumbersMinChars: 0,
        overviewRulerLanes: 0,
        fixedOverflowWidgets: false,
        overviewRulerBorder: false,
        lineDecorationsWidth: 3,
        hideCursorInOverviewRuler: true,
        renderLineHighlight: "none",
        glyphMargin: false,
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
        readOnly: props.readOnly,
      });

      editorObj.onDidChangeModelContent(
        debounce((e: any) => {
          emit("update-query", e, editorObj.getValue());
          emit("update:query", editorObj.getValue());
        }, props.debounceTime),
      );

      editorObj.createContextKey("ctrlenter", true);
      editorObj.addCommand(
        monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
        function () {
          setTimeout(() => {
            emit("run-query");
          }, 300);
        },
        "ctrlenter",
      );

      editorObj.onDidFocusEditorWidget(() => {
        emit("focus");
      });

      editorObj.onDidBlurEditorWidget(() => {
        emit("blur");
      });

      window.addEventListener("click", () => {
        editorObj?.layout();
      });

      window.addEventListener("resize", async () => {
        await nextTick();
        editorObj?.layout();
        // queryEditorRef.value.resetEditorLayout();
      });
    };

    onMounted(async () => {
      provider.value?.dispose();

      if (props.language === "vrl") {
        monaco.languages.register({ id: "vrl" });

        // Register a tokens provider for the language
        monaco.languages.setMonarchTokensProvider(
          "vrl",
          vrlLanguageDefinition as any,
        );
      }

      if (props.language === "sql") {
        await import(
          "monaco-editor/esm/vs/basic-languages/sql/sql.contribution.js"
        );
      }

      if (props.language === "json") {
        await import(
          "monaco-editor/esm/vs/language/json/monaco.contribution.js"
        );
      }

      if (props.language === "html") {
        await import(
          "monaco-editor/esm/vs/language/html/monaco.contribution.js"
        );
      }

      if (props.language === "markdown") {
        await import(
          "monaco-editor/esm/vs/basic-languages/markdown/markdown.contribution.js"
        );
      }

      setupEditor();
    });

    onActivated(async () => {
      provider.value?.dispose();
      registerAutoCompleteProvider();

      if (!editorObj) {
        setupEditor();
        editorObj?.layout();
      }
    });

    onDeactivated(() => {
      provider.value?.dispose();
    });

    onUnmounted(() => {
      provider.value?.dispose();
    });

    const enableCodeFolding = computed(() => {
      return ["json", "html"].includes(props.language);
    });

    // update readonly when prop value changes
    watch(
      () => props.readOnly,
      () => {
        editorObj.updateOptions({ readOnly: props.readOnly });
      },
    );

    watch(
      () => store.state.theme,
      () => {
        monaco.editor.setTheme(
          store.state.theme == "dark" ? "vs-dark" : "myCustomTheme",
        );
      },
    );

    // update readonly when prop value changes
    watch(
      () => props.query,
      () => {
        if (props.readOnly || !editorObj?.hasWidgetFocus()) {
          editorObj?.getModel().setValue(props.query);
        }
      },
    );

    const setValue = (value: string) => {
      if (editorObj?.setValue) {
        editorObj.setValue(value);
        editorObj?.layout();
      }
    };

    const registerAutoCompleteProvider = () => {
      if (!props.showAutoComplete) return;
      provider.value = monaco.languages.registerCompletionItemProvider(
        props.language,
        {
          provideCompletionItems: function (model, position) {
            // find out if we are completing a property in the 'dependencies' object.
            var textUntilPosition = model.getValueInRange({
              startLineNumber: 1,
              startColumn: 1,
              endLineNumber: position.lineNumber,
              endColumn: position.column,
            });

            var word = model.getWordUntilPosition(position);
            var range = {
              startLineNumber: position.lineNumber,
              endLineNumber: position.lineNumber,
              startColumn: word.startColumn,
              endColumn: word.endColumn,
            };

            let arr = textUntilPosition.trim().split(" ");
            let filteredSuggestions = [];
            filteredSuggestions = createDependencyProposals(range);
            filteredSuggestions = filteredSuggestions.filter((item) => {
              return item.label.toLowerCase().includes(word.word.toLowerCase());
            });

            const lastElement = arr.pop();
            props.suggestions.forEach((suggestion: any) => {
              filteredSuggestions.push({
                label: suggestion.label(lastElement),
                kind: monaco.languages.CompletionItemKind[
                  suggestion.kind || "Text"
                ],
                insertText: suggestion.insertText(lastElement),
                range: range,
              });
            });

            return {
              suggestions: filteredSuggestions,
            };
          },
        },
      );
    };

    const resetEditorLayout = () => {
      editorObj?.layout();
    };

    const triggerAutoComplete = async (value: string) => {
      disableSuggestionPopup();
      await nextTick();
      editorObj.trigger(value, "editor.action.triggerSuggest", {});
    };

    const disableSuggestionPopup = () => {
      const escEvent = new KeyboardEvent("keydown", {
        keyCode: 27,
        code: "Escape",
        key: "Escape",
        bubbles: true,
      });
      editorRef.value.dispatchEvent(escEvent);
    };

    const formatDocument = () => {
      editorObj?.getAction("editor.action.formatDocument")?.run();
    };

    const getCursorIndex = () => {
      const currentPosition = editorObj.getPosition();
      const cursorIndex =
        editorObj?.getModel().getOffsetAt(currentPosition) - 1;
      return cursorIndex || null;
    };

    return {
      editorRef,
      editorObj,
      setValue,
      resetEditorLayout,
      disableSuggestionPopup,
      triggerAutoComplete,
      getCursorIndex,
      searchObj,
      formatDocument,
    };
  },
});
</script>

<style scoped>
#editor {
  width: 100%;
  height: 78%;
  border-radius: 5px;
}
</style>

<style lang="scss">
.logs-query-editor {
  .monaco-editor,
  .monaco-editor .monaco-editor {
    padding: 0px 0px 0px 0px !important;

    .editor-widget .suggest-widget {
      z-index: 9999;
      display: flex !important;
      visibility: visible !important;
    }
  }
}
</style>
