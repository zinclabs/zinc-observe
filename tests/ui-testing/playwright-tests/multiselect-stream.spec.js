import { test, expect } from "@playwright/test";
import logData from "../../ui-testing/cypress/fixtures/log.json";
import logsdata from "../../test-data/logs_data.json";
import { toZonedTime } from "date-fns-tz";

test.describe.configure({ mode: "parallel" });
const folderName = `Folder ${Date.now()}`;
const dashboardName = `AutomatedDashboard${Date.now()}`;

async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"]);
  await page.waitForTimeout(1000);
  // await page.getByText("Login as internal user").click();
  await page
    .locator('[data-cy="login-user-id"]')
    .fill(process.env["ZO_ROOT_USER_EMAIL"]);
  await page.locator("label").filter({ hasText: "Password *" }).click();
  await page
    .locator('[data-cy="login-password"]')
    .fill(process.env["ZO_ROOT_USER_PASSWORD"]);
  await page.locator('[data-cy="login-sign-in"]').click();
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

const getHeaders = () => {
  const basicAuthCredentials = Buffer.from(
    `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
  ).toString("base64");

  return {
    Authorization: `Basic ${basicAuthCredentials}`,
    "Content-Type": "application/json",
  };
};

const getIngestionUrl = (orgId, streamName) => {
  return `${process.env.INGESTION_URL}/api/${orgId}/${streamName}/_json`;
};

const sendRequest = async (page, url, payload, headers) => {
  return await page.evaluate(
    async ({ url, headers, payload }) => {
      const response = await fetch(url, {
        method: "POST",
        headers: headers,
        body: JSON.stringify(payload),
      });
      return await response.json();
    },
    { url, headers, payload }
  );
};

test.describe("Stream multiselect testcases", () => {
  function removeUTFCharacters(text) {
    return text.replace(/[^\x00-\x7F]/g, " ");
  }

  async function applyQueryButton(page) {
    const search = page.waitForResponse(logData.applyQuery);
    await page.waitForTimeout(3000);
    await page.locator("[data-test='logs-search-bar-refresh-btn']").click({
      force: true,
    });
    await expect.poll(async () => (await search).status()).toBe(200);
  }

  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.waitForTimeout(5000);

    const orgId = process.env["ORGNAME"];
    const streamNames = ["e2e_automate", "e2e_stream1"];
    const headers = getHeaders();

    for (const streamName of streamNames) {
      const ingestionUrl = getIngestionUrl(orgId, streamName);

      // Payload data
      const payload = {
        level: "info",
        job: "test",
        log: "test message for openobserve",
        e2e: "1",
      };

      const response = await sendRequest(page, ingestionUrl, payload, headers);
      console.log(`Response from ${streamName}:`, response);
    }

    await page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );
    const allsearch = page.waitForResponse("**/api/default/_search**");
    await selectStreamAndStreamTypeForLogs(page, logData.Stream);
    await applyQueryButton(page);
  });


async function multistreamselect(page) {
    await page.locator('[data-test="menu-link-\\/-item"]').click();
    await page.locator('[data-test="menu-link-\\/logs-item"]').click();
    await page.waitForTimeout(2000);
    await page.locator('[data-test="log-search-index-list-select-stream"]').fill('e2e_stream1');
    await page.waitForTimeout(2000);
    await page.locator('[data-test="log-search-index-list-stream-toggle-e2e_stream1"] div').nth(2).click();
    await page.waitForTimeout(4000);
    await page.locator('#fnEditor > .monaco-editor > .overflow-guard > .monaco-scrollable-element > .lines-content > .view-lines').click()
//   await page.locator('[data-test="log-search-index-list-stream-toggle-e2e_stream1"] div').nth(2).click({force:true});
    const cell = await page.getByRole('cell', { name: /Common Group Fields/ });

  // Extract the text content of the cell
    const cellText = await cell.textContent();

  // Verify that the text contains 'Common Group Fields'
  expect(cellText).toContain('Common Group Fields');
  
    await page.getByRole('cell', { name: /E2e_automate/ }).click();
    await page.getByRole('cell', { name: /E2e_stream1/  }).click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-h-btn"]').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('[data-test="log-table-column-0-\\@timestamp"] [data-test="table-row-expand-menu"]').click();
  }
  

  test("should add a function and display it in streams", async ({ page }) => {
await multistreamselect(page);
await page.locator('#fnEditor').getByLabel('Editor content;Press Alt+F1').fill('.a=2');
await page.waitForTimeout(1000);
    await applyQueryButton(page);
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

  test("should click on live mode on button and select 5 sec, switch off, and then click run query", async ({
    
    page,
  }) => {
    await multistreamselect(page);
    await page.route("**/logData.ValueQuery", (route) => route.continue());
    await page.locator('[data-cy="date-time-button"]').click({ force: true });

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

  test("should redirect to logs after clicking on stream explorer via stream page", async ({
    page,
  }) => {
    await multistreamselect(page);
    await page.locator('[data-cy="date-time-button"]').click({ force: true });
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

  test("should click on interesting fields icon and display query in editor", async ({
    page,
  }) => {
    await multistreamselect(page);
    await page.locator('[data-test="logs-search-bar-quick-mode-toggle-btn"] div').first().click();
    await page
      .locator('[data-cy="index-field-search-input"]')
      .fill("_timestamp");
    await page.waitForTimeout(2000);
    await page
      .locator(".field-container")
      .locator(
        '[data-test="log-search-index-list-interesting-_timestamp-field-btn"]'
      )
      .last()
      .click({
        force: true,
      });
      await page
      .locator('[data-test="table-row-expand-menu"]')
      .first()
      .click({ force: true });
      // await page.getByText('{ arrow_drop_down_stream_name').click();
      // await page.getByText('_stream_name:').click();
      // await page.getByText('_timestamp:').click();
    
  });

  // test('should display search around in histogram mode', async ({ page }) => {
  //   await multistreamselect(page);
  //   await page.waitForTimeout(1000);
  //   await page.locator('[data-test="log-table-column-0-source"]').click();
  //   const isDisabled = await page.locator('[data-test="logs-detail-table-search-around-btn"]').isDisabled();
  //   expect(isDisabled).toBe(true);
  // });
  

})
