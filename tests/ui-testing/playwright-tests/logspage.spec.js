import { test, expect } from "@playwright/test";
import logData from "../../ui-testing/cypress/fixtures/log.json";
import { log } from "console";
import logsdata from "../../test-data/logs_data.json";

test.describe.configure({ mode: "parallel" });

async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"]);
//  await page.getByText('Login as internal user').click();
  console.log("ZO_BASE_URL", process.env["ZO_BASE_URL"]);
  await page.waitForTimeout(1000);
  await page
    .locator('[data-cy="login-user-id"]')
    .fill(process.env["ZO_ROOT_USER_EMAIL"]);
  //Enter Password
  await page
    .locator('[data-cy="login-password"]')
    .fill(process.env["ZO_ROOT_USER_PASSWORD"]);
  await page.locator('[data-cy="login-sign-in"]').click();
  await page.waitForTimeout(4000);
  await page.goto(process.env["ZO_BASE_URL"]);
}

async function ingestion(page) {
  const orgId = process.env["ORGNAME"];
  const streamName = "e2e_automate";
  const basicAuthCredentials = Buffer.from(
    `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
  ).toString('base64');

  const headers = {
    "Authorization": `Basic ${basicAuthCredentials}`,
    "Content-Type": "application/json",
  };
  const response = await page.evaluate(async ({ url, headers, orgId, streamName, logsdata }) => {
    const fetchResponse = await fetch(`${url}/api/${orgId}/${streamName}/_json`, {
      method: 'POST',
      headers: headers,
      body: JSON.stringify(logsdata)
    });
    return await fetchResponse.json();
  }, {
    url: process.env.INGESTION_URL,
    headers: headers,
    orgId: orgId,
    streamName: streamName,
    logsdata: logsdata
  });
  console.log(response);
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
test.describe("Logs UI testcases", () => {
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
    await page.waitForTimeout(1000)
    await ingestion(page);
    await page.waitForTimeout(2000)

    await page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );
    const allsearch = page.waitForResponse("**/api/default/_search**");
    await selectStreamAndStreamTypeForLogs(page, logData.Stream);
    await applyQueryButton(page);
    // const streams = page.waitForResponse("**/api/default/streams**");
  });

  test("should click run query after SQL toggle on but without any query", async ({
    page,
  }) => {
    await page.waitForTimeout(3000);
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.getByLabel("SQL Mode").locator("div").nth(2).click();
    await page.locator('[data-test="logs-search-bar-query-editor"]').click();
    await page.keyboard.press(
      process.platform === "darwin" ? "Meta+A" : "Control+A"
    ); // Select all text
    await page.keyboard.press("Backspace");
    await page.waitForTimeout(3000);
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.getByText("No column found in selected stream.").click();
  });

  test("should be able to enter valid text in VRL and run query", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click({ force: true });
    await page
      .locator('[data-test="date-time-relative-6-w-btn"] > .q-btn__content')
      .click({
        force: true,
      });
    await applyQueryButton(page);


    await page.locator('#fnEditor').getByLabel('Editor content;Press Alt+F1').fill('.a=2');
    await page.waitForTimeout(1000);
    await applyQueryButton(page);
    const warningElement = page.locator('text=warning Query execution');
    await expect(warningElement).toBeHidden()
  
    await page
      .locator('[data-test="table-row-expand-menu"]')
      .first()
      .click({ force: true });
    await expect(page.locator("text=.a=2")).toBeVisible();
    await expect(
      page.locator('[data-test="logs-search-result-logs-table"]')
    ).toBeVisible();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
  });

  test("should hide and display again after clicking the arrow", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click({ force: true });
    await page
      .locator('[data-test="date-time-relative-6-w-btn"] > .q-btn__content')
      .click({
        force: true,
      });
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"]')
      .click({ force: true });
    await page.locator('[data-test="logs-search-field-list-collapse-btn"]').click();
    await page.waitForTimeout(1000)
    await page.locator('[data-test="logs-search-field-list-collapse-btn"]').click();
    // await page
    //   .locator(".bg-primary > .q-btn__content > .q-icon")
    //   .click({ force: true });
    // await page.waitForTimeout(2000);
    // await page
    //   .locator(".bg-primary > .q-btn__content > .q-icon")
    //   .click({ force: true });
    await expect(
      page.locator('[data-cy="index-field-search-input"]')
    ).toBeVisible();
  });

  test("should verify if special characters allowed in saved views name", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click({ force: true });
    await page
      .locator('[data-test="date-time-relative-6-w-btn"] > .q-btn__content')
      .click({
        force: true,
      });
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"]')
      .click({ force: true });
    await page
      .locator(
        '[data-test="logs-search-saved-views-btn"] > .q-btn-dropdown--current > .q-btn__content > :nth-child(1)'
      )
      .click({
        force: true,
      });
    await page.locator('[data-test="add-alert-name-input"]').type("e2e@@@@@");
    await page.waitForTimeout(2000);
    await page
      .locator('[data-test="saved-view-dialog-save-btn"]')
      .click({
        force: true,
      });
    await expect(page.locator(".q-notification__message")).toContainText(
      "Please provide valid view name"
    );
  });

  test("should allow alphanumeric name under saved view", async ({ page }) => {
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page
      .locator('[data-test="logs-search-saved-views-btn"]')
      .getByLabel("Expand")
      .click();
    await page
      .locator("button")
      .filter({ hasText: "savesaved_search" })
      .click();
    await page.locator('[data-test="add-alert-name-input"]').click();
    await page.locator('[data-test="add-alert-name-input"]').fill("e2enewtest");
    await page
      .locator('[data-test="saved-view-dialog-save-btn"]')
      .click({ force: true });
    await page.waitForTimeout(5000);
    await page
      .locator('[data-test="logs-search-saved-views-btn"]')
      .getByLabel("Expand")
      .click();
    await page
      .locator('[data-test="log-search-saved-view-field-search-input"]')
      .click({ force: true });
    await page
      .locator('[data-test="log-search-saved-view-field-search-input"]')
      .fill("e2enewtest");
    await page.waitForTimeout(3000);
    await page.getByText("e2enewtest").click();
    await page
      .locator('[data-test="logs-search-saved-views-btn"]')
      .getByLabel("Expand")
      .click();
    await page
      .locator('[data-test="log-search-saved-view-field-search-input"]')
      .click();
    await page
      .locator('[data-test="log-search-saved-view-field-search-input"]')
      .fill("e2enewtest");
    await page.getByText("delete").click();
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("should display error when user directly clicks on OK without adding name", async ({
    page,
  }) => {
    // Click on the date-time button
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page
      .locator('[data-test="logs-search-saved-views-btn"]')
      .getByLabel("Expand")
      .click();
    await page
      .locator("button")
      .filter({ hasText: "savesaved_search" })
      .click();
    await page.locator('[data-test="add-alert-name-input"]');
    const saveButton = await page.locator(
      '[data-test="saved-view-dialog-save-btn"]'
    );
    await saveButton.scrollIntoViewIfNeeded();
    await saveButton.click({ force: true });
    // await expect(page.locator('.q-notification__message')).toContainText('Please provide valid view name');
  });

  test("should display the details of logs results on graph", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click({ force: true });
    await page
      .locator('[data-test="date-time-relative-6-w-btn"] > .q-btn__content')
      .click({ force: true });
    await applyQueryButton(page);
    await page.waitForTimeout(5000);
    await expect(
      page.locator(".search-list > :nth-child(1) > .text-left")
    ).toBeVisible();
    await page.waitForTimeout(2000);
    await page.locator('[data-test="log-table-column-0-source"]').click();
    await page.waitForTimeout(1000);
    await page.locator('[data-test="close-dialog"]').click({
      force: true,
    });
    await expect(
      page.locator(".search-list > :nth-child(1) > .text-left")
    ).not.toBeHidden();
  });

  test("should click on live mode on button and select 5 sec, switch off, and then click run query", async ({
    page,
  }) => {
    await page.route("**/logData.ValueQuery", (route) => route.continue());
    await page.locator('[data-test="date-time-btn"]').click({ force: true });

    await page
      .locator('[data-test="date-time-relative-6-w-btn"] > .q-btn__content')
      .click({
        force: true,
      });
    await page
      .locator(".q-pl-sm > .q-btn > .q-btn__content")
      .click({ force: true });
    await page.locator('[data-test="logs-search-bar-refresh-time-5"]').click({
      force: true,
    });
    await expect(page.locator(".q-notification__message")).toContainText(
      "Live mode is enabled"
    );
    await page
      .locator(".q-pl-sm > .q-btn > .q-btn__content")
      .click({ force: true });
    await page
      .locator(
        '[data-test="logs-search-off-refresh-interval"] > .q-btn__content'
      )
      .click({ force: true });
    await applyQueryButton(page);
  });

  test("should click on VRL toggle and display the field, then disable toggle and make the VRL field disappear", async ({
    page,
  }) => {
    await expect(page.locator("#fnEditor .view-lines")).toBeVisible();
    await page
      .locator(
        '[data-test="logs-search-bar-show-query-toggle-btn"] > .q-toggle__inner'
      )
      .click({ force: true });
    await expect(page.locator("#fnEditor .view-lines")).not.toBeVisible();
  });

  test("should switch from past 6 weeks to past 6 days on date-time UI", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click({ force: true });
    await page
      .locator('[data-test="date-time-relative-6-w-btn"] > .q-btn__content')
      .click({ force: true });
    await expect(page.locator('[data-test="date-time-btn"]')).toContainText(
      "Past 6 Weeks"
    );
    await applyQueryButton(page);
    await page.locator('[data-test="date-time-btn"]').click({ force: true });
    await page
      .locator('[data-test="date-time-relative-6-d-btn"]')
      .click({ force: true });
    await expect(page.locator('[data-test="date-time-btn"]')).toContainText(
      "Past 6 Days"
    );
    await applyQueryButton(page);
  });
  test("should display SQL query on switching between Menu options & navigating to Logs again", async ({
    page,
  }) => {
    // Intercept the GET request, replace 'logData.ValueQuery' with your actual endpoint
    await page.route("**/logData.ValueQuery", (route) => route.continue());

    // Click on the date-time button
    await page.locator('[data-test="date-time-btn"]').click({ force: true });

    // Click on the SQL Mode toggle
    await page.locator('[aria-label="SQL Mode"]').click({ force: true });

    // Assert that the SQL query is visible
    const expectedQuery =
      'SELECT * FROM "e2e_automate"';
    const text = await page.evaluate(() => {
      const editor = document
        .querySelector('[data-test="logs-search-bar-query-editor"]')
        .querySelector(".view-lines"); // Adjust selector if needed
      return editor ? editor.textContent.trim() : null;
    });

    console.log(text);
    await expect(text.replace(/\s/g, "")).toContain(
      expectedQuery.replace(/\s/g, "")
    );
    await page.locator('[data-test="menu-link-/-item"]').click({ force: true });
    await page
      .locator('[data-test="menu-link-/logs-item"]')
      .click({ force: true });
    await page.waitForTimeout(2000);
    await page
      .locator('[data-test="logs-search-bar-query-editor"]')
      .getByRole("code")
      .locator("div")
      .filter({ hasText: 'SELECT * FROM "e2e_automate"' })
      .nth(4);
  });
  test("should display ingested logs - search logs, navigate on another tab, revisit logs page", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click({ force: true });
    await page
      .locator(
        '[data-test="date-time-relative-15-m-btn"] > .q-btn__content > .block'
      )
      .click({ force: true });
    await applyQueryButton(page);
    await page
      .locator('[data-test="menu-link-/traces-item"]')
      .click({ force: true });
    await page.waitForTimeout(100);
    await page
      .locator('[data-test="menu-link-/logs-item"]')
      .click({ force: true });
    await page.waitForTimeout(5000);
    const barChart = await page.locator(
      '[data-test="logs-search-result-bar-chart"]'
    );
    await expect(barChart).toBeTruthy();
  });

  test("should redirect to logs after clicking on stream explorer via stream page", async ({
    page,
  }) => {
    await page.locator('[data-test="date-time-btn"]').click({ force: true });
    await page
      .locator('[data-test="menu-link-/streams-item"]')
      .click({ force: true });
    await page.waitForTimeout(1000);
    await page
      .locator('[data-test="menu-link-\\/streams-item"]')
      .click({ force: true });
    await page.getByPlaceholder("Search Stream").click();
    await page.getByPlaceholder("Search Stream").fill("e2e");
    await page
      .getByRole("button", { name: "Explore" })
      .first()
      .click({ force: true });
    await expect(page.url()).toContain("logs");
  });

  test('should display error when save function is clicked without any VRL function', async ({ page }) => {
    // await page.locator('[data-test="menu-link-\\/logs-item"]').click();
    await page.locator('[data-test="logs-search-bar-function-dropdown"] button').filter({ hasText: 'save' }).click();
    await page.locator('#q-notify div').filter({ hasText: 'warningNo function definition' }).nth(3).click();
  });

  test('should create a function and then delete it', async ({ page }) => {
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-search-bar-function-dropdown"] button').filter({ hasText: 'save' }).click();
    await page.locator('#fnEditor > .monaco-editor > .overflow-guard > .monaco-scrollable-element > .lines-content > .view-lines > .view-line').click();
    await page.locator('#fnEditor').getByLabel('Editor content;Press Alt+F1').fill('.a=2');
    await page.waitForTimeout(1000);
    await page.locator('[data-test="logs-search-bar-function-dropdown"] button').filter({ hasText: 'save' }).click();
    await page.locator('[data-test="saved-function-name-input"]').click();
    await page.locator('[data-test="saved-function-name-input"]').fill('e2efunction');
    await page.locator('[data-test="saved-view-dialog-save-btn"]').click();
    // await page.locator('[data-test="menu-link-\\/functions-item"]').click();
    await page.locator('[data-test="menu-link-\\/pipeline-item"]').click();
    await page.locator('[data-test="tab-realtime"]').click();

    await page.locator('[data-test="function-stream-tab"]').click();
    await page.getByPlaceholder('Search Function').click();
    await page.getByPlaceholder('Search Function').fill('e2e');
    await page.getByRole('button', { name: 'Delete Function' }).click();
    await page.locator('[data-test="confirm-button"]').click();
  });

  test('should display click save directly while creating a function', async ({ page }) => {
    await page.waitForTimeout(1000);
    await page.locator('#fnEditor').getByLabel('Editor content;Press Alt+F1').fill('.a=2');
    await page.waitForTimeout(1000);
    await page.locator('[data-test="logs-search-bar-function-dropdown"] button').filter({ hasText: 'save' }).click();
    await page.locator('[data-test="saved-view-dialog-save-btn"]').click();
    await page.getByText('Function name is not valid.').click();
  });

  test('should display error on adding only blank spaces under function name', async ({ page }) => {
    await page.waitForTimeout(1000);
    await page.locator('#fnEditor').getByLabel('Editor content;Press Alt+F1').fill('.a=2');
    await page.waitForTimeout(1000);
    await page.locator('[data-test="logs-search-bar-function-dropdown"] button').filter({ hasText: 'save' }).click();
    await page.locator('[data-test="saved-function-name-input"]').fill(' ');
    await page.locator('[data-test="saved-view-dialog-save-btn"]').click();
    await page.getByText('Function name is not valid.').click();
  });


  test('should display error on adding invalid characters under function name', async ({ page }) => {
    await page.waitForTimeout(1000);
    await page.locator('#fnEditor').getByLabel('Editor content;Press Alt+F1').fill('.a=2');
    await page.waitForTimeout(1000);
    await page.locator('[data-test="logs-search-bar-function-dropdown"] button').filter({ hasText: 'save' }).click();
    await page.locator('[data-test="saved-function-name-input"]').fill('e2e@@@');
    await page.locator('[data-test="saved-view-dialog-save-btn"]').click();
    await page.getByText('Function name is not valid.').click();
  });

  test('should display added function on switching between tabs and again navigate to log', async ({ page }) => {
    await page.waitForTimeout(1000);
    await page.locator('#fnEditor').getByLabel('Editor content;Press Alt+F1').fill('.a=2');
    await page.waitForTimeout(1000);
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="menu-link-\\/metrics-item"]').click();
    await page.locator('[data-test="menu-link-\\/logs-item"]').click();
    await page.locator('[data-test="menu-link-/logs-item"]').click({ force: true });
    // Assert that ".a=2" is visible
    const logsPage = await page.locator('.q-page-container');
    await expect(logsPage).toContainText(".a=2");

  });

  test('should display bar chart when histogram toggle is on', async ({ page }) => {
    await page.locator('[data-test="log-search-index-list-field-search-input"]').click();
    await page.locator('[data-test="log-search-index-list-field-search-input"]').fill('code');
    await page.waitForTimeout(4000);
    await page.getByLabel('Expand "code"').click();
    await page.waitForTimeout(4000);
    // await page.locator('[data-test="logs-search-subfield-add-code-200"] [data-test="log-search-subfield-list-equal-code-field-btn"]').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.getByLabel('SQL Mode').locator('div').nth(2).click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-search-result-bar-chart"] canvas').click({
    });
    await page.getByLabel('SQL Mode').locator('div').nth(2).click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-search-result-bar-chart"] canvas').click({
    });
    await page.locator('[data-test="logs-search-bar-show-histogram-toggle-btn"] div').nth(2).click();
  });

  test('should display search around in histogram mode', async ({ page }) => {
    await page.waitForTimeout(1000);
    await page.locator('[data-test="log-table-column-0-source"]').click();
    await page.locator('[data-test="logs-detail-table-search-around-btn"]').click();
    await page.waitForTimeout(2000);
    const element = await page.locator('[data-test="log-table-column-0-source"]');
    const isVisible = await element.isVisible();
    expect(isVisible).toBeTruthy();

  });


  test.skip('should display results for search around after adding function', async ({ page }) => {
    await page.waitForTimeout(1000);
    await page.locator('#fnEditor > .monaco-editor > .overflow-guard > .monaco-scrollable-element > .lines-content > .view-lines > .view-line').click();
    await page.locator('#fnEditor').getByLabel('Editor content;Press Alt+F1').fill('.a=1');
    await page.waitForTimeout(1000);
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="log-table-column-3-source"]').getByText('{"_timestamp":').click();
    await page.locator('[data-test="logs-detail-table-search-around-btn"]').click();
    await page.waitForTimeout(1000);
    const element = await page.locator('[data-test="log-table-column-0-source"]');
    const isVisible = await element.isVisible();
    expect(isVisible).toBeTruthy();

  });

  test('should display search around in SQL mode', async ({ page }) => {
    await page.waitForTimeout(1000);
    await page.getByLabel('SQL Mode').locator('div').nth(2).click();
    await page.locator('[data-test="log-table-column-0-source"]').click();
    await page.locator('[data-test="logs-detail-table-search-around-btn"]').click();
    await page.waitForTimeout(2000)
    const element = await page.locator('[data-test="log-table-column-0-source"]');
    const isVisible = await element.isVisible();
    expect(isVisible).toBeTruthy();

  });

  test("should display results for search around with limit query", async ({ page }) => {
    await page.waitForTimeout(2000);
    await page.locator('[data-test="date-time-btn"]').click({ force: true });
    await page.locator('[data-test="date-time-relative-15-m-btn"] > .q-btn__content > .block').click({ force: true });
    await page.click('[data-test="logs-search-bar-query-editor"]')
    await page.keyboard.type("match_all('code') limit 5");
    await page.waitForTimeout(2000);
    await page.getByLabel('SQL Mode').locator('div').nth(2).click();
    await page.waitForTimeout(2000);
    await page.locator('[data-test="log-table-column-0-source"]').click();
    await page.locator('[data-test="logs-detail-table-search-around-btn"]').click();
    await page.waitForTimeout(2000);
    const element = await page.locator('[data-test="log-table-column-0-source"]');
    const isVisible = await element.isVisible();
    expect(isVisible).toBeTruthy();
    await expect(page.locator('[data-test="log-table-column-0-source"]')).toBeVisible();
  });
});
