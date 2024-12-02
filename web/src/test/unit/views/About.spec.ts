import { mount } from "@vue/test-utils";
import About from "../../../views/About.vue";
import { Quasar, Dialog, Notify } from "quasar";
import { expect, it } from "vitest";
import i18n from "../../../locales";
import { createStore } from "vuex";
import { installQuasar } from "../helpers/install-quasar-plugin";

installQuasar();
const store = createStore({
  state: {
    zoConfig: {
      version: "1",
      commit_hash: "test",
      build_date: "01-01-23",
    },
    API_ENDPOINT: "test",
  },
});
it("should mount About view", async () => {
  const wrapper = mount(About, {
    shallow: false,
    components: {
      Notify,
      Dialog,
    },
    global: {
      plugins: [i18n],
      provide: {
        store: store,
      },
    },
  });
  expect(About).toBeTruthy();
});
