import { describe, expect, it, beforeEach, vi, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { installQuasar } from "../helpers/install-quasar-plugin";
import { Dialog, Notify } from "quasar";
import LogStream from "../../../views/LogStream.vue";
import i18n from "../../../locales";
import store from "../helpers/store";
import router from "../helpers/router";

installQuasar({
  plugins: [Dialog, Notify],
});

// import { useRouter, useRoute } from "vue-router";

// vi.mock("vue-router", () => ({
//   useRoute: vi.fn(),
//   useRouter: vi.fn(() => ({
//     push: () => {},
//   })),
// }));

describe("Streams", async () => {
  let wrapper: any;
  beforeEach(() => {
    vi.useFakeTimers();
    wrapper = mount(LogStream, {
      props: {
        currOrgIdentifier: "zinc_next",
        currUserEmail: "example@gmail.com",
        orgAPIKey: 'L"4R{8f~56e72`0319V',
      },
      global: {
        provide: {
          store: store,
        },
        plugins: [i18n, router],
      },
    });
  });

  afterEach(() => {
    wrapper.unmount();
  });

  it("should display title", () => {
    const title = wrapper.find('[data-test="log-stream-title-text"]');
    expect(title.text()).toBe("Streams");
  });

  it("Should display streams table", () => {
    const table = wrapper.find('[data-test="log-stream-table"]');
    expect(table.exists()).toBeTruthy();
  });

  it("Should match previous table snapshot", async () => {
    await flushPromises();
    const table = wrapper
      .find('[data-test="log-stream-table"]')
      .find("table")
      .html();
    expect(table).toMatchSnapshot();
  });

  it("Should display table column headers", async () => {
    await flushPromises();
    const tableData = wrapper
      .find('[data-test="log-stream-table"]')
      .find("thead")
      .find("tr")
      .findAll("th");
    expect(tableData[0].text()).toBe("#");
    expect(tableData[1].text()).toContain("Name");
    expect(tableData[2].text()).toContain("Type");
    expect(tableData[3].text()).toContain("Doc Num");
    expect(tableData[4].text()).toContain("Ingested Data");
    expect(tableData[5].text()).toContain("Compressed Size");
    expect(tableData[6].text()).toContain("Actions");
  });

  it("Should display table row data", async () => {
    await flushPromises();
    const tableData = wrapper
      .find('[data-test="log-stream-table"]')
      .find("tbody")
      .find("tr")
      .findAll("td");
    expect(tableData[0].text()).toBe("01");
    expect(tableData[1].text()).toBe("k8s_json");
    expect(tableData[2].text()).toBe("logs");
    expect(tableData[3].text()).toBe("400");
    expect(tableData[4].text()).toBe("0 MB");
    expect(tableData[5].text()).toBe("0.03 MB");
  });

  it("Should display refresh stats btn", async () => {
    const searchInput = wrapper.find(
      '[data-test="log-stream-refresh-stats-btn"]',
    );
    expect(searchInput.exists()).toBeTruthy();
    expect(searchInput.text()).toContain("Refresh Stats");
  });

  it("Should display pagination buttons", async () => {
    const pagination = wrapper.find(
      '[data-test="log-stream-table-pagination"]',
    );
    expect(pagination.exists()).toBeTruthy();
  });
});
