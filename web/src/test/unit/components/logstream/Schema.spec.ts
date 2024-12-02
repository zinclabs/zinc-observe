import { describe, expect, it, beforeEach, vi, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { installQuasar } from "../../helpers/install-quasar-plugin";
import { Dialog, Notify } from "quasar";
import LogStream from "@/components/logstream/schema.vue";
import i18n from "@/locales";
// @ts-ignore
import { rest } from "msw";
import store from "@/test/unit/helpers/store";
import StreamService from "@/services/stream";

installQuasar({
  plugins: [Dialog, Notify],
});

describe("Streams", async () => {
  let wrapper: any;

  beforeEach(async () => {
    vi.useFakeTimers();

    wrapper = mount(LogStream, {
      props: {
        modelValue: {
          name: "k8s_json",
          schema: [],
          stream_type: "logs",
        },
      },
      global: {
        provide: {
          store: store,
        },
        plugins: [i18n],
      },
    });
    await flushPromises();
  });

  afterEach(() => {
    wrapper.unmount();
    vi.restoreAllMocks();
  });

  it("should display title", () => {
    const pageTitle = wrapper.find('[data-test="schema-title-text"]');
    expect(pageTitle.text()).toBe("Stream Detail");
  });

  it("Should display stream title", () => {
    const streamTitle = wrapper.find('[data-test="schema-stream-title-text"]');
    expect(streamTitle.text()).toBe("k8s_json");
  });

  it("Should have cancel button", () => {
    const cancelButton = wrapper.find('[data-test="schema-cancel-button"]');
    expect(cancelButton.exists()).toBeTruthy();
    expect(cancelButton.text()).toBe("Cancel");
  });

  it("Should have Update Settings button", () => {
    const updateSettingsButton = wrapper.find(
      '[data-test="schema-update-settings-button"]',
    );
    expect(updateSettingsButton.exists()).toBeTruthy();
    expect(updateSettingsButton.text()).toBe("Update Settings");
  });

  it("Should have Cancel button ", () => {
    const table = wrapper.find('[data-test="schema-stream-meta-data-table"]');
    expect(table.exists()).toBeTruthy();
  });

  it("Should display stream meta data table header", () => {
    store.state.zoConfig.show_stream_stats_doc_num = true;
    const tableHeaders = wrapper
      .find('[data-test="schema-stream-meta-data-table"]')
      .find("thead")
      .find("tr")
      .findAll("th");

    expect(tableHeaders[0].text()).toBe("Docs Count");
    expect(tableHeaders[1].text()).toBe("Ingested Data");
    expect(tableHeaders[2].text()).toBe("Compressed Size");
    expect(tableHeaders[3].text()).toBe("Time");
  });

  it("Should display stream meta data table data", () => {
    const tableHeaders = wrapper
      .find('[data-test="schema-stream-meta-data-table"]')
      .find("tbody")
      .find("tr")
      .findAll("td");
    expect(tableHeaders[0].text()).toBe("400");
    expect(tableHeaders[1].text()).toBe("0.74 MB");
    expect(tableHeaders[2].text()).toBe("0.03 MB");
    expect(tableHeaders[3].text()).toBe(
      "2023-03-10T17:13:48:63+05:30  to  2023-03-10T17:13:48:65+05:30",
    );
  });

  it("Should display stream fields mapping table", () => {
    const table = wrapper.find(
      '[data-test="schema-log-stream-field-mapping-table"]',
    );
    expect(table.exists()).toBeTruthy();
  });

  it("Should display stream fields mapping title", () => {
    const table = wrapper.find(
      '[data-test="schema-log-stream-mapping-title-text"]',
    );
    expect(table.text()).toBe(
      "Mapping - Using default fts keys, as no fts keys are set for stream.Store Original Data",
    );
  });

  it("Should display stream fields mapping table headers", () => {
    const tableHeaders = wrapper
      .find('[data-test="schema-log-stream-field-mapping-table"]')
      .find("thead")
      .find("tr")
      .findAll("th");

    expect(tableHeaders[0].text()).toBe("");
    expect(tableHeaders[1].text()).toBe("Fieldarrow_upward");
    expect(tableHeaders[2].text()).toBe("Typearrow_upward");
    expect(tableHeaders[3].text()).toBe("Index Type");
  });

  it("Should display stream fields mapping table data", () => {
    const tableData = wrapper
      .find('[data-test="schema-log-stream-field-mapping-table"]')
      .find("tbody")
      .findAll("tr")[0]
      .findAll("td");

    expect(tableData[0].text()).toBe("");
    expect(tableData[1].text()).toBe("_timestamp");
    expect(tableData[2].text()).toBe("Int64");
  });

  // TODO : Check if we can update this test case
  // - expect(logCheckbox.find(".q-checkbox__inner--truthy").exists()).toBeTruthy();
  // + expect(wrapper.vm.ftsKeys.includes('log')).toBeTruthy();
  it("Should check if log and message full text search checkbox is active in field mapping table", () => {
    const logCheckbox = wrapper
      .find('[data-test="schema-log-stream-field-mapping-table"]')
      .find('[data-test="schema-stream-log-field-fts-key-checkbox"]');
    const messageCheckbox = wrapper
      .find('[data-test="schema-log-stream-field-mapping-table"]')
      .find("tbody")
      .find('[data-test="schema-stream-log-field-fts-key-checkbox"]');
    expect(
      logCheckbox.find(".q-checkbox__inner--truthy").exists(),
    ).toBeTruthy();
    expect(
      messageCheckbox.find(".q-checkbox__inner--truthy").exists(),
    ).toBeTruthy();
  });

  it("Should check if _timestamp and kubernetes.container_hash full text search checkbox is inactive in field mapping table", () => {
    const timeStampCheckbox = wrapper
      .find('[data-test="schema-log-stream-field-mapping-table"]')
      .find('[data-test="schema-stream-_timestamp-field-fts-key-checkbox"]');
    const KubHashCheckbox = wrapper
      .find('[data-test="schema-log-stream-field-mapping-table"]')
      .find("tbody")
      .find(
        '[data-test="schema-stream-kubernetes.container_hash-field-fts-key-checkbox"]',
      );
    expect(
      timeStampCheckbox.find(".q-checkbox__inner--truthy").exists(),
    ).toBeFalsy();
    expect(
      KubHashCheckbox.find(".q-checkbox__inner--truthy").exists(),
    ).toBeFalsy();
  });

  describe("When user make changes and update settings", () => {
    const updateStream = vi.spyOn(StreamService, "updateSettings");
    let logPartition, timeStampCheckbox, updateSettingsButton: any;
    beforeEach(async () => {
      logPartition = wrapper
        .find('[data-test="schema-log-stream-field-mapping-table"]')
        .find('[data-test="schema-stream-log-field-partition-key-checkbox"]');
      timeStampCheckbox = wrapper
        .find('[data-test="schema-log-stream-field-mapping-table"]')
        .find('[data-test="schema-stream-_timestamp-field-fts-key-checkbox"]');
      updateSettingsButton = wrapper.find(
        '[data-test="schema-update-settings-button"]',
      );

      await logPartition.trigger("click");
      await timeStampCheckbox.trigger("click");
    });

    it("Should make api call to save updated settings", async () => {
      global.server.use(
        rest.post(
          `${store.state.API_ENDPOINT}/api/${store.state.selectedOrganization.identifier}/k8s_json/settings`,
          (req: any, res: any, ctx: any) => {
            return res(ctx.status(200), ctx.json({ code: 200 }));
          },
        ),
      );
      await updateSettingsButton.trigger("submit");
      expect(updateStream).toHaveBeenCalledTimes(1);
    });
  });
});
