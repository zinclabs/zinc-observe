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
import { mount, flushPromises } from "@vue/test-utils";
import { installQuasar } from "../../helpers/install-quasar-plugin";
import { Dialog, Notify } from "quasar";

import Index from "@/plugins/logs/Index.vue";
import SearchResult from "@/plugins/logs/SearchResult.vue";
// import BarChart from "@/components/logBarChart.vue";
import i18n from "@/locales";
import store from "../../helpers/store";
// @ts-ignore
import { rest } from "msw";
// import "plotly.js";
import DetailTable from "@/plugins/logs/DetailTable.vue";
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
          SearchBar: true,
          IndexList: true,
        },
      },
    });
    await flushPromises();
    vi.advanceTimersByTime(500);
    await flushPromises();
  });

  afterEach(() => {
    wrapper.unmount();
    vi.restoreAllMocks();
    // vi.clearAllMocks();
  });

  it("Should show the BarChart component when showHistogram is true and sqlMode is false", async () => {
    expect(
      wrapper.find('[data-test="logs-search-result-bar-chart"]').exists()
    ).toBeTruthy();
  });

  // it("Should call the 'onChartUpdate' method when the 'updated:chart' event is emitted by the BarChart component", async () => {
  //   await wrapper.findComponent(BarChart).vm.$emit("updated:chart", {
  //     start: "2000-04-08 05:13:29",
  //     end: "2000-07-06 18:53:45",
  //   });
  //   expect(
  //     wrapper.findComponent(SearchResult).emitted()["update:datetime"]
  //   ).toBeTruthy();
  // });

  //   it("Should render the q-virtual-scroll component with the correct props and call the 'onScroll' method when the 'virtual-scroll' event is emitted", () => {
  //     expect(
  //       wrapper.find('[data-test="logs-search-result-logs-table"]').html()
  //     ).toMatchSnapshot();
  //   });

  describe("When user opens log details", () => {
    let expandRowDetail: any, hits: any[], addSearchTerm: any;
    beforeEach(async () => {
      expandRowDetail = vi.spyOn(
        wrapper.findComponent(SearchResult).vm,
        "expandRowDetail"
      );
      addSearchTerm = vi.spyOn(
        wrapper.findComponent(SearchResult).vm,
        "addSearchTerm"
      );
      hits = wrapper.vm.searchObj.data.queryResults.hits;
      await wrapper
        .find(
          `[data-test="logs-search-result-detail-${wrapper.vm.searchObj.data.queryResults.hits[0]["_timestamp"]}"]`
        )
        .trigger("click");
    });
    it('Should call the "expandRowDetail" method when a row is clicked', async () => {
      expect(expandRowDetail).toHaveBeenCalledTimes(1);
      expect(wrapper.findComponent(DetailTable).text()).toContain(
        hits[0]["_timestamp"]
      );
    });

    it('Should call "navigateRowDetail" on "showNextDetail" or "showPrevDetail" event from DetailTable', async () => {
      await wrapper
        .findComponent(DetailTable)
        .find('[data-test="log-detail-next-detail-btn"]')
        .trigger("click");
      expect(wrapper.findComponent(DetailTable).text()).toContain(
        hits[1]["_timestamp"]
      );
      expect(wrapper.findComponent(DetailTable).text()).not.toContain(
        hits[0]["_timestamp"]
      );
    });

    it('Should add or remove search terms on "add:searchterm" or "remove:searchterm" event from DetailTable', () => {
      wrapper
        .findComponent(DetailTable)
        .vm.$emit("add:searchterm", "_timestamp");
      expect(addSearchTerm).toHaveBeenCalledTimes(1);

      wrapper
        .findComponent(DetailTable)
        .vm.$emit("remove:searchterm", "_timestamp");
      expect(
        wrapper.findComponent(SearchResult).emitted()["remove:searchTerm"]
      ).toBeTruthy();
    });

    it('Should execute timeboxed search on "search:timeboxed" event from DetailTable component', async () => {
      global.server.use(
        rest.get(
          `${store.state.API_ENDPOINT}/api/${store.state.selectedOrganization.identifier}/k8s_json/_around`,
          (req: any, res: any, ctx: any) => {
            return res(
              ctx.status(200),
              ctx.json({
                took: 10,
                hits: [
                  {
                    _timestamp: 1680246906641572,
                    kubernetes_annotations_kubernetes_io_psp: "eks.privileged",
                    kubernetes_container_hash:
                      "058694856476.dkr.ecr.us-west-2.amazonaws.com/ziox@sha256:3dbbb0dc1eab2d5a3b3e4a75fd87d194e8095c92d7b2b62e7cdbd07020f54589",
                    kubernetes_container_image:
                      "058694856476.dkr.ecr.us-west-2.amazonaws.com/ziox:v0.0.3",
                    kubernetes_container_name: "ziox",
                    kubernetes_docker_id:
                      "65917867b45dbe2cd429361796b5e7d42581124411bec968acfbd80ad0f163e2",
                    kubernetes_host:
                      "ip-10-2-15-197.us-east-2.compute.internal",
                    kubernetes_labels_app: "ziox",
                    kubernetes_labels_name: "ziox-querier",
                    kubernetes_labels_pod_template_hash: "595748494c",
                    kubernetes_labels_role: "querier",
                    kubernetes_namespace_name: "ziox",
                    kubernetes_pod_id: "ff96944f-ca8e-413f-a696-3236b1145482",
                    kubernetes_pod_name: "ziox-querier-595748494c-vd9zw",
                    log: '[2022-12-27T14:12:29Z INFO  zinc_enl::service::search::datafusion] search file: Bhargav_organization_29/logs/default/2022/12/27/05/7013381009471324160.parquet, need add columns: ["from"]',
                    stream: "stderr",
                  },
                ],
                total: 1,
                from: 0,
                size: 1,
                scan_size: 110,
              })
            );
          }
        )
      );
      await wrapper
        .findComponent(DetailTable)
        .find('[data-test="logs-detail-table-search-around-btn"]')
        .trigger("click");
      expect(
        wrapper.findComponent(SearchResult).emitted()["search:timeboxed"]
      ).toBeTruthy();
    });
  });
});
