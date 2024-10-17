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

import { date } from "quasar";
import { DateTime as _DateTime } from "luxon";

// Parses the duration string and returns the number is seconds
export const parseDuration = (durationString: string) => {
  const regex = /^(\d+)([smhdwMy])?$/;
  const match = durationString.match(regex);
  if (match) {
    const value = parseInt(match[1]);
    const unit = match[2];
    if (unit === "s") {
      return value;
    } else if (unit === "m") {
      return value * 60;
    } else if (unit === "h") {
      return value * 3600;
    } else if (unit === "d") {
      return value * 86400;
    } else if (unit === "w") {
      return value * 604800;
    } else if (unit === "M") {
      return value * 2592000;
    } else if (unit === "y") {
      return value * 31536000;
    } else {
      return value;
    }
  } else {
    return 0;
  }
};

export const generateDurationLabel = (seconds: number) => {
  const units = [
    { label: "s", value: 1 },
    { label: "m", value: 60 },
    { label: "h", value: 3600 },
    { label: "d", value: 86400 },
    { label: "w", value: 604800 },
    { label: "M", value: 2592000 },
    { label: "y", value: 31536000 },
  ];

  const duration = seconds;
  let label = "";

  for (let i = units.length - 1; i >= 0; i--) {
    const unit = units[i];

    if (duration >= unit.value) {
      const unitDuration = Math.floor(duration / unit.value);
      const remainingDuration = duration % unit.value;

      if (remainingDuration === 0) {
        label = `${unitDuration}${unit.label}`;
        break;
      } else {
        label = `${duration}s`;
        break;
      }
    }
  }

  if (label === "") {
    label = `Off`;
  }

  return label;
};

export const getQueryParamsForDuration = (obj: any) => {
  const params: any = {};
  const tab = obj.tab;
  const period = obj.relative.period;
  const from = obj.absolute.date.from + " " + obj.absolute.startTime;
  const to = obj.absolute.date.to + " " + obj.absolute.endTime;

  if (tab === "relative") {
    const labelsToUnitMapping: any = {
      Minutes: "m",
      Hours: "h",
      Days: "d",
      Weeks: "w",
      Months: "M",
    };

    const periodValue = obj.relative.value;
    const periodLabel = period.label;
    const periodUnit = labelsToUnitMapping[periodLabel];

    if (periodLabel === "Minutes" && periodUnit === "m") {
      params["period"] = `${periodValue}${periodUnit}`;
    } else if (periodLabel === "Hours" && periodUnit === "h") {
      params["period"] = `${periodValue}${periodUnit}`;
    } else if (periodLabel === "Days" && periodUnit === "d") {
      params["period"] = `${periodValue}${periodUnit}`;
    } else if (periodLabel === "Weeks" && periodUnit === "w") {
      params["period"] = `${periodValue}${periodUnit}`;
    } else if (periodLabel === "Months" && periodUnit === "M") {
      params["period"] = `${periodValue}${periodUnit}`;
    }
  } else if (tab === "absolute") {
    const fromTime = new Date(from).getTime();
    const toTime = new Date(to).getTime();

    params["from"] = `${fromTime}`;
    params["to"] = `${toTime}`;
  }

  return params;
};

export const getDurationObjectFromParams = (params: any) => {
  const obj = {
    tab: "relative",
    relative: {
      period: { label: "Minutes", value: "Minutes" },
      value: 15,
    },
    absolute: {
      date: {
        from: new Date().toLocaleDateString("en-ZA"),
        to: new Date().toLocaleDateString("en-ZA"),
      },
      startTime: "00:00",
      endTime: "23:59",
    },
  };

  const period = params?.period?.match(/(\d+)([mhdwM])/);
  const from = params?.from?.match(/(\d+)/);
  const to = params?.to?.match(/(\d+)/);

  if (period) {
    const periodValue = period[1];
    const periodUnit = period[2];

    if (periodUnit === "m") {
      obj.relative.period = { label: "Minutes", value: "Minutes" };
    } else if (periodUnit === "h") {
      obj.relative.period = { label: "Hours", value: "Hours" };
    } else if (periodUnit === "d") {
      obj.relative.period = { label: "Days", value: "Days" };
    } else if (periodUnit === "w") {
      obj.relative.period = { label: "Weeks", value: "Weeks" };
    } else if (periodUnit === "M") {
      obj.relative.period = { label: "Months", value: "Months" };
    }

    obj.relative.value = parseInt(periodValue);
  } else if (from && to) {
    const fromTime = parseInt(from[1]);
    const toTime = parseInt(to[1]);

    obj.tab = "absolute";
    obj.absolute.date.from = new Date(fromTime).toLocaleDateString("en-ZA");
    obj.absolute.date.to = new Date(toTime).toLocaleDateString("en-ZA");

    const startTimeDateObj = new Date(fromTime);
    obj.absolute.startTime = `${startTimeDateObj
      .getHours()
      .toString()
      .padStart(2, "0")}:${startTimeDateObj
      .getMinutes()
      .toString()
      .padStart(2, "0")}`;

    const toTimeDateObj = new Date(toTime);
    obj.absolute.endTime = `${toTimeDateObj
      .getHours()
      .toString()
      .padStart(2, "0")}:${toTimeDateObj
      .getMinutes()
      .toString()
      .padStart(2, "0")}`;
  }

  return obj;
};

export const getConsumableRelativeTime = (period: string) => {
  const periodString = period?.match(/(\d+)([smhdwM])/);
  if (periodString) {
    let periodValue: number = parseInt(periodString[1]);
    let periodUnit: string = periodString[2];

    // quasar does not support arithmetic on weeks. convert to days.
    if (periodUnit == "w") {
      periodUnit = "d";
      periodValue = periodValue * 7;
    }

    const subtractObject =
      '{"' +
      getRelativePeriod(periodUnit).toLowerCase() +
      '":' +
      periodValue +
      "}";

    const endTimeStamp = new Date();

    const startTimeStamp = date.subtractFromDate(
      endTimeStamp,
      JSON.parse(subtractObject)
    );

    return {
      startTime: new Date(startTimeStamp).getTime() * 1000,
      endTime: new Date(endTimeStamp).getTime() * 1000,
    };
  }
};

export const getRelativePeriod = (period: string) => {
  const mapping: { [key: string]: string } = {
    s: "Seconds",
    m: "Minutes",
    h: "Hours",
    d: "Days",
    w: "Weeks",
    M: "Months",
  };
  return mapping[period];
};

export const isInvalidDate = (date: any) => {
  return !(date && date instanceof Date && !isNaN(date.getTime()));
};

export const convertUnixToQuasarFormat = (unixMicroseconds: any) => {
  try {
    if (!unixMicroseconds) return "";
    const unixSeconds = unixMicroseconds / 1e6;
    const dateToFormat = new Date(unixSeconds * 1000);
    const formattedDate = dateToFormat.toISOString();
    return date.formatDate(formattedDate, "YYYY-MM-DDTHH:mm:ssZ");
  } catch (error) {
    console.log("Error converting unix to quasar format");
    return "";
  }
};

/**
 * @param {string} date - date in DD-MM-YYYY format
 * @param {string} time - time in HH:MM 24hr format
 * @param {string} timezone - timezone
 */
export const convertDateToTimestamp = (
  date: string,
  time: string,
  timezone: string
) => {
  try {
    const browserTime =
      "Browser Time (" + Intl.DateTimeFormat().resolvedOptions().timeZone + ")";

    const [day, month, year] = date.split("-");
    const [hour, minute] = time.split(":");

    const _date = {
      year: Number(year),
      month: Number(month),
      day: Number(day),
      hour: Number(hour),
      minute: Number(minute),
    };

    if (timezone.toLowerCase() == browserTime.toLowerCase()) {
      timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    }

    // Create a DateTime instance from date and time, then set the timezone
    const dateTime = _DateTime.fromObject(_date, { zone: timezone });

    // Convert the DateTime to a Unix timestamp in milliseconds
    const unixTimestampMillis = dateTime.toMillis();

    return { timestamp: unixTimestampMillis * 1000, offset: dateTime.offset }; // timestamp in microseconds
  } catch (error) {
    console.log("Error converting date to timestamp");
    return { timestamp: 0, offset: 0 };
  }
};
