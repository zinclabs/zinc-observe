import { test, expect } from "@playwright/test";
import logData from "../cypress/fixtures/log.json";
import logsdata from "../../test-data/logs_data.json";

const randomDashboardName =
  "Dashboard_" + Math.random().toString(36).substr(2, 9);

test.describe.configure({ mode: "parallel" });

// Reusable function to log in
async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"], { waitUntil: "networkidle" });
  await page
    .locator('[data-cy="login-user-id"]')
    .fill(process.env["ZO_ROOT_USER_EMAIL"]);

  const waitForLogin = page.waitForResponse(
    (response) =>
      response.url().includes("/auth/login") && response.status() === 200
  );

  await page
    .locator('[data-cy="login-password"]')
    .fill(process.env["ZO_ROOT_USER_PASSWORD"]);
  await page.locator('[data-cy="login-sign-in"]').click();
  await waitForLogin;
  await page.waitForURL(process.env["ZO_BASE_URL"] + "/web/", {
    waitUntil: "networkidle",
  });
}

// Reusable function to create a dashboard
async function createDashboard(page, dashboardName) {
  await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
  await waitForDashboardPage(page);
  await page.locator('[data-test="dashboard-add"]').click();
  await page.locator('[data-test="add-dashboard-name"]').fill(dashboardName);
  await page.locator('[data-test="dashboard-add-submit"]').click();
}

// Reusable function to delete a dashboard by name
async function deleteDashboard(page, dashboardName) {
  console.log(`Deleting dashboard with name: ${dashboardName}`);
  const dashboardNameLocator = page.locator(
    `//tr[.//td[text()="${dashboardName}"]]`
  );
  const dashboardDeleteButton = dashboardNameLocator.locator(
    '[data-test="dashboard-delete"]'
  );
  await expect(dashboardNameLocator).toBeVisible();
  await dashboardDeleteButton.click();
  await page.locator('[data-test="confirm-button"]').click();
}

// Wait for the dashboard page to load completely
async function waitForDashboardPage(page) {
  const dashboardListApi = page.waitForResponse(
    (response) =>
      /\/api\/.+\/dashboards/.test(response.url()) && response.status() === 200
  );
  await page.waitForURL(process.env["ZO_BASE_URL"] + "/web/dashboards**");
  await page.waitForSelector('text="Please wait while loading dashboards..."', {
    state: "hidden",
  });
  await dashboardListApi;
  await page.waitForTimeout(500); // Add a slight delay to ensure page is stable
}

test.describe("VRL UI test cases", () => {
  // Function to apply the query and validate
  async function applyQueryButton(page) {
    const search = page.waitForResponse(logData.applyQuery);
    await page.waitForTimeout(3000);
    await page
      .locator("[data-test='logs-search-bar-refresh-btn']")
      .click({ force: true });
    await expect.poll(async () => (await search).status()).toBe(200);
  }

  // Before each test, login and prepare the environment
  test.beforeEach(async ({ page }) => {
    await login(page);

    const orgNavigation = page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );

    const orgId = process.env["ORGNAME"];
    const streamName = "e2e_automate";
    const basicAuthCredentials = Buffer.from(
      `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
    ).toString("base64");

    const headers = {
      Authorization: `Basic ${basicAuthCredentials}`,
      "Content-Type": "application/json",
    };

    const fetchResponse = await fetch(
      `${process.env.INGESTION_URL}/api/${orgId}/${streamName}/_json`,
      {
        method: "POST",
        headers: headers,
        body: JSON.stringify(logsdata),
      }
    );
    const response = await fetchResponse.json();

    await orgNavigation;
    console.log(response);
  });
  test("should verify that the Transpose toggle button is working correctly", async ({
    page,
    browser,
  }) => {
    // Navigate to dashboards
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Create a new dashboard
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add panel to the dashboard
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("bhj");
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="selected-chart-table-item"] img').click();
    await page.getByRole("cell", { name: "Kubernetes Container Name" }).click();
    await page.locator('[data-test="dashboard-sidebar"]').click();
    await page
      .locator('[data-test="dashboard-config-table_transpose"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(1000);
  });
  // Helper function to validate data before and after the transpose action
  // async function validateDynamicDataBeforeAndAfterAction(page) {
  //   // Step 1: Capture dynamic data before the action
  //   const initialDate = await page.getByRole('cell', { name: /.*-.*-.* .*/ }).textContent();
  //   const initialValue = await page.getByRole('cell', { name: /\d+\.\d+/ }).first().textContent();

  //   // Perform transpose and apply action
  //   await page.locator('[data-test="dashboard-config-table_transpose"] div').nth(2).click();
  //   await page.locator('[data-test="dashboard-apply"]').click();

  //   // Step 2: Capture dynamic data after the action
  //   const finalDate = await page.getByRole('cell', { name: /.*-.*-.* .*/ }).textContent();
  //   const finalValue = await page.getByRole('cell', { name: /\d+\.\d+/ }).first().textContent();

  //   // Step 3: Assert that the data is the same before and after the action
  //   expect(finalDate.trim()).toBe(initialDate.trim());
  //   expect(finalValue.trim()).toBe(initialValue.trim());
  //  )};

  // The main test case

  test("1should verify that the Transpose toggle button is working correctly", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Create a new dashboard
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add panel to the dashboard
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByRole("option", { name: "e2e_automate" }).click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("bhj");
    await page.locator('[data-test="date-time-btn"]').click();

    // await page.locator('[data-test="date-time-absolute-tab"]').click();
    // await page.getByRole('button', { name: '1', exact: true }).click();
    // await page.getByRole('button', { name: '3', exact: true }).click();
    // await page.locator('[data-test="date-time-apply-btn"]').click();


    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="selected-chart-table-item"] img').click();
    await page.getByRole("cell", { name: "Kubernetes Container Name" }).click();
    await page.locator('[data-test="dashboard-sidebar"]').click();
    await page
      .locator('[data-test="dashboard-config-table_transpose"] div')
      .nth(2)
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();

    // Validate dynamic data before and after transpose
    await validateDynamicDataBeforeAndAfterAction(page);

    // Helper function to validate data before and after the transpose action
    async function validateDynamicDataBeforeAndAfterAction(page) {
      // Step 1: Capture the first dynamic date and value before the action
      // const initialDateElement = await page
      //   .getByRole("cell", { name: /.*-.*-.* .*/ })
      //   .first();
      const initialValueElement = await page
        .getByRole("cell", { name: /\d+\.\d+/ })
        .first();

      // const initialDate = await initialDateElement.textContent();
      const initialValue = await initialValueElement.textContent();

      // Perform transpose and apply action
      await page
        .locator('[data-test="dashboard-config-table_transpose"] div')
        .nth(2)
        .click();
      await page.locator('[data-test="dashboard-apply"]').click();

      // Step 2: Capture the first dynamic date and value after the action
      // const finalDateElement = await page
      //   .getByRole("cell", { name: /.*-.*-.* .*/ })
      //   .first();
      const finalValueElement = await page
        .getByRole("cell", { name: /\d+\.\d+/ })
        .first();

      // const finalDate = await finalDateElement.textContent();
      const finalValue = await finalValueElement.textContent();

      // Step 3: Assert that the data is the same before and after the action
      // expect(finalDate.trim()).toBe(initialDate.trim());
      expect(finalValue.trim()).toBe(initialValue.trim());
    }
  });

  test('verify if desible the Tanspose button chart should be Default format ', async ({ page }) => {
   
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Create a new dashboard
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add panel to the dashboard
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
await page.getByRole("option", { name: "e2e_automate" }).click();

    await page.locator('[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]').click();
    await page.locator('[data-test="field-list-item-logs-e2e_automate-kubernetes_docker_id"] [data-test="dashboard-add-b-data"]').click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="selected-chart-table-item"] img').click();
    await page.locator('[data-test="dashboard-sidebar"]').click();
    await page.locator('[data-test="dashboard-config-table_transpose"] div').first().click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-config-table_transpose"] div').nth(2).click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await expect(page.getByRole('cell', { name: 'Kubernetes Container Name' })).toBeVisible();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill('jdjf');
    await page.locator('[data-test="dashboard-panel-save"]').click();
  });



});