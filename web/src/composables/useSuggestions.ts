import { ref, nextTick } from "vue";
import { useStore } from "vuex";

const defaultKeywords = [
  {
    label: "and",
    kind: "Keyword",
    insertText: "and ",
  },
  {
    label: "or",
    kind: "Keyword",
    insertText: "or ",
  },
  {
    label: "like",
    kind: "Keyword",
    insertText: "like '%${1:params}%' ",
    insertTextRules: "InsertAsSnippet",
  },
  {
    label: "in",
    kind: "Keyword",
    insertText: "in ('${1:params}') ",
    insertTextRules: "InsertAsSnippet",
  },
  {
    label: "not in",
    kind: "Keyword",
    insertText: "not in ('${1:params}') ",
    insertTextRules: "InsertAsSnippet",
  },
  {
    label: "between",
    kind: "Keyword",
    insertText: "between '${1:params}' and '${1:params}' ",
    insertTextRules: "InsertAsSnippet",
  },
  {
    label: "not between",
    kind: "Keyword",
    insertText: "not between '${1:params}' and '${1:params}' ",
    insertTextRules: "InsertAsSnippet",
  },
  {
    label: "is null",
    kind: "Keyword",
    insertText: "is null ",
  },
  {
    label: "is not null",
    kind: "Keyword",
    insertText: "is not null ",
  },
  {
    label: ">",
    kind: "Operator",
    insertText: "> ",
  },
  {
    label: "<",
    kind: "Operator",
    insertText: "< ",
  },
  {
    label: ">=",
    kind: "Operator",
    insertText: ">= ",
  },
  {
    label: "<=",
    kind: "Operator",
    insertText: "<= ",
  },
  {
    label: "<>",
    kind: "Operator",
    insertText: "<> ",
  },
  {
    label: "=",
    kind: "Operator",
    insertText: "= ",
  },
  {
    label: "!=",
    kind: "Operator",
    insertText: "!= ",
  },
  {
    label: "()",
    kind: "Keyword",
    insertText: "(${1:condition}) ",
    insertTextRules: "InsertAsSnippet",
  },
];
const defaultSuggestions = [
  {
    label: (_keyword: string) => `match_all('${_keyword}')`,
    kind: "Text",
    insertText: (_keyword: string) => `match_all('${_keyword}')`,
  },
  {
    label: (_keyword: string) => `match_all_raw('${_keyword}')`,
    kind: "Text",
    insertText: (_keyword: string) => `match_all_raw('${_keyword}')`,
  },
  {
    label: (_keyword: string) => `match_all_raw_ignore_case('${_keyword}')`,
    kind: "Text",
    insertText: (_keyword: string) => `match_all_raw_ignore_case('${_keyword}')`,
  },
  {
    label: (_keyword: string) => `re_match(fieldname: string, regular_expression: string)`,
    kind: "Text",
    insertText: (_keyword: string) => `re_match(fieldname, '')`,
  },
  {
    label: (_keyword: string) => `re_not_match(fieldname: string, regular_expression: string)`,
    kind: "Text",
    insertText: (_keyword: string) => `re_not_match(fieldname, '')`,
  },
  {
    label: (_keyword: string) => `str_match(fieldname, '${_keyword}')`,
    kind: "Text",
    insertText: (_keyword: string) => `str_match(fieldname, '${_keyword}')`,
  },
  {
    label: (_keyword: string) =>
      `str_match_ignore_case(fieldname, '${_keyword}')`,
    kind: "Text",
    insertText: (_keyword: string) =>
      `str_match_ignore_case(fieldname, '${_keyword}')`,
  },
  {
    label: (_keyword: string) => `fuzzy_match(fieldname, '${_keyword}', 1)`,
    kind: "Text",
    insertText: (_keyword: string) => `fuzzy_match(fieldname, '${_keyword}', 1)`,
  },
  {
    label: (_keyword: string) => `fuzzy_match_all('${_keyword}', 1)`,
    kind: "Text",
    insertText: (_keyword: string) => `fuzzy_match_all('${_keyword}', 1)`,
  },
];

const useSqlSuggestions = () => {
  const autoCompleteData = ref({
    fieldValues: {} as any, // { kubernetes_host: new Set([value1, value2]) }
    query: "",
    position: {
      cursorIndex: 0,
    },
    popup: {
      open: (val: string) => {},
      close: (val: string) => {},
    },
  });
  const autoCompleteSuggestions: any = ref([]);
  const loading = ref(false);
  const autoCompleteKeywords: any = ref([]);
  const store = useStore();
  const functionKeywords: any = ref([]);
  let fieldKeywords: any = [];

  function analyzeSqlWhereClause(whereClause: string, cursorIndex: number) {
    const labelMeta = {
      hasLabels: false,
      isFocused: false,
      isEmpty: true,
      focusOn: "", // label or value
      meta: {
        label: "",
        value: "",
      },
    };

    const columnValueRegex = /(\w+)\s*=\s*$|\w+\s+IN\s+\($/i;

    let match;
    while ((match = columnValueRegex.exec(whereClause)) !== null) {
      const column = match[1];

      if (cursorIndex <= match.index + match[0].length - 1) {
        labelMeta.focusOn = "value";
        labelMeta.isFocused = true;
        labelMeta.meta.label = column;

        break; // Exit the loop after processing the match
      }
    }
    return labelMeta;
  }

  const getSuggestions = async () => {
    autoCompleteKeywords.value = [];
    const cursorIndex = autoCompleteData.value.position.cursorIndex;
    const sqlWhereClause = analyzeSqlWhereClause(
      autoCompleteData.value.query,
      cursorIndex
    );
    if (
      sqlWhereClause.meta.label &&
      autoCompleteData.value.fieldValues[sqlWhereClause.meta.label]
    ) {
      const values = Array.from(
        autoCompleteData.value.fieldValues[sqlWhereClause.meta.label] ||
          new Set()
      ).map((item) => {
        return {
          label: item,
          insertText: `'${item}'`,
          kind: "Keyword",
        };
      });

      autoCompleteKeywords.value = [...values];
      autoCompleteSuggestions.value = [];
      autoCompleteData.value.popup.open(autoCompleteData.value.query);
    } else {
      updateAutoComplete();
    }
  };

  const updateAutoComplete = () => {
    autoCompleteKeywords.value.push(...functionKeywords.value);
    for (const item of fieldKeywords) {
      autoCompleteKeywords.value.push(item);
    }
    autoCompleteKeywords.value.push(...defaultKeywords);
    autoCompleteSuggestions.value = [...defaultSuggestions];
  };

  const updateFieldKeywords = (fields: any[]) => {
    autoCompleteKeywords.value = [];
    fieldKeywords = [];
    let itemObj: any = {};
    fields.forEach((field: any) => {
      if (field.name == store.state.zoConfig.timestamp_column) {
        return;
      }
      itemObj = {
        label: field.name,
        kind: "Field",
        insertText: field.name,
        insertTextRules: "InsertAsSnippet",
      };
      fieldKeywords.push(itemObj);
    });
    updateAutoComplete();
  };

  const updateFunctionKeywords = (functions: any[]) => {
    functionKeywords.value = [];
    functions.forEach((field: any) => {
      const itemObj = {
        label: field.name,
        kind: "Function",
        insertText: field.name + field.args,
        insertTextRules: "InsertAsSnippet",
      };
      functionKeywords.value.push(itemObj);
    });
    updateAutoComplete();
  };

  return {
    autoCompleteData,
    autoCompleteKeywords,
    autoCompleteSuggestions,
    loading,
    getSuggestions,
    updateFieldKeywords,
    updateFunctionKeywords,
  };
};

export default useSqlSuggestions;
