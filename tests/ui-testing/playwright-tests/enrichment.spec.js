import { test, expect } from "./baseFixtures";
import logData from "../../ui-testing/cypress/fixtures/log.json";
// import { log } from "console";
import logsdata from "../../test-data/logs_data.json";
import PipelinePage from "../pages/pipelinePage";
// import { pipeline } from "stream";
// import fs from "fs";
import { v4 as uuidv4 } from "uuid";

test.describe.configure({ mode: "parallel" });

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
  await page.locator("label").filter({ hasText: "Password *" }).click();
  await page
    .locator('[data-cy="login-password"]')
    .fill(process.env["ZO_ROOT_USER_PASSWORD"]);
  await page.locator('[data-cy="login-sign-in"]').click();
  //     await page.waitForTimeout(4000);
  // await page.goto(process.env["ZO_BASE_URL"]);
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


test.describe("Enrichment data testcases", () => {
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
    await page.waitForTimeout(5000);
    

    // ("ingests logs via API", () => {
    const orgId = process.env["ORGNAME"];
    const streamName = "e2e_automate";
    const basicAuthCredentials = Buffer.from(
      `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
    ).toString("base64");

    const headers = {
      Authorization: `Basic ${basicAuthCredentials}`,
      "Content-Type": "application/json",
    };

    await ingestion(page);
    await page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );
    const allsearch = page.waitForResponse("**/api/default/_search**");
    await selectStreamAndStreamTypeForLogs(page, logData.Stream);
    await applyQueryButton(page);
  });



  test("should upload an enrichment table under functions", async ({
    page,
  }) => {
    const pipelinePage = new PipelinePage(page);
    await pipelinePage.navigateToAddEnrichmentTable();
  
    // Generate a unique file name and replace hyphens with underscores
    let fileName = `enrichment_info_${uuidv4()}_csv`.replace(/-/g, "_");
    console.log("Generated File Name:", fileName);

    // Set the file to be uploaded
    const fileContentPath = "../test-data/enrichment_info.csv";
    const inputFile = await page.locator('input[type="file"]');
    await inputFile.setInputFiles(fileContentPath);

    // Enter the file name
    await page.fill(".q-input > .q-field__inner > .q-field__control", fileName);

    // Click on 'Save'
    await page.getByText("Save").click({ force: true });

    // Wait for the process to complete
    await page.reload();
    await page.waitForTimeout(3000);
    await page.getByPlaceholder("Search Enrichment Table").fill(fileName);
    await page.waitForTimeout(3000);
    // Explore the uploaded table
    await page.getByRole("button", { name: "Explore" }).click();
    await page.locator('[data-test="log-table-column-0-_timestamp"]').click();
    await page.locator('[data-test="close-dialog"]').click();
    await page.waitForTimeout(3000);

    // Return to the pipeline menu and verify the uploaded table
    await page.locator('[data-test="menu-link-\\/pipeline-item"]').click();
    await page.locator('[data-test="function-enrichment-table-tab"]').click();

    // Verify and delete enrichment tables
    await pipelinePage.deleteEnrichmentTableByName(fileName)
  });

  test.skip("should upload an enrichment table under functions with VRL", async ({
    page,
  }) => {
    const pipelinePage = new PipelinePage(page);
    await pipelinePage.navigateToAddEnrichmentTable();

    // Generate a unique file name
    let fileName = `protocols_${uuidv4()}_csv`;
    console.log("Generated File Name:", fileName);
    fileName = fileName.replace(/-/g, "_");

    // Set the file to be uploaded
    const fileContentPath = "../test-data/protocols.csv";
    const inputFile = await page.locator('input[type="file"]');
    await inputFile.setInputFiles(fileContentPath);

    // Enter the file name
    await page.fill(".q-input > .q-field__inner > .q-field__control", fileName);

    // Click on 'Save'
    await page.getByText("Save").click({ force: true });

    // Reload the page to ensure table is updated
    await page.reload();
    await page.waitForTimeout(3000);

    // Search for the uploaded file name
    await page.getByPlaceholder("Search Enrichment Table").fill(fileName);
    await page.waitForTimeout(3000);

    // Verify the file name in the enrichment table
    const rows = await page.locator("tbody tr");
    let fileFound = false;

    for (let i = 0; i < (await rows.count()); i++) {
      const row = rows.nth(i);
      const displayedName = await row
        .locator("td.text-left")
        .nth(1)
        .textContent();
      if (displayedName?.trim() === fileName) {
        fileFound = true;
        console.log("File found in table:", displayedName);
        break;
      }
    }

    if (!fileFound) {
      throw new Error(`File name "${fileName}" not found in the table.`);
    }

    // Explore the file
    await page.getByRole("button", { name: "Explore" }).click();
    await page.locator("#fnEditor").getByLabel("Editor content;Press Alt+F1")
      .fill(`
abc, err = get_enrichment_table_record("${fileName}", {
  "protocol_number": to_string!(.protocol_number)
})
.protocol_keyword = abc.keyword
`);

    // Wait briefly to allow input processing
    await page.waitForTimeout(1000);

    // Apply the query
    await applyQueryButton(page);

    // Verify that no warning is shown for query execution
    const warningElement = page.locator("text=warning Query execution");
    await expect(warningElement).toBeHidden();

    // Expand the first row in the logs table
    await page.waitForTimeout(3000);
    await page
      .locator(
        '[data-test="log-table-column-0-_timestamp"] [data-test="table-row-expand-menu"]'
      )
      .click();
      await page.waitForTimeout(1000);
    await page
      .locator('[data-test="log-expand-detail-key-protocol_keyword-text"]')
      .click();
    // Return to the pipeline menu and delete the uploaded table
    await page.locator('[data-test="menu-link-\\/pipeline-item"]').click();
    await page.locator('[data-test="function-enrichment-table-tab"]').click();
    await pipelinePage.deleteEnrichmentTableByName(fileName)

  });


  test("should display error when CSV not added in enrichment table", async ({
    page,
  }) => {
    const pipelinePage = new PipelinePage(page);
    await pipelinePage.navigateToAddEnrichmentTable();
    await page.getByLabel('Name').fill('test')
  
    await page.getByRole('button', { name: 'Save' }).click();
    await page.getByText('CSV File is required!').click();
  })


  test("should append an enrichment table under functions", async ({ page }) => {
    const pipelinePage = new PipelinePage(page);
    await pipelinePage.navigateToAddEnrichmentTable();
  
    // Generate a unique file name and replace hyphens with underscores
    let fileName = `append_${uuidv4()}_csv`.replace(/-/g, "_");
    console.log("Generated File Name:", fileName);
  
    // First upload step
    const fileContentPath = "../test-data/append.csv";
    const inputFile = await page.locator('input[type="file"]');
    await inputFile.setInputFiles(fileContentPath);
  
    // Enter the file name
    await page.fill(".q-input > .q-field__inner > .q-field__control", fileName);
  
    // Click on 'Save'
    await page.getByText("Save").click({ force: true });
  
    // Wait for the process to complete
    await page.reload();
    await page.waitForTimeout(3000);
    await page.getByPlaceholder("Search Enrichment Table").fill(fileName);
    await page.waitForTimeout(3000);
  
    // Explore the uploaded table
    await page.getByRole("button", { name: "Explore" }).click();
    await page.locator('[data-test="log-table-column-0-_timestamp"]').click();
    await page.locator('[data-test="close-dialog"]').click();
    await page.waitForTimeout(3000);
  
    // Navigate to append data to the existing enrichment table
    await page.locator('[data-test="menu-link-\\/pipeline-item"]').click();
    await page.locator('[data-test="function-enrichment-table-tab"]').click();
    await page.getByPlaceholder("Search Enrichment Table").fill(fileName);
    await page.getByRole("button", { name: "Enrichment Tables" }).click();
  
    // Select the append toggle
    await page.getByLabel("Append data to existing").locator("div").first().click();
 
  
    // Upload the CSV again for appending
    await inputFile.setInputFiles(fileContentPath);
    await page.waitForTimeout(3000);
    await page.getByRole('button', { name: 'Save' }).click();
    await page.waitForTimeout(3000);
    await page.getByPlaceholder("Search Enrichment Table").fill(fileName);
    await page.waitForTimeout(3000);
    // Explore the uploaded table
    await page.getByRole("button", { name: "Explore" }).click();

    const showingText = page.getByText('Showing 1 to 2 out of 2');
  await expect(showingText).toBeVisible()

  await page.locator('[data-test="menu-link-\\/pipeline-item"]').click();
  await page.locator('[data-test="function-enrichment-table-tab"]').click();
  await page.getByPlaceholder("Search Enrichment Table").fill(fileName);
  await pipelinePage.deleteEnrichmentTableByName(fileName)
   });
  
});
