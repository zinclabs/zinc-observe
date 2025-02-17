import { test, expect } from './baseFixtures';
import logData from "../../ui-testing/cypress/fixtures/log.json";
import { LogsPage } from '../pages/logsPage.js';

test.describe.configure({ mode: "parallel" });


async function login(page) {
 
  await page.goto(process.env["ZO_BASE_URL"]);
  if (await page.getByText('Login as internal user').isVisible()) {
    await page.getByText('Login as internal user').click();
}
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



test.describe("Sanity testcases", () => {
  let logsPage;
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
  }

  test.beforeEach(async ({ page }) => {
    await login(page);
    logsPage = new LogsPage(page);
    await page.waitForTimeout(1000)
    await ingestion(page);
    await page.waitForTimeout(2000)

    await page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );
  

  });



  

  test('should send a large log payload to the API and verify success', async ({ page }) => {
    // Setup: Define organization info and the base URL from environment variables
    const baseUrl = process.env.INGESTION_URL;  // Base URL from the environment variable
    const orgId = process.env["ORGNAME"];  // Organization ID from environment variable
    const streamName = 'stress_test1';  // You can change this as needed
    // const streamSchemaResponse = page.waitForResponse(response =>
    //   response.url().includes('/api/default/streams/stress_test1/schema?type=logs') && response.status() === 200
    // );
    // const valuesResponse = page.waitForResponse(response =>
    //   response.url().includes('/api/default/stress_test1/_values') && response.status() === 200
    // );
    
  
    // Generate log data dynamically (79000 fields)
    const logData = generateLogData();
  
    // Call the ingestion function to send the data
    await ingestion(page, baseUrl, orgId, streamName, logData);
  
    console.log(`✅ Log ingestion test passed`);
  
    // Optional: Add a delay to prevent rate limiting
    await page.waitForTimeout(100); // Wait 100ms
  
    // Perform additional steps
    await page.locator('[data-test="menu-link-\\/-item"]').click();
    await page.reload(); // Reload after this step
    await page.locator('[data-test="menu-link-\\/logs-item"]').click();
    await page.waitForTimeout(4000)
  
    // Search and toggle stream
    await page.locator('[data-test="log-search-index-list-select-stream"]').click();
    await page.locator('[data-test="log-search-index-list-select-stream"]').fill('stress_test1');
    await page.waitForTimeout(4000)
    await page.locator('[data-test="log-search-index-list-stream-toggle-stress_test1"] div').first().click();
    await page.waitForTimeout(4000)  
  
    // Expand log attributes and click the log text
    await page.locator('[data-test="log-search-index-list-field-search-input"]').click();
  await page.locator('[data-test="log-search-index-list-field-search-input"]').fill('10007');
  
  await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
  await page.waitForTimeout(4000)
  await page.getByRole('button', { name: 'Expand "' }).click();
  await page.locator('[data-test="logs-search-subfield-add-log_log_attribute10007-Lorem ipsum dolor sit amet\\, consectetur adipiscing elit\\. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua\\."]').getByText('Lorem ipsum dolor sit amet,').click();
  await expect(page.locator('[data-test="log-table-column-0-_timestamp"]')).toBeVisible();


  await page.locator('[data-test="menu-link-\\/streams-item"]').click();
  await page.getByPlaceholder('Search Stream').click();
  await page.getByPlaceholder('Search Stream').fill('stress_test1');
  await page.getByRole('button', { name: 'Stream Detail' }).click();
  await page.locator('[data-test="log-stream-store-original-data-toggle-btn"] div').nth(2).click();
  await page.getByRole('button').filter({ hasText: 'close' }).click();
  await page.getByRole('button', { name: 'Explore' }).click();
  await page.waitForTimeout(10000);
  // await expect(page.getByText('Loading...')).toBeHidden();
  // await page.waitForTimeout(1000);
  await expect(page.locator('[data-test="log-table-column-0-_timestamp"]')).toBeVisible();
  
  });
  
  async function ingestion(page, baseUrl, orgId, streamName, logsdata) {
    const basicAuthCredentials = Buffer.from(
      `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
    ).toString('base64');
  
    const headers = {
      "Authorization": `Basic ${basicAuthCredentials}`,
      "Content-Type": "application/json",
    };
  
    try {
      const response = await page.evaluate(async ({ url, headers, orgId, streamName, logsdata }) => {
        const fetchResponse = await fetch(`${url}/api/${orgId}/${streamName}/_json`, {
          method: 'POST',
          headers: headers,
          body: JSON.stringify(logsdata)
        });
        return await fetchResponse.json();
      }, {
        url: baseUrl,
        headers: headers,
        orgId: orgId,
        streamName: streamName,
        logsdata: logsdata
      });
  
      console.log('Response:', response);
    } catch (error) {
      console.error('Error during log ingestion:', error);
    }
  }
  
  function generateLogData() {
    const attributes = {};
    for (let j = 1; j <= 79000; j++) {
      attributes[`log_attribute${j}`] =
        'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.';
    }
  
    if (Object.keys(attributes).length !== 79000) {
      throw new Error(`Expected 79000 fields but got ${Object.keys(attributes).length}`);
    }
  
    return {
      level: 'info',
      job: 'test',
      log: attributes,
    };
  }

})