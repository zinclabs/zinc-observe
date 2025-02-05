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
    data-test="dashboard-panel-query-editor-div"
    class="dashboard-query-editor"
    ref="editorRef"
    id="editor"
  ></div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onMounted,
  onUpdated,
  watch,
  onActivated,
  onUnmounted,
  onDeactivated,
  type Ref,
  nextTick,
} from "vue";

import "monaco-editor/esm/vs/editor/editor.all.js";
import "monaco-editor/esm/vs/basic-languages/sql/sql.contribution.js";
import "monaco-editor/esm/vs/basic-languages/sql/sql.js";
import * as monaco from "monaco-editor/esm/vs/editor/editor.api";

import { useStore } from "vuex";

export default defineComponent({
  props: {
    query: {
      type: String,
      default: "",
    },
    fields: {
      type: Array,
      default: [],
    },
    keywords: {
      type: Array,
      default: () => [],
    },
    suggestions: {
      type: Array,
      default: () => [],
    },
    functions: {
      type: Array,
      default: [],
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
  emits: ["update-query", "run-query", "update:query"],
  setup(props, { emit }) {
    const store = useStore();
    const editorRef: any = ref();
    let editorObj: any = null;
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
      let keywords: any[] = [];
      if (props.language === "sql") {
        keywords = [
          {
            label: "and",
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: "and ",
            range: range,
          },
          {
            label: "or",
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: "or ",
            range: range,
          },
          {
            label: "like",
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: "like '%${1:params}%' ",
            range: range,
          },
          {
            label: "in",
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: "in ('${1:params}') ",
            range: range,
            insertTextRules:
              monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          },
          {
            label: "not in",
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: "not in ('${1:params}') ",
            range: range,
            insertTextRules:
              monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          },
          {
            label: "between",
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: "between('${1:params}','${1:params}') ",
            range: range,
            insertTextRules:
              monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          },
          {
            label: "not between",
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: "not between('${1:params}','${1:params}') ",
            range: range,
            insertTextRules:
              monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          },
          {
            label: "is null",
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: "is null ",
            range: range,
          },
          {
            label: "is not null",
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: "is not null ",
            range: range,
          },
          {
            label: ">",
            kind: monaco.languages.CompletionItemKind.Operator,
            insertText: "> ",
            range: range,
          },
          {
            label: "<",
            kind: monaco.languages.CompletionItemKind.Operator,
            insertText: "< ",
            range: range,
          },
          {
            label: ">=",
            kind: monaco.languages.CompletionItemKind.Operator,
            insertText: ">= ",
            range: range,
          },
          {
            label: "<=",
            kind: monaco.languages.CompletionItemKind.Operator,
            insertText: "<= ",
            range: range,
          },
          {
            label: "<>",
            kind: monaco.languages.CompletionItemKind.Operator,
            insertText: "<> ",
            range: range,
          },
          {
            label: "=",
            kind: monaco.languages.CompletionItemKind.Operator,
            insertText: "= ",
            range: range,
          },
          {
            label: "!=",
            kind: monaco.languages.CompletionItemKind.Operator,
            insertText: "!= ",
            range: range,
          },
          {
            label: "()",
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: "(${1:condition}) ",
            range: range,
            insertTextRules:
              monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          },
        ];

        props.fields.forEach((field: any) => {
          if (field.name == store.state.zoConfig.timestamp_column) {
            return;
          }
          let itemObj = {
            label: field.name,
            kind: monaco.languages.CompletionItemKind.Text,
            insertText: field.name,
            range: range,
            insertTextRules:
              monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          };
          keywords.push(itemObj);
        });

        props.functions.forEach((field: any) => {
          let itemObj = {
            label: field.name,
            kind: monaco.languages.CompletionItemKind.Text,
            insertText: field.name + field.args,
            range: range,
            insertTextRules:
              monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          };
          keywords.push(itemObj);
        });
      } else {
        props.keywords.forEach((keyword: any) => {
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
          keywords.push(itemObj);
        });
      }

      return keywords;
    };

    watch(
      () => store.state.theme,
      () => {
        monaco.editor.setTheme(
          store.state.theme == "dark" ? "vs-dark" : "myCustomTheme",
        );
      },
    );

    onMounted(async () => {
      //   editorRef.value.addEventListener("keyup", onKeyUp);

      monaco.editor.defineTheme("myCustomTheme", {
        base: "vs", // can also be vs-dark or hc-black
        inherit: true, // can also be false to completely replace the builtin rules
        rules: [{ token: "comment", background: "FF0000" }],
        colors: {
          "editor.foreground": "#000000",
          "editor.background": "#fafafa",
          "editorCursor.foreground": "#000000",
          "editor.lineHighlightBackground": "#FFFFFF",
          "editorLineNumber.foreground": "#000000",
          "editor.border": "#ececec",
        },
      });

      // Dispose the provider if it already exists before registering a new one
      provider.value?.dispose();
      registerAutoCompleteProvider();

      editorObj = monaco.editor.create(editorRef.value, {
        value: props.query?.trim(),
        language: "sql",
        theme: store.state.theme == "dark" ? "vs-dark" : "myCustomTheme",
        showFoldingControls: "never",
        wordWrap: "on",
        lineNumbers: "on",
        lineNumbersMinChars: 0,
        overviewRulerLanes: 0,
        fixedOverflowWidgets: true,
        overviewRulerBorder: false,
        lineDecorationsWidth: 15,
        hideCursorInOverviewRuler: true,
        renderLineHighlight: "none",
        glyphMargin: false,
        folding: false,
        scrollBeyondLastColumn: 0,
        scrollBeyondLastLine: true,
        scrollbar: { horizontal: "auto", vertical: "visible" },
        find: {
          addExtraSpaceOnTop: false,
          autoFindInSelection: "never",
          seedSearchStringFromSelection: "never",
        },
        minimap: { enabled: false },
        readOnly: props.readOnly,
      });

      editorObj.onDidChangeModelContent((e: any) => {
        emit("update-query", e, editorObj.getValue()?.trim());
        emit("update:query", editorObj.getValue()?.trim());
      });

      editorObj.onDidBlurEditorWidget(() => {
        setValue(editorObj.getValue()?.trim());
      });

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
    });

    onMounted(async () => {
      provider.value?.dispose();
      registerAutoCompleteProvider();
      window.addEventListener("resize", async () => {
        await nextTick();
        editorObj.layout();
        // queryEditorRef.value.resetEditorLayout();
      });
    });

    onActivated(async () => {
      provider.value?.dispose();
      registerAutoCompleteProvider();
    });

    onUnmounted(() => {
      provider.value?.dispose();
    });

    onDeactivated(() => {
      provider.value?.dispose();
    });

    const registerAutoCompleteProvider = () => {
      provider.value = monaco.languages.registerCompletionItemProvider("sql", {
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
          filteredSuggestions = filteredSuggestions.filter((item: any) => {
            return item.label.toLowerCase().includes(word.word.toLowerCase());
          });

          // if (filteredSuggestions.length == 0) {
          const lastElement = arr.pop();

          if (props.language == "sql") {
            filteredSuggestions.push({
              label: `match_all('${lastElement}')`,
              kind: monaco.languages.CompletionItemKind.Text,
              insertText: `match_all('${lastElement}')`,
              range: range,
            });
            filteredSuggestions.push({
              label: `match_all_raw('${lastElement}')`,
              kind: monaco.languages.CompletionItemKind.Text,
              insertText: `match_all_raw('${lastElement}')`,
              range: range,
            });
            filteredSuggestions.push({
              label: `match_all_raw_ignore_case('${lastElement}')`,
              kind: monaco.languages.CompletionItemKind.Text,
              insertText: `match_all_raw_ignore_case('${lastElement}')`,
              range: range,
            });
            filteredSuggestions.push({
              label: `re_match(fieldname: string, regular_expression: string)`,
              kind: monaco.languages.CompletionItemKind.Text,
              insertText: `re_match(fieldname, '')`,
              range: range,
            });
            filteredSuggestions.push({
              label: `re_not_match(fieldname: string, regular_expression: string)`,
              kind: monaco.languages.CompletionItemKind.Text,
              insertText: `re_not_match(fieldname, '')`,
              range: range,
            });
            filteredSuggestions.push({
              label: `str_match_ignore_case(fieldname, '${lastElement}')`,
              kind: monaco.languages.CompletionItemKind.Text,
              insertText: `str_match_ignore_case(fieldname, '${lastElement}')`,
              range: range,
            });
            filteredSuggestions.push({
              label: `str_match(fieldname, '${lastElement}')`,
              kind: monaco.languages.CompletionItemKind.Text,
              insertText: `str_match(fieldname, '${lastElement}')`,
              range: range,
            });
            filteredSuggestions.push({
              label: `fuzzy_match(fieldname, '${lastElement}', 1)`,
              kind: monaco.languages.CompletionItemKind.Function,
              insertText: `fuzzy_match(fieldname, '${lastElement}', 1)`,
              range: range,
            });
            filteredSuggestions.push({
              label: `fuzzy_match_all('${lastElement}', 1)`,
              kind: monaco.languages.CompletionItemKind.Function,
              insertText: `fuzzy_match_all('${lastElement}', 1)`,
              range: range,
            });
          } else {
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
          }

          return {
            suggestions: filteredSuggestions,
          };
        },
      });
    };

    const setValue = (value: string) => {
      editorObj.setValue(value);
    };

    // update readonly when prop value changes
    watch(
      () => props.readOnly,
      () => {
        editorObj.updateOptions({ readOnly: props.readOnly });
      },
    );

    // update readonly when prop value changes
    watch(
      () => props.query,
      () => {
        if (props.readOnly || !editorObj.hasWidgetFocus()) {
          editorObj.getModel().setValue(props.query);
        }
      },
    );

    const resetEditorLayout = () => {
      editorObj.layout();
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

    const getCursorIndex = () => {
      const currentPosition = editorObj.getPosition();
      const cursorIndex = editorObj.getModel().getOffsetAt(currentPosition) - 1;
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
    };
  },
});
</script>

<style>
#editor {
  height: 100%;
  width: 100%;
  /* min-height: 4rem; */
  border-radius: 5px;
  border: 0px solid #dbdbdb;
}

.monaco-editor,
.monaco-editor .monaco-editor {
  padding: 0px 0px 0px 0px !important;
}
</style>

<style lang="scss">
.dashboard-query-editor {
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
