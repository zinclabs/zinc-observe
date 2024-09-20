// Copyright 2023 Zinc Labs Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

import { toZonedTime } from "date-fns-tz";
import {
  formatDate,
  formatUnitValue,
  getUnitValue,
  isTimeSeries,
  isTimeStamp,
} from "./convertDataIntoUnitValue";

/**
 * Converts table data based on the panel schema and search query data.
 *
 * @param {any} panelSchema - The panel schema containing queries and fields.
 * @param {any} searchQueryData - The search query data.
 * @return {object} An object containing rows and columns.
 */
export const convertTableData = (
  panelSchema: any,
  searchQueryData: any,
  store: any,
) => {
  // if no data than return it
  if (
    !Array.isArray(searchQueryData) ||
    searchQueryData.length === 0 ||
    !searchQueryData[0] ||
    !panelSchema
  ) {
    return { rows: [], columns: [] };
  }

  const x = panelSchema?.queries[0].fields?.x || [];
  const y = panelSchema?.queries[0].fields?.y || [];
  let columnData = [...x, ...y];
  let tableRows = JSON.parse(JSON.stringify(searchQueryData[0]));
  const histogramFields: string[] = [];
  console.log("transposeColumns---------", columnData[0], columnData);

  // use all response keys if tableDynamicColumns is true
  if (panelSchema?.config?.table_dynamic_columns == true) {
    let responseKeys: any = new Set();

    // insert all keys of searchQueryData into responseKeys
    tableRows?.forEach((row: any) => {
      Object.keys(row).forEach((key) => {
        responseKeys?.add(key);
      });
    });

    // set to array
    responseKeys = Array.from(responseKeys);

    // remove x and y keys
    const xAxisKeys = x.map((key: any) => key.alias);
    const yAxisKeys = y.map((key: any) => key.alias);
    responseKeys = responseKeys?.filter(
      (key: any) => !xAxisKeys.includes(key) && !yAxisKeys.includes(key),
    );

    // create panelSchema fields object
    responseKeys = responseKeys.map((key: any) => ({
      label: key,
      alias: key,
      column: key,
      color: null,
      isDerived: false,
    }));

    // add responseKeys to columnData
    columnData = [...columnData, ...responseKeys];
  }

  // identify histogram fields for auto and custom sql
  if (panelSchema?.queries[0]?.customQuery === false) {
    for (const field of columnData) {
      if (field?.aggregationFunction === "histogram") {
        histogramFields.push(field.alias);
      } else {
        const sample = tableRows
          ?.slice(0, Math.min(20, tableRows.length))
          ?.map((it: any) => it[field.alias]);
        const isTimeSeriesData = isTimeSeries(sample);
        const isTimeStampData = isTimeStamp(sample);

        if (isTimeSeriesData || isTimeStampData) {
          histogramFields.push(field.alias);
        }
      }
    }
  } else {
    // need sampling to identify timeseries data
    for (const field of columnData) {
      if (field?.aggregationFunction === "histogram") {
        histogramFields.push(field.alias);
      } else {
        const sample = tableRows
          ?.slice(0, Math.min(20, tableRows.length))
          ?.map((it: any) => it[field.alias]);
        const isTimeSeriesData = isTimeSeries(sample);
        const isTimeStampData = isTimeStamp(sample);

        if (isTimeSeriesData || isTimeStampData) {
          histogramFields.push(field.alias);
        }
      }
    }
  }

  const isTransposeEnabled = panelSchema.config.table_transpose;
  const transposeColumn = columnData[0]?.alias || "";
  const transposeColumnLabel = columnData[0]?.label || "";
  let columns;

  if (!isTransposeEnabled) {
    columns = columnData.map((it: any) => {
      let obj: any = {};
      const isNumber = isSampleValuesNumbers(tableRows, it.alias, 20);
      obj["name"] = it.label;
      obj["field"] = it.alias;
      obj["label"] = it.label;
      obj["align"] = !isNumber ? "left" : "right";
      obj["sortable"] = true;
      // if number then sort by number and use decimal point config option in format
      if (isNumber) {
        obj["sort"] = (a: any, b: any) => parseFloat(a) - parseFloat(b);
        obj["format"] = (val: any) => {
          return !Number.isNaN(val)
            ? `${
                formatUnitValue(
                  getUnitValue(
                    val,
                    panelSchema.config?.unit,
                    panelSchema.config?.unit_custom,
                    panelSchema.config?.decimals ?? 2,
                  ),
                ) ?? 0
              }`
            : val;
        };
      }

      // if current field is histogram field then return formatted date
      if (histogramFields.includes(it.alias)) {
        // if current field is histogram field then return formatted date
        obj["format"] = (val: any) => {
          return formatDate(
            toZonedTime(
              typeof val === "string"
                ? `${val}Z`
                : new Date(val)?.getTime() / 1000,
              store.state.timezone,
            ),
          );
        };
      }
      return obj;
    });
  } else {
    // lets get all columns from a particular field
    const transposeColumns = searchQueryData[0].map(
      (it: any) => it[transposeColumn] ?? "",
    );
    console.log("transposeColumns---", searchQueryData[0]);
    
    const uniqueTransposeColumns: any = [];
    const columnDuplicationMap: any = {};

    transposeColumns.forEach((col: any, index: any) => {
      if (!columnDuplicationMap[col]) {
        uniqueTransposeColumns.push(col);
        columnDuplicationMap[col] = 1;
      } else {
        const uniqueCol = `${col}_${columnDuplicationMap[col]}`;
        uniqueTransposeColumns.push(uniqueCol);
        columnDuplicationMap[col] += 1;
      }
    });

    // Filter out the first column but retain the label
    columnData = columnData.filter((it: any) => it.alias !== transposeColumn);
    console.log("transposeColumns---------", columnData[0]);
    
    // Generate label and corresponding transposed data rows
    columns = [
      { name: "label", field: "label", label: transposeColumnLabel, align: "left" }, // Add label column with the first column's label
      ...uniqueTransposeColumns.map((it: any) => {
        let obj: any = {};
        const isNumber = isSampleValuesNumbers(tableRows, it, 20);

        obj["name"] = it;
        obj["field"] = it;
        obj["label"] = it;
        obj["align"] = !isNumber ? "left" : "right";
        obj["sortable"] = true;

        if (isNumber) {
          obj["sort"] = (a: any, b: any) => parseFloat(a) - parseFloat(b);
          obj["format"] = (val: any) => {
            return !Number.isNaN(val)
              ? `${
                  formatUnitValue(
                    getUnitValue(
                      val,
                      panelSchema.config?.unit,
                      panelSchema.config?.unit_custom,
                      panelSchema.config?.decimals ?? 2,
                    ),
                  ) ?? 0
                }`
              : val;
          };
        }

        if (histogramFields.includes(it)) {
          obj["format"] = (val: any) => {
            return formatDate(
              toZonedTime(
                typeof val === "string"
                  ? `${val}Z`
                  : new Date(val)?.getTime() / 1000,
                store.state.timezone,
              ),
            );
          };
        }

        return obj;
      }),
    ];

    // Transpose rows, adding 'label' as the first column
    tableRows = columnData.map((it: any) => {
      let obj = uniqueTransposeColumns.reduce(
        (acc: any, curr: any, reduceIndex: any) => {
          acc[curr] = searchQueryData[0][reduceIndex][it.alias] ?? "";
          return acc;
        },
        {},
      );
      obj["label"] = it.label || transposeColumnLabel; // Add the label corresponding to each column
      return obj;
    });
  }

  console.log("tableRows return", tableRows);
  console.log("columns return", columns);

  return {
    rows: tableRows,
    columns,
  };
};

/**
 * Checks if the sample values of a given array are numbers based on a specified key.
 *
 * @param {any[]} arr - The array to check.
 * @param {string} key - The key to access the values.
 * @param {number} sampleSize - The number of sample values to check.
 * @return {boolean} True if all sample values are numbers or are undefined, null, or empty strings; otherwise, false.
 */
const isSampleValuesNumbers = (arr: any, key: string, sampleSize: number) => {
  if (!Array.isArray(arr)) {
    return false;
  }
  const sample = arr.slice(0, Math.min(sampleSize, arr.length));
  return sample.every((obj) => {
    const value = obj[key];
    return (
      value === undefined ||
      value === null ||
      value === "" ||
      typeof value === "number"
    );
  });
};
