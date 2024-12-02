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
import TemplateService from "@/services/alert_templates";
// @ts-ignore
import { rest } from "msw";
import { AddTemplate } from "@/components/alerts";
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
    wrapper = mount(AddTemplate, {
      attachTo: "#app",
      props: {
        template: null,
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

  it("Should render add template title", () => {
    expect(wrapper.find('[data-test="add-template-title"]').text()).toBe(
      "Add Template"
    );
  });

  it("Should render name input", () => {
    expect(
      wrapper.find('[data-test="add-template-name-input"]').exists()
    ).toBeTruthy();
  });

  it("Should render template body input", () => {
    expect(
      wrapper.find('[data-test="add-template-body-input-title"]').text()
    ).toBe("Body");
    expect(
      wrapper.find('[data-test="add-template-body-input"]').exists()
    ).toBeTruthy();
  });

  describe("When user fills form and clicks submit", () => {
    const submitBtn = vi.spyOn(TemplateService, "create");
    const template_name = "template1";
    afterEach(() => {
      vi.clearAllMocks();
    });
    beforeEach(async () => {
      global.server.use(
        rest.post(
          `${store.state.API_ENDPOINT}/api/${store.state.selectedOrganization.identifier}/alerts/templates/${template_name}`,
          (req: any, res: any, ctx: any) => {
            return res(ctx.status(200), ctx.json({ code: 200 }));
          }
        )
      );
      await wrapper
        .find('[data-test="add-template-name-input"]')
        .setValue(template_name);
      wrapper.vm.formData.body = JSON.stringify("This is alert.");
    });

    it("Should create alert on clicking Submit", async () => {
      await wrapper
        .find('[data-test="add-template-submit-btn"]')
        .trigger("click");
      await flushPromises();
      expect(submitBtn).toHaveBeenCalledTimes(1);
      expect(wrapper.emitted()["get:templates"]).toHaveLength(1);
    });

    it("Should create alert on clicking Submit", async () => {
      await wrapper
        .find('[data-test="add-template-cancel-btn"]')
        .trigger("click");
      expect(wrapper.emitted()["cancel:hideform"]).toHaveLength(1);
    });
  });
});
