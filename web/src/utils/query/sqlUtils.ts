let parser: any;
let parserInitialized = false;

const importSqlParser = async () => {
  if (!parserInitialized) {
    const useSqlParser: any = await import("@/composables/useParser");
    const { sqlParser }: any = useSqlParser.default();
    parser = await sqlParser();
    parserInitialized = true;
  }
  return parser;
};

export const addLabelsToSQlQuery = async (originalQuery: any, labels: any) => {
  await importSqlParser();

  let dummyQuery = "select * from 'default'";

  for (let i = 0; i < labels.length; i++) {
    const label = labels[i];
    dummyQuery = await addLabelToSQlQuery(
      dummyQuery,
      label.name,
      label.value,
      label.operator
    );
  }

  const astOfOriginalQuery: any = parser.astify(originalQuery);
  const astOfDummy: any = parser.astify(dummyQuery);

  // if ast already has a where clause
  if (astOfOriginalQuery.where) {
    const newWhereClause = {
      type: "binary_expr",
      operator: "AND",
      left: {
        ...astOfOriginalQuery.where,
        parentheses: true,
      },
      right: {
        ...astOfDummy.where,
        parentheses: true,
      },
    };
    const newAst = {
      ...astOfOriginalQuery,
      where: newWhereClause,
    };
    const sql = parser.sqlify(newAst);
    const quotedSql = sql.replace(/`/g, '"');
    return quotedSql;
  } else {
    const newAst = {
      ...astOfOriginalQuery,
      where: astOfDummy.where,
    };
    const sql = parser.sqlify(newAst);
    const quotedSql = sql.replace(/`/g, '"');
    return quotedSql;
  }
};

export const addLabelToSQlQuery = async (
  originalQuery: any,
  label: any,
  value: any,
  operator: any
) => {
  await importSqlParser();

  let condition: any;

  switch (operator) {
    case "Contains":
      operator = "LIKE";
      value = "%" + value + "%";
      break;
    case "Not Contains":
      operator = "NOT LIKE";
      value = "%" + value + "%";
      break;
    case "Is Null":
      operator = "IS NULL";
      break;
    case "Is Not Null":
      operator = "IS NOT NULL";
      break;
  }

  // Construct condition based on operator
  condition =
    operator === "IS NULL" || operator === "IS NOT NULL"
      ? {
          type: "binary_expr",
          operator: operator,
          left: {
            type: "column_ref",
            table: null,
            column: label,
          },
          right: {
            type: "",
          },
        }
      : {
          type: "binary_expr",
          operator: operator,
          left: {
            type: "column_ref",
            table: null,
            column: label,
          },
          right: {
            type: "string",
            value: value,
          },
        };

  const ast: any = parser.astify(originalQuery);

  let query = "";
  if (!ast.where) {
    // If there is no WHERE clause, create a new one
    const newWhereClause = condition;

    const newAst = {
      ...ast,
      where: newWhereClause,
    };

    const sql = parser.sqlify(newAst);
    const quotedSql = sql.replace(/`/g, '"');
    query = quotedSql;
  } else {
    const newCondition = {
      type: "binary_expr",
      operator: "AND",
      // parentheses: true,
      left: {
        // parentheses: true,
        ...ast.where,
      },
      right: condition,
    };

    const newAst = {
      ...ast,
      where: newCondition,
    };

    const sql = parser.sqlify(newAst);
    const quotedSql = sql.replace(/`/g, '"');

    query = quotedSql;
  }

  return query;
};

export const getStreamFromQuery = async (query: any) => {
  await importSqlParser();

  try {
    const ast: any = parser.astify(query);
    return ast?.from[0]?.table || "";
  } catch (e: any) {
    return "";
  }
};

// returns 'ASC' or 'DESC' if exist
// return null if not exist

export const isGivenFieldInOrderBy = async (
  sqlQuery: string,
  fieldAlias: string
) => {
  await importSqlParser();
  const ast: any = parser.astify(sqlQuery);

  if (ast?.orderby) {
    for (const item of ast.orderby) {
      if (item?.expr?.column === fieldAlias) {
        return item.type; // 'ASC' or 'DESC'
      }
    }
  }

  return null;
};

// Function to extract field names, aliases, and aggregation functions
function extractFields(parsedAst: any, timeField: string) {
  let fields = parsedAst.columns.map((column: any) => {
    const field = {
      column: "",
      alias: "",
      aggregationFunction: null,
    };

    if (column.expr.type === "column_ref") {
      field.column = column?.expr?.column ?? timeField;
    } else if (column.expr.type === "aggr_func") {
      field.column = column?.expr?.args?.expr?.column ?? timeField;
      field.aggregationFunction = column?.expr?.name?.toLowerCase() ?? "count";
    } else if (column.expr.type === "function") {
      // histogram field
      field.column = column?.expr?.args?.value[0]?.column ?? timeField;
      field.aggregationFunction =
        column?.expr?.name[0]?.value?.toLowerCase() ?? "histogram";
    }

    field.alias = column?.as ?? field?.column ?? timeField;

    return field;
  });

  // Check if all fields are selected and remove the `*` entry
  const allFieldsSelected = parsedAst.columns.some(
    (column: any) => column.expr && column.expr.column === "*"
  );

  if (allFieldsSelected) {
    // Add histogram(_timestamp) and count(_timestamp) to the fields array
    fields.push(
      {
        column: timeField,
        alias: "x_axis_1",
        aggregationFunction: "histogram",
      },
      {
        column: timeField,
        alias: "y_axis_1",
        aggregationFunction: "count",
      }
    );

    // Filter out the `*` entry from fields
    fields = fields.filter((field: any) => field.column !== "*");
  }

  return fields;
}

// Function to extract conditions from the WHERE clause
function extractConditions(parsedAst: any) {
  const conditions: any = [];

  function traverseCondition(node: any) {
    if (node.type === "binary_expr") {
      const condition = {
        column: "",
        operator: node.operator,
        value: "",
      };

      if (node.left.type === "column_ref") {
        condition.column = node.left.column;
      }

      if (
        node.right.type === "string" ||
        node.right.type === "number" ||
        node.right.type === "single_quote_string"
      ) {
        condition.value = node.right.value;
      } else if (node.right.type === "column_ref") {
        condition.value = node.right.column;
      } else if (node.right.type === "expr_list") {
        condition.value = node.right.value.map(
          (item: any) => item.value || item.column
        );
      } else if (node.right.type === "null") {
        condition.operator += " NULL";
      }

      conditions.push(condition);
    } else if (node.type === "unary_expr" && node.operator === "NOT") {
      const condition = {
        column: "",
        operator: "NOT",
        value: "",
      };

      if (node.expr.type === "binary_expr") {
        condition.operator = `NOT ${node.expr.operator}`;
        if (node.expr.left.type === "column_ref") {
          condition.column = node.expr.left.column;
        }
        if (
          node.expr.right.type === "string" ||
          node.expr.right.type === "number" ||
          node.expr.right.type === "single_quote_string"
        ) {
          condition.value = node.expr.right.value;
        } else if (node.expr.right.type === "column_ref") {
          condition.value = node.expr.right.column;
        }
      }

      conditions.push(condition);
    }

    if (node.left && node.left.type === "binary_expr") {
      traverseCondition(node.left);
    }

    if (node.right && node.right.type === "binary_expr") {
      traverseCondition(node.right);
    }
  }

  if (parsedAst.where) {
    traverseCondition(parsedAst.where);
  }

  return conditions;
}

// Function to extract the table name
function extractTableName(parsedAst: any) {
  return parsedAst.from[0].table ?? null;
}

export const getFieldsFromQuery = async (
  query: any,
  timeField: string = "_timestamp"
) => {
  try {
    await importSqlParser();

    const ast: any = parser.astify(query);

    const streamName = extractTableName(ast) ?? null;
    const fields = extractFields(ast, timeField);
    const conditions = extractConditions(ast);

    // filter fields and conditions
    const filteredFields = fields.filter((field: any) => field.column);
    const filteredConditions = conditions.filter(
      (condition: any) => condition.column
    );

    return {
      fields: filteredFields,
      conditions: filteredConditions,
      streamName,
    };
  } catch (error) {
    return {
      fields: [
        {
          column: timeField,
          alias: "x_axis_1",
          aggregationFunction: "histogram",
        },
        {
          column: timeField,
          alias: "y_axis_1",
          aggregationFunction: "count",
        },
      ],
      conditions: [],
      streamName: null,
    };
  }
};

export const buildSqlQuery = (
  tableName: string,
  fields: any,
  whereClause: string
) => {
  let query = "SELECT ";

  // If the fields array is empty, use *, otherwise join the fields with commas
  if (fields.length === 0) {
    query += "*";
  } else {
    query += fields.join(", ");
  }

  // Add the table name
  query += ` FROM "${tableName}"`;

  // If the whereClause is not empty, add it
  if (whereClause.trim() !== "") {
    query += " WHERE " + whereClause;
  }

  // Return the constructed query
  return query;
};

export const getValidConditionObj = (condition: any) => {
  switch (condition.operator) {
    case "in": {
      condition.type = "list";
      condition.values = condition.value;
      condition.operator = "IN";
      condition.operator = null;
      condition.value = null;
      break;
    }
    case "not in": {
      // currently not supported
      break;
    }
    case "is null": {
      condition.type = "condition";
      condition.values = [];
      condition.operator = "Is Null";
      condition.value = null;

      break;
    }
    case "is not null": {
      condition.type = "condition";
      condition.values = [];
      condition.operator = "Is Not Null";
      condition.value = null;
      break;
    }
    case "like": {
      condition.type = "condition";
      condition.values = [];
      condition.operator = "Contains";
      // remove % from the start and end
      condition.value = condition?.value?.replace(/^%|%$/g, "");
      break;
    }
    case "not like": {
      condition.type = "condition";
      condition.values = [];
      condition.operator = "Not Contains";
      // remove % from the start and end
      condition.value = condition?.value?.replace(/^%|%$/g, "");
      break;
    }
    case "<>":
    case "!=": {
      condition.type = "condition";
      condition.values = [];
      condition.operator = "<>";
      condition.value = `'${condition.value}'`;
      break;
    }
    case "=":
    case "<":
    case ">":
    case "<=":
    case ">=": {
      condition.type = "condition";
      condition.values = [];
      condition.value = `'${condition.value}'`;
      break;
    }
    default:
      break;
  }

  return condition;
};

export const changeHistogramInterval = async (
  query: any,
  histogramInterval: any,
) => {
  // if histogramInterval is null or query is null or query is empty, return query
  if (histogramInterval === null || query === null || query === "") {
    return query;
  }

  await importSqlParser();
  const ast: any = parser.astify(query);

  // Iterate over the columns to check if the column is histogram
  ast.columns.forEach((column: any) => {
    // check if the column is histogram
    if (
      column.expr.type === "function" &&
      column?.expr?.name?.name[0]?.value === "histogram"
    ) {
      const histogramExpr = column.expr;
      if (histogramExpr.args && histogramExpr.args.type === "expr_list") {
        // if selected histogramInterval is null then remove interval argument
        if (!histogramInterval) {
          histogramExpr.args.value = histogramExpr.args.value.slice(0, 1);
        }
        // else update interval argument
        else {
          // check if there is existing interval value
          // if have then do not do anything
          // else insert new arg with given histogramInterval
          if (histogramExpr.args.value[1]) {
            // Update existing interval value
            // histogramExpr.args.value[1] = {
            //   type: "single_quote_string",
            //   value: `${histogramInterval}`,
            // };
          } else {
            // create new arg for interval
            histogramExpr.args.value.push({
              type: "single_quote_string",
              value: `${histogramInterval}`,
            });
          }
        }
      }
    }
  });

  const sql = parser.sqlify(ast);
  return sql.replace(/`/g, '"');
};
