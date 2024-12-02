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
import AlertList from "@/components/alerts/AlertList.vue";
import i18n from "@/locales";
import store from "../../helpers/store";
// @ts-ignore
import { rest } from "msw";
import AlertService from "@/services/alerts";
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
    wrapper = mount(AlertList, {
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
    expect(wrapper.find('[data-test="alerts-list-title"]').text()).toBe(
      "Alerts"
    );
  });
  it("Should reder table with alerts", () => {
    expect(
      wrapper.find('[data-test="alert-list-table"]').exists()
    ).toBeTruthy();
  });

  it("Should display table column headers", async () => {
    await flushPromises();
    const tableData = wrapper
      .find('[data-test="alert-list-table"]')
      .find("thead")
      .find("tr")
      .findAll("th");
    expect(tableData[0].text()).toBe("#");
    expect(tableData[1].text()).toContain("Name");
    expect(tableData[2].text()).toContain("Stream Type");
    expect(tableData[3].text()).toContain("Stream Name");
    expect(tableData[4].text()).toContain("Query");
    expect(tableData[5].text()).toContain("Condition");
    expect(tableData[6].text()).toContain("Destination");
    expect(tableData[7].text()).toContain("Actions");
  });

  it("Should display table row data", async () => {
    await flushPromises();
    const tableData = wrapper
      .find('[data-test="alert-list-table"]')
      .find("tbody")
      .find("tr")
      .findAll("td");
    expect(tableData[0].text()).toBe("01");
    expect(tableData[1].text()).toBe("alert1");
    expect(tableData[2].text()).toContain("logs");
    expect(tableData[3].text()).toBe("default");
    expect(tableData[4].text()).toBe("--");
    expect(tableData[5].text()).toBe("code EqualTo 500");
    expect(tableData[6].text()).toBe("dest1");
  });

  it("Should display add alert button", () => {
    expect(wrapper.find('[data-test="alert-list-add-alert-btn"]').text()).toBe(
      "Add alert"
    );
  });
  it("Should move to add alerts page on clicking on add alert", async () => {
    const routerPush = vi.spyOn(router, "push");
    await wrapper
      .find('[data-test="alert-list-add-alert-btn"]')
      .trigger("click");
    expect(routerPush).toHaveBeenCalledTimes(1);
    expect(routerPush).toHaveBeenCalledWith({
      name: "alertList",
      query: {
        action: "add",
        org_identifier: "default_organization_01",
      },
    });
  });

  describe("When user clicks on edit alert", () => {
    const stream_name = "default";
    const alert_name = "alert1";
    const deleteAlert = vi.spyOn(AlertService, "delete");
    beforeEach(async () => {
      global.server.use(
        rest.delete(
          `${store.state.API_ENDPOINT}/api/${store.state.selectedOrganization.identifier}/${stream_name}/alerts/${alert_name}`,
          (req: any, res: any, ctx: any) => {
            return res(ctx.status(200), ctx.json({ code: 200 }));
          }
        )
      );
      global.server.use(
        rest.get(
          `${store.state.API_ENDPOINT}/api/${store.state.selectedOrganization.identifier}/alerts`,
          (req: any, res: any, ctx: any) => {
            return res(
              ctx.status(200),
              ctx.json({
                list: [
                  {
                    name: "alert2",
                    stream: "default",
                    query: {
                      sql: "select * from  'default'",
                      from: 0,
                      size: 100,
                      start_time: 0,
                      end_time: 0,
                      sql_mode: "full",
                      query_type: "",
                      track_total_hits: false,
                    },
                    condition: {
                      column: "code",
                      operator: "EqualTo",
                      ignore_case: null,
                      value: 500,
                    },
                    duration: 1,
                    frequency: 2,
                    time_between_alerts: 2,
                    destination: "dest1",
                    is_real_time: false,
                  },
                ],
              })
            );
          }
        )
      );
      await wrapper
        .find(`[data-test=alert-list-${alert_name}-delete-alert]`)
        .trigger("click");
      const mainWrapper = new DOMWrapper(document.body);
      await mainWrapper.find('[data-test="confirm-button"]').trigger("click");
      await flushPromises();
    });

    it("Should delete alert from the list", () => {
      expect(deleteAlert).toHaveBeenCalledTimes(1);
    });
    it("Should refetch all alerts", () => {
      const tableRows = wrapper
        .find('[data-test="alert-list-table"]')
        .find("thead")
        .findAll("tr");
      expect(tableRows.length).toBe(1);
      expect(tableRows[0].html()).not.toContain(alert_name);
    });
  });
});
