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
  <div
    class="full-height"
    :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
  >
    <q-form @submit="savePipeline">
      <div class="flex justify-between items-center q-px-md q-py-sm">
        <div data-test="add-pipeline-section-title" style="font-size: 18px">
          {{ t("pipeline.addPipeline") }}
        </div>
        <q-icon
          data-test="add-pipeline-close-dialog-btn"
          name="cancel"
          class="cursor-pointer"
          size="20px"
          v-close-popup="true"
        />
      </div>

      <div class="full-width bg-grey-4" style="height: 1px" />

      <div class="q-px-md">
        <div
          data-test="add-pipeline-name-input"
          class="alert-name-input o2-input"
          style="padding-top: 12px"
        >
          <q-input
            v-model="formData.name"
            :label="t('alerts.name') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            v-bind:readonly="isUpdating"
            v-bind:disable="isUpdating"
            :rules="[
              (val: any, rules: any) =>
                !!val
                  ? isValidName ||
                    `Use alphanumeric and '+=,.@-_' characters only, without spaces.`
                  : t('common.nameRequired'),
            ]"
            tabindex="0"
            style="min-width: 480px"
          />
        </div>
        <div
          data-test="add-pipeline-description-input"
          class="alert-name-input o2-input q-mb-sm"
        >
          <q-input
            v-model="formData.description"
            :label="t('alerts.description')"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            tabindex="0"
            style="min-width: 480px"
          />
        </div>
        <div
          data-test="add-pipeline-stream-type-select"
          class="alert-stream-type o2-input q-mr-sm q-mb-sm"
          style="padding-top: 0"
        >
          <q-select
            v-model="formData.stream_type"
            :options="streamTypes"
            :label="t('alerts.streamType') + ' *'"
            :popup-content-style="{ textTransform: 'lowercase' }"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm showLabelOnTop no-case"
            emit-value
            map-options
            stack-label
            outlined
            filled
            dense
            v-bind:readonly="isUpdating"
            v-bind:disable="isUpdating"
            @update:model-value="updateStreams()"
            :rules="[(val: any) => !!val || 'Field is required!']"
            style="min-width: 220px"
          />
        </div>
        <div
          data-test="add-pipeline-stream-select"
          class="o2-input"
          style="padding-top: 0"
        >
          <q-select
            v-model="formData.stream_name"
            :options="filteredStreams"
            :label="t('alerts.stream_name') + ' *'"
            :loading="isFetchingStreams"
            :popup-content-style="{ textTransform: 'lowercase' }"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm showLabelOnTop no-case"
            filled
            stack-label
            dense
            use-input
            hide-selected
            fill-input
            :input-debounce="400"
            v-bind:readonly="isUpdating"
            v-bind:disable="isUpdating"
            @filter="filterStreams"
            behavior="menu"
            :rules="[(val: any) => !!val || 'Field is required!']"
            style="min-width: 250px !important"
          />
        </div>
      </div>

      <div class="flex justify-start q-mt-lg q-px-md">
        <q-btn
          data-test="add-pipeline-cancel-btn"
          v-close-popup="true"
          class="q-mb-md text-bold"
          :label="t('alerts.cancel')"
          text-color="light-text"
          padding="sm md"
          no-caps
        />
        <q-btn
          data-test="add-pipeline-submit-btn"
          :label="t('alerts.save')"
          class="q-mb-md text-bold no-border q-ml-md"
          color="secondary"
          padding="sm xl"
          no-caps
          type="submit"
        />
      </div>
    </q-form>
  </div>
</template>

<script lang="ts" setup>
import useStreams from "@/composables/useStreams";
import { ref, computed, type Ref, defineEmits } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";

const props = defineProps({
  isUpdating: {
    type: Boolean,
    required: false,
    default: false,
  },
});

const emit = defineEmits(["save"]);

const store = useStore();

const formData = ref({
  name: "",
  description: "",
  stream_type: "",
  stream_name: "",
});

const { getStreams } = useStreams();

const isFetchingStreams = ref(false);

const { t } = useI18n();

const streamTypes = ref([
  { label: "Logs", value: "logs" },
  { label: "Metrics", value: "metrics" },
  { label: "Traces", value: "traces" },
]);

const filteredStreams: Ref<string[]> = ref([]);

const indexOptions = ref([]);

const isValidName = computed(() => {
  const roleNameRegex = /^[a-zA-Z0-9+=,.@_-]+$/;
  // Check if the role name is valid
  return roleNameRegex.test(formData.value.name);
});

const updateStreams = (resetStream = true) => {
  if (resetStream) formData.value.stream_name = "";

  if (!formData.value.stream_type) return Promise.resolve();

  isFetchingStreams.value = true;
  return getStreams(formData.value.stream_type, false)
    .then((res: any) => {
      indexOptions.value = res.list.map((data: any) => {
        return data.name;
      });

      return Promise.resolve();
    })
    .catch(() => Promise.reject())
    .finally(() => (isFetchingStreams.value = false));
};

const filterStreams = (val: string, update: any) => {
  filteredStreams.value = filterColumns(indexOptions.value, val, update);
};

const filterColumns = (options: any[], val: String, update: Function) => {
  let filteredOptions: any[] = [];
  if (val === "") {
    update(() => {
      filteredOptions = [...options];
    });
    return filteredOptions;
  }
  update(() => {
    const value = val.toLowerCase();
    filteredOptions = options.filter(
      (column: any) => column.toLowerCase().indexOf(value) > -1
    );
  });
  return filteredOptions;
};

const savePipeline = () => {
  emit("save", formData.value);
};
</script>

<style lang="scss" scoped></style>
