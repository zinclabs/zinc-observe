import { test, expect } from "./baseFixtures";
import logData from "../../ui-testing/cypress/fixtures/log.json";
import { log } from "console";
import logsdata from "../../test-data/logs_data.json";

test.describe.configure({ mode: "parallel" });
const dashboardName = `dashboard${Date.now()}`;

async function createDashboard(page) {
  await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
  await page.waitForTimeout(5000);
  await page.locator('[data-test="dashboard-add"]').click();
  await page.waitForTimeout(5000);
  await page.locator('[data-test="add-dashboard-name"]').click();

  await page.locator('[data-test="add-dashboard-name"]').fill(dashboardName);
  await page.locator('[data-test="dashboard-add-submit"]').click();
  await page.waitForTimeout(2000);
  await page
    .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
    .click();
  await page.waitForTimeout(3000);
  await page.locator('[data-test="index-dropdown-stream"]').click();
  await page.locator('[data-test="index-dropdown-stream"]').fill("e2e");
  await page
    .getByRole("option", { name: "e2e_automate" })
    .locator("div")
    .nth(2)
    .click();
  await page.waitForTimeout(3000);
  await page
    .locator(
      '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubectl_kubernetes_io_default_container"] [data-test="dashboard-add-y-data"]'
    )
    .click();
  await page.locator('[data-test="dashboard-apply"]').click();
  await page.locator('[data-test="date-time-btn"]').click();
  await page.locator('[data-test="date-time-relative-5-d-btn"]').click();
  await page.locator('[data-test="dashboard-apply"]').click();
  await page.locator('[data-test="chart-renderer"] canvas').click({
    position: {
      x: 753,
      y: 200,
    },
  });
  await page.locator('[data-test="dashboard-panel-save"]').click();
  await page.locator('[data-test="dashboard-panel-name"]').click();
  await page.locator('[data-test="dashboard-panel-name"]').fill("sanitydash");
  await page.waitForTimeout(2000);
  await page.locator('[data-test="dashboard-panel-save"]').click();
  await page.waitForTimeout(2000);
  await page.locator('[data-test="dashboard-back-btn"]').click();
}
async function deleteDashboard(page) {
  await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
  await page.getByRole("cell", { name: dashboardName }).click();
  await page
    .locator('[data-test="dashboard-edit-panel-sanitydash-dropdown"]')
    .click();
  await page.locator('[data-test="dashboard-delete-panel"]').click();
  await page.locator('[data-test="confirm-button"]').click();
  await page.locator('[data-test="dashboard-back-btn"]').click();
  await page.locator('[data-test="dashboard-search"]').click();
  await page.locator('[data-test="dashboard-search"]').fill(dashboardName);
  await page.locator('[data-test="dashboard-delete"]').click();
  await page.locator('[data-test="confirm-button"]').click();
}

async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"]);
  // await page.getByText('Login as internal user').click();
  await page.waitForTimeout(1000);
  await page
    .locator('[data-cy="login-user-id"]')
    .fill(process.env["ZO_ROOT_USER_EMAIL"]);
  //Enter Password
  await page.locator("label").filter({ hasText: "Password *" }).click();
  await page
    .locator('[data-cy="login-password"]')
    .fill(process.env["ZO_ROOT_USER_PASSWORD"]);
  await page.locator('[data-cy="login-sign-in"]').click();
  //     await page.waitForTimeout(4000);
  // await page.goto(process.env["ZO_BASE_URL"]);
}

const selectStreamAndStreamTypeForLogs = async (page, stream) => {
  await page.waitForTimeout(4000);
  await page
    .locator('[data-test="log-search-index-list-select-stream"]')
    .click({ force: true });
  await page
    .locator("div.q-item")
    .getByText(`${stream}`)
    .first()
    .click({ force: true });
};
test.describe("Report testcases", () => {
  // let logData;
  function removeUTFCharacters(text) {
    // console.log(text, "tex");
    // Remove UTF characters using regular expression
    return text.replace(/[^\x00-\x7F]/g, " ");
  }
  async function applyQueryButton(page) {
    // click on the run query button
    // Type the value of a variable into an input field
    const search = page.waitForResponse(logData.applyQuery);
    await page.waitForTimeout(3000);
    await page.locator("[data-test='logs-search-bar-refresh-btn']").click({
      force: true,
    });
    // get the data from the search variable
    await expect.poll(async () => (await search).status()).toBe(200);
    // await search.hits.FIXME_should("be.an", "array");
  }
  // tebefore(async function () {
  //   // logData("log");
  //   // const data = page;
  //   // logData = data;

  //   console.log("--logData--", logData);
  // });
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.waitForTimeout(5000);

    // ("ingests logs via API", () => {
    const orgId = process.env["ORGNAME"];
    const streamName = "e2e_automate";
    const basicAuthCredentials = Buffer.from(
      `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
    ).toString("base64");

    const headers = {
      Authorization: `Basic ${basicAuthCredentials}`,
      "Content-Type": "application/json",
    };

    // const logsdata = {}; // Fill this with your actual data

    // Making a POST request using fetch API
    const response = await page.evaluate(
      async ({ url, headers, orgId, streamName, logsdata }) => {
        const fetchResponse = await fetch(
          `${url}/api/${orgId}/${streamName}/_json`,
          {
            method: "POST",
            headers: headers,
            body: JSON.stringify(logsdata),
          }
        );
        return await fetchResponse.json();
      },
      {
        url: process.env.INGESTION_URL,
        headers: headers,
        orgId: orgId,
        streamName: streamName,
        logsdata: logsdata,
      }
    );

    console.log(response);
    //  });
    // const allorgs = page.waitForResponse("**/api/default/organizations**");
    // const functions = page.waitForResponse("**/api/default/functions**");
    await page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );
    const allsearch = page.waitForResponse("**/api/default/_search**");
    await selectStreamAndStreamTypeForLogs(page, logData.Stream);
    await applyQueryButton(page);
  });

  test("should create once type report and delete it", async ({ page }) => {
    await createDashboard(page);
    await page.locator('[data-test="menu-link-\\/reports-item"]').click();
    await page.locator('[data-test="report-list-add-report-btn"]').click();
    await page.getByLabel("Name *").click();
    await page.getByLabel("Name *").fill("rreport1");
    await page.getByLabel("Folder *").click();
    await page.getByLabel("Folder *").click();
    await page.getByLabel("Folder *").fill("default");
    await page.waitForTimeout(2000);
    await page
      .getByRole("option", { name: "default" })
      .locator("div")
      .nth(2)
      .click();
    await page.getByLabel("Dashboard *").click();
    await page.getByLabel("Dashboard *").fill("sanitydash");
    await page.getByRole("option", { name: dashboardName }).click();
    await page.getByLabel("Dashboard Tab *").click();
    await page.getByRole("option", { name: "Default" }).click();
    await page.locator('[data-test="add-report-step1-continue-btn"]').click();
    await page.locator('[data-test="add-report-schedule-sendNow-btn"]').click();
    await page.locator('[data-test="add-report-step2-continue-btn"]').click();
    await page.getByLabel("Title *").click();
    await page.getByLabel("Title *").fill("e2eautomate");
    await page.getByPlaceholder("Invite by email. Multiple").click();
    await page
      .getByPlaceholder("Invite by email. Multiple")
      .fill("neha@test.com");
    await page.getByPlaceholder("Invite by email. Multiple").click({
      clickCount: 3,
    });
    await page
      .getByPlaceholder("Invite by email. Multiple")
      .fill("e2e@test.com");
    await page
      .locator('[data-test="add-report-share-message-input"]')
      .getByLabel("")
      .click();
    await page
      .locator('[data-test="add-report-share-message-input"]')
      .getByLabel("")
      .fill("testingnew");
    await page.locator('[data-test="add-report-save-btn"]').click();
    await page.locator('[data-test="report-list-search-input"]').click();
    await page
      .locator('[data-test="report-list-search-input"]')
      .fill("report1");
    await page
      .locator('[data-test="report-list-rreport1-delete-report"]')
      .click();
    await page.locator('[data-test="confirm-button"]').click();
    await page.waitForTimeout(2000);
    await deleteDashboard(page);
  });

  test("should create hourly report and delete it ", async ({ page }) => {
    await createDashboard(page);
    await page.locator('[data-test="menu-link-\\/reports-item"]').click();
    await page.locator('[data-test="report-list-add-report-btn"]').click();
    await page.getByLabel("Name *").click();
    await page.getByLabel("Name *").fill("e2ereport");
    await page
      .locator('[data-test="add-report-folder-select"]')
      .getByText("arrow_drop_down")
      .click();
    await page
      .getByRole("option", { name: "default" })
      .locator("div")
      .nth(2)
      .click();
    await page
      .locator(
        ".q-my-sm > div:nth-child(2) > .q-field > .q-field__inner > .q-field__control > .q-field__control-container > .q-field__native"
      )
      .click();
    await page.getByRole("option", { name: dashboardName }).click();
    await page.getByLabel("Dashboard Tab *").click();
    await page
      .getByRole("option", { name: "Default" })
      .locator("div")
      .nth(2)
      .click();
    await page.locator('[data-test="add-report-step1-continue-btn"]').click();
    await page
      .locator('[data-test="add-report-schedule-frequency-hours-btn"]')
      .click();
    await page.getByText("event").click();
    await page.getByRole("button", { name: "22" }).click();
    await page
      .locator("div")
      .filter({ hasText: /^access_time$/ })
      .click();
    await page
      .locator("#q-portal--menu--10 div")
      .filter({ hasText: /^4$/ })
      .click();
    await page.getByText("20").click();
    await page.getByRole("button", { name: "Close" }).click();
    await page
      .locator("label")
      .filter({ hasText: "Timezone *arrow_drop_down" })
      .click();
    await page
      .getByRole("option", { name: "UTC" })
      .locator("div")
      .nth(2)
      .click();
    await page.locator('[data-test="add-report-step2-continue-btn"]').click();
    await page.getByLabel("Title *").click();
    await page.getByLabel("Title *").fill("test hourly");
    await page.getByPlaceholder("Invite by email. Multiple").click();
    await page
      .getByPlaceholder("Invite by email. Multiple")
      .fill("automate@test.com");
    await page
      .locator('[data-test="add-report-share-message-input"]')
      .getByLabel("")
      .click();
    await page
      .locator('[data-test="add-report-share-message-input"]')
      .getByLabel("")
      .fill("testmessage");
    await page.locator('[data-test="add-report-save-btn"]').click();
    await page.locator('[data-test="report-list-search-input"]').click();
    await page
      .locator('[data-test="report-list-search-input"]')
      .fill("e2ereport");
    await page
      .locator('[data-test="report-list-e2ereport-delete-report"]')
      .click();
    await page.locator('[data-test="confirm-button"]').click();
    await page.waitForTimeout(2000);
    await deleteDashboard(page);
  });

  test("should create hourly- send now report and delete it ", async ({
    page,
  }) => {
    await createDashboard(page);
    await page.locator('[data-test="menu-link-\\/reports-item"]').click();
    await page.locator('[data-test="report-list-add-report-btn"]').click();
    await page.getByLabel("Name *").click();
    await page.getByLabel("Name *").fill("e2ereport");
    await page.getByLabel("Folder *").click();
    await page.getByRole("option", { name: "default" }).click();
    await page.getByLabel("Dashboard *").click();
    await page.getByRole("option", { name: dashboardName }).click();
    await page.locator('[data-test="add-report-dashboard-0"]').click();
    await page.getByLabel("Dashboard Tab *").click();
    await page.getByRole("option", { name: "Default" }).click();
    await page.locator('[data-test="add-report-step1-continue-btn"]').click();
    await page.locator('[data-test="add-report-schedule-sendNow-btn"]').click();
    await page.locator('[data-test="add-report-step2-continue-btn"]').click();
    await page.getByLabel("Title *").click();
    await page.getByLabel("Title *").fill("testautomate");
    await page.getByPlaceholder("Invite by email. Multiple").click();
    await page
      .getByPlaceholder("Invite by email. Multiple")
      .fill("report@gmail.com");
    await page
      .locator('[data-test="add-report-share-message-input"]')
      .getByLabel("")
      .click();
    await page
      .locator('[data-test="add-report-share-message-input"]')
      .getByLabel("")
      .fill("testreport");
    await page.locator('[data-test="add-report-save-btn"]').click();
    await page.locator('[data-test="report-list-search-input"]').click();
    await page
      .locator('[data-test="report-list-search-input"]')
      .fill("e2ereport");
    await page
      .locator('[data-test="report-list-e2ereport-delete-report"]')
      .click();
    await page.locator('[data-test="confirm-button"]').click();
    await page.waitForTimeout(2000);
    await deleteDashboard(page);
  });

  test("should create daily- send now report and delete it ", async ({
    page,
  }) => {
    await createDashboard(page);
    await page.locator('[data-test="menu-link-\\/reports-item"]').click();
    await page.locator('[data-test="report-list-add-report-btn"]').click();
    await page.getByLabel("Name *").click();
    await page.getByLabel("Name *").fill("e2eautomate");
    await page
      .locator(
        ".q-my-sm > div > .q-field > .q-field__inner > .q-field__control > .q-field__control-container > .q-field__native"
      )
      .first()
      .click();
    await page.getByRole("option", { name: "default" }).click();
    await page.getByLabel("Dashboard *").click();
    await page
      .getByRole("option", { name: dashboardName })
      .locator("div")
      .nth(2)
      .click();
    await page.getByLabel("Dashboard Tab *").click();
    await page.getByRole("option", { name: "Default" }).click();
    await page.getByLabel("Description").click();
    await page.locator('[data-test="add-report-step1-continue-btn"]').click();
    await page
      .locator('[data-test="add-report-schedule-frequency-days-btn"]')
      .click();
    await page.locator('[data-test="add-report-schedule-sendNow-btn"]').click();
    await page.locator('[data-test="add-report-step2-continue-btn"]').click();
    await page.getByLabel("Title *").click();
    await page.getByLabel("Title *").fill("daily-sendnow");
    await page.getByPlaceholder("Invite by email. Multiple").click();
    await page
      .getByPlaceholder("Invite by email. Multiple")
      .fill("test@gmail.com");
    await page
      .locator('[data-test="add-report-share-message-input"]')
      .getByLabel("")
      .click();
    await page
      .locator('[data-test="add-report-share-message-input"]')
      .getByLabel("")
      .fill("testreport");
    await page.locator('[data-test="add-report-save-btn"]').click();
    await page.locator('[data-test="report-list-search-input"]').click();
    await page
      .locator('[data-test="report-list-search-input"]')
      .fill("e2eautomate");
    await page
      .locator('[data-test="report-list-e2eautomate-delete-report"]')
      .click();
    await page.locator('[data-test="confirm-button"]').click();
    await page.waitForTimeout(2000);
    await deleteDashboard(page);
  });

  test("should create weekly- send now report and delete it ", async ({
    page,
  }) => {
    await createDashboard(page);
    await page.locator('[data-test="menu-link-\\/reports-item"]').click();
    await page.locator('[data-test="report-list-add-report-btn"]').click();
    await page.getByLabel('Name *').click();
    await page.getByLabel('Name *').fill('weekly');
    await page.locator('.q-my-sm > div > .q-field > .q-field__inner > .q-field__control > .q-field__control-container > .q-field__native').first().click();
    await page.getByRole('option', { name: 'default' }).click();
    await page.getByText('Dashboard *arrow_drop_down').click();
    await page.getByText(dashboardName).click();
    await page.locator('div:nth-child(3) > .q-field > .q-field__inner > .q-field__control > .q-field__control-container > .q-field__native').click();
    await page.getByRole('option', { name: 'Default' }).click();
    await page.locator('[data-test="add-report-step1-continue-btn"]').click();
    await page.locator('[data-test="add-report-schedule-sendNow-btn"]').click();
    await page.locator('[data-test="add-report-step2-continue-btn"]').click();
    await page.getByLabel('Title *').click();
    await page.getByLabel('Title *').fill('weekly');
    await page.getByPlaceholder('Invite by email. Multiple').click();
    await page.getByPlaceholder('Invite by email. Multiple').fill('test@gmail.com');
    await page.locator('[data-test="add-report-save-btn"]').click();
    await page.locator('[data-test="report-list-search-input"]').click();
    await page.locator('[data-test="report-list-search-input"]').fill('weekly');
    await page.locator('[data-test="report-list-weekly-delete-report"]').click();
    await page.locator('[data-test="confirm-button"]').click();
    await page.getByText('Delete report successfully.').click();
    await page.waitForTimeout(2000);
    await deleteDashboard(page);
  });


  test("should create monthly- send now report and delete it ", async ({
    page,
  }) => {
    await createDashboard(page);
    await page.locator('[data-test="menu-link-\\/reports-item"]').click();
    await page.locator('[data-test="report-list-add-report-btn"]').click();
    await page.getByLabel('Name *').click();
    await page.getByLabel('Name *').click();
    await page.getByLabel('Name *').fill('monthly');
    await page.getByLabel('Folder *').click();
    await page.getByRole('option', { name: 'default' }).click();
    await page.locator('.q-my-sm > div:nth-child(2) > .q-field > .q-field__inner > .q-field__control > .q-field__control-container > .q-field__native').click();
    await page.getByRole('option', { name:dashboardName }).click();
    await page.locator('[data-test="add-report-tab-select"] label').click();
    await page.getByRole('option', { name: 'Default' }).click();
    await page.locator('[data-test="add-report-step1-continue-btn"]').click();
    await page.locator('[data-test="add-report-schedule-frequency-months-btn"]').click();
    await page.locator('[data-test="add-report-schedule-sendNow-btn"]').click();
    await page.locator('[data-test="add-report-step2-continue-btn"]').click();
    await page.getByLabel('Title *').click();
    await page.getByLabel('Title *').fill('monthly');
    await page.getByPlaceholder('Invite by email. Multiple').click();
    await page.getByPlaceholder('Invite by email. Multiple').fill('test@gmail.comt');
    await page.locator('[data-test="add-report-share-message-input"]').getByLabel('').fill('e');
    await page.locator('[data-test="add-report-share-message-input"]').getByLabel('').click();
    await page.locator('[data-test="add-report-share-message-input"]').getByLabel('').fill('est');
    await page.locator('[data-test="add-report-share-message-input"]').getByLabel('').click();
    await page.locator('[data-test="add-report-share-message-input"]').getByLabel('').fill('test');
    await page.locator('[data-test="add-report-save-btn"]').click();
    await page.locator('[data-test="report-list-search-input"]').click();
    await page.locator('[data-test="report-list-search-input"]').fill('monthly');
    await page.locator('[data-test="report-list-monthly-delete-report"]').click();
    await page.locator('[data-test="confirm-button"]').click();
    await page.getByText('Delete report successfully.').click();
    await page.waitForTimeout(2000);
    await deleteDashboard(page);
  });
});
