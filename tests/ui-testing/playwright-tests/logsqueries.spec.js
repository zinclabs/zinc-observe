import { test, expect } from "./baseFixtures";
import logData from "../../ui-testing/cypress/fixtures/log.json";
import logsdata from "../../test-data/logs_data.json";

test.describe.configure({ mode: 'parallel' });

async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"]);
  await page.waitForTimeout(4000);
  // await page.getByText('Login as internal user').click();
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
  await page.waitForTimeout(
    4000); await page.locator(
      '[data-test="log-search-index-list-select-stream"]').click({ force: true }); await page.locator(
        "div.q-item").getByText(`${stream}`).first().click({ force: true });
};
test.describe("Logs Queries testcases", () => {
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

  test("should display quick mode toggle button", async ({ page }) => {
    await expect(
      page.locator('[data-test="logs-search-bar-quick-mode-toggle-btn"]')
    ).toBeVisible();
  });

  test("should add timestamp to editor save this view and switch", async ({ page }) => {
    await page.waitForTimeout(3000); 
    await page.locator('[data-test="log-table-column-0-source"]').click();

    await page.locator(':nth-child(1) > [data-test="log-details-include-exclude-field-btn"] > .q-btn__content > .q-icon').click(); await page.locator(
      '[data-test="log-details-include-field-btn"]').click(); await page.locator(
        '[data-test="close-dialog"] > .q-btn__content').click(); await page.locator(
          '[data-test="logs-search-saved-views-btn"] > .q-btn-dropdown--current > .q-btn__content > :nth-child(1)').click(); await page.locator(
            '[data-test="add-alert-name-input"]').fill("e2etimestamp"); await page.locator(
              '[data-test="saved-view-dialog-save-btn"] > .q-btn__content').click(); await page.locator(

                '[data-test="logs-search-saved-views-btn"] > .q-btn-dropdown__arrow-container > .q-btn__content > .q-icon').click(); await page.locator(
                  '.q-item__label').getByText(/timestamp/).first().click({ force: true });
    await page.waitForTimeout(
      3000);
    await page.locator('[data-test="logs-search-saved-views-btn"]').getByLabel('Expand').click();
    await page.locator('[data-test="log-search-saved-view-field-search-input"]').click();
    await page.locator('[data-test="log-search-saved-view-field-search-input"]').fill('e2e');
    await page.getByTitle('e2etimestamp').click();
    await page.locator('[data-test="logs-search-saved-views-btn"]').getByLabel('Expand').click();
    await page.locator('[data-test="log-search-saved-view-field-search-input"]').click();
    await page.getByTitle('e2etimestamp').click();
    // await page.locator('[data-test="logs-search-saved-views-btn"]').getByLabel('Expand').click();
    await page.getByText('delete').click();
    await page.locator('[data-test="confirm-button"]').click();

  });




  test("should redirect to logs after clicking on stream explorer via stream page", async ({ page }) => {
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="logs-search-saved-views-btn"]').getByLabel('Expand').click();
    await page.locator('button').filter({ hasText: 'savesaved_search' }).click();
    await page.locator('[data-test="add-alert-name-input"]').click();
    await page.locator('[data-test="add-alert-name-input"]').fill('streamslognave');
    await page.locator('[data-test="saved-view-dialog-save-btn"]').click({ force: true });
    await page.waitForTimeout(5000);
    await page.locator('[data-test="menu-link-\\/streams-item"]').click({ force: true });
    await page.getByPlaceholder('Search Stream').click();
    await page.getByPlaceholder('Search Stream').fill('e2e');
    await page.getByRole('button', { name: 'Explore' }).first().click({ force: true });
    await page.waitForTimeout(5000);
    await page.waitForSelector('[data-test="logs-search-saved-views-btn"]')
    await page.locator('[data-test="logs-search-saved-views-btn"]').getByLabel('Expand').click();
    await page.locator('[data-test="log-search-saved-view-field-search-input"]').click({ force: true });
    await page.locator('[data-test="log-search-saved-view-field-search-input"]').fill('streamslognave');
    await page.waitForTimeout(3000);
    await page.waitForSelector(':text("streamslognav")');
    await page.click(':text("streamslognave")');
    await page.locator('[data-test="logs-search-saved-views-btn"]').getByLabel('Expand').click();
    await page.locator('[data-test="log-search-saved-view-field-search-input"]').click();
    await page.getByTitle('streamslognave').click();
    await page.getByText('delete').click();
    await page.locator('[data-test="confirm-button"]').click();

  });


  



  test("should reset the editor on clicking reset filter button", async ({ page }) => {
    await page.locator('[data-test="date-time-btn"]').click({ force: true });
    await page.locator('[data-test="date-time-relative-15-m-btn"] > .q-btn__content > .block').click({ force: true });
    await page.locator('[data-test="logs-search-bar-query-editor"] > .monaco-editor').click();
    await page.keyboard.type("match_all_raw_ignore_case('provide_credentials')");
    await page.locator('[data-cy="search-bar-refresh-button"] > .q-btn__content').waitFor({ state: "visible" });
    await page.locator("[data-test='logs-search-bar-refresh-btn']").click({ force: true });
    await page.locator('[data-test="logs-search-bar-reset-filters-btn"]').click({ force: true });
    await page.locator('[data-test="logs-search-bar-query-editor"] > .monaco-editor .view-lines').waitFor({ state: "visible" });
    const text = await page.evaluate(() => {
      const editor = document.querySelector('[data-test="logs-search-bar-query-editor"]').querySelector('.view-lines');
      return editor ? editor.textContent : null;
    });
    await expect(text).toEqual("");
  });

  test("should add invalid query and display error", async ({ page }) => {
    // Type the value of a variable into an input field

    await page.waitForTimeout(2000);
    await page.locator(
      '[data-test="date-time-btn"]').click({ force: true }); await page.locator(

        '[data-test="date-time-relative-15-m-btn"] > .q-btn__content > .block').click({ force: true });
    await page.click('[data-test="logs-search-bar-query-editor"]')
    await page.keyboard.type("kubernetes");
    await page.waitForTimeout(2000);
    await page.locator('[data-cy="search-bar-refresh-button"] > .q-btn__content').click({ force: true })
    await expect(page.locator('[data-test="logs-search-error-message"]')).toBeVisible();
  });

  test("should not display error if match all case added in log query search", async ({ page }) => {
    // Type the value of a variable into an input field
    await page.locator('[data-test="date-time-btn"]').click({ force: true });
    await page.locator('[data-test="date-time-relative-15-m-btn"] > .q-btn__content > .block').click({ force: true });
  
    // Ensure the query editor is visible and clickable before typing
    const queryEditor = page.locator('[data-test="logs-search-bar-query-editor"]');
    await expect(queryEditor).toBeVisible();
    await queryEditor.click();
    await page.keyboard.type("match_all('code')");
  
    // Ensure the refresh button is visible and clickable before clicking
    const refreshButton = page.locator('[data-cy="search-bar-refresh-button"] > .q-btn__content');
    await expect(refreshButton).toBeVisible();
    await refreshButton.click({ force: true });
  
    // Verify that the expected log table column is visible
    await expect(page.locator('[data-test="log-table-column-0-source"]')).toBeVisible();
  });
  


  test("should change stream settings and click on search stream", async ({ page }) => {
    // Type the value of a variable into an input field
    await page.locator('[data-test="menu-link-/streams-item"]').click({ force: true });
    await page.waitForTimeout(3000);
    await page.click('[data-test="streams-search-stream-input"]')
    await page.keyboard.type("e2e_automate");
    await page.locator("[title=\"Stream Detail\"]").first().click({ force: true });
    await page.locator(':nth-child(2) > [data-test="schema-stream-index-select"]').click();
    const scope = page.locator(".q-virtual-scroll__content");
    await scope.getByText(/Full text search/).first().click();
    await page.locator('[data-test="schema-update-settings-button"]').click({ force: true });
    await page.waitForTimeout(3000);
    await page.locator(".col-auto > .q-btn > .q-btn__content").click({ force: true });
    await page.waitForTimeout(3000);
    await page.locator("[title=\"Explore\"]").first().click({ force: true });
    await expect(page.locator('[data-test="log-table-column-0-source"]')).toBeVisible();
  });
  

  test("should display error if blank spaces added under stream name and clicked create stream ", async ({ page }) => {
    await page.locator('[data-test="menu-link-/streams-item"]').click({ force: true });
    await page.locator('[data-test="log-stream-add-stream-btn"]').click({ force: true });
    await page.click('[data-test="add-stream-name-input"]')
    await page.keyboard.type("  ");
    await page.locator('[data-test="save-stream-btn"]').click({ force: true });
    await expect(page.getByText(/Field is required/).first()).toBeVisible();
  });



  test("should display error if create stream is clicked without adding name", async ({ page }) => {
    await page.locator('[data-test="menu-link-/streams-item"]').click({ force: true });
    await page.locator('[data-test="log-stream-add-stream-btn"]').click({ force: true });
    await page.locator('[data-test="save-stream-btn"]').waitFor({ state: "visible" });
    await page.locator('[data-test="save-stream-btn"]').scrollIntoViewIfNeeded();
    await page.locator('[data-test="save-stream-btn"]').click({ force: true });
    await expect(page.getByText(/Field is required/).first()).toBeVisible();
  });
  
  test("should display enter count query", async ({ page }) => {
    await page.locator(

      '[data-test="logs-search-bar-query-editor"] > .monaco-editor').click();
    await page.click('[data-test="logs-search-bar-query-editor"] > .monaco-editor')
    await page.keyboard.type('SELECT COUNT(_timestamp) AS xyz, _timestamp FROM "e2e_automate"  Group by _timestamp ORDER BY _timestamp DESC');
    await page.waitForTimeout(4000);
    await page.getByLabel("SQL Mode").locator("div").nth(2).click();
    await expect(page.locator(
      '[data-cy="search-bar-refresh-button"] > .q-btn__content')).toBeVisible(); await page.waitForTimeout(
        3000); await page.locator(
          "[data-test='logs-search-bar-refresh-btn']").click({

            force: true
          });
    await page.waitForTimeout(4000);
    await expect(page.getByRole('heading', { name: 'Error while fetching' })).not.toBeVisible();
    await page.locator('[data-test="logs-search-result-bar-chart"] canvas').click({
      position: {
        x: 182,
        y: 66
      }
    });

  });

  test("should display values API text successfully and error to not be displayed", async ({ page }) => {
    await page.locator('[data-test="log-search-index-list-field-search-input"]').fill('kubernetes_pod_name');
    await page.getByLabel('Expand "kubernetes_pod_name"').click();
    const errorMessage = page.getByText('Error while fetching field values');
    await expect(errorMessage).not.toBeVisible();
    await page.getByText('ziox-ingester-').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.getByLabel('Collapse "kubernetes_pod_name"').click();
    await page.getByLabel('Expand "kubernetes_pod_name"').click();
    const targetElement = page.locator('[data-test="logs-search-subfield-add-kubernetes_pod_name-ziox-ingester-0"]').getByText('ziox-ingester-');
    await expect(targetElement).toBeVisible(); // Assertion to ensure visibility
    await targetElement.click()
  })

})