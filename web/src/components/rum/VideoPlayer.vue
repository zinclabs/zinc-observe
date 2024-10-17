<!-- Copyright 2023 OpenObserve Inc.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
-->

<template>
  <div class="player-container full-height q-pa-sm">
    <div
      v-if="isLoading"
      class="q-pb-lg flex items-center justify-center text-center full-width"
      style="height: calc(100vh - 200px)"
    >
      <div>
        <q-spinner-hourglass
          color="primary"
          size="40px"
          style="margin: 0 auto; display: block"
        />
        <div class="text-center full-width">
          Hold on tight, we're fetching session.
        </div>
      </div>
    </div>
    <div
      ref="playerContainerRef"
      class="flex items-center justify-center"
      style="height: calc(100vh - 198px)"
    >
      <div
        ref="playerRef"
        id="player"
        class="player flex items-center cursor-pointer"
        @click="togglePlay"
      />
    </div>
    <div class="full-width q-pa-sm q-pt-md controls-container">
      <div
        ref="playbackBarRef"
        class="playback_bar q-mt-sm q-mb-md relative-position cursor-pointer"
        @click="handlePlaybackBarClick"
      >
        <div
          class="progressTime bg-primary absolute"
          :style="{
            width: playerState.progressWidth + 'px',
            left: 0,
            top: 0,
            height: '100%',
            transition: 'all 0.1s linear',
          }"
        />
        <div
          class="progressTime bg-primary absolute"
          :style="{
            width: '2px',
            left: playerState.progressWidth - 2 + 'px',
            bottom: '-5px',
            height: '15px',
            transition: 'all 0.1s linear',
          }"
        />

        <div
          v-for="event in (events as any[])"
          :key="event.id"
          class="progressTime bg-secondary absolute cursor-pointer"
          :style="{
            width: '2px',
            left:
              (event.relativeTime / playerState.totalTime) * playerState.width +
              'px',
            bottom: '-5px',
            height: '15px',
          }"
          :title="
            event.name.length > 100
              ? event.name.slice(0, 100) + '...'
              : event.name
          "
        />
      </div>
      <div class="controls flex justify-between items-center">
        <div class="flex items-center">
          <div>
            <q-icon
              name="replay_10"
              size="24px"
              class="q-mr-sm cursor-pointer"
              @click="skipTo('backward')"
            />
            <q-icon
              :name="
                playerState.isPlaying
                  ? 'pause_circle_filled'
                  : 'play_circle_filled'
              "
              size="32px"
              class="cursor-pointer"
              @click="togglePlay"
            />
            <q-icon
              name="forward_10"
              size="24px"
              class="q-ml-sm cursor-pointer"
              @click="skipTo('forward')"
            />
          </div>
          <div class="flex q-ml-lg items-center">
            <div>{{ playerState.time }}</div>
            <div class="q-px-xs">/</div>
            <div>{{ playerState.duration }}</div>
          </div>
        </div>
        <div class="flex items-center">
          <q-toggle
            class="q-mr-md"
            v-model="playerState.skipInactivity"
            :label="t('rum.skipInactivity')"
            size="xs"
            @update:model-value="toggleSkipInactive"
          />
          <q-select
            class="speed-selector"
            v-model="playerState.speed"
            :options="speedOptions"
            color="input-border"
            bg-color="input-bg"
            stack-label
            outlined
            filled
            dense
            size="xs"
            @update:model-value="setSpeed"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { cloneDeep } from "lodash-es";
import {
  nextTick,
  ref,
  watch,
  type Ref,
  onBeforeUnmount,
  onMounted,
  onBeforeMount,
} from "vue";
import { useStore } from "vuex";
import { getPath } from "@/utils/zincutils";
import { useI18n } from "vue-i18n";

const props = defineProps({
  events: {
    type: Array,
    required: true,
  },
  segments: {
    type: Array,
    required: true,
  },
  isLoading: {
    type: Boolean,
    required: true,
  },
});

const { t } = useI18n();

const store = useStore();

let rrwebPlayer: any;

const player = ref<any>();

const playerRef = ref<HTMLElement | null>(null);

const playbackBarRef = ref<HTMLElement | null>(null);

const session = ref<any>([]);

const playerContainerRef = ref<HTMLElement | null>(null);

const worker: Ref<Worker | null> = ref(null);

const workerProcessId = ref(0);

const speedOptions = [
  {
    label: "0.5x",
    value: 0.5,
  },
  {
    label: "1x",
    value: 1,
  },
  {
    label: "1.5x",
    value: 1.5,
  },
  {
    label: "2x",
    value: 2,
  },
  {
    label: "3x",
    value: 3,
  },
  {
    label: "4x",
    value: 4,
  },
];

const playerState = ref({
  isPlaying: false,
  time: "00.00",
  duration: "00.00", // in ms
  speed: {
    label: "4x",
    value: 4,
  },
  progressWidth: 0,
  playBackEvents: {
    views: true,
    actions: true,
    errors: true,
  },
  skipInactivity: true,
  fullScreen: false,
  startTime: 0,
  endTime: 0,
  totalTime: 0,
  width: 0,
  height: 0,
  actualTime: 0,
});

watch(
  () => props.segments,
  (value) => {
    if (value.length) setupSession();
  },
  { deep: true, immediate: true }
);

onBeforeMount(async () => {
  await importVideoPlayer();
  initializeWorker();
});

const importVideoPlayer = async () => {
  const rrwebPlayerModule: any = await import("@openobserve/rrweb-player");

  await import("@openobserve/rrweb-player/dist/style.css");

  rrwebPlayer = rrwebPlayerModule.default;
};

onBeforeUnmount(() => {
  if (worker.value) {
    worker.value.terminate();
  }
});

const setupSession = async () => {
  session.value = [];
  if (!props.segments.length) return;

  props.segments.forEach((segment: any) => {
    segment.records.forEach((record: any) => {
      let segCopy = cloneDeep(record);
      const supportedTypes = [2, 3, 4, 8];
      if (!supportedTypes.includes(segCopy.type)) return;
      if (segCopy.type === 8) {
        const seg = {
          ...segCopy,
          data: {
            payload: {
              ...segCopy.data,
            },
            tag: "viewport",
          },
          type: 5,
        };
        segCopy = seg;
      }
      try {
        if (segCopy.type === 2 && segCopy.data.node.type === 0) {
          segCopy.data.node.childNodes.forEach((child: any) => {
            if (child.type === 2 && child.tagName === "html") {
              child.childNodes.forEach((_child: any) => {
                if (_child.type === 2 && _child.tagName === "head") {
                  _child.childNodes.forEach((__child: any) => {
                    if (
                      __child.type === 2 &&
                      __child.tagName === "link" &&
                      __child.attributes.rel === "stylesheet" &&
                      typeof __child.attributes.href === "string" &&
                      __child.attributes.href.endsWith(".css") &&
                      __child.attributes._cssText
                    ) {
                      workerProcessId.value++;
                      processCss(
                        __child.attributes._cssText,
                        workerProcessId.value
                      ).then((res: any) => {
                        __child.attributes._cssText = res.updatedCssString;
                      });
                    }
                  });
                }
              });
            }
          });
        }
      } catch (e) {
        console.log(e);
      }
      session.value.push(segCopy);
    });
  });

  // let lastEventTime = 1692884586897;
  // const inactivityThreshold = 5000; // 5 seconds

  // session.value.forEach((event) => {
  //   const currentTime = event.timestamp;
  //   if (currentTime - lastEventTime > inactivityThreshold) {
  //     console.log(
  //       `Inactivity detected between timestamps ${lastEventTime} and ${currentTime} is of ${formatTimeDifference(
  //         currentTime - lastEventTime
  //       )} ms.`
  //     );
  //   }
  //   lastEventTime = currentTime;
  // });

  let sessionWidth: number = 0;
  let sessionHeight: number = 0;

  session.value.every((segment: any) => {
    if (segment.data.height && segment.data.width) {
      sessionWidth = segment.data.width;
      sessionHeight = segment.data.height;
      return false;
    }
    return true;
  });

  let playerWidth = playerContainerRef.value?.clientWidth || 0;
  let playerHeight = (sessionHeight / sessionWidth) * playerWidth;

  if (!sessionHeight) {
    playerHeight = playerWidth * 0.5625;
  }

  if (
    playerContainerRef.value?.clientHeight &&
    playerHeight > playerContainerRef.value?.clientHeight - 90
  ) {
    playerHeight = playerContainerRef.value?.clientHeight - 90 || 0;
    playerWidth = (sessionWidth / sessionHeight) * playerHeight;
  }

  if (playerRef.value) {
    playerRef.value.style.width = `${playerWidth}px`;
  }

  await nextTick();
  if (!playerRef.value) return;
  if (player.value) return;
  player.value = new rrwebPlayer({
    target: playerRef.value as HTMLElement,
    props: {
      events: session.value,
      UNSAFE_replayCanvas: false,
      mouseTail: false,
      autoPlay: false,
      showController: false,
      width: playerWidth,
      height: playerHeight,
      mutateChildNodes: true,
      speed: playerState.value.speed.value,
      skipInactive: playerState.value.skipInactivity,
    },
  });

  // events.forEach((event) => {
  //   if (event.type === 2 || event.type === 4) {
  //     player.value?.addEvent(event);
  //   }
  // });

  player.value.addEventListener("ui-update-current-time", updateProgressBar);

  player.value.addEventListener("finish", () => {
    playerState.value.isPlaying = false;
  });

  player.value.addEventListener("error", () => {
    console.error("Playback error:");
  });

  // player.value.addEventListener("event-cast", (e) => {
  //   // console.log("event casted:", e);
  // });

  if (!player.value) return;
  const playerMeta = player.value?.getMetaData();
  playerState.value.startTime = playerMeta.startTime;
  playerState.value.endTime = playerMeta.endTime;
  playerState.value.totalTime = playerMeta.totalTime;
  playerState.value.duration = formatTimeDifference(
    playerState.value.totalTime
  );

  const playbackBarWidth = playbackBarRef.value?.clientWidth || 0;
  // calculate width of progress bar
  playerState.value.width = playbackBarWidth;

  player.value.triggerResize();
};

function formatTimeDifference(milliSeconds: number) {
  // Calculate hours, minutes, and seconds
  let hours: string | number = Math.floor(milliSeconds / (1000 * 60 * 60));
  let minutes: string | number = Math.floor(
    (milliSeconds % (1000 * 60 * 60)) / (1000 * 60)
  );
  let seconds: string | number = Math.floor(
    (milliSeconds % (1000 * 60)) / 1000
  );

  // Add leading zeros if needed
  hours = hours < 10 ? "0" + hours : hours;
  minutes = minutes < 10 ? "0" + minutes : minutes;
  seconds = seconds < 10 ? "0" + seconds : seconds;

  if (hours === "00") {
    return `${minutes}:${seconds}`;
  }

  if (hours === "00" && minutes === "00") {
    return `${seconds}`;
  }

  return `${hours}:${minutes}:${seconds}`;
}

const updateProgressBar = (time: { payload: number }) => {
  playerState.value.progressWidth =
    (time.payload / playerState.value.totalTime) * playerState.value.width;
  playerState.value.actualTime = time.payload;
  playerState.value.time = formatTimeDifference(time.payload);
};

const handlePlaybackBarClick = (event: any) => {
  if (!playbackBarRef.value) return;
  const playbackBarEl = playbackBarRef.value.getBoundingClientRect();

  let time =
    ((event.clientX - playbackBarEl.left) / playerState.value.width) *
    playerState.value.totalTime;

  goto(time, playerState.value.isPlaying);
};

// -------------- Player control methods ----------------
const togglePlay = () => {
  if (playerState.value.isPlaying) {
    pause();
  } else {
    play();
  }
};

const play = () => {
  playerState.value.isPlaying = true;
  player.value?.play();
};

const pause = () => {
  playerState.value.isPlaying = false;
  player.value?.pause();
};

const setSpeed = (speed: { label: string; value: number }) => {
  player.value?.setSpeed(speed.value);
};
const toggleSkipInactive = () => {
  player.value?.toggleSkipInactive();
};

const goto = (timeOffset: number, play: boolean = false) => {
  playerState.value.isPlaying = play;
  player.value?.goto(timeOffset, play);
};

const skipTo = (skipTo: string) => {
  const seconds = 10;
  if (skipTo === "forward")
    goto(playerState.value.actualTime + seconds * 1000, false);
  else goto(playerState.value.actualTime - seconds * 1000, false);
};

const initializeWorker = () => {
  if (window.Worker) {
    // Creating the Web Worker
    worker.value = new Worker(
      new URL("../../workers/rumcssworker.js", import.meta.url),
      { type: "module" }
    );

    console.log("Worker created successfully.");
  } else {
    console.error("Web Workers are not supported in this browser.");
  }
};

const processCss = (cssString: string, id: string | number) => {
  return new Promise((resolve, reject) => {
    if (worker.value) {
      const handleWorkerMessage = (event: any) => {
        if (event.data.id === id) {
          if (worker.value)
            worker.value.removeEventListener("message", handleWorkerMessage);
          resolve(event.data);
        }
      };
      worker.value.addEventListener("message", handleWorkerMessage);
      worker.value.postMessage({
        cssString: cssString,
        proxyUrl: `${store.state.API_ENDPOINT}/proxy/${store.state.selectedOrganization.identifier}`,
        id,
      });
    } else {
      reject("Worker not initialized");
    }
  });
};

defineExpose({
  goto,
  play,
  pause,
  togglePlay,
  setSpeed,
  toggleSkipInactive,
  playerState,
});
</script>

<style scoped lang="scss">
.player {
  height: 100%;
}

.playback_bar {
  width: 100%;
  height: 5px;
  background-color: #ebebeb;
}
</style>

<style lang="scss">
.speed-selector {
  .q-field__control {
    padding: 0 8px !important;
  }

  .q-field__marginal,
  .q-field__native,
  .q-field__control {
    min-height: 30px !important;
    height: 30px !important;
  }
}
</style>
