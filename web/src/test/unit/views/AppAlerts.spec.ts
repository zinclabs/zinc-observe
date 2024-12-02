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
import { mount } from "@vue/test-utils";
import { installQuasar } from "../helpers/install-quasar-plugin";
import { Dialog, Notify } from "quasar";
import AppAlerts from "../../../views/AppAlerts.vue";
import i18n from "../../../locales";
import store from "../helpers/store";
import router from "../helpers/router";

installQuasar({
  plugins: [Dialog, Notify],
});

describe("Streams", async () => {
  let wrapper: any;
  beforeEach(() => {
    vi.useFakeTimers();
    wrapper = mount(AppAlerts, {
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

  it("Should render tabs", () => {
    expect(wrapper.find('[data-test="alert-tabs"]').exists()).toBeTruthy();
  });
  it("Should render alerts tab", () => {
    expect(wrapper.find('[data-test="alert-alerts-tab"]').text()).toBe(
      "Alerts"
    );
  });
  it("Should render destinations tabs", () => {
    expect(wrapper.find('[data-test="alert-destinations-tab"]').text()).toBe(
      "Destinations"
    );
  });
  it("Should render templates tabs", () => {
    expect(wrapper.find('[data-test="alert-templates-tab"]').text()).toBe(
      "Templates"
    );
  });
});
