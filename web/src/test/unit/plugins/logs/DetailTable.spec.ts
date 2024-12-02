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

import DetailTable from "@/plugins/logs/DetailTable.vue";
import i18n from "@/locales";
import store from "../../helpers/store";
import router from "../../helpers/router";

const node = document.createElement("div");
node.setAttribute("id", "app");
document.body.appendChild(node);

installQuasar({
  plugins: [Dialog, Notify],
});

describe("Alert List", async () => {
  let wrapper: any;
  beforeEach(async () => {
    vi.useFakeTimers();
    wrapper = mount(DetailTable, {
      attachTo: "#app",
      props: {
        currentIndex: 0,
        modelValue: {
          _timestamp: "1680246906650420",
          kubernetes_container_name: "ziox",
          kubernetes_container_hash:
            "058694856476.dkr.ecr.us-west-2.amazonaws.com/ziox@sha256:3dbbb0dc1eab2d5a3b3e4a75fd87d194e8095c92d7b2b62e7cdbd07020f54589",
        },
        totalLength: 10,
      },
      global: {
        provide: {
          store: store,
        },
        plugins: [i18n, router],
      },
    });
    await flushPromises();
  });

  afterEach(() => {
    wrapper.unmount();
  });

  it("should render log details title", async () => {
    expect(wrapper.find('[data-test="log-detail-title-text"]').text()).toBe(
      "Source Details"
    );
  });

  it("should render tabs and tab panels", () => {
    expect(wrapper.find('[data-test="log-detail-table-tab"]').exists()).toBe(
      true
    );
    expect(wrapper.find('[data-test="log-detail-json-tab"]').exists()).toBe(
      true
    );
    expect(wrapper.find('[data-test="log-detail-table-tab"]').text()).toBe(
      "Table"
    );
    expect(wrapper.find('[data-test="log-detail-json-tab"]').text()).toBe(
      "JSON"
    );
  });

  it("Should render active tab as table", () => {
    expect(
      wrapper.find('[data-test="log-detail-table-tab"]').classes()
    ).toContain("q-tab--active");
    expect(
      wrapper.find('[data-test="log-detail-json-tab"]').classes()
    ).not.toContain("q-tab--active");
  });

  it('should render "No data available." if rowData is empty', async () => {
    wrapper.vm.rowData = [];
    await flushPromises();
    expect(
      wrapper.find('[data-test="log-detail-tab-container"]').text()
    ).toContain("No data available.");
  });

  it("should render rowData if it is not empty", async () => {
    const logDetails = Object.entries(wrapper.props().modelValue);
    expect(
      wrapper.find(`[data-test="log-detail-${logDetails[0][0]}-key"]`).text()
    ).toBe(logDetails[0][0]);
    expect(
      wrapper.find(`[data-test="log-detail-${logDetails[0][0]}-value"]`).text()
    ).toBe(logDetails[0][1]);
    expect(
      wrapper.find(`[data-test="log-detail-${logDetails[1][0]}-key"]`).text()
    ).toBe(logDetails[1][0]);
    expect(
      wrapper.find(`[data-test="log-detail-${logDetails[1][0]}-value"]`).text()
    ).toBe(logDetails[1][1]);
  });

  it("should toggle shouldWrapValues when toggleWrapLogDetails is called", async () => {
    expect(
      wrapper.find('[data-test="log-detail-wrap-values-toggle-btn"]').text()
    ).toBe("Wrap");
    expect(wrapper.vm.shouldWrapValues).toBe(false);
    await wrapper
      .find('[data-test="log-detail-wrap-values-toggle-btn"]')
      .trigger("click");
    expect(wrapper.vm.shouldWrapValues).toBe(true);
  });

  it('should emit "showPrevDetail" when "Previous" button is clicked', async () => {
    await wrapper
      .find('[data-test="log-detail-next-detail-btn"]')
      .trigger("click");
    expect(wrapper.emitted().showNextDetail).toBeTruthy();
  });

  it("Should render json when user clicks on json tab", async () => {
    await wrapper.find('[data-test="log-detail-json-tab"]').trigger("click");
    expect(
      wrapper.find('[data-test="log-detail-table-content"]').exists()
    ).toBeFalsy();
    expect(
      wrapper.find('[data-test="log-detail-json-content"]').exists()
    ).toBeTruthy();
    expect(
      wrapper.find('[data-test="log-detail-json-content"]').text()
    ).toMatchSnapshot();
  });

  describe("When user includes or excludes log field", () => {
    const domWrapper = new DOMWrapper(document.body);
    beforeEach(async () => {
      await wrapper
        .find('[data-test="log-details-include-exclude-field-btn"]')
        .trigger("click");
      await flushPromises();
    });

    it("Should show include and exclude field buttons", () => {
      expect(
        domWrapper.find('[data-test="log-details-include-field-btn"]').exists()
      ).toBe(true);
      expect(
        domWrapper.find('[data-test="log-details-exclude-field-btn"]').exists()
      ).toBe(true);
    });

    it("Should show include and exclude field buttons", () => {
      expect(
        domWrapper.find('[data-test="log-details-include-field-btn"]').exists()
      ).toBe(true);
      expect(
        domWrapper.find('[data-test="log-details-exclude-field-btn"]').exists()
      ).toBe(true);
    });

    it("Should emit event add:searchterm when clicked on include", async () => {
      await domWrapper
        .find('[data-test="log-details-include-field-btn"]')
        .trigger("click");
      expect(wrapper.emitted()["add:searchterm"]).toBeTruthy();
    });

    it("Should emit event add:searchterm when clicked on exclude", async () => {
      await domWrapper
        .find('[data-test="log-details-exclude-field-btn"]')
        .trigger("click");
      expect(wrapper.emitted()["add:searchterm"]).toBeTruthy();
    });
  });
});
