import { mount } from "@vue/test-utils";
import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";
import { installQuasar } from "../helpers/install-quasar-plugin";
import HighLight from "../../../components/HighLight.vue";

installQuasar();

// const init = vi.fn()

describe("HighLight", async () => {
  let wrapper: any = null;
  beforeEach(() => {
    // render the component
    wrapper = mount(HighLight, {
      shallow: false,
      props: {
        content: "Content Data",
        queryString: "",
      },
    });
  });
  afterEach(() => {
    wrapper.unmount();
  });

  it("should mount HighLight component", async () => {
    expect(wrapper).toBeTruthy();
  });
  it("watcher - content", async () => {
    const localThis = {
      init: vi.fn(),
    };
    // await HighLight.watch.content.handler.call(localThis)

    // const init = vi.spyOn(HighLight.methods, 'init')
    // expect(HighLight.methods.init).toBeCalled()
  });
  it("watcher - queryString", () => {
    // const getKeywords = vi.fn()
    const localThis = {
      getKeywords: vi.fn(),
      init: vi.fn(),
    };
    // HighLight.watch.queryString.handler.call(localThis)
  });
});
