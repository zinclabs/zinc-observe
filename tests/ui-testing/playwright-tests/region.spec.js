import { test, expect } from "./baseFixtures";
import logData from "../cypress/fixtures/log.json";
import logsdata from "../../test-data/logs_data.json";

test.describe.configure({ mode: 'parallel' });
const randomSavedView = `Savedview${Math.floor(Math.random() * 1000)}`;



async function login(page) {
      await page.goto(process.env["ZO_BASE_URL"]);
      // await page.getByText('Login as internal user').click();
      await page.waitForTimeout(1000);
      await page
        .locator('[data-cy="login-user-id"]')
        .fill(process.env["ZO_ROOT_USER_EMAIL"]);
      //Enter Password
      await page.locator('label').filter({ hasText: 'Password *' }).click();
      await page
        .locator('[data-cy="login-password"]')
        .fill(process.env["ZO_ROOT_USER_PASSWORD"]);
      await page.locator('[data-cy="login-sign-in"]').click();
  //     await page.waitForTimeout(4000);
  // await page.goto(process.env["ZO_BASE_URL"]);
}


const selectStreamAndStreamTypeForLogs = async (page,stream) => {await page.waitForTimeout(
  4000);await page.locator(
  '[data-test="log-search-index-list-select-stream"]').click({ force: true });await page.locator(
  "div.q-item").getByText(`${stream}`).first().click({ force: true });
};

test.describe("Region testcases", () => {
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
    await page.waitForTimeout(5000)

    // ("ingests logs via API", () => {
      const orgId = process.env["ORGNAME"];
      const streamName = "e2e_automate";
      const basicAuthCredentials = Buffer.from(
        `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
      ).toString('base64');
    
      const headers = {
        "Authorization": `Basic ${basicAuthCredentials}`,
        "Content-Type": "application/json",
      };
    
      // const logsdata = {}; // Fill this with your actual data
    
      // Making a POST request using fetch API
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
    //  });
    // const allorgs = page.waitForResponse("**/api/default/organizations**");
    // const functions = page.waitForResponse("**/api/default/functions**");
    await page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );
    const allsearch = page.waitForResponse("**/api/default/_search**");
    await selectStreamAndStreamTypeForLogs(page,logData.Stream);
    await applyQueryButton(page);
    const searchBarRegionBtn = await page.locator('[data-test="logs-search-bar-region-btn"]');
    if (!(await searchBarRegionBtn.isVisible())) {
        test.skip('Skipping test because region button is not visible');
        return;
    }

    // Button is visible, proceed with test logic
    await searchBarRegionBtn.click();
    // const streams = page.waitForResponse("**/api/default/streams**");
  });
  
  test("should display region button and click on radio buttons", async ({ page }) => {
    await page.locator('[data-test="logs-search-bar-region-btn"]').click();
    await page.getByText('radio_button_uncheckedus-west-')
    await page.getByText('radio_button_uncheckedus-east-').click();
    await page.getByText('radio_button_uncheckedus-west-').click();
  });

  test("should display save view with 1 region", async ({ page }) => {
    await page.locator('[data-test="log-search-index-list-select-stream"]').click();
    await page.locator('[data-test="logs-search-bar-region-btn"]').click();
    

    await page.waitForSelector('.q-item__label');

    // Click on the radio button
    const radioButton = await page.locator('.q-item__label:has-text("us-east-1")');
    await radioButton.click();
    
    // Verify if the radio button is selected (true)
    const isChecked = await page.$eval('.q-item__label:has-text("us-east-1")', node => {
        const parentElement = node.parentElement;
        if (parentElement && parentElement.querySelector('input')) {
            return parentElement.querySelector('input').checked;
        }
        return false;
    });
    console.log('Radio button is selected:', isChecked);

    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.locator('button').filter({ hasText: 'savesaved_search' }).click();
    await page.locator('[data-test="add-alert-name-input"]').click();
    await page.locator('[data-test="add-alert-name-input"]').fill(randomSavedView);
    await page.locator('[data-test="saved-view-dialog-save-btn"]').click();
    await page.locator('[data-test="menu-link-\\/streams-item"]').click();
    await page.getByRole('button', { name: 'Explore' }).nth(1).click();
    await page.waitForTimeout(2000)
    await page.locator('[data-test="logs-search-saved-views-btn"]').getByLabel('Expand').click();
    await page.waitForTimeout(2000)
    await page.locator('[data-test="log-search-saved-view-field-search-input"]').click();
    await page.locator('[data-test="log-search-saved-view-field-search-input"]')
    await page.locator('[data-test="log-search-saved-view-field-search-input"]').fill(randomSavedView);
    await page.getByTitle(randomSavedView).click();
    await page.getByText('view applied').click();
    page.reload();
    await page.locator('[data-test="logs-search-bar-region-btn"]').click();
    const isCircleChecked = await page.$eval('[data-test="logs-search-bar-region-btn"] i.material-icons', node => node.innerText === 'check_circle');
    console.log('Circle is checked:', isCircleChecked);
    await page.waitForTimeout(2000)
    await page.locator('[data-test="logs-search-saved-views-btn"]').getByLabel('Expand').click();
    await page.waitForTimeout(2000)
    await page.locator('[data-test="log-search-saved-view-field-search-input"]').click();
    await page.locator('[data-test="log-search-saved-view-field-search-input"]').fill(randomSavedView);
    await page.locator(`[data-test="logs-search-bar-delete-${randomSavedView}-saved-view-btn"]`).click();
    await page.locator('[data-test="confirm-button"]').click();
  })

  test('select timezone and searcharound', async ({ page }) => {
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="datetime-timezone-select"]')
    await page.locator('[data-test="datetime-timezone-select"]').click();
    await page.locator('[data-test="datetime-timezone-select"]').fill('asia/dubai');
    await page.getByRole('option', { name: 'Asia/Dubai' }).locator('div').nth(2).click();
    await page.waitForTimeout(3000)
    await page.locator('[data-test="logs-search-bar-region-btn"]').click();
    await page.waitForTimeout(2000)
    await page.getByText('radio_button_uncheckedus-east-').click();
    await page.locator('[data-test="logs-search-bar-refresh-btn"]').click();
    await page.waitForTimeout(2000)
    await page.locator('[data-test="log-table-column-0-source"]').getByText('{"_timestamp":').click();
    await page.waitForTimeout(2000)
    await page.locator('[data-test="logs-detail-table-search-around-btn"]').click();
    await page.waitForTimeout(2000)
    await page.locator('[data-test="log-table-column-0-source"]').getByText('{"_timestamp":').click();
  });
  
 
})