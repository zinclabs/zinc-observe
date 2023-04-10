// Copyright 2022 Zinc Labs Inc. and Contributors

//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at

//      http:www.apache.org/licenses/LICENSE-2.0

//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

import { describe, expect, it, beforeEach, vi, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { installQuasar } from "../../helpers/install-quasar-plugin";
import { Dialog, Notify } from "quasar";

import Index from "@/plugins/logs/Index.vue";
import IndexList from "@/plugins/logs/IndexList.vue";
import i18n from "@/locales";
import store from "../../helpers/store";
import "plotly.js";
import SearchResult from "@/plugins/logs/SearchResult.vue";
import router from "../../helpers/router";

const node = document.createElement("div");
node.setAttribute("id", "app");
document.body.appendChild(node);

installQuasar({
  plugins: [Dialog, Notify],
});

describe("Search Result", async () => {
  let wrapper: any;
  beforeEach(async () => {
    vi.useFakeTimers();
    wrapper = mount(Index, {
      attachTo: "#app",
      global: {
        provide: {
          store: store,
        },
        plugins: [i18n, router],
        stubs: {},
      },
    });
    await flushPromises();
    vi.advanceTimersByTime(2000);
    await flushPromises();
  });

  afterEach(() => {
    wrapper.unmount();
    vi.restoreAllMocks();
    // vi.clearAllMocks();
  });

  it("Should show stream select component", async () => {
    expect(
      wrapper
        .findComponent(IndexList)
        .find('[data-test="log-search-index-list-select-stream"]')
        .exists()
    ).toBeTruthy();
  });
  it("Should show the BarChart component when showHistogram is true and sqlMode is false", async () => {
    expect(
      wrapper
        .findComponent(IndexList)
        .find('[data-test="log-search-index-list-field-search-input"]')
        .exists()
    ).toBeTruthy();
  });
  it("Should show the BarChart component when showHistogram is true and sqlMode is false", async () => {
    expect(
      wrapper
        .findComponent(IndexList)
        .find('[data-test="log-search-index-list-fields-table"]')
        .exists()
    ).toBeTruthy();
  });

  it("Should add field to table when clicked on add field", async () => {
    const field = "kubernetes_container_hash";

    await wrapper
      .findComponent(IndexList)
      .find(`[data-test="log-search-index-list-add-${field}-field-btn"]`)
      .trigger("click");

    await vi.advanceTimersByTime(500);
    expect(
      wrapper
        .findComponent(SearchResult)
        .find(`[data-test="log-search-result-table-th-${field}"]`)
        .exists()
    ).toBeTruthy();
  });

  it("Should add field to query when clicked on filter field", async () => {
    const field = "kubernetes_container_hash";

    await wrapper
      .findComponent(IndexList)
      .find(`[data-test="log-search-index-list-filter-${field}-field-btn"]`)
      .trigger("click");

    expect(wrapper.vm.searchObj.data.query).toContain(field);
  });

  it("Should remove field from table when clicked on added field", async () => {
    const field = "kubernetes.container_hash";
    await wrapper
      .findComponent(IndexList)
      .find(`[data-test="log-search-index-list-add-${field}-field-btn"]`)
      .trigger("click");

    await wrapper
      .findComponent(IndexList)
      .find(`[data-test="log-search-index-list-remove-${field}-field-btn"]`)
      .trigger("click");

    expect(
      wrapper
        .findComponent(SearchResult)
        .find(`[data-test="log-search-result-table-th-${field}"]`)
        .exists()
    ).toBeFalsy();
  });

  it("Should filter fields when searched for specific field", async () => {
    const field = "kubernetes.container_hash";
    expect(wrapper.findComponent(IndexList).text()).toContain("_timestamp");
    await wrapper
      .findComponent(IndexList)
      .find(`[data-test="log-search-index-list-field-search-input"]`)
      .setValue(field);
    expect(wrapper.findComponent(IndexList).text()).not.toContain("_timestamp");
    expect(wrapper.findComponent(IndexList).text()).toContain(field);
  });
});
