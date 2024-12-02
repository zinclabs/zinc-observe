// Copyright 2023 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

import { describe, expect, it, beforeEach, vi, afterEach } from "vitest";
import { mount, flushPromises, DOMWrapper } from "@vue/test-utils";
import { installQuasar } from "../../helpers/install-quasar-plugin";
import { Dialog, Notify } from "quasar";

import Index from "@/plugins/logs/Index.vue";
import SearchBar from "@/plugins/logs/SearchBar.vue";
import i18n from "@/locales";
import store from "../../helpers/store";
import "plotly.js";
import searchService from "@/services/search";
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
        stubs: {
          IndexList: true,
        },
      },
    });
    wrapper.vm.router.currentRoute.value.name = "logs";
    await flushPromises();
    vi.advanceTimersByTime(500);
    await flushPromises();
  });

  afterEach(() => {
    wrapper.unmount();
    vi.restoreAllMocks();
    // vi.clearAllMocks();
  });

  it("Should render show query toggle btn", () => {
    expect(
      wrapper
        .find('[data-test="logs-search-bar-show-query-toggle-btn"]')
        .exists()
    ).toBeTruthy();
  });

  it("Should render show fields toggle btn", () => {
    expect(
      wrapper
        .find('[data-test="logs-search-bar-show-fields-toggle-btn"]')
        .exists()
    ).toBeTruthy();
  });

  it("Should render show histogram toggle btn", () => {
    expect(
      wrapper
        .find('[data-test="logs-search-bar-show-histogram-toggle-btn"]')
        .exists()
    ).toBeTruthy();
  });

  it("Should render sql mode toggle btn", () => {
    expect(
      wrapper.find('[data-test="logs-search-bar-sql-mode-toggle-btn"]').exists()
    ).toBeTruthy();
  });

  it("should enable histogram toggle btn and histogram chart when custom sql is on", async () => {
    expect(
      wrapper
        .findComponent(SearchResult)
        .find('[data-test="logs-search-result-bar-chart"]')
        .isVisible()
    ).toBe(true);

    expect(
      wrapper
        .findComponent(SearchBar)
        .find('[data-test="logs-search-bar-show-histogram-toggle-btn"]')
        .classes("disabled")
    ).toBe(false);
  });

  it("should disable histogram when full sql mode is on", async () => {
    await wrapper
      .find('[data-test="logs-search-bar-sql-mode-toggle-btn"]')
      .trigger("click");

    expect(
      wrapper
        .findComponent(SearchResult)
        .find('[data-test="logs-search-result-bar-chart"]')
        .isVisible()
    ).toBe(false);

    expect(
      wrapper
        .findComponent(SearchBar)
        .find('[data-test="logs-search-bar-show-histogram-toggle-btn"]')
        .classes("disabled")
    ).toBe(true);
  });

  it("Should emit searchdata when sql mode is on and run query is clicked", async () => {
    wrapper.vm.searchObj.data.query =
      "SELECT *, fn6(_timestamp) FROM 'k8s_json' WHERE code=200 limit 100 offset 100";

    await wrapper
      .find('[data-test="logs-search-bar-refresh-btn"]')
      .trigger("click");

    expect(
      wrapper.findComponent(SearchBar).emitted()["searchdata"]
    ).toBeTruthy();
  });

  it("Should add where clause when switched to sql mode", async () => {
    if (wrapper.vm.searchObj.meta.sqlMode) {
      await wrapper
        .find('[data-test="logs-search-bar-sql-mode-toggle-btn"]')
        .trigger("click");
    }

    wrapper.vm.searchObj.data.query = "fn6(_timestamp) as t1 | code=500";

    await wrapper
      .find('[data-test="logs-search-bar-sql-mode-toggle-btn"]')
      .trigger("click");

    expect(wrapper.vm.searchObj.data.query).toBe(
      'SELECT *,fn6(_timestamp) as t1 FROM "k8s_json" WHERE code=500'
    );
  });

  it("should disable histogram chart when show histogram is toggled", async () => {
    if (wrapper.vm.searchObj.meta.sqlMode) {
      await wrapper
        .find('[data-test="logs-search-bar-sql-mode-toggle-btn"]')
        .trigger("click");
    }

    await wrapper
      .find('[data-test="logs-search-bar-show-histogram-toggle-btn"]')
      .trigger("click");

    expect(
      wrapper
        .findComponent(SearchResult)
        .find('[data-test="logs-search-result-bar-chart"]')
        .isVisible()
    ).toBe(false);
  });

  it("Should render date selection", () => {
    expect(
      wrapper.find('[data-test="logs-search-bar-date-time-dropdown"]').exists()
    ).toBeTruthy();
  });

  it("Should render Run Query btn and emit searchdata on click", async () => {
    if (wrapper.vm.searchObj.meta.sqlMode) {
      await wrapper
        .find('[data-test="logs-search-bar-sql-mode-toggle-btn"]')
        .trigger("click");
    }

    wrapper.vm.searchObj.data.editorValue = "fn6(_timestamp) as t1 | code=500";

    await wrapper
      .find('[data-test="logs-search-bar-refresh-btn"]')
      .trigger("click");

    expect(
      wrapper.findComponent(SearchBar).emitted()["searchdata"]
    ).toBeTruthy();
  });

  it("Should get logs after 5 secs when search interval is updated to 5 secs", async () => {
    const search = vi.spyOn(searchService, "search");
    const domWrapper = new DOMWrapper(document.body);
    wrapper.vm.router.currentRoute.value.name = "logs";
    await wrapper
      .find('[data-test="logs-search-refresh-interval-dropdown-btn"]')
      .trigger("click");

    await domWrapper
      .find('[data-test="logs-search-bar-refresh-time-5"]')
      .trigger("click");

    await vi.advanceTimersByTime(5000);
    expect(search).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTime(5000);
    expect(search).toHaveBeenCalledTimes(2);
  });

  it("Should get logs when date time is updated", async () => {
    const search = vi.spyOn(searchService, "search");
    const domWrapper = new DOMWrapper(document.body);
    await wrapper.find('[data-test="date-time-btn"]').trigger("click");

    await domWrapper
      .find('[data-test="date-time-relative-1-Weeks-btn"]')
      .trigger("click");

    await vi.advanceTimersByTime(300);
    expect(search).toHaveBeenCalledTimes(1);
  });
});
