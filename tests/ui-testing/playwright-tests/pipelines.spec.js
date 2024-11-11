import { test, expect } from "@playwright/test";
import logData from "../../ui-testing/cypress/fixtures/log.json";
import { log } from "console";
import logsdata from "../../test-data/logs_data.json";
import PipelinePage from "../pages/pipelinePage";

test.describe.configure({ mode: 'parallel' });

const randomPipelineName = `Pipeline${Math.floor(Math.random() * 1000)}`;
const randomFunctionName = `Pipeline${Math.floor(Math.random() * 1000)}`;

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
async function deletePipeline(page, randomPipelineName) {
  // Click the back button
  await page.locator('[data-test="add-pipeline-back-btn"]').click();
  await page.locator('[data-test="confirm-button"]').click();
  await page.waitForTimeout(2000);

  // Search for the pipeline
  await page.locator('[data-test="pipeline-list-search-input"]').click();
  await page.locator('[data-test="pipeline-list-search-input"]').fill(randomPipelineName);

  // Delete the pipeline
  await page.locator(`[data-test="pipeline-list-${randomPipelineName}-delete-pipeline"]`).click();
  await page.locator('[data-test="confirm-button"]').click();
}
test.describe("Pipeline testcases", () => {
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
    
    //   // Making a POST request using fetch API
    //   const response = await page.evaluate(async ({ url, headers, orgId, streamName, logsdata }) => {
    //     const fetchResponse = await fetch(`${url}/api/${orgId}/${streamName}/_json`, {
    //       method: 'POST',
    //       headers: headers,
    //       body: JSON.stringify(logsdata)
    //     });
    //     return await fetchResponse.json();
    //   }, {
    //     url: process.env.INGESTION_URL,
    //     headers: headers,
    //     orgId: orgId,
    //     streamName: streamName,
    //     logsdata: logsdata
    //   });
    
    //   console.log(response);
    // //  });
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
  
  
  test('should display error when stream not selected while adding source', async ({ page }) => {
    const pipelinePage = new PipelinePage(page);

    await pipelinePage.openPipelineMenu();
    await page.waitForTimeout(1000)
    await pipelinePage.addPipeline();
    await pipelinePage.selectStream();
    await pipelinePage.dragStreamToTarget(pipelinePage.streamButton);;
    await pipelinePage.selectLogs();
    await pipelinePage.saveStream();
    await pipelinePage.confirmStreamError()
  });

  test('should display error when user directly clicks on save without adding details', async ({ page }) => {
    const pipelinePage = new PipelinePage(page);
    await pipelinePage.openPipelineMenu();
    await page.waitForTimeout(1000)
    await pipelinePage.addPipeline();
    await pipelinePage.savePipeline();
    await pipelinePage.confirmPipelineNameRequired()
  });

  test('should display error on entering only pipeline name and save', async ({ page }) => {
    const pipelinePage = new PipelinePage(page);
    await pipelinePage.openPipelineMenu();
    await page.waitForTimeout(1000)
    await pipelinePage.addPipeline();
    const pipelineName = `pipeline-${Math.random().toString(36).substring(7)}`;
    await pipelinePage.enterPipelineName(pipelineName);
    await pipelinePage.savePipeline();
    await pipelinePage.confirmSourceNodeRequired() 
  });

  test('should display error on entering only source node and save', async ({ page }) => {
    const pipelinePage = new PipelinePage(page);
  
    await pipelinePage.openPipelineMenu();
    await page.waitForTimeout(1000)
    await pipelinePage.addPipeline();
    await pipelinePage.selectStream();
    await pipelinePage.dragStreamToTarget(pipelinePage.streamButton);
    await pipelinePage.selectLogs();
  
    // Interact with stream name and save
    await pipelinePage.enterStreamName('e2e');
    await pipelinePage.enterStreamName('e2e_automate');
    await page.waitForTimeout(1000)
    await pipelinePage.selectStreamOption();
    await pipelinePage.saveInputNodeStream();
  
    // Generate a random pipeline name and save
    const pipelineName = `pipeline-${Math.random().toString(36).substring(7)}`;
    await pipelinePage.enterPipelineName(pipelineName);
    await pipelinePage.savePipeline();
  
    // Confirm the destination node required error message
    await pipelinePage.confirmDestinationNodeRequired();
  });

  test('should delete the create source node on confirmation', async ({ page }) => {
    const pipelinePage = new PipelinePage(page);
  
    await pipelinePage.openPipelineMenu();
    await page.waitForTimeout(1000)
    await pipelinePage.addPipeline();
    await pipelinePage.selectStream();
    await pipelinePage.streamButton.waitFor({ state: 'visible' })
    await pipelinePage.dragStreamToTarget(pipelinePage.streamButton);
    await pipelinePage.selectLogs();
  
    // Interact with stream name and save
    await page.waitForTimeout(2000)
    await pipelinePage.enterStreamName('e2e');
    await pipelinePage.enterStreamName('e2e_automate')
    await page.waitForTimeout(2000);
    await pipelinePage.selectStreamOption();
    await pipelinePage.saveInputNodeStream();

    // Delete the pipeline and confirm
    await pipelinePage.deletePipeline();
    await pipelinePage.confirmDelete();
  });


  test('should add source & destination node and then delete the pipeline', async ({ page }) => {
    const pipelinePage = new PipelinePage(page);
  
    await pipelinePage.openPipelineMenu();
    await page.waitForTimeout(1000)
    await pipelinePage.addPipeline();
    await pipelinePage.selectStream();
    await pipelinePage.dragStreamToTarget(pipelinePage.streamButton);
    await pipelinePage.selectLogs();
  
    // Interact with stream name and save
    await pipelinePage.enterStreamName('e2e');
    await pipelinePage.enterStreamName('e2e_automate');
    await page.waitForTimeout(2000)
    await pipelinePage.selectStreamOption();
    await pipelinePage.saveInputNodeStream();
    await pipelinePage.dragStreamToTarget(pipelinePage.streamButton); // First stream drag

    // Select the second stream, drag, and drop
    await pipelinePage.selectAndDragSecondStream();
    await page.waitForTimeout(2000);
    
    // Select the previous node
    await pipelinePage.selectPreviousNode()
    await pipelinePage.selectPreviousNodeDrop();
    await pipelinePage.toggleCreateStream();
    await page.getByLabel('Name *').click();
    await page.getByLabel('Name *').fill('destination-node');
    await page.locator('.q-form > div:nth-child(2) > .q-field > .q-field__inner > .q-field__control > .q-field__control-container > .q-field__native').click();
    await page.getByRole('option', { name: 'Logs' }).locator('div').nth(2).click();
    await pipelinePage.clickSaveStream();
    await pipelinePage.clickInputNodeStreamSave()
    const pipelineName = `pipeline-${Math.random().toString(36).substring(7)}`;
    await pipelinePage.enterPipelineName(pipelineName);
    await pipelinePage.savePipeline();
    await pipelinePage.searchPipeline(pipelineName)
    await page.waitForTimeout(1000);
    const deletePipelineButton = page.locator(`[data-test="pipeline-list-${pipelineName}-delete-pipeline"]`);
    await deletePipelineButton.waitFor({ state: 'visible' });
    await deletePipelineButton.click()
    await pipelinePage.confirmDeletePipeline();
    await pipelinePage.verifyPipelineDeleted(); 
  }); 
  
  test('should create query source and delete it', async ({ page }) => {
    const pipelinePage = new PipelinePage(page);
    
    // Open the pipeline menu and wait for a moment
    await pipelinePage.openPipelineMenu();
    await page.waitForTimeout(1000);
  
    // Add a new pipeline
    await pipelinePage.addPipeline();
    
    // Drag and drop the stream to target
    await pipelinePage.dragStreamToTarget(pipelinePage.queryButton);
  
    // Select logs from the list
    await page.locator('div').filter({ hasText: /^logs$/ }).click();
    await page.getByRole('option', { name: 'logs' }).click();
  
    // Click on the editor to type the query
    await page.locator('.view-lines').first().click();
    const sqlQuery = 'select * from "default"';
  
    // Locate the editor and type the SQL query if it's not already typed
    await page.click('[data-test="scheduled-pipeline-sql-editor"]');
    await page.keyboard.type(sqlQuery);
  
    // Wait for a moment to ensure the query is properly typed
    await page.waitForTimeout(1000);
  
    // Check if the query exists in the editor (using `hasText` to ensure it's typed)
    const queryTyped = await page.locator('code').locator('div').filter({ hasText: sqlQuery }).count();
    
    // If the query is found, click the frequency unit
    if (queryTyped > 0) {
      await page.locator('[data-test="scheduled-pipeline-frequency-unit"]').click(); // Click frequency unit
    }
    await pipelinePage.saveQuery();
    await page.waitForTimeout(2000);
    await pipelinePage.deletePipeline();
    await pipelinePage.confirmDelete();
  });


  test('should add source, function,destination and then delete pipeline', async ({ page }) => {
    const pipelinePage = new PipelinePage(page);
  
    await pipelinePage.openPipelineMenu();
    await page.waitForTimeout(1000)
    await pipelinePage.addPipeline();
    await pipelinePage.selectStream();
    await pipelinePage.dragStreamToTarget(pipelinePage.streamButton);
    await pipelinePage.selectLogs();
  
    // Interact with stream name and save
    await pipelinePage.enterStreamName('e2e');
    await pipelinePage.enterStreamName('e2e_automate');
    await page.waitForTimeout(2000)
    await pipelinePage.selectStreamOption();
    await pipelinePage.saveInputNodeStream();
   await pipelinePage.selectAndDragFunction()// Function drag
    await page.waitForTimeout(2000); 
    await page.getByLabel('Select Previous Node').click();
    await page.getByText('input').click();
    await page.locator('[data-test="create-function-toggle"] div').nth(2).click();
    await page.locator('[data-test="add-function-node-routing-section"]').getByLabel('Name').click();
    await page.locator('[data-test="add-function-node-routing-section"]').getByLabel('Name').fill('automatefunc222_2')

    await page.locator('.view-lines').click();

// Type the function text with a delay to ensure each character registers
await page.keyboard.type('.a=2', { delay: 100 });
await page.keyboard.press('Enter');
await page.keyboard.type('.', { delay: 100 });
await page.getByText('Note: The function will be').click();


// Check if the required text is present in the editor
await page.getByText('.a=2 .');


// If the text is present, click the "Function Save" button
// Optional: Add a short wait to confirm the action is processed
await page.waitForTimeout(2000)

// Optional: Add a brief wait to allow any validation messages to process
await page.waitForTimeout(2000)


await page.locator('[data-test="associate-function-save-btn"]').click();

await page.waitForTimeout(4000)




    // await pipelinePage.dragStreamToTarget(pipelinePage.streamButton); // First stream drag

    // Select the second stream, drag, and drop
    // await pipelinePage.selectAndDragSecondStream();
    // await page.waitForTimeout(2000);
    
    // // Select the previous node
    //   await pipelinePage.selectPreviousNode()
    //   await pipelinePage.selectPreviousNodeDrop();
    //   await pipelinePage.toggleCreateStream();
    //   await page.getByLabel('Name *').click();
    //   await page.getByLabel('Name *').fill('destination-node');
    //   await page.locator('.q-form > div:nth-child(2) > .q-field > .q-field__inner > .q-field__control > .q-field__control-container > .q-field__native').click();
    //   await page.getByRole('option', { name: 'Logs' }).locator('div').nth(2).click();
    //   await pipelinePage.clickSaveStream();
    //   await pipelinePage.clickInputNodeStreamSave()
    //   const pipelineName = `pipeline-${Math.random().toString(36).substring(7)}`;
    //   await pipelinePage.enterPipelineName(pipelineName);
    //   await pipelinePage.savePipeline();
    //   await pipelinePage.searchPipeline(pipelineName)
    //   await page.waitForTimeout(1000);
    //   const deletePipelineButton = page.locator(`[data-test="pipeline-list-${pipelineName}-delete-pipeline"]`);
    //   await deletePipelineButton.waitFor({ state: 'visible' });
    //   await deletePipelineButton.click()
    //   await pipelinePage.confirmDeletePipeline();
    //   await pipelinePage.verifyPipelineDeleted(); 
  }); 
 
})