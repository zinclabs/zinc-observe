// Copyright 2023 OpenObserve Inc.
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

/**
 * Converts SQL data into a format suitable for rendering a chart.
 *
 * @param {any} panelSchema - the panel schema object
 * @param {any} searchQueryData - the search query data
 * @param {any} store - the store object
 * @return {Object} - the options object for rendering the chart
 */
export const runJavaScriptCode = (panelSchema: any, searchQueryData: any) => {
  try {
    // Retrieve the user function code from the editor
    const userFunctionCode = panelSchema.customChartContent.trim();

    // Remove comments from the user code
    const cleanedCode = userFunctionCode
      .replace(/\/\*[\s\S]*?\*\/|\/\/.*|--.*/g, "")
      .trim();

    // Use a regular expression to extract the function name or arrow function
    const functionNameMatch = cleanedCode.match(
      /function\s+([a-zA-Z0-9_$]+)\s*\(|([a-zA-Z0-9_$]+)\s*=\s*\(\s*.*\)\s*=>/,
    );

    if (!functionNameMatch) {
      throw new Error("The provided code must define a function.");
    }

    // Determine the function name based on the match
    const functionName = functionNameMatch[1] || functionNameMatch[2];

    // Wrap the user's code in a Function constructor
    const userFunction = new Function(
      "data",
      `${cleanedCode}; return ${functionName}(data);`,
    );

    // Execute the function
    const result = userFunction(searchQueryData);
    console.log(result,'result')

    // Use the result (if any) from the user's function
    panelSchema.customChartResult = result;
    return result;
  } catch (error) {
    // Handle any errors gracefully
    console.error("Error executing user code:", error.message);
    console.error("Code causing error:", cleanedCode);
  }
};
