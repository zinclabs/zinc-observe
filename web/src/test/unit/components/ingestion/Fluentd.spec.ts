import { describe, expect, it, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { installQuasar } from "../../helpers/install-quasar-plugin";
import { createStore } from "vuex";
import Fluentd from "@/components/ingestion/logs/Fluentd.vue";

installQuasar();
const store = createStore({
  state: {
    organizationPasscode: 11,
    API_ENDPOINT: "http://localhost:808",
  },
});
describe("Fluentd", async () => {
  let wrapper: any;
  beforeEach(() => {
    wrapper = mount(Fluentd, {
      props: {
        currOrgIdentifier: "zinc_next",
        currUserEmail: "tulsiraval2828@gmail.com",
        orgAPIKey: 'L"4R{8f~56e72`0319V',
      },
      global: {
        provide: {
          store: store,
        },
      },
    });
  });
  it("should mount Fluentd", async () => {
    expect(wrapper.vm.currOrgIdentifier).not.toBe("");
    expect(wrapper.vm.currUserEmail).not.toBe("");
    expect(wrapper.vm.orgAPIKey).not.toBe("");
  });

  it("should render title", () => {
    const title = wrapper.find('[data-test="fluentd-title-text"]');
    expect(title.text()).toBe("Fluentd");
  });

  it("should render content", () => {
    const content = wrapper.find('[data-test="fluentd-content-text"]');

    expect(content.text()).matchSnapshot();
  });

  it("should render copy icon", () => {
    const btn = wrapper.find('[data-test="fluentd-copy-btn"]');
    expect(btn.exists()).toBeTruthy();
  });
});
