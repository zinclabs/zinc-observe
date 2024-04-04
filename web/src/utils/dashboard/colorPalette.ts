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

function shadeColor(color: any, value: any, min: any, max: any) {
  let percent = (value - min) / (max - min);
  let num = parseInt(color.replace("#", ""), 16),
    amt = Math.round(1.55 * percent * 100),
    R = (num >> 16) + amt,
    B = ((num >> 8) & 0x00ff) + amt,
    G = (num & 0x0000ff) + amt;

  let newColor = (
    0x1000000 +
    (R < 255 ? (R < 1 ? 0 : R) : 255) * 0x10000 +
    (B < 255 ? (B < 1 ? 0 : B) : 255) * 0x100 +
    (G < 255 ? (G < 1 ? 0 : G) : 255)
  )
    .toString(16)
    .slice(1);
  return "#" + newColor;
}

export const getMetricMinMaxValue = (searchQueryData: any) => {
  // need min and max value for color
  let min = Infinity;
  let max = -Infinity;
  searchQueryData.forEach((metric: any) => {
    if (metric.result && Array.isArray(metric.result)) {
      metric?.result?.forEach((valuesArr: any) => {
        if (valuesArr.values && Array.isArray(valuesArr.values)) {
          valuesArr.values.forEach((val: any) => {
            // val[1] should not NaN
            if (!isNaN(val[1])) {
              min = Math.min(min, val[1]);
              max = Math.max(max, val[1]);
            }
          });
        }
      });
    }
  });
  return [min, max];
};

export const getSQLMinMaxValue = (yaxiskeys: any, searchQueryData: any) => {
  // need min and max value for color
  let min = Infinity;
  let max = -Infinity;

  searchQueryData[0]?.forEach((data: any) => {
    yaxiskeys?.forEach((key: any) => {
      if (data[key] !== undefined && !isNaN(data[key]) && data[key] !== null) {
        min = Math.min(min, data[key]);
        max = Math.max(max, data[key]);
      }
    });
  });

  return [min, max];
};

const getSeriesValueFrom2DArray = (panelSchema: any, values: any) => {
  // if color is based on value then need to find seriesmin or seriesmax or last value
  let seriesvalue =
    values?.length > 0
      ? !isNaN(values[values.length - 1][1])
        ? +values[values.length - 1][1]
        : 50
      : 50;

  if (
    [
      "shades",
      "continuous-green-yellow-red",
      "continuous-red-yellow-green",
    ].includes(panelSchema?.config?.color?.mode)
  ) {
    if (panelSchema?.config?.color?.seriesBy == "min") {
      values.forEach((value: any) => {
        // value[1] should not NaN
        if (!isNaN(value[1])) {
          seriesvalue = +Math.min(+value[1], +seriesvalue);
        }
      });
    } else if (panelSchema?.config?.color?.seriesBy == "max") {
      values.forEach((value: any) => {
        // value[1] should not NaN
        if (!isNaN(value[1])) {
          seriesvalue = +Math.max(+value[1], +seriesvalue);
        }
      });
    }
  }

  return seriesvalue;
};

const getSeriesValueFromArray = (panelSchema: any, values: any) => {
  // if color is based on value then need to find seriesmin or seriesmax or last value
  let seriesvalue =
    values?.length > 0
      ? !isNaN(values[values.length - 1])
        ? values[values.length - 1]
        : 50
      : 50;
  if (
    [
      "shades",
      "continuous-green-yellow-red",
      "continuous-red-yellow-green",
    ].includes(panelSchema?.config?.color?.mode)
  ) {
    if (panelSchema?.config?.color?.seriesBy == "min") {
      values.forEach((value: any) => {
        // value[1] should not NaN
        if (!isNaN(value)) {
          seriesvalue = Math.min(value, seriesvalue);
        }
      });
    } else if (panelSchema?.config?.color?.seriesBy == "max") {
      values.forEach((value: any) => {
        // value[1] should not NaN
        if (!isNaN(value)) {
          seriesvalue = Math.max(value, seriesvalue);
        }
      });
    }
  }

  return seriesvalue;
};

export const classicColorPalette = [
  "#5470c6",
  "#91cc75",
  "#fac858",
  "#ee6666",
  "#73c0de",
  "#3ba272",
  "#fc8452",
  "#9a60b4",
  "#ea7ccc",
  "#59c4e6",
  "#edafda",
  "#93b7e3",
  "#a5e7f0",
  "#cbb0e3",
  "#3fb1e3",
  "#6be6c1",
  "#a0a7e6",
  "#c4ebad",
  "#96dee8",
  "#2ec7c9",
  "#b6a2de",
  "#5ab1ef",
  "#ffb980",
  "#d87a80",
  "#8d98b3",
  "#e5cf0d",
  "#97b552",
  "#dc69aa",
  "#07a2a4",
  "#9a7fd1",
  "#588dd5",
  "#f5994e",
  "#c9ab00",
  "#7eb00a",
  "#fcce10",
  "#b5c334",
  "#fe8463",
  "#9bca63",
  "#fad860",
  "#60c0dd",
  "#c6e579",
  "#f4e001",
  "#f0805a",
  "#26c0c0",
  "#ff715e",
  "#ffaf51",
  "#ffee51",
  "#8c6ac4",
  "#e6b600",
  "#0098d9",
  "#339ca8",
  "#32a487",
  "#d95850",
  "#eb8146",
  "#ffb248",
  "#f2d643",
  "#759aa0",
  "#e69d87",
  "#8dc1a9",
  "#ea7e53",
  "#eedd78",
  "#73b9bc",
  "#91ca8c",
  "#f49f42",
  "#4ea397",
  "#22c3aa",
  "#7bd9a5",
  "#d0648a",
  "#f58db2",
  "#f2b3c9",
  "#b8d2c7",
  "#a4d8c2",
  "#f3d999",
  "#d3758f",
  "#dcc392",
  "#82b6e9",
  "#a092f1",
  "#eaf889",
  "#6699FF",
  "#ff6666",
  "#3cb371",
  "#d5b158",
  "#38b6b6",
  "#5AB2E0",
  "#32C5E9",
  "#94E9EC",
  "#BFEFCF",
  "#FF9F7F",
  "#FB7293",
  "#E26BB3",
  "#9D96F5",
  "#8378EA",
  "#98C0FF",
];

function getColorBasedOnValue(
  currentValue: any,
  minValue: any,
  maxValue: any,
  colorArray: any
) {
  // Calculate the size of each partition
  var partitionSize = (maxValue - minValue) / colorArray.length;

  // Calculate the index based on the current value's position within the partitions
  var index = Math.floor((currentValue - minValue) / partitionSize);

  // Ensure the index is within the bounds of the array
  index = Math.max(0, index); // Ensure the index is not less than 0
  index = Math.min(colorArray.length - 1, index); // Ensure the index is not greater than the array length - 1

  return colorArray[index];
}

function getHashFromSeriesName(str: string) {
  // remove unwanted space
  str = str.trim();

  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash += str.charCodeAt(i);
  }
  return hash;
}

const getColorForSeries = (seriesName: string, colorArray: string[]) => {
  // Calculate the hash of the series name
  let hash = getHashFromSeriesName(seriesName);

  // Use the hash to select a color from the color array
  let index = Math.abs(hash) % colorArray.length;
  return colorArray[index];
};

export const getColor = (
  panelSchema: any,
  seriesName: any,
  valuesArr?: any,
  chartMin?: any,
  chartMax?: any
) => {
  // switch case based on panelSchema.color type
  switch (panelSchema?.config?.color?.mode) {
    case "fixed": {
      return panelSchema?.config?.color?.fixedColor
        ? panelSchema?.config?.color?.fixedColor[0] ?? "#53ca53"
        : "#53ca53";
    }
    case "shades": {
      // based on selected color pass different shades of same color
      return shadeColor(
        panelSchema?.config?.color?.fixedColor
          ? panelSchema?.config?.color?.fixedColor[0] ?? "#53ca53"
          : "#53ca53",
        panelSchema.queryType == "promql"
          ? getSeriesValueFrom2DArray(panelSchema, valuesArr) ?? 50
          : getSeriesValueFromArray(panelSchema, valuesArr) ?? 50,
        chartMin ?? 0,
        chartMax ?? 100
      );
    }

    case "palette-classic-by-series": {
      return getColorForSeries(seriesName, classicColorPalette);
    }

    case "palette-classic": {
      return null;
    }

    case "continuous-green-yellow-red":
    case "continuous-red-yellow-green": {
      const value =
        panelSchema?.queryType == "promql"
          ? getSeriesValueFrom2DArray(panelSchema, valuesArr) ?? 50
          : getSeriesValueFromArray(panelSchema, valuesArr) ?? 50;

      return getColorBasedOnValue(
        value,
        chartMin ?? 0,
        chartMax ?? 100,
        panelSchema?.config?.color?.fixedColor ?? ["5470c6"]
      );
    }

    default: {
      // return color using colorArrayBySeries
      return getColorForSeries(seriesName, classicColorPalette);
      // return null
    }
  }
};
