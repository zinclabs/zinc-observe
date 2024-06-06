<template>
  <div class="q-py-xs flex justify-start q-px-md copy-log-btn">
    <q-btn
      :label="t('common.copyToClipboard')"
      dense
      size="sm"
      no-caps
      class="q-px-sm"
      icon="content_copy"
      @click="copyLogToClipboard"
    />
  </div>
  <div class="q-pl-md">
    {
    <div
      class="log_json_content"
      v-for="(key, index) in Object.keys(value)"
      :key="key"
    >
      <q-btn-dropdown
        data-test="log-details-include-exclude-field-btn"
        size="0.5rem"
        flat
        outlined
        filled
        dense
        class="q-ml-sm pointer"
        :name="'img:' + getImageURL('images/common/add_icon.svg')"
      >
        <q-list>
          <q-item
            clickable
            v-close-popup
            v-if="searchObj.data.stream.selectedStreamFields.some(
                                (item: any) =>
                                  item.name === key
                                    ? item.isSchemaField
                                    : ''
                              )
                              && multiStreamFields.includes(key)
                            "
          >
            <q-item-section>
              <q-item-label
                data-test="log-details-include-field-btn"
                @click.stop="addSearchTerm(`${key}='${value[key]}'`)"
                v-close-popup
                ><q-btn
                  title="Add to search query"
                  size="6px"
                  round
                  class="q-mr-sm pointer"
                >
                  <q-icon color="currentColor">
                    <EqualIcon></EqualIcon>
                  </q-icon> </q-btn
                >{{ t("common.includeSearchTerm") }}</q-item-label
              >
            </q-item-section>
          </q-item>
          
          <q-item
            clickable
            v-close-popup
            v-if="searchObj.data.stream.selectedStreamFields.some(
                                (item: any) =>
                                  item.name === key
                                    ? item.isSchemaField
                                    : ''
                              )
                              && multiStreamFields.includes(key)
                            "
          >
            <q-item-section>
              <q-item-label
                data-test="log-details-exclude-field-btn"
                @click.stop="addSearchTerm(`${key}!='${value[key]}'`)"
                v-close-popup
                ><q-btn
                  title="Add to search query"
                  size="6px"
                  round
                  class="q-mr-sm pointer"
                >
                  <q-icon color="currentColor">
                    <NotEqualIcon></NotEqualIcon>
                  </q-icon> </q-btn
                >{{ t("common.excludeSearchTerm") }}</q-item-label
              >
            </q-item-section>
          </q-item>
          <q-item clickable v-close-popup>
            <q-item-section>
              <q-item-label
                data-test="log-details-add-field-btn"
                @click.stop="addFieldToTable(key)"
                v-close-popup
                ><q-btn
                  title="Add field to table"
                  icon="visibility"
                  size="6px"
                  round
                  class="q-mr-sm pointer"
                ></q-btn
                >{{ t("common.addFieldToTable") }}</q-item-label
              >
            </q-item-section>
          </q-item>
        </q-list>
      </q-btn-dropdown>

      <span class="q-pl-xs">
        <span
          :class="store.state.theme === 'dark' ? 'text-red-5' : 'text-red-10'"
          >{{ key }}:</span
        ><span class="q-pl-xs"
          ><template v-if="index < Object.keys(value).length - 1"
            >{{ value[key] }},</template
          >
          <template v-else>
            {{ value[key] }}
          </template>
        </span>
      </span>
    </div>
    }
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, onBeforeMount } from "vue";
import { getImageURL } from "@/utils/zincutils";
import { useStore } from "vuex";
import EqualIcon from "@/components/icons/EqualIcon.vue";
import NotEqualIcon from "@/components/icons/NotEqualIcon.vue";
import { useI18n } from "vue-i18n";
import useLogs from "../../composables/useLogs";

export default {
  name: "JsonPreview",
  props: {
    value: {
      type: Object,
      required: true,
      default: () => ({}),
    },
    showCopyButton: {
      type: Boolean,
      default: true,
    },
  },
  components: { NotEqualIcon, EqualIcon },
  emits: ["copy", "addSearchTerm", "addFieldToTable"],
  setup(props: any, { emit }: any) {
    const { t } = useI18n();
    const store = useStore();
    const copyLogToClipboard = () => {
      emit("copy", props.value);
    };
    const addSearchTerm = (value: string) => {
      emit("addSearchTerm", value);
    };
    const addFieldToTable = (value: string) => {
      emit("addFieldToTable", value);
    };
    const { searchObj } = useLogs();
    let multiStreamFields: any = ref([]);

    onBeforeMount(() => {
      searchObj.data.stream.selectedStreamFields.forEach((item: any) => {
        if (
          item.streams.length == searchObj.data.stream.selectedStream.length
        ) {
          multiStreamFields.value.push(item.name);
        }
      });
    });

    return {
      t,
      copyLogToClipboard,
      getImageURL,
      addSearchTerm,
      addFieldToTable,
      store,
      searchObj,
      multiStreamFields,
    };
  },
};
</script>

<style lang="scss" scoped>
.log_json_content {
  white-space: pre-wrap;
  font-family: monospace;
  font-size: 12px;
}
</style>
