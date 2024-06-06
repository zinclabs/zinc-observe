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

import config from "../aws-exports";
import { ref } from "vue";
import { DateTime } from "luxon";
import { v4 as uuidv4 } from "uuid";
import { useQuasar } from "quasar";
import { useStore } from "vuex";
import useStreams from "@/composables/useStreams";
import userService from "@/services/users";

let moment: any;
let momentInitialized = false;

const importMoment = async () => {
  if (!momentInitialized) {
    const momentModule: any = await import("moment-timezone");
    moment = momentModule.default;
    momentInitialized = true;
  }
  return moment;
};

const useLocalStorage = (
  key: string,
  defaultValue: unknown,
  isDelete: boolean = false,
  isJSONValue: boolean = false
) => {
  try {
    const value = ref(defaultValue);
    const read = () => {
      const v = window.localStorage.getItem(key);
      if (v != null && isJSONValue === true) value.value = JSON.parse(v);
      else if (v != null) value.value = v;
      else value.value = null;
    };

    read();

    window.addEventListener("load", () => {
      window.addEventListener("storage", read);
    });

    // window.addEventListener("unload", () => {
    //   alert('asd')
    //   window.removeEventListener("storage", read);
    // });

    // onMounted(() => {
    //   window.addEventListener("storage", read);
    // });
    // onUnmounted(() => {
    //   alert("ad");
    //   window.removeEventListener("storage", read);
    // });

    const write = () => {
      const val: unknown = isJSONValue
        ? JSON.stringify(defaultValue)
        : defaultValue;
      window.localStorage.setItem(key, String(val));
    };

    if (
      window.localStorage.getItem(key) == null &&
      !isDelete &&
      defaultValue != ""
    )
      write();
    else if (value.value != defaultValue && defaultValue != "") write();

    const remove = () => {
      window.localStorage.removeItem(key);
    };

    if (isDelete) {
      remove();
    }

    return value;
  } catch (e) {
    console.log(
      `Error: Error in UseLocalStorage for key: ${key}, error-message : ${e}`
    );
  }
};

export const getUserInfo = (loginString: string) => {
  try {
    let decToken = null;
    const tokens = loginString.substring(1).split("&");
    for (const token of tokens) {
      const propArr = token.split("=");
      if (propArr[0] == "id_token") {
        const tokenString = propArr[1];
        const parts = tokenString.split(".");
        if (parts.length === 3) {
          try {
            // Attempt to parse the token as a JWT
            const header = JSON.parse(atob(parts[0]));
            const payload = JSON.parse(atob(parts[1]));
            const signature = parts[2];
            payload["family_name"] = payload["name"];
            payload["given_name"] = "";
            const encodedSessionData: any = b64EncodeStandard(
              JSON.stringify(payload)
            );
            useLocalUserInfo(encodedSessionData);
            decToken = payload;
          } catch (error) {
            // If parsing fails, it's not a valid JWT
            console.error("Invalid JWT token:", error);
            return null;
          }
        } else {
          decToken = getDecodedAccessToken(propArr[1]);
          const encodedSessionData: any = b64EncodeStandard(
            JSON.stringify(decToken)
          );
          useLocalUserInfo(encodedSessionData);
        }
      }
    }

    return decToken;
  } catch (e) {
    console.log(`Error in getUserInfo util with loginString: ${loginString}`);
  }
};

export const invlidateLoginData = () => {
  userService.logout().then((res: any) => {});
};

export const getLoginURL = () => {
  return `https://${config.oauth.domain}/oauth/v2/authorize?client_id=${config.aws_user_pools_web_client_id}&response_type=${config.oauth.responseType}&redirect_uri=${config.oauth.redirectSignIn}&scope=${config.oauth.scope}`;
};

export const getLogoutURL = () => {
  return `https://${config.oauth.domain}/oidc/v1/end_session?client_id=${
    config.aws_user_pools_web_client_id
  }&id_token_hint=${useLocalUserInfo()}&post_logout_redirect_uri=${
    config.oauth.redirectSignOut
  }&state=random_string`;
};

export const getDecodedAccessToken = (token: string) => {
  try {
    const decodedString = b64DecodeStandard(token.split(".")[1]);
    if (typeof decodedString == "string") {
      return JSON.parse(decodedString);
    } else {
      return "";
    }
  } catch (e) {
    console.log("error decoding token");
  }
};

// url safe base64 encode
export const b64EncodeUnicode = (str: string) => {
  try {
    return btoa(
      encodeURIComponent(str).replace(/%([0-9A-F]{2})/g, (match, p1) => {
        return String.fromCharCode(parseInt(`0x${p1}`));
      })
    )
      .replace(/\+/g, "-")
      .replace(/\//g, "_")
      .replace(/=/g, ".");
  } catch (e) {
    console.log("Error: getBase64Encode: error while encoding.");
    return null;
  }
};

//url safe base64 decode
export const b64DecodeUnicode = (str: string) => {
  try {
    return decodeURIComponent(
      Array.prototype.map
        .call(
          atob(str.replace(/\-/g, "+").replace(/\_/g, "/").replace(/\./g, "=")),
          function (c) {
            return "%" + ("00" + c.charCodeAt(0).toString(16)).slice(-2);
          }
        )
        .join("")
    );
  } catch (e) {
    console.log("Error: getBase64Decode: error while decoding.");
  }
};

export const b64EncodeStandard = (str: string) => {
  try {
    return btoa(
      encodeURIComponent(str).replace(
        /%([0-9A-F]{2})/g,
        function (match, p1: any) {
          return String.fromCharCode(parseInt(`0x${p1}`));
        }
      )
    );
  } catch (e) {
    console.log("Error: getBase64Encode: error while encoding.");
  }
};

export const b64DecodeStandard = (str: string) => {
  try {
    return decodeURIComponent(
      Array.prototype.map
        .call(atob(str), function (c) {
          return "%" + ("00" + c.charCodeAt(0).toString(16)).slice(-2);
        })
        .join("")
    );
  } catch (e) {
    console.log("Error: getBase64Decode: error while decoding.");
  }
};

export const getSessionStorageVal = (key: string) => {
  try {
    return window.sessionStorage.getItem(key);
  } catch (e) {
    console.log(`Error: Error while pull sessionstorage value ${key}`);
  }
};

export const useLocalOrganization = (val: any = "", isDelete = false) => {
  return useLocalStorage("organization", val, isDelete, true);
};

export const useLocalCurrentUser = (val = "", isDelete = false) => {
  return useLocalStorage("currentuser", val, isDelete, true);
};

export const useLocalLogsObj = (val = "", isDelete = false) => {
  return useLocalStorage("logsobj", val, isDelete, true);
};

export const useLocalLogFilterField = (val = "", isDelete = false) => {
  return useLocalStorage("logFilterField", val, isDelete, true);
};

export const useLocalTraceFilterField = (val = "", isDelete = false) => {
  return useLocalStorage("traceFilterField", val, isDelete, true);
};

export const useLocalInterestingFields = (val = "", isDelete = false) => {
  return useLocalStorage("interestingFields", val, isDelete, true);
};

export const useLocalSavedView = (val = "", isDelete = false) => {
  return useLocalStorage("savedViews", val, isDelete, true);
};

export const useLocalUserInfo = (val = "", isDelete = false) => {
  const userInfo: any = useLocalStorage("userInfo", val, isDelete);
  return userInfo.value;
};

export const useLocalTimezone = (val = "", isDelete = false) => {
  const timezone: any = useLocalStorage("timezone", val, isDelete);
  return timezone.value;
};

export const useLocalWrapContent = (val = "", isDelete = false) => {
  const wrapcontent: any = useLocalStorage("wrapcontent", val, isDelete);
  return wrapcontent.value;
};

export const deleteSessionStorageVal = (key: string) => {
  try {
    return sessionStorage.removeItem(key);
  } catch (e) {
    console.log(`Error: Error while pull sessionstorage value ${key}`);
  }
};

export const getDecodedUserInfo = () => {
  try {
    if (useLocalUserInfo() != null) {
      const userinfo: any = useLocalUserInfo();
      return b64DecodeStandard(userinfo);
    } else {
      return null;
    }
  } catch (e) {
    console.log("Error: Error while pull sessionstorage value.");
  }
};

export const validateEmail = (email: string) => {
  try {
    if (email != null) {
      const re = /\S+@\S+\.\S+/;
      return re.test(email);
    } else {
      return false;
    }
  } catch (e) {
    console.log(`Error: Error while validatig email id ${email}`);
  }
};

export const getLocalTime = (datetime: string) => {
  try {
    if (datetime !== null) {
      const event = new Date(datetime);
      const local = event.toString();
      const dateobj = new Date(local);

      return (
        dateobj.getFullYear() +
        "/" +
        (dateobj.getMonth() + 1) +
        "/" +
        dateobj.getDate() +
        " " +
        dateobj.getHours() +
        ":" +
        dateobj.getMinutes()
      );
    } else {
      return datetime;
    }
  } catch (e) {
    console.log(`Error: Error while covert localtime ${datetime}`);
  }
};

export const getBasicAuth = (username: string, password: string) => {
  const token = username + ":" + password;
  const hash = window.btoa(token);
  return "Basic " + hash;
};

export const getImageURL = (image_path: string) => {
  return getPath() + "src/assets/" + image_path;
};

export const getPath = () => {
  const pos = window.location.pathname.indexOf("/web/");
  const path =
    window.location.origin == "http://localhost:8081"
      ? "/"
      : pos > -1
      ? window.location.pathname.slice(0, pos + 5)
      : "";
  const cloudPath = import.meta.env.BASE_URL;
  return config.isCloud == "true" ? cloudPath : path;
};

export const routeGuard = async (to: any, from: any, next: any) => {
  const store = useStore();
  const q = useQuasar();
  const { getStreams } = useStreams();

  // if (
  //   config.isCloud &&
  //   store.state.selectedOrganization.subscription_type == config.freePlan
  // ) {
  //   await billings
  //     .list_subscription(store.state.selectedOrganization.identifier)
  //     .then((res: any) => {
  //       if (res.data.data.length == 0) {
  //         next({ path: "/billings/plans" });
  //       }

  //       if (
  //         res.data.data.CustomerBillingObj.customer_id == null ||
  //         res.data.data.CustomerBillingObj.customer_id == ""
  //       ) {
  //         next({ path: "/billings/plans" });
  //       }
  //     });
  // }

  if (
    to.path.indexOf("/ingestion") == -1 &&
    to.path.indexOf("/iam") == -1 &&
    store.state.zoConfig.hasOwnProperty("restricted_routes_on_empty_data") &&
    store.state.zoConfig.restricted_routes_on_empty_data == true &&
    store.state.organizationData.isDataIngested == false
  ) {
    await getStreams("", false).then((response: any) => {
      if (response.list.length == 0) {
        store.dispatch("setIsDataIngested", false);
        next({ path: "/ingestion" });
      } else {
        store.dispatch("setIsDataIngested", true);
        next();
      }
    });
  } else {
    next();
  }

  // if (local_organization.value.value == null || config.isCloud == "false") {
  //   next();
  // }

  // if (local_organization.value.value.status == "pending-subscription") {
  //   Dialog.create({
  //     title: "Confirmation",
  //     message: "Please subscribe to a paid plan to continue.",
  //     cancel: true,
  //     persistent: true,
  //   })
  //     .onOk(() => {
  //       const nextURL = getPath().replace(".", "") + "billings/plans";
  //       next(nextURL);
  //     })
  //     .onCancel(() => {
  //       return false;
  //     });
  // } else {
  //   next();
  // }
};

export const convertToTitleCase = (str: string) => {
  return str
    .split("-")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
};

export const verifyOrganizationStatus = (Organizations: any, Router: any) => {
  // for (const org of Organizations) {
  //   if (org.status == "pending-subscription") {
  //     Dialog.create({
  //       title: "Warning",
  //       message: "Please subscribe to a paid plan to continue.",
  //     });
  //     Router.push({ name: "plans" });
  //   }
  // }
};

export const convertTimeFromMicroToMilli = (time: number) => {
  const milliseconds = Math.floor(time / 1000);
  const date = new Date(milliseconds);
  return date.getTime();
};

export const formatLargeNumber = (number: number) => {
  if (number >= 1000000000) {
    return (number / 1000000000).toFixed(1) + "B";
  } else if (number >= 1000000) {
    return (number / 1000000).toFixed(1) + "M";
  } else if (number >= 1000) {
    return (number / 1000).toFixed(1) + "K";
  } else {
    return number.toString();
  }
};

export const formatSizeFromMB = (sizeInMB: string) => {
  let size = parseFloat(sizeInMB);

  const units = ["KB", "MB", "GB", "TB", "PB"];
  let index = 1; // Start from MB

  while (size >= 1024 && index < units.length - 1) {
    size /= 1024;
    index++;
  }

  return `${size.toFixed(2)} ${units[index]}`;
};

/**
 * @param us : Time in microseconds
 */
export const formatTimeWithSuffix = (us: number) => {
  if (us >= 1000 * 1000) {
    return `${(us / 1000 / 1000).toFixed(2)}s`;
  }

  if (us >= 1000) {
    return `${(us / 1000).toFixed(2)}ms`;
  }

  return `${us.toFixed(2)}us`;
};

export const mergeRoutes: any = (route1: any, route2: any) => {
  const mergedRoutes = [];

  // Iterate through route1 and add its elements to mergedRoutes
  for (const r1 of route1) {
    const matchingRoute = route2.find(
      (r2: any) => r2.path === r1.path && r2.name === r1.name
    );

    if (matchingRoute) {
      // If a matching route is found in route2, merge the children
      const mergedChildren = mergeRoutes(
        r1.children || [],
        matchingRoute.children || []
      );
      mergedRoutes.push({
        ...r1,
        children: mergedChildren.length ? mergedChildren : undefined,
      });

      // Remove the matching route from route2
      route2 = route2.filter((r2: any) => r2 !== matchingRoute);
    } else {
      // If no matching route is found in route2, add the route from route1 as is
      mergedRoutes.push({ ...r1 });
    }
  }

  // Add any remaining routes from route2 to mergedRoutes
  mergedRoutes.push(...route2);

  return mergedRoutes;
};

export function formatDuration(ms: number) {
  const seconds = (ms / 1000).toFixed(2);
  const minutes = (Number(seconds) / 60).toFixed(2);
  const hours = (Number(minutes) / 60).toFixed(2);
  const days = (Number(hours) / 24).toFixed(2);

  let formatted = `${seconds} sec`;

  if (ms > 86400000) {
    formatted = `${days} days ${hours} hr`;
  } else if (ms > 3600000) {
    formatted = `${hours} hr `;
  } else if (ms > 60000) {
    formatted = `${minutes} min`;
  }

  return formatted.trim();
}
export const timestampToTimezoneDate = (
  unixMilliTimestamp: number,
  timezone: string = "UTC",
  format: string = "yyyy-MM-dd HH:mm:ss.SSS"
) => {
  return DateTime.fromMillis(Math.floor(unixMilliTimestamp))
    .setZone(timezone)
    .toFormat(format);
};

export const histogramDateTimezone: any = (
  utcTime: any,
  timezone: string = "UTC"
) => {
  if (timezone == "UTC") return Math.floor(new Date(utcTime).getTime());
  else {
    return (
      Math.floor(
        DateTime.fromISO(utcTime, { zone: "UTC" }).setZone(timezone).toSeconds()
      ) * 1000
    );
  }
};

// // Example usage:
// const inputDatetime = "2023/10/26 17:34:00";
// const inputTimezone = "Pacific/Pitcairn";
export const convertToUtcTimestamp = (
  inputDatetime: string,
  inputTimezone: string
) => {
  // Create a DateTime object with the input datetime and timezone
  const dt = DateTime.fromFormat(inputDatetime, "yyyy/MM/dd HH:mm:ss", {
    zone: inputTimezone,
  });

  // Get the UTC timestamp in seconds (rounded to the nearest second)
  const utcTimestamp = Math.round(dt.toUTC().toMillis());

  return utcTimestamp * 1000;
};

export const localTimeSelectedTimezoneUTCTime = async (
  time: any,
  timezone: string
) => {
  await importMoment();
  // Creating a Date object using the timestamp
  const date = new Date(time);

  // Extracting date and time components
  const year = date.getFullYear();
  const month = date.getMonth(); // Months are zero-indexed
  const day = date.getDate();
  const hour = date.getHours();
  const minute = date.getMinutes();
  const second = date.getSeconds();

  // Create a moment object using the provided date, time, and timezone
  const convertedDate = moment.tz(
    { year, month, day, hour, minute, second },
    timezone
  );

  // Convert the moment object to a Unix timestamp (in seconds)
  const unixTimestamp = convertedDate.unix() * 1000000;

  console.log(unixTimestamp);
  return unixTimestamp;
};

const isObject = (obj: any) => obj !== null && typeof obj === "object";

const isValidKey = (key: string) => {
  // Add any additional checks to ensure key validity
  return key !== "__proto__" && key !== "constructor" && key !== "prototype";
};

export const mergeDeep = (target: any, source: any) => {
  if (isObject(target) && isObject(source)) {
    for (const key in source) {
      if (
        isValidKey(key) &&
        Object.prototype.hasOwnProperty.call(source, key)
      ) {
        if (isObject(source[key])) {
          if (!target[key]) target[key] = {};
          mergeDeep(target[key], source[key]);
        } else {
          target[key] = source[key];
        }
      }
    }
  }
  return target;
};

export function getUUID() {
  return uuidv4();
}

export const maskText = (text: string) => {
  // Disabled masking as it was not great usefully
  // const visibleChars = 4; // Number of characters to keep visible at the beginning and end
  // const maskedChars = text.length - visibleChars * 2;

  // if (maskedChars > 0) {
  //   const maskedText =
  //     text.substring(0, visibleChars) +
  //     "*".repeat(maskedChars) +
  //     text.slice(-visibleChars);
  //   return maskedText;
  // } else {
  //   return "*".repeat(text.length); // If the text is too short, mask all characters
  // }

  return text;
};

export const convertToCamelCase = (str: string) => {
  if (!str) {
    return ''; // or handle the case as needed
  }

  return str.charAt(0).toUpperCase() + str.slice(1).toLowerCase();
};
