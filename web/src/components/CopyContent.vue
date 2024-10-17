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
  <div class="tabContent">
    <div class="tabContent__head">
      <div>
        <div class="copy_action">
          <q-btn
            data-test="rum-copy-btn"
            flat
            round
            size="0.5rem"
            padding="0.6rem"
            icon="content_copy"
            color="grey"
            @click="copyToClipboardFn()"
          />
        </div>
      </div>
    </div>
    <pre data-test="rum-content-text">{{ computedContent }}</pre>
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar, copyToClipboard } from "quasar";
import { maskText, b64EncodeStandard } from "../utils/zincutils";

export default defineComponent({
  name: "CopyContent",
  props: {
    content: {
      type: String,
      default: "", // Default value for content prop (empty string in this case)
    },
    displayContent: {
      type: String,
      default: "", // Default value for displayContent prop (empty string in this case)
    },
  },
  setup(props, { emit }) {
    const store = useStore();
    const { t } = useI18n();
    const q = useQuasar();
    const email = ref(store.state.userInfo.email);
    const passcode = ref(store.state.organizationData.organizationPasscode);
    const basicPasscode = ref();

    const replaceValues = (data: string, isMask: boolean = false) => {
      email.value = store.state.userInfo.email;
      passcode.value = store.state.organizationData.organizationPasscode;
      basicPasscode.value = b64EncodeStandard(
        `${store.state.userInfo.email}:${store.state.organizationData.organizationPasscode}`
      );
      if (isMask) {
        return data
          .replaceAll("[EMAIL]", maskText(email.value))
          .replaceAll("[PASSCODE]", maskText(passcode.value))
          .replaceAll("[BASIC_PASSCODE]", maskText(basicPasscode.value));
      } else {
        return data
          .replaceAll("[EMAIL]", email.value)
          .replaceAll("[PASSCODE]", passcode.value)
          .replaceAll("[BASIC_PASSCODE]", basicPasscode.value);
      }
    };

    const copyToClipboardFn = () => {
      const content = replaceValues(props.content, false);
      copyToClipboard(content)
        .then(() => {
          q.notify({
            type: "positive",
            message: "Content Copied Successfully!",
            timeout: 5000,
          });
        })
        .catch(() => {
          q.notify({
            type: "negative",
            message: "Error while copy content.",
            timeout: 5000,
          });
        });
    };

    const displayData = ref(props.displayContent || props.content);
    const computedContent = ref();
    computedContent.value = replaceValues(displayData.value, true);

    const refreshData = () => {
      computedContent.value = replaceValues(displayData.value, true);
    };

    return {
      t,
      store,
      copyToClipboardFn,
      displayData,
      replaceValues,
      computedContent,
      refreshData,
    };
  },
  computed: {
    computedData() {
      return this.store.state.organizationData.organizationPasscode;
    },
  },
  watch: {
    computedData() {
      this.refreshData();
    },
  },
});
</script>
