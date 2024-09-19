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

import { useStore } from "vuex";
import StreamService from "@/services/stream";
import { reactive } from "vue";
import { ref } from "vue";
import { useQuasar } from "quasar";

let streams: any = reactive({});

let streamsIndexMapping: any = reactive({});

const areAllStreamsFetched = ref(false);

const getStreamsPromise: any = ref(null);

const useStreams = () => {
  const store = useStore();

  const q = useQuasar();

  const getStreams = async (
    _streamName: string = "",
    schema: boolean,
    notify: boolean = true,
  ) => {
    return new Promise(async (resolve, reject) => {
      const streamName = _streamName || "all";

      // We don't fetch schema while fetching all streams or specific type all streams
      // So keeping it false, don't change this
      schema = false;

      if (getStreamsPromise.value) {
        await getStreamsPromise.value;
      }
      try {
        if (!isStreamFetched(streamName || "all")) {
          // Added adddtional check to fetch all streamstype separately if streamName is all
          const dismiss = notify
            ? q.notify({
                spinner: true,
                message: "Please wait while loading streams...",
                timeout: 5000,
              })
            : () => {};
          if (streamName === "all") {
            // As in RBAC there can be permission on certain types of streams
            // So here added some additional logic to handle those

            const streamList = [
              "logs",
              "metrics",
              "traces",
              "enrichment_tables",
              "index",
              "metadata",
            ];

            const streamsToFetch = streamList.filter(
              (_stream) => !streams[_stream],
            );

            getStreamsPromise.value = Promise.allSettled(
              [...streamsToFetch].map((streamType) =>
                StreamService.nameList(
                  store.state.selectedOrganization.identifier,
                  streamType,
                  schema,
                ),
              ),
            );

            getStreamsPromise.value
              .then((results: any) => {
                results.forEach((result: any, index: number) => {
                  if (result.status === "fulfilled") {
                    setStreams(streamsToFetch[index], result.value.data.list);
                  }
                });

                areAllStreamsFetched.value = streamList.every(
                  (stream) => !!streams[stream],
                );

                getStreamsPromise.value = null;

                dismiss();
                resolve(getAllStreamsPayload());
              })
              .catch((e: any) => {
                getStreamsPromise.value = null;
                dismiss();
                reject(new Error(e.message));
              });
          } else {
            getStreamsPromise.value = StreamService.nameList(
              store.state.selectedOrganization.identifier,
              _streamName,
              schema,
            );
            getStreamsPromise.value
              .then((res: any) => {
                setStreams(streamName, res.data.list);
                const streamData = {
                  name: streamName,
                  list: res.data.list,
                  schema: false,
                };
                getStreamsPromise.value = null;
                dismiss();
                resolve(streamData);
              })
              .catch((e: any) => {
                getStreamsPromise.value = null;
                dismiss();
                reject(new Error(e.message));
              });
          }
        } else {
          if (streamName === "all") {
            resolve(getAllStreamsPayload());
          } else {
            resolve(streams[streamName]);
          }
        }
      } catch (e: any) {
        reject(new Error(e.message));
      }
    });
  };

  const getAllStreamsPayload = () => {
    const streamObject = getStreamPayload();
    streamObject.name = "all";
    streamObject.schema = false;

    Object.keys(streams).forEach((key) => {
      streamObject.list.push(...streams[key].list);
    });

    return streamObject;
  };

  const getStream = async (
    streamName: string,
    streamType: string,
    schema: boolean,
    force: boolean = false,
  ): Promise<any> => {
    return new Promise(async (resolve, reject) => {
      if (!streamName || !streamType) {
        resolve(null);
      }

      // Wait for the streams to be fetched if they are being fetched
      if (getStreamsPromise.value) {
        await getStreamsPromise.value;
      }

      // If the stream is not fetched, and trying to fetch the specific stream. First fetch all streams
      if (!isStreamFetched(streamType)) {
        try {
          await getStreams(streamType, false);
        } catch (e: any) {
          reject(new Error(e.message));
        }
      }

      try {
        if (
          streams[streamType] &&
          streams[streamType].list.length &&
          Object.prototype.hasOwnProperty.call(
            streamsIndexMapping[streamType],
            streamName,
          )
        ) {
          const streamIndex = streamsIndexMapping[streamType][streamName];
          const hasSchema = !!streams[streamType].list[streamIndex]?.schema;

          if ((schema && !hasSchema) || force) {
            try {
              const _stream: any = await StreamService.schema(
                store.state.selectedOrganization.identifier,
                streamName,
                streamType,
              ); 
              streams[streamType].list[streamIndex] = removeSchemaFields(_stream.data);
            } catch (err) {
              return reject("Error while fetching schema");
            }
          }
          return resolve(streams[streamType].list[streamIndex]);
        } else {
          // await StreamService.schema(
          //   store.state.selectedOrganization.identifier,
          //   streamName,
          //   streamType
          // )
          //   .then((res: any) => {
          //     addStream(res.data);
          //     const streamIndex = streamsIndexMapping[streamType][streamName];
          //     return resolve(streams[streamType].list[streamIndex]);
          //   })
          //   .catch(() => reject("Stream Not Found"));
          return reject("Stream Not Found");
        }
      } catch (e: any) {
        reject(new Error(e.message));
      }
    });
  };

  const getMultiStreams = async (
    _streams: Array<{
      streamName: string;
      streamType: string;
      schema: boolean;
    }>,
  ): Promise<any[]> => {
    return Promise.all(
      _streams.map(async ({ streamName, streamType, schema }) => {
        // Return null immediately if either streamName or streamType is not provided.
        if (!streamName || !streamType) {
          return null;
        }

        try {
          // Check if the stream exists and has been indexed.
          if (
            streams[streamType] &&
            streams[streamType].list.length &&
            Object.prototype.hasOwnProperty.call(
              streamsIndexMapping[streamType],
              streamName,
            )
          ) {
            const streamIndex = streamsIndexMapping[streamType][streamName];
            const hasSchema = !!streams[streamType].list[streamIndex]?.schema;

            // If schema is requested but not present, fetch and update it.
            if (schema && !hasSchema) {
              const fetchedStream = await StreamService.schema(
                store.state.selectedOrganization.identifier,
                streamName,
                streamType,
              );

              streams[streamType].list[streamIndex] = removeSchemaFields(fetchedStream.data);
            }
          }

          // Return the stream object (with or without updated schema).
          return streams[streamType]?.list[
            streamsIndexMapping[streamType][streamName]
          ];
        } catch (e: any) {
          // Use reject in Promise.all to catch errors specifically.
          throw new Error(e.message);
        }
      }),
    );
  };

  function removeSchemaFields (streamData: any) {
    if(streamData.schema){
      streamData.schema = streamData.schema.filter((field: any) => {
        return field.name != '_original' && field.name != '_o2_id';
      });
    }
    return streamData
  }

  const isStreamFetched = (streamType: string) => {
    let isStreamFetched = false;
    if (streamType === "all") {
      isStreamFetched = areAllStreamsFetched.value;
    } else {
      isStreamFetched = !!streams[streamType];
    }

    return isStreamFetched;
  };

  const setStreams = (streamName: string = "all", streamList: any[] = []) => {
    if (isStreamFetched(streamName || "all")) return;

    if (!store.state.organizationData.isDataIngested && !!streamList.length)
      store.dispatch("setIsDataIngested", !!streamList.length);

    const streamObject = getStreamPayload();
    streamObject.name = streamName;
    streamObject.list = streamList;
    streamObject.schema = false;

    // If the stream name is all, then we need to store each type of stream separately manually
    if (streamName === "all") {
      streams = reactive({});

      streamList.forEach((stream: any) => {
        if (streams[stream.stream_type]) {
          streams[stream.stream_type].list.push(stream);
        } else {
          streams[stream.stream_type] = {
            name: stream.stream_type,
            list: [stream],
            schema: false,
          };
        }
      });
    } else {
      streams[streamName] = streamObject;
    }

    // Mapping stream index for each stream type
    if (streamName === "all") {
      streamsIndexMapping = reactive({});
      Object.keys(streams).forEach((key) => {
        streamsIndexMapping[key] = {};
        streams[key].list.forEach((stream: any, index: number) => {
          streamsIndexMapping[key][stream.name] = index;
        });
      });
    } else {
      streamsIndexMapping[streamName] = {};
      streams[streamName].list.forEach((stream: any, index: number) => {
        streamsIndexMapping[streamName][stream.name] = index;
      });
    }

    return streamObject;
  };

  const getStreamPayload = () => {
    return {
      name: "",
      list: [],
      schema: false,
    } as {
      name: string;
      list: any;
      schema: boolean;
    };
  };

  // Don't add delete log here, it will create issue
  // This method is to remove specific stream from cache
  const removeStream = (streamName: string, streamType: string) => {
    if (
      Object.prototype.hasOwnProperty.call(
        streamsIndexMapping[streamType],
        streamName,
      )
    ) {
      const indexToRemove = streamsIndexMapping[streamType][streamName];

      // Deleting stream index that was mapped
      delete streamsIndexMapping[streamType][streamName];

      if (
        indexToRemove >= 0 &&
        indexToRemove < streams[streamType].list.length
      ) {
        for (
          let i = indexToRemove;
          i < streams[streamType].list.length - 1;
          i++
        ) {
          // Shift each element one position to the left
          streams[streamType].list[i] = streams[streamType].list[i + 1];
          streamsIndexMapping[streamType][streams[streamType].list[i].name] = i;
        }
        // Remove the last element since it's now duplicated
        streams[streamType].list.length = streams[streamType].list.length - 1;
      }
    }
  };

  const addStream = async (stream: any) => {
    if (
      !!streams[stream.stream_type] &&
      streamsIndexMapping[stream.stream_type][stream.name] >= 0
    )
      return;

    // If that stream type is not present create that stream
    if (!streams[stream.stream_type]) {
      getStream(stream.name, stream.stream_type, true);
    } else {
      streams[stream.stream_type].list.push(stream);
      streamsIndexMapping[stream.stream_type][stream.name] =
        streams[stream.stream_type].list.length - 1;
    }
  };

  const resetStreams = () => {
    streams = reactive({});
    streamsIndexMapping = reactive({});
    areAllStreamsFetched.value = false;
    getStreamsPromise.value = null;
    store.dispatch("setIsDataIngested", false);
  };

  function compareArrays(previousArray: any, currentArray: any) {
    const add = [];
    const remove = [];

    // Convert previousArray into a map for easy lookup
    const previousMap = new Map();
    for (const key in previousArray) {
      const prevItem = previousArray[key];
      previousMap.set(prevItem.field, prevItem);
    }

    // Convert currentArray into a map for easy lookup
    const currentMap = new Map();
    for (const currentItem of currentArray) {
      currentMap.set(currentItem.field, currentItem);
    }

    // Check for items in currentArray that are not in previousArray
    for (const currentItem of currentArray) {
      const prevItem = previousMap.get(currentItem.field);
      if (!prevItem || !deepEqual(currentItem, prevItem)) {
        add.push(currentItem);
      }
    }

    // Check for items in previousArray that are not in currentArray
    for (const [field, prevItem] of previousMap) {
      const currentItem = currentMap.get(field);
      if (!currentItem || !deepEqual(prevItem, currentItem)) {
        remove.push(prevItem);
      }
    }

    return { add, remove };
  }

  // Helper function to deeply compare two objects, considering the "types" object and ignoring the "disabled" attribute
  function deepEqual(objA: any, objB: any) {
    if (
      typeof objA !== "object" ||
      typeof objB !== "object" ||
      objA === null ||
      objB === null
    ) {
      return objA === objB;
    }

    const keysA = Object.keys(objA)
    const keysB = Object.keys(objB)

    if (keysA.length !== keysB.length) return false;

    for (const key of keysA) {
      if (typeof objA[key] === "object" && typeof objB[key] === "object") {
        if (!deepEqual(objA[key], objB[key])) return false;
      } else if (objA[key] !== objB[key]) {
        return false;
      }
    }

    return true;
  }

  const getUpdatedSettings = (previousSettings: any, currentSettings: any) => {
    const attributesToCompare: Array<string> = [
      "partition_keys",
      "index_fields",
      "full_text_search_keys",
      "bloom_filter_fields",
      "defined_schema_fields",
    ];

    let updatedSettings: any = {};
    updatedSettings = { ...currentSettings };

    attributesToCompare.forEach((attribute) => {
      const previousArray =
        Array.isArray(previousSettings[attribute]) ||
        typeof previousSettings[attribute] === "object"
          ? previousSettings[attribute]
          : [];
      const currentArray =
        Array.isArray(currentSettings[attribute]) ||
        typeof currentSettings[attribute] === "object"
          ? currentSettings[attribute]
          : [];

      let add: any[] = [];
      let remove: any[] = [];

      if (
        attribute === "partition_keys" &&
        typeof previousArray === "object" &&
        typeof currentArray === "object"
      ) {
        // add = currentArray.filter(
        //   (currentItem: any) =>
        //     !Object.values(previousArray).some((previousItem: any) =>
        //       deepEqual(currentItem, previousItem),
        //     ),
        // );

        // remove = Object.values(previousArray).filter(
        //   (previousItem: any) =>
        //     !Object.values(currentArray).some((currentItem: any) =>
        //       deepEqual(previousItem, currentItem),
        //     ),
        // );
        const result: any = compareArrays(previousArray, currentArray);
        add = result.add;
        remove = result.remove;
        remove = remove.filter((item: any) => {
          const isInAdd = add.some((addItem: any) => addItem.field === item.field);
        
          // Only keep in `remove` if not in `add` and `disabled` is false
          return !isInAdd && item.disabled === false;
        });
        
      } else {
        // For other attributes, do a simple array comparison
        add = currentArray.filter((item: any) => !previousArray.includes(item));
        remove = previousArray.filter(
          (item: any) => !currentArray.includes(item),
        );
      }

      // Add the _add and _remove arrays to the result
      updatedSettings[`${attribute}`] = { add: add, remove: remove };
    });

    return updatedSettings;
  };

  return {
    getStreams,
    getStream,
    setStreams,
    getMultiStreams,
    resetStreams,
    removeStream,
    addStream,
    getUpdatedSettings,
  };
};

export default useStreams;
