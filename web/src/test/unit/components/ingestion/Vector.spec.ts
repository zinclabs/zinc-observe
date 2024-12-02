import { describe, expect, it, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { installQuasar } from "../../helpers/install-quasar-plugin";
import { createStore } from "vuex";
import Vector from "@/components/ingestion/logs/Vector.vue";

installQuasar();
const store = createStore({
  state: {
    organizationPasscode: 11,
    API_ENDPOINT: "http://localhost:808",
  },
});
describe("Vector", async () => {
  let wrapper: any;
  beforeEach(() => {
    wrapper = mount(Vector, {
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
  it("should mount Vector", async () => {
    expect(wrapper.vm.currOrgIdentifier).not.toBe("");
    expect(wrapper.vm.currUserEmail).not.toBe("");
    expect(wrapper.vm.orgAPIKey).not.toBe("");
  });

  it("should render title", () => {
    const title = wrapper.find('[data-test="vector-title-text"]');
    expect(title.text()).toBe("Vector");
  });

  it("should render content", () => {
    const content = wrapper.find('[data-test="vector-content-text"]');

    expect(content.text()).toMatchSnapshot();
  });

  it("should render copy icon", () => {
    const btn = wrapper.find('[data-test="vector-copy-btn"]');
    expect(btn.exists()).toBeTruthy();
  });
});
