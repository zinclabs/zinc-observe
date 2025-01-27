import { describe, expect, it, beforeEach, vi, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { installQuasar } from "../../helpers/install-quasar-plugin";
import { Dialog, Notify } from "quasar";
import LogStream from "@/components/logstream/schema.vue";
import i18n from "@/locales";
import axios, { AxiosResponse } from "axios";
import MockAdapter from "axios-mock-adapter";
import streamFieldInputs from "@/components/logstream/streamFieldInputs.vue";
// @ts-ignore
import { rest } from "msw";
import store from "@/test/unit/helpers/store";
import StreamService from "@/services/stream";

import useStreams from "@/composables/useStreams";
import { onBeforeMount } from "vue";

installQuasar({
  plugins: [Dialog, Notify],
});
const mock = new MockAdapter(axios, { delayResponse: 0 });

mock
  .onGet("http://localhost:5080/api/default_organization_01/streams?type=logs")
  .reply((config) => {
    const list = [
      {
        name: "k8s_json",
        storage_type: "s3",
        stream_type: "logs",
        stats: {
          doc_time_min: 1678448628630259,
          doc_time_max: 1678448628652947,
          doc_num: 400,
          file_num: 1,
          storage_size: 0.74,
          compressed_size: 0.03,
        },
        schema: [
          {
            name: "_timestamp",
            type: "Int64",
          },
          {
            name: "kubernetes.container_hash",
            type: "Utf8",
          },
          {
            name: "log",
            type: "Utf8",
          },
          {
            name: "message",
            type: "Utf8",
          },
        ],
        settings: {
          partition_keys: {},
          full_text_search_keys: [],
          index_fields: [],
          bloom_filter_fields: [],
          defined_schema_fields: [],
          data_retention: 45,
        },
      },
    ];

    return [
      200,
      {
        list: list,
        message: "Stream settings updated successfully",
        code: 200,
      },
    ];
  });

mock
  .onGet(
    "http://localhost:5080/api/default_organization_01/streams/k8s_json/schema?type=logs",
  )
  .reply((config) => {
    return [
      200,
      {
        name: "k8s_json",
        storage_type: "s3",
        stream_type: "logs",
        stats: {
          doc_time_min: 1678448628630259,
          doc_time_max: 1678448628652947,
          doc_num: 400,
          file_num: 1,
          storage_size: 0.74,
          compressed_size: 0.03,
        },
        schema: [
          {
            name: "_timestamp",
            type: "Int64",
          },
          {
            name: "kubernetes.container_hash",
            type: "Utf8",
          },
          {
            name: "log",
            type: "Utf8",
          },
          {
            name: "message",
            type: "Utf8",
          },
        ],
        settings: {
          partition_keys: {},
          full_text_search_keys: [],
          index_fields: [],
          bloom_filter_fields: [],
          defined_schema_fields: [],
          data_retention: 45,
        },
      },
    ];
  });

mock
  .onPut(
    "http://localhost:5080/api/default_organization_01/streams/k8s_json/settings?type=logs",
  )
  .reply((config) => {
    console.log(config.data, "config.data");
    const data = {
      name: "k8s_json",
      storage_type: "s3",
      stream_type: "logs",
      stats: {
        doc_time_min: 1678448628630259,
        doc_time_max: 1678448628652947,
        doc_num: 400,
        file_num: 1,
        storage_size: 0.74,
        compressed_size: 0.03,
      },
      schema: [
        {
          name: "_timestamp",
          type: "Int64",
        },
        {
          name: "kubernetes.container_hash",
          type: "Utf8",
        },
        {
          name: "log",
          type: "Utf8",
        },
        {
          name: "message",
          type: "Utf8",
        },
      ],
      settings: {
        partition_keys: {},
        full_text_search_keys: [],
        index_fields: [],
        bloom_filter_fields: [],
        defined_schema_fields: [],
        data_retention: 45,
      },
    };
    return [
      200,
      {
        data: {
          message: "Stream settings updated successfully",
          code: 200,
        },
      },
    ];
  });

describe("Streams", async () => {
  let wrapper: any;

  beforeEach(async () => {
    vi.useFakeTimers();
    const updateStream = vi.spyOn(StreamService, "updateSettings");

    wrapper = mount(LogStream, {
      props: {
        modelValue: {
          name: "k8s_json",
          storage_type: "s3",
          stream_type: "logs",
          stats: {
            doc_time_min: 1678448628630259,
            doc_time_max: 1678448628652947,
            doc_num: 400,
            file_num: 1,
            storage_size: 0.74,
            compressed_size: 0.03,
          },
          schema: [
            {
              name: "_timestamp",
              type: "Int64",
            },
            {
              name: "kubernetes.container_hash",
              type: "Utf8",
            },
            {
              name: "log",
              type: "Utf8",
            },
            {
              name: "message",
              type: "Utf8",
            },
            {
              name: "test_this_field",
              type: "Utf8",
            },
          ],
          settings: {
            partition_keys: {},
            full_text_search_keys: [],
            index_fields: [],
            bloom_filter_fields: [],
            defined_schema_fields: [],
            data_retention: 45,
          },
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
    beforeEach(async () => {});
    it("Should make api call when user updates the form and click update settings ", async () => {
      // Find the q-toggle component wrapper
      const toggleWrapper = await wrapper.find(
        '[data-test="log-stream-store-original-data-toggle-btn"]',
      );

      // Verify initial state of the toggle
      const checkBox = await toggleWrapper.find('input[type="checkbox"]');
      expect(checkBox.element.checked).toBe(false);

      // Simulate a click on the toggle
      await checkBox.trigger("click");
      await wrapper.vm.$nextTick();

      await checkBox.setValue(true);
      expect(checkBox.element.checked).toBe(true);

      // Verify that formDirtyFlag has been updated
      expect(wrapper.vm.formDirtyFlag).toBe(true);

      // Find and trigger the update settings button
      const updateSettingsButton = wrapper.find(
        '[data-test="schema-update-settings-button"]',
      );
      expect(updateSettingsButton.attributes("disabled")).toBe(undefined); // Ensure it's enabled
      await updateSettingsButton.trigger("submit");

      // Simulate form submission
      // const settingsForm = wrapper.find('[data-test="settings-form"]');
      // await settingsForm.trigger("submit");
      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();

      // Verify that the API call was made (mocked in your setup)
      expect(updateStream).toHaveBeenCalledTimes(1); // Uncomment if you have a mocked function
    });
    it("Should prevent multiple API calls on rapid form submissions", async () => {
      // Simulate form changes
      const toggleWrapper = wrapper.find(
        '[data-test="log-stream-store-original-data-toggle-btn"]',
      );
      const checkBox = toggleWrapper.find('input[type="checkbox"]');
      await checkBox.setValue(true);
      await checkBox.trigger("click");
      await wrapper.vm.$nextTick();

      await checkBox.setValue(true);
      expect(checkBox.element.checked).toBe(true);

      const updateSettingsButton = wrapper.find(
        '[data-test="schema-update-settings-button"]',
      );

      // Rapidly trigger multiple submissions
      await Promise.all([
        updateSettingsButton.trigger("submit"),
        updateSettingsButton.trigger("submit"),
        updateSettingsButton.trigger("submit"),
      ]);

      await flushPromises();

      // Verify that only one API call was made
      expect(updateStream).toHaveBeenCalledTimes(1);
    });
    it("Should not call update settings API if retention days is less than 1", async () => {
      // Simulate form changes
      const retentionDaysInput = await wrapper.find(
        '[data-test="stream-details-data-retention-input"]',
      );

      await retentionDaysInput.setValue(0); // Set retention days to less than 1
      await retentionDaysInput.trigger("input");
      await wrapper.vm.$nextTick();

      // Verify that formDirtyFlag has been updated
      expect(wrapper.vm.formDirtyFlag).toBe(true);

      // Find and trigger the update settings button
      const updateSettingsButton = wrapper.find(
        '[data-test="schema-update-settings-button"]',
      );
      expect(updateSettingsButton.attributes("disabled")).toBe(undefined); // Ensure it's enabled
      await updateSettingsButton.trigger("submit");

      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();

      // Verify that the API call was not made
      expect(updateStream).toHaveBeenCalledTimes(0);
    });

    it("Should handle API error when updating settings", async () => {
      // Mock the API to return an error
      mock
        .onPut(
          "http://localhost:5080/api/default_organization_01/streams/k8s_json/settings?type=logs",
        )
        .reply(500, {
          message: "Internal Server Error",
        });

      // Simulate form changes
      const toggleWrapper = wrapper.find(
        '[data-test="log-stream-store-original-data-toggle-btn"]',
      );
      const checkBox = toggleWrapper.find('input[type="checkbox"]');
      await checkBox.setValue(true);
      await checkBox.trigger("click");
      await wrapper.vm.$nextTick();

      await checkBox.setValue(true);
      expect(checkBox.element.checked).toBe(true);

      // Verify that formDirtyFlag has been updated
      expect(wrapper.vm.formDirtyFlag).toBe(true);

      // Find and trigger the update settings button
      const updateSettingsButton = wrapper.find(
        '[data-test="schema-update-settings-button"]',
      );
      expect(updateSettingsButton.attributes("disabled")).toBe(undefined); // Ensure it's enabled
      await updateSettingsButton.trigger("submit");

      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();

      // Verify that the API call was made and handled the error
      expect(updateStream).toHaveBeenCalledTimes(1);
      expect(
        updateStream.mock.settledResults[0].value.response.data.message,
      ).toBe("Internal Server Error"); // Assuming you set an errorMessage in your component
    });
    it("Should update index type to prefixPartition when selected and verify using update call", async () => {
      mock
        .onPut(
          "http://localhost:5080/api/default_organization_01/streams/k8s_json/settings?type=logs",
        )
        .reply((config) => {
          console.log(config.data, "config.data");
          const data = {
            name: "k8s_json",
            storage_type: "s3",
            stream_type: "logs",
            stats: {
              doc_time_min: 1678448628630259,
              doc_time_max: 1678448628652947,
              doc_num: 400,
              file_num: 1,
              storage_size: 0.74,
              compressed_size: 0.03,
            },
            schema: [
              {
                name: "_timestamp",
                type: "Int64",
              },
              {
                name: "kubernetes.container_hash",
                type: "Utf8",
              },
              {
                name: "log",
                type: "Utf8",
              },
              {
                name: "message",
                type: "Utf8",
              },
            ],
            settings: {
              partition_keys: {},
              full_text_search_keys: [],
              index_fields: [],
              bloom_filter_fields: [],
              defined_schema_fields: [],
              data_retention: 45,
            },
          };
          return [
            200,
            {
              data: {
                message: "Stream settings updated successfully",
                code: 200,
              },
            },
          ];
        });
      // Find the index type select dropdown within the table row
      const indexTypeSelectWrapper = wrapper
        .find('[data-test="schema-log-stream-field-mapping-table"]')
        .findAll("tbody tr")
        .at(1) // Assuming you want to select the first row for this test
        .find('[data-test="schema-stream-index-select"]');
      console.log(indexTypeSelectWrapper.html(), "indexTypeSelectWrapper");
      const indexTypeSelector = indexTypeSelectWrapper.findComponent({
        name: "QSelect",
      });

      // Simulate selecting the "prefixPartition" option
      await indexTypeSelector.vm.$emit("update:modelValue", [
        "prefixPartition",
      ]);

      await wrapper.vm.$nextTick();

      // Verify that the selected value is "prefixPartition"
      // expect(indexTypeSelect.element.value).toBe("prefixPartition");

      // Verify that formDirtyFlag has been updated
      expect(wrapper.vm.formDirtyFlag).toBe(true);
      // Find and trigger the update settings button
      const updateSettingsButton = wrapper.find(
        '[data-test="schema-update-settings-button"]',
      );
      expect(updateSettingsButton.attributes("disabled")).toBe(undefined); // Ensure it's enabled
      await updateSettingsButton.trigger("submit");
      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      const parsedRequest = JSON.parse(
        updateStream.mock.settledResults[0].value.config.data,
      );

      parsedRequest.partition_keys.add.forEach((index: any) => {
        if (index.field == "kubernetes.container_hash") {
          expect(index.types).toEqual("prefix");
        }
      });
      // Verify that the API call was made
      expect(updateStream).toHaveBeenCalledTimes(1);
    });
    it("Should update index type to fullTextSearchKey when selected and verify using update call", async () => {
      mock
        .onPut(
          "http://localhost:5080/api/default_organization_01/streams/k8s_json/settings?type=logs",
        )
        .reply((config) => {
          const data = {
            name: "k8s_json",
            storage_type: "s3",
            stream_type: "logs",
            stats: {
              doc_time_min: 1678448628630259,
              doc_time_max: 1678448628652947,
              doc_num: 400,
              file_num: 1,
              storage_size: 0.74,
              compressed_size: 0.03,
            },
            schema: [
              {
                name: "_timestamp",
                type: "Int64",
              },
              {
                name: "kubernetes.container_hash",
                type: "Utf8",
              },
              {
                name: "log",
                type: "Utf8",
              },
              {
                name: "message",
                type: "Utf8",
              },
            ],
            settings: {
              partition_keys: {},
              full_text_search_keys: [],
              index_fields: [],
              bloom_filter_fields: [],
              defined_schema_fields: [],
              data_retention: 45,
            },
          };
          return [
            200,
            {
              data: {
                message: "Stream settings updated successfully",
                code: 200,
              },
            },
          ];
        });
      // Find the index type select dropdown within the table row
      const indexTypeSelectWrapper = wrapper
        .find('[data-test="schema-log-stream-field-mapping-table"]')
        .findAll("tbody tr")
        .at(2) // Assuming you want to select the first row for this test
        .find('[data-test="schema-stream-index-select"]');
      console.log(indexTypeSelectWrapper.html(), "indexTypeSelectWrapper");
      const indexTypeSelector = indexTypeSelectWrapper.findComponent({
        name: "QSelect",
      });

      // Simulate selecting the "prefixPartition" option
      await indexTypeSelector.vm.$emit("update:modelValue", [
        "fullTextSearchKey",
      ]);

      await wrapper.vm.$nextTick();

      // Verify that the selected value is "prefixPartition"
      // expect(indexTypeSelect.element.value).toBe("prefixPartition");

      // Verify that formDirtyFlag has been updated
      expect(wrapper.vm.formDirtyFlag).toBe(true);
      // Find and trigger the update settings button
      const updateSettingsButton = wrapper.find(
        '[data-test="schema-update-settings-button"]',
      );
      expect(updateSettingsButton.attributes("disabled")).toBe(undefined); // Ensure it's enabled
      await updateSettingsButton.trigger("submit");
      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      const parsedRequest = JSON.parse(
        updateStream.mock.settledResults[0].value.config.data,
      );
      expect(parsedRequest.full_text_search_keys.add[0]).toEqual("log");
      expect(parsedRequest.full_text_search_keys.add[1]).toEqual("message");
      // Verify that the API call was made
      expect(updateStream).toHaveBeenCalledTimes(1);
    });
    it("Should update index type to keyPartition when selected and verify using update call", async () => {
      mock
        .onPut(
          "http://localhost:5080/api/default_organization_01/streams/k8s_json/settings?type=logs",
        )
        .reply((config) => {
          const data = {
            name: "k8s_json",
            storage_type: "s3",
            stream_type: "logs",
            stats: {
              doc_time_min: 1678448628630259,
              doc_time_max: 1678448628652947,
              doc_num: 400,
              file_num: 1,
              storage_size: 0.74,
              compressed_size: 0.03,
            },
            schema: [
              {
                name: "_timestamp",
                type: "Int64",
              },
              {
                name: "kubernetes.container_hash",
                type: "Utf8",
              },
              {
                name: "log",
                type: "Utf8",
              },
              {
                name: "message",
                type: "Utf8",
              },
            ],
            settings: {
              partition_keys: {},
              full_text_search_keys: [],
              index_fields: [],
              bloom_filter_fields: [],
              defined_schema_fields: [],
              data_retention: 45,
            },
          };
          return [
            200,
            {
              data: {
                message: "Stream settings updated successfully",
                code: 200,
              },
            },
          ];
        });
      // Find the index type select dropdown within the table row
      const indexTypeSelectWrapper = wrapper
        .find('[data-test="schema-log-stream-field-mapping-table"]')
        .findAll("tbody tr")
        .at(2) // Assuming you want to select the first row for this test
        .find('[data-test="schema-stream-index-select"]');
      console.log(indexTypeSelectWrapper.html(), "indexTypeSelectWrapper");
      const indexTypeSelector = indexTypeSelectWrapper.findComponent({
        name: "QSelect",
      });

      // Simulate selecting the "prefixPartition" option
      await indexTypeSelector.vm.$emit("update:modelValue", ["keyPartition"]);

      await wrapper.vm.$nextTick();

      // Verify that the selected value is "prefixPartition"
      // expect(indexTypeSelect.element.value).toBe("prefixPartition");

      // Verify that formDirtyFlag has been updated
      expect(wrapper.vm.formDirtyFlag).toBe(true);
      // Find and trigger the update settings button
      const updateSettingsButton = wrapper.find(
        '[data-test="schema-update-settings-button"]',
      );
      expect(updateSettingsButton.attributes("disabled")).toBe(undefined); // Ensure it's enabled
      await updateSettingsButton.trigger("submit");
      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      const parsedRequest = JSON.parse(
        updateStream.mock.settledResults[0].value.config.data,
      );

      parsedRequest.partition_keys.add.forEach((index: any) => {
        console.log(index, "index");
        if (index.field == "log") {
          expect(index.types).toEqual("value");
        }
      });
      // Verify that the API call was made
      expect(updateStream).toHaveBeenCalledTimes(1);
    });
    it("Should update index type to hashPartition_8 when selected and verify using update call", async () => {
      mock
        .onPut(
          "http://localhost:5080/api/default_organization_01/streams/k8s_json/settings?type=logs",
        )
        .reply((config) => {
          const data = {
            name: "k8s_json",
            storage_type: "s3",
            stream_type: "logs",
            stats: {
              doc_time_min: 1678448628630259,
              doc_time_max: 1678448628652947,
              doc_num: 400,
              file_num: 1,
              storage_size: 0.74,
              compressed_size: 0.03,
            },
            schema: [
              {
                name: "_timestamp",
                type: "Int64",
              },
              {
                name: "kubernetes.container_hash",
                type: "Utf8",
              },
              {
                name: "log",
                type: "Utf8",
              },
              {
                name: "message",
                type: "Utf8",
              },
            ],
            settings: {
              partition_keys: {},
              full_text_search_keys: [],
              index_fields: [],
              bloom_filter_fields: [],
              defined_schema_fields: [],
              data_retention: 45,
            },
          };
          return [
            200,
            {
              data: {
                message: "Stream settings updated successfully",
                code: 200,
              },
            },
          ];
        });
      // Find the index type select dropdown within the table row
      const indexTypeSelectWrapper = wrapper
        .find('[data-test="schema-log-stream-field-mapping-table"]')
        .findAll("tbody tr")
        .at(3) // Assuming you want to select the first row for this test
        .find('[data-test="schema-stream-index-select"]');
      console.log(indexTypeSelectWrapper.html(), "indexTypeSelectWrapper");
      const indexTypeSelector = indexTypeSelectWrapper.findComponent({
        name: "QSelect",
      });

      // Simulate selecting the "prefixPartition" option
      await indexTypeSelector.vm.$emit("update:modelValue", [
        "hashPartition_8",
      ]);

      await wrapper.vm.$nextTick();

      // Verify that the selected value is "prefixPartition"
      // expect(indexTypeSelect.element.value).toBe("prefixPartition");

      // Verify that formDirtyFlag has been updated
      expect(wrapper.vm.formDirtyFlag).toBe(true);
      // Find and trigger the update settings button
      const updateSettingsButton = wrapper.find(
        '[data-test="schema-update-settings-button"]',
      );
      expect(updateSettingsButton.attributes("disabled")).toBe(undefined); // Ensure it's enabled
      await updateSettingsButton.trigger("submit");
      await flushPromises();
      await vi.advanceTimersByTime(500);
      await flushPromises();
      const parsedRequest = JSON.parse(
        updateStream.mock.settledResults[0].value.config.data,
      );

      parsedRequest.partition_keys.add.forEach((index: any) => {
        if (index.field == "message") {
          expect(index.types.hash).toEqual(8);
        }
      });
      // Verify that the API call was made
      expect(updateStream).toHaveBeenCalledTimes(1);
    });

    describe("When user adds a new field", () => {
      it("Should add a new field to the schema", async () => {
        
        });
      });
    });

    describe("disable options test cases", () => {
      it("disable Options: hould disable options correctly based on row data", async () => {
        const wrapper = mount(LogStream, {
          props: {
            modelValue: {
              name: "k8s_json",
              storage_type: "s3",
              stream_type: "logs",
              stats: {
                doc_time_min: 1678448628630259,
                doc_time_max: 1678448628652947,
                doc_num: 400,
                file_num: 1,
                storage_size: 0.74,
                compressed_size: 0.03,
              },
              schema: [
                {
                  name: "_timestamp",
                  type: "Int64",
                },
                {
                  name: "kubernetes.container_hash",
                  type: "Utf8",
                },
                {
                  name: "log",
                  type: "Utf8",
                },
                {
                  name: "message",
                  type: "Utf8",
                },
                {
                  name: "test_this_field",
                  type: "Utf8",
                },
              ],
              settings: {
                partition_keys: {},
                full_text_search_keys: [],
                index_fields: [],
                bloom_filter_fields: [],
                defined_schema_fields: [],
                data_retention: 45,
              },
            },
          },
          global: {
            provide: {
              store: store,
            },
            plugins: [i18n],
          },
        });

        const disableOptions = wrapper.vm.disableOptions;

        const row = {
          name: "log",
          index_type: ["fullTextSearchKey"],
        };

        const option = { value: "prefixPartition" };
        const result = disableOptions(row, option);
        expect(result).toBe(false);

        row.index_type = ["keyPartition"];
        const result2 = disableOptions(row, option);
        expect(result2).toBe(true);
      });
    });
    describe("filterFieldFn test cases", () => {
      const rows = [
        { name: "log" },
        { name: "message" },
        { name: "kubernetes.container_hash" },
        { name: "_timestamp" },
      ];

      const indexData = {
        value: {
          defined_schema_fields: ["log", "message"],
        },
      };

      it("should filter rows based on field name", () => {
        const filterFieldFn = wrapper.vm.filterFieldFn;

        const terms = "log";
        const result = filterFieldFn(rows, terms);
        expect(result).toEqual([{ name: "log" }]);
      });

      it("should filter rows based on field name and schemaFields type", () => {
        const filterFieldFn = wrapper.vm.filterFieldFn;

        const terms = "message";
        const result = filterFieldFn(rows, terms);
        expect(result).toEqual([{ name: "message" }]);
      });

      it("should return all rows if field is empty and type is schemaFields", () => {
        const filterFieldFn = wrapper.vm.filterFieldFn;

        const terms = "@schemaFields";
        const result = filterFieldFn(rows, terms);
        expect(result).toEqual([]);
      });

      it("should return all rows if field is empty", () => {
        const filterFieldFn = wrapper.vm.filterFieldFn;

        const terms = "";
        const result = filterFieldFn(rows, terms);
        expect(result).toEqual(rows);
      });

      it("should return empty array if no rows match the field", () => {
        const filterFieldFn = wrapper.vm.filterFieldFn;

        const terms = "nonexistent";
        const result = filterFieldFn(rows, terms);
        expect(result).toEqual([]);
      });

      it("should filter rows based on field name case insensitively", () => {
        const filterFieldFn = wrapper.vm.filterFieldFn;

        const terms = "LOG";
        const result = filterFieldFn(rows, terms);
        expect(result).toEqual([{ name: "log" }]);
      });
    });
  });
});
