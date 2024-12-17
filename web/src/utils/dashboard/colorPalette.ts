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

import { scaleLinear } from "d3-scale";
import { isNumber } from "lodash-es";

export enum ColorModeWithoutMinMax {
  PALETTE_CLASSIC_BY_SERIES = "palette-classic-by-series",
  PALETTE_CLASSIC = "palette-classic",
  FIXED = "fixed",
}

export const classicColorPalette = [
  "#FFCDD2",
  "#EF9A9A",
  "#E57373",
  "#EF5350",
  "#F8BBD0",
  "#F48FB1",
  "#F06292",
  "#EC407A",
  "#E1BEE7",
  "#CE93D8",
  "#BA68C8",
  "#AB47BC",
  "#D1C4E9",
  "#B39DDB",
  "#9575CD",
  "#7E57C2",
  "#C5CAE9",
  "#9FA8DA",
  "#7986CB",
  "#5C6BC0",
  "#BBDEFB",
  "#90CAF9",
  "#64B5F6",
  "#42A5F5",
  "#B3E5FC",
  "#81D4FA",
  "#4FC3F7",
  "#29B6F6",
  "#B2EBF2",
  "#80DEEA",
  "#4DD0E1",
  "#26C6DA",
  "#B2DFDB",
  "#80CBC4",
  "#4DB6AC",
  "#26A69A",
  "#C8E6C9",
  "#A5D6A7",
  "#81C784",
  "#66BB6A",
  "#DCEDC8",
  "#C5E1A5",
  "#AED581",
  "#9CCC65",
  "#F0F4C3",
  "#E6EE9C",
  "#DCE775",
  "#D4E157",
  "#FFF9C4",
  "#FFF59D",
  "#FFF176",
  "#FFEE58",
  "#FFECB3",
  "#FFE082",
  "#FFD54F",
  "#FFCA28",
  "#FFE0B2",
  "#FFCC80",
  "#FFB74D",
  "#FFA726",
  "#FFCCBC",
  "#FFAB91",
  "#FF8A65",
  "#FF7043",
];

const isValidHexColor = (color: string): boolean => {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(color);
  return result !== null;
};

export const shadeColor = (
  color: string,
  value: number,
  min: number,
  max: number,
): string | null => {
  if (!isValidHexColor(color)) {
    return null;
  }
  if (
    typeof value !== "number" ||
    typeof min !== "number" ||
    typeof max !== "number"
  ) {
    return null;
  }
  let percent = 0;
  if (max === min) {
    percent = 0;
  } else {
    percent = (value - min) / (max - min);
  }
  const num = parseInt(color.replace("#", ""), 16),
    amt = Math.round(1.55 * percent * 100),
    R = ((num >> 16) & 0xff) + amt,
    G = ((num >> 8) & 0xff) + amt,
    B = (num & 0xff) + amt;

  const newColor = (
    0x1000000 +
    (R < 255 ? (R < 0 ? 0 : R) : 255) * 0x10000 +
    (G < 255 ? (G < 0 ? 0 : G) : 255) * 0x100 +
    (B < 255 ? (B < 0 ? 0 : B) : 255)
  )
    .toString(16)
    .slice(1);
  return "#" + newColor;
};

interface MetricResult {
  values: [number, number][];
}

interface Metric {
  result?: MetricResult[];
}

export const getMetricMinMaxValue = (
  searchQueryData: Metric[],
): [number, number] => {
  let min = Infinity;
  let max = -Infinity;

  const allValues = searchQueryData
    .flatMap((metric) => metric.result ?? [])
    .flatMap((result) => result.values ?? [])
    .map(([, value]) => value)
    .filter((value) => !Number.isNaN(value));

  if (allValues.length > 0) {
    min = Math.min(...allValues);
    max = Math.max(...allValues);
  }

  return [min, max];
};

interface SQLData {
  [key: string]: number | null | undefined;
}

export const getSQLMinMaxValue = (
  yaxiskeys: string[],
  searchQueryData: SQLData[],
): [number, number] => {
  // need min and max value for color
  let min = Infinity;
  let max = -Infinity;

  searchQueryData?.forEach((data: SQLData) => {
    yaxiskeys?.forEach((key: string) => {
      if (
        data[key] !== undefined &&
        isNumber(data[key]) &&
        !Number.isNaN(data[key]) &&
        data[key] !== null
      ) {
        min = Math.min(min, data[key]);
        max = Math.max(max, data[key]);
      }
    });
  });

  return [min, max];
};

// need one function which will take some series name and will return hash which will be between 1 to 1000
const getSeriesHash = (seriesName: string) => {
  // Initialize a hash variable
  let hash = 0;
  const classicColorPaletteLength = classicColorPalette.length;
  // If the seriesName is empty, return 1 as a default hash value
  if (seriesName.length === 0) return 1;

  // Loop through each character in the series name
  for (let i = 0; i < seriesName.length; i++) {
    const char = seriesName.charCodeAt(i); // Get the Unicode value of the character
    hash = (hash << 5) - hash + char; // Bitwise hash computation
    hash |= 0; // Convert to 32-bit integer
  }

  // Ensure the hash is positive and between 0 to 99
  return Math.abs(hash) % classicColorPaletteLength;
};

type SeriesBy = "last" | "min" | "max";

const getSeriesValueBasedOnSeriesBy = (
  values: any[],
  seriesBy: SeriesBy,
): number | null => {
  try {
    const validValues = values.filter(
      (value) =>
        value != null &&
        value !== "" &&
        isNumber(value) &&
        !Number.isNaN(value),
    );

    if (validValues.length === 0) return null;

    switch (seriesBy) {
      case "last":
        return +validValues[validValues.length - 1];
      case "min":
        return Math.min(...validValues);

      case "max":
        return Math.max(...validValues);
      default:
        return +validValues[validValues.length - 1];
    }
  } catch (error) {
    return null;
  }
};

interface ColorConfig {
  mode:
    | "fixed"
    | "shades"
    | "palette-classic-by-series"
    | "palette-classic"
    | "continuous-green-yellow-red"
    | "continuous-red-yellow-green"
    | "continuous-temperature"
    | "continuous-positive"
    | "continuous-negative"
    | "continuous-light-to-dark-blue";
  fixedColor?: string[];
  seriesBy?: SeriesBy;
}

function getDomainPartitions(
  chartMin: number,
  chartMax: number,
  numPartitions: number,
) {
  // If there's only one partition, return just chartMin and chartMax
  if (numPartitions < 2) {
    return [chartMin, chartMax];
  }

  const partitions = [];
  const step = (chartMax - chartMin) / (numPartitions - 1);

  for (let i = 0; i < numPartitions; i++) {
    partitions.push(chartMin + i * step);
  }

  return partitions;
}

export const getSeriesColor = (
  colorCfg: ColorConfig | null,
  seriesName: string,
  value: number[],
  chartMin: number,
  chartMax: number,
): string | null => {
  if (!colorCfg) {
    return classicColorPalette[getSeriesHash(seriesName)];
  } else if (colorCfg.mode === "fixed") {
    return colorCfg?.fixedColor?.[0] ?? "#53ca53";
  } else if (colorCfg.mode === "shades") {
    return shadeColor(
      colorCfg?.fixedColor?.[0] ?? "#53ca53",
      getSeriesValueBasedOnSeriesBy(value, "last") ?? chartMin,
      chartMin,
      chartMax,
    );
  } else if (colorCfg.mode === "palette-classic-by-series") {
    return classicColorPalette[getSeriesHash(seriesName)];
  } else if (colorCfg.mode === "palette-classic") {
    return null;
  } else {
    const d3ColorObj = scaleLinear(
      getDomainPartitions(
        chartMin,
        chartMax,
        colorCfg?.fixedColor?.length ?? classicColorPalette.length,
      ),
      colorCfg?.fixedColor?.length ? colorCfg.fixedColor : classicColorPalette,
    );
    return d3ColorObj(
      (getSeriesValueBasedOnSeriesBy(value, colorCfg?.seriesBy ?? "last") ??
        chartMin) as number,
    );
  }
};
