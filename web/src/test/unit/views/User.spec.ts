import { describe, expect, it, beforeEach, vi, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { installQuasar } from "../helpers/install-quasar-plugin";
import { Dialog, Notify } from "quasar";
import User from "@/views/User.vue";
import i18n from "@/locales";
import store from "../helpers/store";
import router from "../helpers/router";

installQuasar({
  plugins: [Dialog, Notify],
});

describe("Users", async () => {
  let wrapper: any;
  beforeEach(async () => {
    wrapper = mount(User, {
      props: {
        currOrgIdentifier: "zinc_next",
        currUserEmail: "example@gmail.com",
        orgAPIKey: 'L"4R{8f~56e72`0319V',
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

  it("should display title", () => {
    const title = wrapper.find('[data-test="user-title-text"]');
    expect(title.text()).toBe("Organization Member");
  });

  // it("Should display streams table", () => {
  //   const table = wrapper.find('[data-test="user-table"]');
  //   expect(table.exists()).toBeTruthy();
  // });

  // it("Should display table column headers", async () => {
  //   const tableData = wrapper
  //     .find('[data-test="user-table"]')
  //     .find("thead")
  //     .find("tr")
  //     .findAll("th");
  //   expect(tableData[0].text()).toBe("#");
  //   expect(tableData[1].text()).toContain("Email");
  //   expect(tableData[2].text()).toContain("First Name");
  //   expect(tableData[3].text()).toContain("Last Name");
  //   expect(tableData[4].text()).toContain("Role");
  //   expect(tableData[5].text()).toContain("Actions");
  // });

  // it("Should display table row data", async () => {
  //   const tableData = wrapper
  //     .find('[data-test="user-table"]')
  //     .find("tbody")
  //     .find("tr")
  //     .findAll("td");
  //   expect(tableData[0].text()).toBe("01");
  //   expect(tableData[1].text()).toBe("john@gmail.com");
  //   expect(tableData[2].text()).toBe("John");
  //   expect(tableData[3].text()).toBe("Doe");
  //   expect(tableData[4].text()).toContain("admin");
  // });

  // it("Should display filter user input", async () => {
  //   expect(
  //     wrapper.find('[data-test="user-filter-input"]').exists()
  //   ).toBeTruthy();
  // });

  // describe("When current user is admin and is on cloud", () => {
  //   beforeEach(() => {
  //     import.meta.env.VITE_OPENOBSERVE_CLOUD = true;
  //   });
  //   it("Should display invite user email input", async () => {
  //     expect(
  //       wrapper.find('[data-test="user-add-user-email-input"]').exists()
  //     ).toBeTruthy();
  //   });

  //   it("Should display invite user role select input", async () => {
  //     expect(
  //       wrapper.find('[data-test="user-add-user-role-select-input"]').exists()
  //     ).toBeTruthy();
  //   });

  //   it("Should display invite user button", async () => {
  //     expect(
  //       wrapper.find('[data-test="user-add-user-invite-button"]').exists()
  //     ).toBeTruthy();
  //   });

  //   it("Should delete user when admin deletes user from organization", async () => {
  //     await wrapper
  //       .find('[data-test="user-03-delete-button"]')
  //       .trigger("click");
  //   });

  //   describe("When user invites member to organization", () => {
  //     const addUser = vi.spyOn(OrganizationService, "add_members");

  //     beforeEach(async () => {
  //       await wrapper
  //         .find('[data-test="user-add-user-email-input"]')
  //         .setValue("omk@gmail.com");
  //     });

  //     it("Should select user as Member", async () => {
  //       global.server.use(
  //         rest.post(
  //           `${store.state.API_ENDPOINT}/api/${store.state.selectedOrganization.identifier}/organizations/members`,
  //           (req, res, ctx) => {
  //             return res(
  //               ctx.status(200),
  //               ctx.json({
  //                 data: {
  //                   valid_members: null,
  //                   existing_members: ["omiekesarkhane@gmail.com"],
  //                 },
  //                 message: "Member invitation created successfully",
  //               })
  //             );
  //           }
  //         )
  //       );
  //       await wrapper
  //         .find('[data-test="user-add-user-invite-button"]')
  //         .trigger("click");

  //       expect(addUser).toHaveBeenCalledTimes(1);
  //     });
  //   });
  // });
});
