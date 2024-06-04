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

import { convertPromQLData } from "@/utils/dashboard/convertPromQLData";
import { convertSQLData } from "@/utils/dashboard/convertSQLData";
import { convertTableData } from "@/utils/dashboard/convertTableData";
import { convertMapData } from "@/utils/dashboard/convertMapData";
import { convertSankeyData } from "./convertSankeyData";
/**
 * Converts panel data based on the panel schema and data.
 *
 * @param {any} panelSchema - The panel schema.
 * @param {any} data - The data to be converted.
 * @return {any} The converted data.
 */
export const convertPanelData = async (
  panelSchema: any,
  data: any,
  store: any,
  chartPanelRef: any,
  hoveredSeriesState: any
) => {
  // based on the panel config, using the switch calling the appropriate converter
  // based on panel Data chartType is taken for ignoring unnecessary api calls
  switch (panelSchema.type) {
    case "area":
    case "area-stacked":
    case "bar":
    case "h-bar":
    case "stacked":
    case "heatmap":
    case "h-stacked":
    case "line":
    case "pie":
    case "donut":
    case "scatter":
    case "metric":
    case "gauge": {
      if (
        // panelSchema?.fields?.stream_type == "metrics" &&
        // panelSchema?.customQuery &&
        panelSchema?.queryType == "promql"
      ) {
        // chartpanelref will be used to get width and height of the chart element from DOM
        return {
          chartType: panelSchema.type,
          ...(await convertPromQLData(
            panelSchema,
            data,
            store,
            chartPanelRef,
            hoveredSeriesState
          )),
        };
      } else {
        // chartpanelref will be used to get width and height of the chart element from DOM
        return {
          chartType: panelSchema.type,
          ...(await convertSQLData(
            panelSchema,
            data,
            store,
            chartPanelRef,
            hoveredSeriesState
          )),
        };
      }
    }
    case "table": {
      return {
        chartType: panelSchema.type,
        ...convertTableData(panelSchema, data),
      };
    }
    case "geomap": {
      return {
        chartType: panelSchema.type,
        ...convertMapData(panelSchema, data),
      };
    }
    case "sankey": {
      return {
        chartType: panelSchema.type,
        ...convertSankeyData(panelSchema, data),
      };
    }
    default: {
      console.log("No Chart Type found, skipping");
      return {};
    }
  }
};
