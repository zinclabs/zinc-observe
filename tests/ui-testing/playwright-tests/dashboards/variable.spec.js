import { test, expect } from "../baseFixtures";
import logData from "../../cypress/fixtures/log.json";
import logsdata from "../../../test-data/logs_data.json";

test.describe.configure({ mode: 'parallel' });
const folderName = `Folder ${Date.now()}`
const dashboardName = 'dashboard' + Math.random().toString(36).substring(7);

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
test.describe("Variable testcases", () => {
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
    // const streams = page.waitForResponse("**/api/default/streams**");
  });
  


  test('should select 1, select all variable and delete panel delete dashboard', async ({ page }) => {

    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await page.waitForTimeout(2000);  
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page.locator('[data-test="add-dashboard-name"]').fill(dashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.locator('[data-test="dashboard-if-no-panel-add-panel-btn"]').click();
    await page.locator('[data-test="index-dropdown-stream"]').click();
    await page.waitForTimeout(2000);  
    await page.getByRole('option', { name: 'e2e_automate' }).locator('div').nth(2).click();
    await page.waitForTimeout(2000);  
    await page.locator('[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubectl_kubernetes_io_default_container"] [data-test="dashboard-add-y-data"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill('e2evariable');
    await page.locator('[data-test="dashboard-panel-save"]').click();
    await page.locator('[data-test="dashboard-setting-btn"]').click();
    await page.waitForTimeout(2000); 
    await page.getByRole('tab', { name: 'Variables' }).click();
    await page.getByRole('button', { name: 'Add Variable' }).click();
    await page.waitForTimeout(2000); 
    await page.getByLabel('Name *').click();
    await page.getByLabel('Name *').fill('e2evariable');
    await page.locator('.row > label > .q-field__inner > .q-field__control > .q-field__control-container > .q-field__native').first().click();
    await page.getByRole('option', { name: 'logs' }).click();
    await page.getByLabel('Stream *').click();
    await page.getByRole('option', { name: 'default', exact: true }).locator('div').nth(2).click();
    await page.getByLabel('Stream *').click();
    await page.getByRole('option', { name: 'e2e_automate' }).click();
    await page.getByLabel('Field *').click();
    await page.getByText('kubernetes_host').click();
    await page.locator('[data-test="dashboard-query_values-show_multiple_values"] div').nth(2).click();
    await page.getByLabel('Default max record size(').click();
    await page.getByRole('button', { name: 'Save' }).click();
    await page.getByText('ip-10-2-56-221.us-east-2.compute.internale2evariable').click();
    await page.waitForTimeout(200); 
    await page.getByText('ip-10-2-15-197.us-east-2.').click({force: true});

    await page.getByRole('main').locator('div').filter({ hasText: /^arrow_drop_down$/ }).click();
    await page.getByText('Select All').click();
    await page.waitForTimeout(200)
    await page.getByRole('main').locator('div').filter({ hasText: /^arrow_drop_down$/ }).click()
    await page.waitForTimeout(200);
    const items = await page.$$('div.q-virtual-scroll__content > div[role="option"]');
    for (const item of items) {
        const label = await item.$('.q-item__label');
        const text = await (await label.getProperty('textContent')).jsonValue();
        const isSelected = await item.evaluate(item => item.getAttribute('aria-selected'));
        console.log(`${text} is selected: ${isSelected === 'true'}`);
    }
    await page.locator('[data-test="dashboard-variable-adhoc-add-selector"]').click();
    await page.waitForTimeout(200);
    await page.locator('[data-test="dashboard-edit-panel-e2evariable-dropdown"]').click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await page.locator('[data-test="confirm-button"]').click();
    await page.locator(2000)
    await page.locator('[data-test="dashboard-back-btn"]').click();
    await page.locator(2000)
    await page.getByRole('row', { name: `01 ${dashboardName}` }).locator('[data-test="dashboard-delete"]').click();
    // await page.getByRole('row', { name: '01 testvariable' }).locator('[data-test="dashboard-delete"]').click();
    await page.waitForTimeout(200);
    await page.locator('[data-test="confirm-button"]').click();

  });
    
  });




