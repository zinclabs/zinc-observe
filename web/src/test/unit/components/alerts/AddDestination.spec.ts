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
import i18n from "@/locales";
import store from "../../helpers/store";
import DestinationService from "@/services/alert_destination";
// @ts-ignore
import { rest } from "msw";
import { AddDestination } from "@/components/alerts";
import router from "../../helpers/router";

installQuasar({
  plugins: [Dialog, Notify],
});

const node = document.createElement("div");
node.setAttribute("id", "app");
document.body.appendChild(node);

describe("Alert List", async () => {
  let wrapper: any;
  beforeEach(async () => {
    vi.useFakeTimers();
    wrapper = mount(AddDestination, {
      attachTo: "#app",
      props: {
        templates: [],
        destination: null,
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

  it("Should render add destination title", () => {
    expect(wrapper.find('[data-test="add-destination-title"]').text()).toBe(
      "Add Destination"
    );
  });

  it("Should render name input", () => {
    expect(
      wrapper.find('[data-test="add-destination-name-input"]').exists()
    ).toBeTruthy();
  });

  it("Should render template select", () => {
    expect(
      wrapper.find('[data-test="add-destination-template-select"]').exists()
    ).toBeTruthy();
  });

  it("Should render url input", () => {
    expect(
      wrapper.find('[data-test="add-destination-url-input"]').exists()
    ).toBeTruthy();
  });

  it("Should reder method select input", () => {
    expect(
      wrapper.find('[data-test="add-destination-method-select"]').exists()
    ).toBeTruthy();
  });

  it("Should reder add headers inputs", () => {
    const headerKey = "";
    const headerValue = "";
    expect(
      wrapper
        .find(`[data-test="add-destination-header-${headerKey}-key-input"]`)
        .exists()
    ).toBeTruthy();
    expect(
      wrapper
        .find(`[data-test="add-destination-header-${headerValue}-value-input"]`)
        .exists()
    ).toBeTruthy();
  });

  describe("When user fills form and clicks submit", () => {
    const submitBtn = vi.spyOn(DestinationService, "create");
    const template_name = "template1";
    const dest_name = "dest1";
    afterEach(() => {
      vi.clearAllMocks();
    });
    beforeEach(async () => {
      global.server.use(
        rest.post(
          `${store.state.API_ENDPOINT}/api/${store.state.selectedOrganization.identifier}/alerts/destinations/${dest_name}`,
          (req: any, res: any, ctx: any) => {
            return res(ctx.status(200), ctx.json({ code: 200 }));
          }
        )
      );
      await wrapper
        .find('[data-test="add-destination-name-input"]')
        .setValue(dest_name);
      wrapper.vm.formData.template = template_name;
      await wrapper
        .find('[data-test="add-destination-url-input"]')
        .setValue("abc");
      wrapper.vm.formData.method = "post";
      await wrapper
        .find('[data-test="add-destination-header--key-input"]')
        .setValue("key");
      await wrapper
        .find('[data-test="add-destination-header-key-value-input"]')
        .setValue("value");
    });

    it("Should create alert on clicking Submit", async () => {
      await wrapper
        .find('[data-test="add-destination-submit-btn"]')
        .trigger("click");
      await flushPromises();
      expect(submitBtn).toHaveBeenCalledTimes(1);
      expect(wrapper.emitted()["get:destinations"]).toHaveLength(1);
    });

    it("Should create alert on clicking Submit", async () => {
      await wrapper
        .find('[data-test="add-destination-cancel-btn"]')
        .trigger("click");
      expect(wrapper.emitted()["cancel:hideform"]).toHaveLength(1);
    });
  });
});
