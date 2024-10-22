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
  <q-page class="aboutPage">
    <div class="about_head">
      <img
        :src="
          store.state.theme == 'dark'
            ? getImageURL('images/common/open_observe_logo_2.svg')
            : getImageURL('images/common/open_observe_logo.svg')
        "
        class="logo"
        width="177"
      />
      <div class="para">{{ t("about.logoMsg") }}</div>
    </div>

    <q-separator class="separator" />

    <div class="about_data">
      <div>
        <span class="inline-block text-bold" style="width: 100px"
          >{{ t("about.version") }}:</span
        >
        {{ store.state.zoConfig.version }}
      </div>
      <div>
        <span class="inline-block text-bold" style="width: 100px"
          >{{ t("about.commitHash") }}:</span
        >
        {{ store.state.zoConfig.commit_hash }}
      </div>
      <div>
        <span class="inline-block text-bold" style="width: 100px"
          >{{ t("about.buildDate") }}:</span
        >
        {{ store.state.zoConfig.build_date }}
      </div>
    </div>
    <div class="about_data q-mt-lg">
      <p>Open source libraries used in OpenObserve</p>
      <p>
        OpenObserve depends on many awesome open source libraries. You can find
        the list
        <a
          href="https://github.com/openobserve/openobserve/blob/main/Cargo.toml"
          target="_blank"
          class="hover:tw-underline text-primary"
          >here</a
        >
        and
        <a
          href="https://github.com/openobserve/openobserve/blob/main/web/package.json"
          target="_blank"
          class="hover:tw-underline text-primary"
          >here</a
        >. <br />You can find the packages and links to sources of the libraries
        at
        <a
          href="https://npmjs.com"
          target="_blank"
          class="hover:tw-underline text-primary"
          >npmjs.com</a
        >
        and
        <a
          href="https://crates.io"
          target="_blank"
          class="hover:tw-underline text-primary"
          >crates.io</a
        >
        along with their respective licenses.
      </p>
    </div>
  </q-page>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { useStore } from "vuex";
import { getImageURL } from "../utils/zincutils";
import { useI18n } from "vue-i18n";

export default defineComponent({
  name: "PageAbout",
  setup() {
    const store = useStore();
    const pageData = ref("Page Data");
    const { t } = useI18n();

    return {
      t,
      store,
      pageData,
      getImageURL,
    };
  },
});
</script>

<style lang="scss" scoped>
.aboutPage {
  padding: 2.25rem 1.5rem;

  .about_head,
  .about_data {
    padding: 0 1.5rem;
  }

  .logo {
    margin-bottom: 0.5rem;
    display: block;
  }

  .para {
    font-weight: 600;
  }

  .q-separator {
    margin: 1.5rem 0;
  }

  .about_data {
    .head {
      margin-bottom: 0.5rem;
      font-weight: 700;
    }

    .body {
      font-weight: 600;
    }
  }
}
</style>
