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

declare global {
  interface Window {
    _o2_setDashboardColors: () => any;
  }
}

const defaultColors = [
  // "#a2b6ff",
  // "#889ef9",
  // "#6e87df",
  // "#5470c6",
  // "#385aad",
  // "#c3ffa5",
  // "#aae58d",
  // "#91cc75",
  // "#79b35e",
  // "#619b48",
  // "#ffffa1",
  // "#fffb89",
  // "#ffe170",
  // "#fac858",
  // "#dfaf40",
  // "#ffb1ac",
  // "#ff9794",
  // "#ff7f7d",
  // "#ee6666",
  // "#d24d50",
  // "#c1ffff",
  // "#a6f3ff",
  // "#8dd9f8",
  // "#73c0de",
  // "#59a8c5",
  // "#89eeb9",
  // "#6fd4a1",
  // "#56bb89",
  // "#3ba272",
  // "#1c8a5c",
  // "#ffce98",
  // "#ffb580",
  // "#ff9c69",
  // "#fc8452",
  // "#e06c3c",
  // "#e6a7ff",
  // "#cc8fe6",
  // "#b377cd",
  // "#9a60b4",
  // "#824a9c",
  // "#ffc7ff",
  // "#ffaeff",
  // "#ff95e5",
  // "#ea7ccc",
  // "#d064b3",
  "#F54337",
  "#4CB050",
  "#03A9F5",
  "#FFEB3C",
  "#9C28B1",
  "#009788",
  "#2196F3",
  "#FE5722",
  "#8CC34B",
  "#673BB7",
  "#FEC107",
  "#00BCD5",
  "#EA1E63",
  "#CDDC39",
  "#FF9801",
  "#3F51B5",
  "#EF999A",
  "#A5D6A7",
  "#81D5FA",
  "#FFF59C",
  "#CF93D9",
  "#80CBC4",
  "#90CAF8",
  "#FFAB91",
  "#C5E1A6",
  "#B39DDB",
  "#FFE083",
  "#80DEEA",
  "#F48FB1",
  "#E6EE9B",
  "#FFCC80",
  "#9EA8DB",
  "#B71B1C",
  "#1C5E20",
  "#02569C",
  "#F57E16",
  "#4A148C",
  "#014D41",
  "#0E47A1",
  "#BF360C",
  "#33691E",
  "#301B92",
  "#FF6F00",
  "#016064",
  "#890E4F",
  "#817716",
  "#E65100",
  "#1A237E",
  "#FF5253",
  "#69F0AE",
  "#3FC4FF",
  "#FFFF01",
  "#E040FC",
  "#64FEDA",
  "#4489FE",
  "#FF6E41",
  "#B2FF59",
  "#7C4DFF",
  "#FFD741",
  "#19FFFF",
  "#FF4181",
  "#EEFF41",
  "#FFAB40",
  "#536DFE",
  "#D60000",
  "#01C853",
  "#0091EA",
  "#FFD600",
  "#AA00FF",
  "#01BFA5",
  "#2A62FF",
  "#DD2C00",
  "#64DD16",
  "#6200EB",
  "#FFAB00",
  "#00B8D4",
  "#C41162",
  "#AEEA00",
  "#FF6D00",
  "#304FFF",
];

export const getClassicColorPalette = () => {
  return window._o2_setDashboardColors || defaultColors;
};

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
  let num = parseInt(color.replace("#", ""), 16),
    amt = Math.round(1.55 * percent * 100),
    R = ((num >> 16) & 0xff) + amt,
    G = ((num >> 8) & 0xff) + amt,
    B = (num & 0xff) + amt;

  let newColor = (
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
  const classicColorPaletteLength = getClassicColorPalette().length;
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
    switch (seriesBy) {
      case "last":
        for (let i = values.length - 1; i >= 0; i--) {
          if (
            !Number.isNaN(values[i]) &&
            isNumber(values[i]) &&
            values[i] != null &&
            values[i] != ""
          ) {
            return +values[i];
          }
        }
        return null;
      case "min":
        return Math.min(...values) ?? null;
      case "max":
        return Math.max(...values) ?? null;
      default:
        for (let i = values.length - 1; i >= 0; i--) {
          if (
            !Number.isNaN(values[i]) &&
            isNumber(values[i]) &&
            values[i] != null &&
            values[i] != ""
          ) {
            return +values[i];
          }
        }
        return null;
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
  const palette: any = getClassicColorPalette();
  if (!colorCfg) {
    return palette[getSeriesHash(seriesName)];
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
    return palette[getSeriesHash(seriesName)];
  } else if (colorCfg.mode === "palette-classic") {
    return null;
  } else {
    const d3ColorObj: any = scaleLinear(
      getDomainPartitions(
        chartMin,
        chartMax,
        colorCfg?.fixedColor?.length ?? palette.length,
      ),
      colorCfg?.fixedColor?.length ? colorCfg.fixedColor : palette,
    );
    return d3ColorObj(
      getSeriesValueBasedOnSeriesBy(value, colorCfg?.seriesBy ?? "last") ??
        chartMin,
    );
  }
};
