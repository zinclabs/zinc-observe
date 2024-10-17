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
import AddAlert from "@/components/alerts/AddAlert.vue";
import i18n from "@/locales";
import store from "../../helpers/store";
import AlertService from "@/services/alerts";
// @ts-ignore
import { rest } from "msw";
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
    wrapper = mount(AddAlert, {
      attachTo: "#app",
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

  it("Should render alerts title", () => {
    expect(wrapper.find('[data-test="add-alert-title"]').text()).toBe(
      "Add Alert"
    );
  });

  it("Should render name input", () => {
    expect(
      wrapper.find('[data-test="add-alert-name-input"]').exists()
    ).toBeTruthy();
  });

  it("Should render stream input", () => {
    expect(
      wrapper.find('[data-test="add-alert-stream-select"]').exists()
    ).toBeTruthy();
  });

  it("Should by default select Scheduled type alerts", () => {
    expect(
      wrapper
        .find('[data-test="add-alert-scheduled-alert-radio"]')
        .find(".q-radio__inner--truthy")
        .exists()
    ).toBeTruthy();
  });

  it("Should reder condition", () => {
    expect(wrapper.find('[data-test="add-alert-condition-title"]').text()).toBe(
      "Condition:"
    );
    expect(
      wrapper.find('[data-test="add-alert-condition-column-select"]').exists()
    ).toBeTruthy();
    expect(
      wrapper.find('[data-test="add-alert-condition-operator-select"]').exists()
    ).toBeTruthy();
    expect(
      wrapper.find('[data-test="add-alert-condition-value-input"]').exists()
    ).toBeTruthy();
  });

  it("Should reder sql query input", () => {
    expect(
      wrapper.find('[data-test="add-alert-query-input-title"]').text()
    ).toBe("Query:");
    expect(
      wrapper.find('[data-test="add-alert-query-input"]').exists()
    ).toBeTruthy();
  });

  it("Should reder Duration input", () => {
    expect(wrapper.find('[data-test="add-alert-duration-title"]').text()).toBe(
      "Duration for which alert is evaluated:"
    );
    expect(
      wrapper.find('[data-test="add-alert-duration-input"]').exists()
    ).toBeTruthy();
    expect(
      wrapper.find('[data-test="add-alert-duration-select"]').exists()
    ).toBeTruthy();
  });

  it("Should reder Frequency input", () => {
    expect(wrapper.find('[data-test="add-alert-frequency-title"]').text()).toBe(
      "Frequency at which alert is evaluated:"
    );
    expect(
      wrapper.find('[data-test="add-alert-frequency-input"]').exists()
    ).toBeTruthy();
    expect(
      wrapper.find('[data-test="add-alert-frequency-select"]').exists()
    ).toBeTruthy();
  });

  it("Should reder Delay notification input", () => {
    expect(wrapper.find('[data-test="add-alert-delay-title"]').text()).toBe(
      "Delay notification until, after one notification:"
    );
    expect(
      wrapper.find('[data-test="add-alert-delay-input"]').exists()
    ).toBeTruthy();
    expect(
      wrapper.find('[data-test="add-alert-delay-select"]').exists()
    ).toBeTruthy();
  });

  it("Should reder destination select input", () => {
    expect(
      wrapper.find('[data-test="add-alert-destination-select"]').exists()
    ).toBeTruthy();
  });

  describe("When user fills form and clicks submit", () => {
    const submitBtn = vi.spyOn(AlertService, "create");
    const stream_name = "k8s_json";
    const alert_name = "alert1";
    afterEach(() => {
      vi.clearAllMocks();
    });
    beforeEach(async () => {
      global.server.use(
        rest.post(
          `${store.state.API_ENDPOINT}/api/${store.state.selectedOrganization.identifier}/${stream_name}/alerts/${alert_name}`,
          (req: any, res: any, ctx: any) => {
            return res(ctx.status(200), ctx.json({ code: 200 }));
          }
        )
      );
      await wrapper
        .find('[data-test="add-alert-name-input"]')
        .setValue(alert_name);
      wrapper.vm.formData.stream_name = stream_name;
      wrapper.vm.formData.condition.column = "kubernetes.host";
      wrapper.vm.formData.condition.operator = "=";
      await wrapper
        .find('[data-test="add-alert-condition-value-input"]')
        .setValue("abc");
      await wrapper.find('[data-test="add-alert-duration-input"]').setValue(1);
      await wrapper.find('[data-test="add-alert-frequency-input"]').setValue(1);
      await wrapper.find('[data-test="add-alert-delay-input"]').setValue(1);
      wrapper.vm.formData.destination = "dest1";
    });

    it("Should create alert on clicking Submit", async () => {
      await wrapper.find('[data-test="add-alert-submit-btn"]').trigger("click");
      await flushPromises();
      expect(submitBtn).toHaveBeenCalledTimes(1);
      expect(wrapper.emitted()["update:list"]).toHaveLength(1);
    });

    it("Should create alert on clicking Submit", async () => {
      await wrapper.find('[data-test="add-alert-cancel-btn"]').trigger("click");
      expect(wrapper.emitted()["cancel:hideform"]).toHaveLength(1);
    });
  });
});
