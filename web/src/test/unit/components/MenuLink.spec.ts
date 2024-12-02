import { mount } from "@vue/test-utils";
import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";
import { installQuasar } from "../helpers/install-quasar-plugin";
import MenuLink from "../../../components/MenuLink.vue";
import i18n from "../../../locales";
import store from "../helpers/store";

installQuasar();

describe("MenuLink", async () => {
  let wrapper: any = null;
  beforeEach(() => {
    // render the component
    wrapper = mount(MenuLink, {
      props: {
        title: "Logs",
        caption: "",
        link: "#",
        icon: "",
        mini: false,
      },
      global: {
        plugins: [i18n],
        provide: {
          store,
        },
      },
    });
  });

  afterEach(() => {
    wrapper.unmount();
  });

  it("should mount MenuLink component", async () => {
    expect(wrapper).toBeTruthy();
  });

  it("should render item title", async () => {
    expect(wrapper.find('[data-test="menu-link-#-item"]').text()).toBe("Logs");
  });

  it("should render item title", async () => {
    await wrapper.setProps({ mini: true });
    expect(wrapper.find('[data-test="menu-link-#-item"]').text()).toBe("");
  });

  it("should call window.open after clicking on external url", async () => {
    const windowOpen = vi.spyOn(window, "open");
    await wrapper.setProps({ external: true });
    await wrapper.find('[data-test="menu-link-#-item"]').trigger("click");
    expect(windowOpen).toHaveBeenCalledTimes(1);
    expect(windowOpen).toBeCalledWith("#", "_blank");
  });
});
