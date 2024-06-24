import { onBeforeMount, onUnmounted, ref } from "vue";

import useEventBus from "@/composables/useEventBus";
import streamService from "@/services/stream";
import { useStore } from "vuex";
import { b64EncodeUnicode, formatLargeNumber } from "@/utils/zincutils";

const fieldValues = ref<{
  [key: string | number]: {
    isLoading: boolean;
    values: { key: string; count: string }[];
    size: number;
    searchKeyword: string;
    selectedValues: string[];
    isOpen: boolean;
  };
}>({});

const expandedFields: any = ref([]);

const { on, off, emit } = useEventBus();

const fields = ref([]);

const data: any = ref({});

let parser: any = null;

const useFieldList = () => {
  onBeforeMount(async () => {
    await importSqlParser();
  });

  const importSqlParser = async () => {
    const useSqlParser: any = await import("@/composables/useParser");
    const { sqlParser }: any = useSqlParser.default();
    parser = await sqlParser();
  };

  const store = useStore();

  const streamMeta = ref({
    selectedStream: "",
    selectedStreamType: "",
  });

  const meta: any = ref({
    isFieldItemDraggable: true,
  });

  const fieldFormattingDebug = {
    fieldFormattingStartTime: 0,
    fieldFormattingEndTime: 0,
    extractFieldsStartTime: 0,
    extractFieldsWithAPI: "",
    extractFieldsEndTime: 0,
  };

  //On stream selection, get the schema of stream and set the fields

  // async function extractFields() {
  //   try {
  //     fieldFormattingDebug["extractFieldsStartTime"] = performance.now();
  //     fieldFormattingDebug["extractFieldsWithAPI"] = "";

  //     // reset message, selectedStreamFields and interestingFieldList
  //     searchObj.data.errorMsg = "";
  //     searchObj.data.stream.selectedStreamFields = [];
  //     searchObj.data.stream.interestingFieldList = [];
  //     const schemaFields: any = [];
  //     if (searchObj.data.streamResults.list.length > 0) {
  //       const timestampField = store.state.zoConfig.timestamp_column;
  //       const allField = store.state.zoConfig?.all_fields_name;
  //       const schemaInterestingFields: string[] = [];
  //       let userDefineSchemaSettings: any = [];
  //       const schemaMaps: any = [];
  //       let fieldObj: any = {};
  //       const localInterestingFields: any = useLocalInterestingFields();

  //       for (const stream of searchObj.data.streamResults.list) {
  //         if (searchObj.data.stream.selectedStream.value == stream.name) {
  //           userDefineSchemaSettings =
  //             stream.settings?.defined_schema_fields?.slice() || [];
  //           // check for schema exist in the object or not
  //           // if not pull the schema from server.
  //           if (!stream.hasOwnProperty("schema")) {
  //             fieldFormattingDebug["extractFieldsWithAPI"] = " with API ";
  //             const streamData: any = await loadStreamFileds(stream.name);
  //             const streamSchema: any = streamData.schema;
  //             if (streamSchema == undefined) {
  //               searchObj.loadingStream = false;
  //               searchObj.data.errorMsg = t("search.noFieldFound");
  //               throw new Error(searchObj.data.errorMsg);
  //               return;
  //             }
  //             stream.settings = streamData.settings;
  //             stream.schema = streamSchema;
  //           }

  //           let environmentInterestingFields = [];
  //           if (
  //             store.state.zoConfig.hasOwnProperty("default_quick_mode_fields")
  //           ) {
  //             environmentInterestingFields =
  //               store.state?.zoConfig?.default_quick_mode_fields;
  //           }

  //           if (
  //             stream.settings.hasOwnProperty("defined_schema_fields") &&
  //             userDefineSchemaSettings.length > 0
  //           ) {
  //             searchObj.meta.hasUserDefinedSchemas = true;
  //             if (store.state.zoConfig.hasOwnProperty("timestamp_column")) {
  //               userDefineSchemaSettings.push(
  //                 store.state.zoConfig?.timestamp_column
  //               );
  //             }

  //             if (store.state.zoConfig.hasOwnProperty("all_fields_name")) {
  //               userDefineSchemaSettings.push(
  //                 store.state.zoConfig?.all_fields_name
  //               );
  //             }
  //           } else {
  //             searchObj.meta.hasUserDefinedSchemas = false;
  //           }

  //           searchObj.data.stream.interestingFieldList =
  //             localInterestingFields.value != null &&
  //             localInterestingFields.value[
  //               searchObj.organizationIdetifier +
  //                 "_" +
  //                 searchObj.data.stream.selectedStream.value
  //             ] !== undefined &&
  //             localInterestingFields.value[
  //               searchObj.organizationIdetifier +
  //                 "_" +
  //                 searchObj.data.stream.selectedStream.value
  //             ].length > 0
  //               ? localInterestingFields.value[
  //                   searchObj.organizationIdetifier +
  //                     "_" +
  //                     searchObj.data.stream.selectedStream.value
  //                 ]
  //               : environmentInterestingFields.length > 0
  //               ? [...environmentInterestingFields]
  //               : [...schemaInterestingFields];

  //           // create a schema field mapping based on field name to avoid iteration over object.
  //           // in case of user defined schema consideration, loop will be break once all defined fields are mapped.
  //           for (const field of stream.schema) {
  //             fieldObj = {
  //               ...field,
  //               ftsKey:
  //                 stream.settings.full_text_search_keys.indexOf > -1
  //                   ? true
  //                   : false,
  //               isSchemaField: true,
  //               showValues:
  //                 field.name !== timestampField && field.name !== allField,
  //               isInterestingField:
  //                 searchObj.data.stream.interestingFieldList.includes(
  //                   field.name
  //                 )
  //                   ? true
  //                   : false,
  //             };
  //             if (
  //               store.state.zoConfig.user_defined_schemas_enabled &&
  //               searchObj.meta.useUserDefinedSchemas == "user_defined_schema" &&
  //               stream.settings.hasOwnProperty("defined_schema_fields") &&
  //               userDefineSchemaSettings.length > 0
  //             ) {
  //               if (userDefineSchemaSettings.includes(field.name)) {
  //                 schemaMaps.push(fieldObj);
  //                 schemaFields.push(field.name);
  //               }

  //               if (schemaMaps.length == userDefineSchemaSettings.length) {
  //                 break;
  //               }
  //             } else {
  //               schemaMaps.push(fieldObj);
  //               schemaFields.push(field.name);
  //             }
  //           }

  //           // check for user defined schema is false then only consider checking new fields from result set
  //           if (
  //             searchObj.data.queryResults.hasOwnProperty("hits") &&
  //             searchObj.data.queryResults?.hits.length > 0 &&
  //             (!store.state.zoConfig.user_defined_schemas_enabled ||
  //               searchObj.meta.useUserDefinedSchemas != "user_defined_schema")
  //           ) {
  //             // Find the index of the record with max attributes
  //             const maxAttributesIndex =
  //               searchObj.data.queryResults.hits.reduce(
  //                 (
  //                   maxIndex: string | number,
  //                   obj: {},
  //                   currentIndex: any,
  //                   array: { [x: string]: {} }
  //                 ) => {
  //                   const numAttributes = Object.keys(obj).length;
  //                   const maxNumAttributes = Object.keys(
  //                     array[maxIndex]
  //                   ).length;
  //                   return numAttributes > maxNumAttributes
  //                     ? currentIndex
  //                     : maxIndex;
  //                 },
  //                 0
  //               );
  //             const recordwithMaxAttribute =
  //               searchObj.data.queryResults.hits[maxAttributesIndex];

  //             // Object.keys(recordwithMaxAttribute).forEach((key) => {
  //             for (const key of Object.keys(recordwithMaxAttribute)) {
  //               if (!schemaFields.includes(key)) {
  //                 fieldObj = {
  //                   name: key,
  //                   type: "Utf8",
  //                   ftsKey: false,
  //                   isSchemaField: false,
  //                   showValues: false,
  //                   isInterestingField:
  //                     searchObj.data.stream.interestingFieldList.includes(key)
  //                       ? true
  //                       : false,
  //                 };
  //                 schemaMaps.push(fieldObj);
  //                 schemaFields.push(key);
  //               }
  //             }
  //           }

  //           // cross verify list of interesting fields belowgs to selected stream fields
  //           for (
  //             let i = searchObj.data.stream.interestingFieldList.length - 1;
  //             i >= 0;
  //             i--
  //           ) {
  //             const fieldName = searchObj.data.stream.interestingFieldList[i];
  //             if (!schemaFields.includes(fieldName)) {
  //               searchObj.data.stream.interestingFieldList.splice(i, 1);
  //             }
  //           }

  //           let localFields: any = {};
  //           if (localInterestingFields.value != null) {
  //             localFields = localInterestingFields.value;
  //           }
  //           localFields[
  //             searchObj.organizationIdetifier +
  //               "_" +
  //               searchObj.data.stream.selectedStream.value
  //           ] = searchObj.data.stream.interestingFieldList;
  //           useLocalInterestingFields(localFields);
  //           searchObj.data.stream.userDefinedSchema =
  //             userDefineSchemaSettings || [];
  //         }
  //       }

  //       searchObj.data.stream.selectedStreamFields = schemaMaps;
  //       if (
  //         searchObj.data.stream.selectedStreamFields != undefined &&
  //         searchObj.data.stream.selectedStreamFields.length
  //       )
  //         updateFieldKeywords(searchObj.data.stream.selectedStreamFields);
  //     }
  //     fieldListFields.value = searchObj.data.stream.selectedStreamFields;
  //     console.log(fieldListFields.value);
  //     fieldFormattingDebug["extractFieldsEndTime"] = performance.now();
  //     console.log(
  //       `ExtractFields ${
  //         fieldFormattingDebug["extractFieldsWithAPI"]
  //       } operation took ${
  //         fieldFormattingDebug["extractFieldsEndTime"] -
  //         fieldFormattingDebug["extractFieldsStartTime"]
  //       } milliseconds to complete`
  //     );
  //   } catch (e: any) {
  //     searchObj.loadingStream = false;
  //     console.log("Error while extracting fields");
  //   }
  // }

  onUnmounted(() => {
    // remove all event listeners
  });

  const toggleFilterExpansion = (show: boolean, columnName: string) => {
    if (show) {
      expandedFields.value.push(columnName);
    } else {
      expandedFields.value = expandedFields.value.filter(
        (_column: string) => _column !== columnName
      );
    }
  };

  const getFieldValues = ({
    name,
    streamName,
    streamType,
    editorValue,
    timestamp,
  }: {
    name: string;
    streamName: string;
    streamType: string;
    editorValue: string;
    timestamp: {
      startTime: string;
      endTime: string;
    };
  }) => {
    fieldValues.value[name].size *= 2;

    let query_context = `SELECT * FROM ${streamName} WHERE ${name} is not null`;

    if (editorValue.trim().length) query_context += " AND " + editorValue;

    if (fieldValues.value[name].searchKeyword.trim().length)
      query_context += ` AND ${name} ILIKE '%${fieldValues.value[name].searchKeyword}%'`;

    fieldValues.value[name]["isLoading"] = true;

    let filter = "";

    const isSpecialField = ["service_name", "operation_name"].includes(name);

    if (isSpecialField) {
      if (
        name === "operation_name" &&
        fieldValues.value["service_name"].selectedValues.length
      ) {
        const values = fieldValues.value["service_name"].selectedValues
          .map((value) => value)
          .join(",");
        filter += `service_name=${values}`;
      }
    }

    return new Promise((resolve, reject) => {
      streamService
        .fieldValues({
          org_identifier: store.state.selectedOrganization.identifier,
          stream_name: streamName,
          start_time: timestamp.startTime,
          end_time: timestamp.endTime,
          fields: [name],
          size: fieldValues.value[name].size,
          type: streamType,
          query_context: !isSpecialField && b64EncodeUnicode(query_context),
          filter: isSpecialField ? filter : null,
          keyword: isSpecialField && fieldValues.value[name].searchKeyword,
        })
        .then((res: any) => {
          if (!res.data.hits.length) {
            fieldValues.value[name]["isLoading"] = false;
            fieldValues.value[name]["values"] = [];
            return resolve(fieldValues.value[name]);
          }
          updateSelectedValues(res.data.hits);
          resolve(fieldValues.value[name]);
        })
        .catch((e) => {
          reject({
            message: `Error while fetching values for ${name}`,
            error: e,
          });
        })
        .finally(() => {
          fieldValues.value[name]["isLoading"] = false;
        });
    });
  };

  // Update the values of the selected fields and add the values which are not present in the selected values
  // Also we filter out null values from the values array
  const updateSelectedValues = (hits: any[]) => {
    hits.forEach((field: any) => {
      const values: any = new Set([]);

      fieldValues.value[field.field]["values"] =
        field.values.map((value: any) => {
          values.add(value.zo_sql_key);
          return {
            key: value.zo_sql_key?.toString() ? value.zo_sql_key : "null",
            count: formatLargeNumber(value.zo_sql_num),
          };
        }) || [];

      fieldValues.value[field.field].selectedValues.forEach((value: string) => {
        if (values.has(value)) return;
        else
          fieldValues.value[field.field]["values"].unshift({
            key: value?.toString() || "null",
            count: "0",
          });
      });
    });
  };

  const setupFieldValues = (fields: any[]) => {
    fields.forEach(
      (field: { name: string; showValues: boolean; ftsKey: boolean }) => {
        if (field.showValues && !field.ftsKey) {
          fieldValues.value[field.name] = {
            isLoading: false,
            values: [],
            selectedValues: [],
            isOpen: false,
            size: 5,
            searchKeyword: "",
          };
        }
      }
    );
  };

  const restoreFiltersFromQuery = (node: any) => {
    if (!node) return;
    if (node.type === "binary_expr") {
      if (node.left.column) {
        let values = [];
        if (node.operator === "IN") {
          values = node.right.value.map(
            (_value: { value: string }) => _value.value
          );
        }
        fieldValues.value[node.left.column].selectedValues = values;
      }
    }

    // Recurse through AND/OR expressions
    if (
      node.type === "binary_expr" &&
      (node.operator === "AND" || node.operator === "OR")
    ) {
      restoreFiltersFromQuery(node.left);
      restoreFiltersFromQuery(node.right);
    }
  };

  /**
   *
   * @param whereClause - SQL where clause
   * @param streamName - Name of the stream
   */
  const restoreFilters = (whereClause: string, streamName: string) => {
    // const filters = searchObj.data.stream.filters;

    const defaultQuery = `SELECT * FROM '${streamName}' WHERE `;

    const parsedQuery = parser.astify(defaultQuery + whereClause);

    restoreFiltersFromQuery(parsedQuery.where);
  };

  const generateFilteredQuery = ({
    column,
    values,
    prevValues,
    streamName,
    editorValue = "",
  }: {
    column: string;
    values: string[];
    prevValues: string[];
    streamName: string;
    editorValue: string;
  }) => {
    try {
      let updatedFilterValue = editorValue;

      // Convert values to string
      const valuesString = values
        .map((value: string) => "'" + value + "'")
        .join(",");

      let query = `SELECT * FROM '${streamName}' `;

      // If there is no editor value, then we can directly add the filter
      // If no previous values, then we can directly add the filter e.g curr: ['abc']  prev: []
      if (!editorValue?.length) {
        updatedFilterValue = `${column} IN (${valuesString})`;
      } else if (!prevValues.length) {
        updatedFilterValue += ` AND ${column} IN (${valuesString})`;
      }

      query += " WHERE " + updatedFilterValue;

      const parsedQuery: any = parser.astify(query);

      if (!values.length) {
        parsedQuery.where = removeCondition(parsedQuery.where, column);
      } else if (prevValues.length) {
        modifyWhereClause(parsedQuery?.where, column, values);
      }

      // Convert the AST back to SQL query
      const modifiedQuery = parser.sqlify(parsedQuery);
      if (modifiedQuery) {
        updatedFilterValue = (modifiedQuery.split("WHERE")[1] || "")
          .replace(/`/g, "")
          .trim();
      }

      // filterExpandedFieldValues();

      return updatedFilterValue;
    } catch (e) {
      console.log("Error while creating query from filters");
    }
  };

  const updateExpandedFieldValues = ({
    streamName,
    editorValue,
    streamType,
    timestamp,
  }: {
    streamName: string;
    editorValue: string;
    streamType: string;
    timestamp: {
      startTime: string;
      endTime: string;
    };
  }) => {
    const fields =
      [
        ...expandedFields.value.filter(
          (_value: string) =>
            !["operation_name", "service_name"].includes(_value)
        ),
      ] || [];

    if (expandedFields.value.includes("operation_name")) {
      getFieldValues({
        name: "operation_name",
        streamName,
        streamType,
        editorValue,
        timestamp,
      });
    }

    if (expandedFields.value.includes("service_name")) {
      getFieldValues({
        name: "operation_name",
        streamName,
        streamType,
        editorValue,
        timestamp,
      });
    }

    if (!fields.length) return;

    return Promise.all(
      fields.map((field) => {
        return getFieldValues({
          name: field,
          streamName,
          streamType,
          editorValue,
          timestamp,
        });
      })
    );
  };

  const removeCondition = (node: any, fieldName: string): any => {
    if (!node) return null;

    if (node.type === "binary_expr") {
      if (node.left.column === fieldName) {
        // Return null to indicate this node should be removed
        return null;
      }
    }

    // Recurse through AND/OR expressions
    if (
      node.type === "binary_expr" &&
      (node.operator === "AND" || node.operator === "OR")
    ) {
      const left = removeCondition(node.left, fieldName);
      const right = removeCondition(node.right, fieldName);

      if (!left) return right;
      if (!right) return left;

      node.left = left;
      node.right = right;
    }

    return node;
  };

  const modifyWhereClause = (node: any, fieldName: string, newValue: any[]) => {
    if (!node) return;

    if (node.type === "binary_expr") {
      if (node.left.column === fieldName) {
        // Assuming the right side is a literal
        if (fieldName === "duration" && node.operator === ">=") {
          node.right.value = newValue[0]["min"];
        }

        if (fieldName === "duration" && node.operator === "<=") {
          node.right.value = newValue[0]["max"];
        }

        if (node.operator === "IN") {
          node.right.value = newValue.map((value: string) => {
            return {
              type: "single_quote_string",
              value: value,
            };
          });
        }
      }
    }

    // Recurse through AND/OR expressions
    if (
      node.type === "binary_expr" &&
      (node.operator === "AND" || node.operator === "OR")
    ) {
      modifyWhereClause(node.left, fieldName, newValue);
      modifyWhereClause(node.right, fieldName, newValue);
    }
  };

  return {
    fields,
    data,
    fieldValues,
    streamMeta,
    addFieldListEvent: on,
    removeFieldListEvent: off,
    emitFieldListEvent: emit,
    expandedFields,
    toggleFilterExpansion,
    getFieldValues,
    setupFieldValues,
    restoreFilters,
    generateFilteredQuery,
    updateExpandedFieldValues,
  };
};

export default useFieldList;
