import { test, expect } from "../baseFixtures";
import logData from "../../cypress/fixtures/log.json";
import logsdata from "../../../test-data/logs_data.json";

const randomDashboardName =
  "Dashboard_" + Math.random().toString(36).slice(2, 11);

test.describe.configure({ mode: "parallel" });

async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"], { waitUntil: "networkidle" });
  // await page.getByText('Login as internal user').click();
  await page.waitForTimeout(1000);
  await page
    .locator('[data-cy="login-user-id"]')
    .fill(process.env["ZO_ROOT_USER_EMAIL"]);

  // wait for login api response
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
  await page
    .locator('[data-test="navbar-organizations-select"]')
    .getByText("arrow_drop_down")
    .click();
  await page.getByRole("option", { name: "default", exact: true }).click();
}

async function ingestion(page) {
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
  console.log(response);
}

async function waitForDashboardPage(page) {
  const dashboardListApi = page.waitForResponse(
    (response) =>
      /\/api\/.+\/dashboards/.test(response.url()) && response.status() === 200
  );

  await page.waitForURL(process.env["ZO_BASE_URL"] + "/web/dashboards**");

  await page.waitForSelector(`text="Please wait while loading dashboards..."`, {
    state: "hidden",
  });
  await dashboardListApi;
  await page.waitForTimeout(500);
}

test.describe("dashboard UI testcases", () => {
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
    console.log("running before each");
    await login(page);
    await page.waitForTimeout(1000);
    await ingestion(page);
    await page.waitForTimeout(2000);

    // just to make sure org is set
    const orgNavigation = page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );

    await orgNavigation;
  });
  
  test("should not be displayed an error after saving the panel if the trellis is applied to the chart", async ({ page,  browser, }) => {
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


    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.waitForTimeout(6000);
  
    // Add data to the chart
    await page.locator('[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]').click();
    await page.locator('[data-test="field-list-item-logs-e2e_automate-kubernetes_namespace_name"] [data-test="dashboard-add-b-data"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(5000);

    // Apply Trellis on chart
    await page.locator('[data-test="dashboard-sidebar"]').click();
    await page.locator('[data-test="dashboard-trellis-chart"]').click();
    await page.getByRole('option', { name: 'Auto' }).click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.waitForTimeout(6000);

    // Save the panel
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill('testtttt');
    await page.locator('[data-test="dashboard-panel-save"]').click();
  
    // Edit panel and reapply Trellis
    await page.locator('[data-test="dashboard-edit-panel-testtttt-dropdown"]').click();
    await page.locator('[data-test="dashboard-edit-panel"]').click();
    await page.locator('[data-test="dashboard-sidebar"]').click();
    await page.locator('[data-test="dashboard-trellis-chart"]').click();
    await page.getByRole('option', { name: 'Vertical' }).click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.waitForTimeout(6000);

    let errorDetected = false;

    // Listen for console messages and check for errors
    page.on('console', (msg) => {
      if (msg.type() === 'error' || msg.text().includes('Error')) {
        console.error(`Console Error Detected: ${msg.text()}`);
        errorDetected = true; // Set the flag if an error is detected
      }
    });
    
    // Perform actions that might trigger an error
    await page.locator('[data-test="dashboard-apply"]').click();
    
    // Verify if any errors were detected in the console
    if (errorDetected) {
      throw new Error('Dynamic error detected in console after applying Trellis on chart');
    } else {
      console.log('No errors detected in console after applying Trellis');
    }
  
    // Save the panel again
    await page.locator('[data-test="dashboard-panel-save"]').click();
  });
    

});