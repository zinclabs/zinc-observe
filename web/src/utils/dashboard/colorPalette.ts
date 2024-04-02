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

export const colorArrayBySeries = [
  "#5470c6",
  "#91cc75",
  "#fac858",
  "#ee6666",
  "#73c0de",
  "#3ba272",
  "#fc8452",
  "#9a60b4",
  "#dc1e1e",
  "#9086e9",
  "#27e162",
  "#e08de6",
  "#de6086",
  "#ec8ceb",
  "#99e9f1",
  "#eec59c",
];

// Convert a hex color to HSL
function hexToHsl(hex) {
  let r = parseInt(hex.slice(1, 3), 16) / 255;
  let g = parseInt(hex.slice(3, 5), 16) / 255;
  let b = parseInt(hex.slice(5, 7), 16) / 255;

  let max = Math.max(r, g, b),
    min = Math.min(r, g, b);
  let h,
    s,
    l = (max + min) / 2;

  if (max === min) {
    h = s = 0; // achromatic
  } else {
    let d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0);
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      case b:
        h = (r - g) / d + 4;
        break;
    }
    h /= 6;
  }

  return { h, s, l };
}

// Convert an HSL color to hex
function hslToHex(hsl) {
  let r, g, b;

  if (hsl.s === 0) {
    r = g = b = hsl.l; // achromatic
  } else {
    const hue2rgb = (p, q, t) => {
      if (t < 0) t += 1;
      if (t > 1) t -= 1;
      if (t < 1 / 6) return p + (q - p) * 6 * t;
      if (t < 1 / 2) return q;
      if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
      return p;
    };
    let q = hsl.l < 0.5 ? hsl.l * (1 + hsl.s) : hsl.l + hsl.s - hsl.l * hsl.s;
    let p = 2 * hsl.l - q;
    r = hue2rgb(p, q, hsl.h + 1 / 3);
    g = hue2rgb(p, q, hsl.h);
    b = hue2rgb(p, q, hsl.h - 1 / 3);
  }

  let toHex = (x) => {
    let hex = Math.round(x * 255).toString(16);
    return hex.length === 1 ? "0" + hex : hex;
  };

  return "#" + toHex(r) + toHex(g) + toHex(b);
}

// Get a color for a name
function getColorForName(name) {
  var hash = hashName(name);
  var colorIndex = hash % colorArrayBySeries.length;
  var baseColor = hexToHsl(colorArrayBySeries[colorIndex]);

  // create a unique hue, saturation, and lightness value
  baseColor.h = (baseColor.h + (hash % 360) / 360.0) % 1;
  baseColor.s = baseColor.s + ((hash % 20) - 10) / 100.0; // vary by up to 10%
  baseColor.l = baseColor.l + ((hash % 20) - 10) / 100.0; // vary by up to 10%

  return hslToHex(baseColor);
}

// Define your hash function
function hashName(name) {
  var hash = 0;
  for (var i = 0; i < name.length; i++) {
    hash += name.charCodeAt(i);
  }
  return hash;
}

// // Convert a hex color to RGB
// function hexToRgb(hex: any) {
//     var result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
//     return result ? {
//         r: parseInt(result[1], 16),
//         g: parseInt(result[2], 16),
//         b: parseInt(result[3], 16)
//     } : null;
// }

// // Convert an RGB color to hex
// function rgbToHex(rgb) {
//     return "#" + ((1 << 24) | (rgb.r << 16) | (rgb.g << 8) | rgb.b).toString(16).slice(1);
// }

// // Get a color for a name
// function getColorForName(name: any) {
//   var hash = hashName(name);
//   var colorIndex = hash % colorArrayBySeries.length;
//   var baseColor: any = hexToRgb(colorArrayBySeries[colorIndex]);

//   // Create a unique offset for each RGB value
//   var offset = Math.floor(hash / 256);

//   var gradientColor = {
//       r: (baseColor.r + offset) % 256,
//       g: (baseColor.g + offset * 2) % 256,
//       b: (baseColor.b + offset * 3) % 256
//   };

//   return rgbToHex(gradientColor);
// }

export const getIndexForName = (name: any, colorArray: any) => {
  let index = 0;
  for (let i = 0; i < 15; i++) {
    const charCode = name.charCodeAt(i) || 0;
    index += charCode * (i + 1);
  }
  return index % colorArray.length;
};

// const nameToColor = (name: any, colorArray: any) => {
//   const index = getIndexForName(name, colorArray);
//   return colorArray[index];
// };
const usedColors = new Map();
const nameColorMap = new Map();

const adjustColor = (color, amount) => {
  const usePound = color[0] === "#";
  const num = usePound ? color.slice(1) : color;
  const scale = amount < 0 ? 0 : 255;
  const inverseScale = amount < 0 ? -amount : 1 - amount;

  const r =
    scale -
    Math.round((scale - parseInt(num.substring(0, 2), 16)) * inverseScale);
  const g =
    scale -
    Math.round((scale - parseInt(num.substring(2, 4), 16)) * inverseScale);
  const b =
    scale -
    Math.round((scale - parseInt(num.substring(4, 6), 16)) * inverseScale);

  return (
    (usePound ? "#" : "") +
    ((0x1000000 + (r << 16)) | (g << 8) | b).toString(16).slice(1)
  );
};

const nameToColor = (name, colorArray) => {
  if (nameColorMap.has(name)) {
    return nameColorMap.get(name);
  }

  const index = getIndexForName(name, colorArray);
  let color = colorArray[index];

  if (usedColors.has(color)) {
    const amount = usedColors.get(color) * 99; // Adjust the shade by 10% each time
    color = adjustColor(color, amount);
    usedColors.set(color, usedColors.get(color) + 1);
  } else {
    usedColors.set(color, 1);
  }

  nameColorMap.set(name, color);
  return color;
};

function stringToColor(str) {
  // Create a hash from the string
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash);
  }

  // Convert the hash to an HSL color
  let hue = hash % 360;
  let saturation = (hash % 25) + 100; // Saturation range between 75-100%
  let lightness = (hash % 15) + 100; // Lightness range between 70-85%

  return "hsl(" + hue + ", " + saturation + "%, " + lightness + "%)";
}

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
        ? values[values.length - 1][1]
        : 50
      : 50;
  if (["shades", "continuous"].includes(panelSchema?.config?.color?.mode)) {
    if (panelSchema?.config?.color?.seriesBy == "min") {
      values.forEach((value: any) => {
        // value[1] should not NaN
        if (!isNaN(value[1])) {
          seriesvalue = Math.min(value[1], seriesvalue);
        }
      });
    } else if (panelSchema?.config?.color?.seriesBy == "max") {
      values.forEach((value: any) => {
        // value[1] should not NaN
        if (!isNaN(value[1])) {
          seriesvalue = Math.max(value[1], seriesvalue);
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
  if (["shades", "continuous"].includes(panelSchema?.config?.color?.mode)) {
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

class Scale {
  colors: any;
  domain: any;

  constructor(colors: any) {
    this.colors = colors;
    this.domain = [0, 100];
  }

  setDomain(domain: any) {
    this.domain = domain;
  }

  getColor(value: any) {
    // NOTE: need to check value should not be NaN
    // and it should not be less than domain[0] or greater than domain[1]

    let t = (value - this.domain[0]) / (this.domain[1] - this.domain[0]);
    let index = Math.floor(t * (this.colors.length - 1));
    let color1 = this.colors[index];
    let color2 = this.colors[index + 1] || color1; // use the same color if color2 is not defined
    let tLocal = color2 === color1 ? 0 : t * (this.colors.length - 1) - index; // calculate local interpolation parameter
    let interpolated = color1.interpolate(color2, tLocal);

    return interpolated;
  }
}

class Color {
  r: any;
  g: any;
  b: any;

  constructor(hex: any) {
    // NOTE: use default color if hex is not valid
    let result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    if (!result) {
      throw new Error(`Invalid color: ${hex}`);
    }
    this.r = parseInt(result[1], 16);
    this.g = parseInt(result[2], 16);
    this.b = parseInt(result[3], 16);
  }

  interpolate(other: any, t: any) {
    let r = this.r + t * (other.r - this.r);
    let g = this.g + t * (other.g - this.g);
    let b = this.b + t * (other.b - this.b);

    return new Color(
      this.rgbToHex(Math.round(r), Math.round(g), Math.round(b))
    );
  }

  rgbToHex(r: any, g: any, b: any) {
    return "#" + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1);
  }

  toCssString() {
    return `rgb(${Math.round(this.r)}, ${Math.round(this.g)}, ${Math.round(
      this.b
    )})`;
  }
}

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

    case "palette-classic": {
      // return color using colorArrayBySeries
      // NOTE: here value will be series name
      // return getColorForName(seriesName);
      //   function stringToColor(str) {
      //     let hash = 0;
      //     for (let i = 0; i < str.length; i++) {
      //         hash = str.charCodeAt(i) + ((hash << 5) - hash);
      //     }

      //     // Convert the hash to a number between 0 and 360
      //     let hue = hash % 361;

      //     // Generate the HSL color
      //     return `hsl(${hue}, 50%, 50%)`;
      // }
      // return stringToColor(seriesName);
      function hashCode(s) {
        let hash = 0;
        for (let i = 0; i < s.length; i++) {
          hash = (Math.imul(31, hash) + s.charCodeAt(i)) | 0;
        }
        return Math.abs(hash);
      }

      function hslToRgb(h, s, l) {
        let r, g, b;

        if (s == 0) {
          r = g = b = l; // achromatic
        } else {
          let hue2rgb = function hue2rgb(p, q, t) {
            if (t < 0) t += 1;
            if (t > 1) t -= 1;
            if (t < 1 / 6) return p + (q - p) * 6 * t;
            if (t < 1 / 2) return q;
            if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
            return p;
          };

          let q = l < 0.5 ? l * (1 + s) : l + s - l * s;
          let p = 2 * l - q;
          r = hue2rgb(p, q, h + 1 / 3);
          g = hue2rgb(p, q, h);
          b = hue2rgb(p, q, h - 1 / 3);
        }

        return [Math.round(r * 255), Math.round(g * 255), Math.round(b * 255)];
      }

      function generateColorFromName(name: any, s = 0.8, l = 0.7) {
        let hash = hashCode(name);
        let h = (hash % 360) / 360;
        let rgb = hslToRgb(h, s, l);
        let hex =
          "#" +
          rgb
            .map((x) => {
              const hex = x.toString(16);
              return hex.length === 1 ? "0" + hex : hex;
            })
            .join("");
        return hex;
      }

      return generateColorFromName(seriesName, 0.8, 0.7);
    }

    case "green-yellow-red": {
      const value =
        panelSchema?.queryType == "promql"
          ? getSeriesValueFrom2DArray(panelSchema, valuesArr) ?? 50
          : getSeriesValueFromArray(panelSchema, valuesArr) ?? 50;

      let scale = new Scale(
        panelSchema?.config?.color?.fixedColor?.map(
          (color: any) => new Color(color)
        )
      );

      scale.setDomain([chartMin ?? 0, chartMax ?? 100]);

      return scale.getColor(value).toCssString();
    }

    case "red-yellow-green": {
      const value =
        panelSchema?.queryType == "promql"
          ? getSeriesValueFrom2DArray(panelSchema, valuesArr) ?? 50
          : getSeriesValueFromArray(panelSchema, valuesArr) ?? 50;

      let scale = new Scale(
        panelSchema?.config?.color?.fixedColor?.map(
          (color: any) => new Color(color)
        )
      );

      scale.setDomain([chartMin ?? 0, chartMax ?? 100]);

      return scale.getColor(value).toCssString();
    }

    // case "scheme":

    // case "opacity":

    default: {
      // return color using colorArrayBySeries
      return getColorForName(seriesName);
    }
  }
};
