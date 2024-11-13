import { test, expect } from "./baseFixtures";
import logData from "../../ui-testing/cypress/fixtures/log.json";
import { log } from "console";
import logsdata from "../../test-data/logs_data.json";
const uniqueText = `${new Date().valueOf()}`



// test.describe.configure({ mode: 'parallel' });
const { v4: uuidv4 } = require("uuid");

// Generate UUID
const uuid = uuidv4();

// Remove non-alphabetic characters from UUID
const alphaUuid = uuid.replace(/[^a-zA-Z]/g, "");

// Construct function name
const functionName = `automate${alphaUuid}`;




async function login(page) {
      await page.goto(process.env["ZO_BASE_URL"]);
      await page.waitForTimeout(1000);
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


const selectStreamAndStreamTypeForLogs = async (page,stream) => {await page.waitForTimeout(
  4000);await page.locator(
  '[data-test="log-search-index-list-select-stream"]').click({ force: true });await page.locator(
  "div.q-item").getByText(`${stream}`).first().click({ force: true });
};
test.describe("Functions testcases", () => {
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
      `${logData.functionUrl}functions?org_identifier=${process.env["ORGNAME"]}`

    );
    // const allsearch = page.waitForResponse("**/api/default/_search**");
    // await selectStreamAndStreamTypeForLogs(page,logData.Stream);
    // await applyQueryButton(page);
    // const streams = page.waitForResponse("**/api/default/streams**");
  });



test("should display error when creating function without mandatory fields", async ({ page }) => {await page.getByText(/Create new function/).first().click({
    force: true });await expect(page.getByText(/Save/).first()).toBeVisible();await page.getByText(/Save/).first().click({
    force: true });await expect(page.getByText(/Field is required/).first()).toBeVisible();

});

test("should display error on entering invalid name under function and save", async ({ page }) => {await page.getByText(/Create new function/).first().click({
    force: true });await page.locator(
    ".q-pb-sm > .q-field > .q-field__inner > .q-field__control").fill(
    uniqueText);await expect(page.getByText(/Save/).first()).toBeVisible();await page.getByText(/Save/).first().click({

    force: true });await expect(page.getByText(/Invalid method name\./).first()).toBeVisible();

});

test("should display error on entering invalid function and save", async ({ page }) => {await page.getByText(/Create new function/).first().click({
    force: true });await page.locator(
    ".q-pb-sm > .q-field > .q-field__inner > .q-field__control").fill(
    functionName);
    await page.locator('.view-lines').click();

    // Type into the focused element
    await page.keyboard.type('.test=1');
    await expect(page.getByText(/Save/).first()).toBeVisible();await page.getByText(/Save/).first().click({
    force: true });await expect(page.locator(
    ".q-notification__message").getByText(/Function saved successfully/).first()).toBeVisible();
    await page.locator('[data-test="functions-list-search-input"]').click()
    await page.keyboard.type(functionName);
    await page.waitForTimeout(1000)
    await page.click('button[title="Delete Function"]');
    await page.click('[data-test="confirm-button"] > .q-btn__content')
});

test("should display error on adding function with same name", async ({ page }) => 
   {await page.getByText(/Create new function/).first().click({
    force: true });await page.locator(
    ".q-pb-sm > .q-field > .q-field__inner > .q-field__control").fill(
    functionName);
    await page.locator('.view-lines').click();
    await page.keyboard.type('.test=1');
    await expect(page.getByText(/Save/).first()).toBeVisible();
    await page.getByText(/Save/).first().click({
    force: true });
    await expect(page.locator(
    ".q-notification__message").getByText(/Function saved successfully/).first()).toBeVisible();
    await page.getByText(/Create new function/).first().click({force: true });
    await page.locator(
    ".q-pb-sm > .q-field > .q-field__inner > .q-field__control").fill(functionName);
    await page.locator('.view-lines').click();
    await page.keyboard.type('.test=1');
    await expect(page.getByText(/Save/).first()).toBeVisible();await page.getByText(/Save/).first().click({
    force: true });
    await expect(page.locator(
    ".q-notification__message").getByText(/Function creation failed/).first()).toBeVisible();
    await page.getByText(/Cancel/).first().click({force: true });
    await page.locator('[data-test="functions-list-search-input"]').click()
    await page.keyboard.type(functionName);
    await page.waitForTimeout(1000)
    await page.click('button[title="Delete Function"]');
    await page.click('[data-test="confirm-button"] > .q-btn__content')
});


// test("should add a function and associate a stream with the same", async ({ page }) => {await page.getByText(/Create new function/).first().click({
//     force: true });await page.locator(
//     ".q-pb-sm > .q-field > .q-field__inner > .q-field__control").fill(
//     functionName);
//     await page.locator('.view-lines').click();
//     await page.keyboard.type('.test=1');
//     await expect(page.getByText(/Save/).first()).toBeVisible();await page.getByText(/Save/).first().click({
//     force: true });await expect(page.locator(
//     ".q-notification__message").getByText(/Function saved successfully/).first()).toBeVisible();await page.locator(


//     '[data-test="function-stream-tab"]').click({ force: true });await page.waitForTimeout(
//     2000);
   
//     await page.locator('[data-test="stream-association-search-input"]').click()
//     await page.keyboard.type("e2e_automate");
//     await page.locator( '[data-test="log-stream-table"] >>> :nth-child(2) > :nth-child(1) > :nth-child(2)').click();
//     await page.waitForTimeout(1000)
//     await page.click('button[data-test="stream-association-associate-function-btn"]');


//   // cy.get('[style="cursor: pointer;"] > :nth-child(3)').click({ force: true });
//     // await page.getByText(/Associate Function/).first().click({ force: true });
//     await page.locator(

//     '[style="height: min-content; border: 1px solid black;"] > [colspan="100%"] > [style=""] > .q-table__container > .q-table__middle > .q-table > tbody > .q-tr > :nth-child(2) > .q-field > .q-field__inner > .q-field__control > .q-field__control-container').fill(
//     functionName);await page.waitForTimeout(
//     2000);await page.locator(
//     ".q-item__label > span").click({ force: true });await page.waitForTimeout(
//     1000);const scope = page.locator(


//     "tr").filter({ has: page.locator("tr.q-tr").getByText(functionName).first() });

//   // Click the delete button
//   await scope.locator(
//     'button[data-test="stream-association-delete-function-btn"]').click();await page.locator(



//     '[href="/web/functions/functions?org_identifier=default"] > .q-tab__content').click({
//     force: true });
//     await page.keyboard.type(functionName);
//     await page.waitForTimeout(1000)
//     await page.click('button[title="Delete Function"]');
//     await page.click('[data-test="confirm-button"] > .q-btn__content')
//     // await page.locator(
//     // '[data-test="functions-list-search-input"]').fill(functionName);
//     // {const scope = page.locator(


//     //   "tr").filter({ has: page.locator("tr").getByText(functionName).first() });

//     // // Click the delete button
//     // await scope.locator('button[title="Delete Function"]').click();}await page.locator(

//     // '[data-test="confirm-button"] > .q-btn__content').click();
// });




 
});
