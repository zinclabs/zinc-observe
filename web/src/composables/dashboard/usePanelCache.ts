/**
 * This is the cache object which is used to store the panel level cache data
 * Ex:
 * {
 *    "folder1": {
 *        "dashboard1": {
 *            "panel1": {
 *              "key": {},    // Data against which the cache is stored
 *              "value": {}   // The cache to restore
 *            }
 *          }
 *      }
 * }
 */
const cache: any = {};

declare global {
  interface Window {
    _o2_removeDashboardCache: () => void;
    _o2_getDashboardCache: () => any;
  }
}

window._o2_removeDashboardCache = () => {
  // empty the cache object by removing all keys
  // as we can not reassign const
  Object.keys(cache).forEach((key) => {
    delete cache[key];
  });
};

window._o2_getDashboardCache = () => {
  // empty the cache object by removing all keys
  // as we can not reassign const
  return JSON.parse(JSON.stringify(cache));
};
/**
 * Use Panel Cache Data on a per dashboard basis in combination with folderid, dashboard id and panel id
 */
export const usePanelCache = (
  folderId: string,
  dashboardId: string,
  panelId: string,
) => {
  if (!(folderId && dashboardId && panelId)) {
    const savePanelCache = (data: any) => {
      // do nothing
    };

    const getPanelCache = () => {
      return null;
    };

    return {
      savePanelCache,
      getPanelCache,
    };
  }

  // create nested paths as required
  const createNestedPathsIfRequired = () => {
    // check if cache is there or not
    if (cache?.[folderId]?.[dashboardId]?.[panelId]) {
      return;
    }

    // create nested paths as required
    if (!cache[folderId]) {
      cache[folderId] = {};
    }

    if (!cache[folderId][dashboardId]) {
      cache[folderId][dashboardId] = {};
    }

    if (!cache[folderId][dashboardId][panelId]) {
      cache[folderId][dashboardId][panelId] = {};
    }
  };

  const savePanelCache = (key: any, data: any, cacheTimeRange: any) => {
    createNestedPathsIfRequired();

    cache[folderId][dashboardId][panelId] = {
      key: JSON.parse(JSON.stringify(key)), // deep copy key,
      value: JSON.parse(JSON.stringify(data)), // deep copy data
      cacheTimeRange: JSON.parse(JSON.stringify(cacheTimeRange)),
    };
  };

  const getPanelCache = () => {
    return cache?.[folderId]?.[dashboardId]?.[panelId];
  };

  return {
    savePanelCache,
    getPanelCache,
  };
};
