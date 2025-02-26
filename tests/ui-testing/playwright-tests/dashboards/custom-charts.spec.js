import { test, expect } from "../baseFixtures";
import path from "path";
import fs from "fs";
import logsdata from "../../../test-data/logs_data.json";
import { DashboardPage } from '../../pages/dashboardPage';

// Function to handle login
async function login(page) {

  await page.goto(process.env["ZO_BASE_URL"]);
  if (await page.getByText("Login as internal user").isVisible()) {
    await page.getByText("Login as internal user").click();
  }
  console.log("ZO_BASE_URL", process.env["ZO_BASE_URL"]);
  await page.waitForTimeout(1000);

  await page.locator('[data-cy="login-user-id"]').fill(process.env["ZO_ROOT_USER_EMAIL"]);
  await page.locator('[data-cy="login-password"]').fill(process.env["ZO_ROOT_USER_PASSWORD"]);
  await page.locator('[data-cy="login-sign-in"]').click();
  await page.waitForTimeout(4000);
  await page.goto(process.env["ZO_BASE_URL"]);
}

async function ingestion(page) {
  const orgId = process.env["ORGNAME"];
  const streamName = "e2e_automate";
  const basicAuthCredentials = Buffer.from(
    `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
  ).toString("base64");

  const headers = {
    "Authorization": `Basic ${basicAuthCredentials}`,
    "Content-Type": "application/json",
  };
  const response = await page.evaluate(async ({ url, headers, orgId, streamName, logsdata }) => {
    const fetchResponse = await fetch(`${url}/api/${orgId}/${streamName}/_json`, {
      method: "POST",
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

// Read JSON test files
function readJsonFile(filename) {
  const filePath = path.join(__dirname, `../../../test-data/${filename}`);
  if (!fs.existsSync(filePath)) {
    console.error(`Error: JSON file does not exist at: ${filePath}`);
    return null;
  }
  return fs.readFileSync(filePath, "utf8");
}

test.describe("Sanity Tests", () => { let dashboardPage
  let pictorialJSON, lineJSON;

  test.beforeEach(async ({ page }) => {
  
    dashboardPage = new DashboardPage(page);
    
});

  test.beforeAll(() => {
    
    const jsonString = readJsonFile("pictorial.json");
    pictorialJSON = jsonString
    lineJSON = readJsonFile("line.json");
  });

  test.beforeEach(async ({ page }) => {
  
    await login(page);
   
    
    await page.waitForTimeout(1000);
    await ingestion(page);
  });

  test("Add Pictorial JSON in Monaco Editor", async ({ page }) => {
    if (!pictorialJSON) {
      console.error("Skipping test: pictorial.json not found");
      return;
    }
    await dashboardPage.addCustomChart();
  // Type the content with raw modifier to bypass autocomplete
  await page.keyboard.insertText(pictorialJSON);
  
  await page.waitForTimeout(1000);
  await page.locator('[data-test="dashboard-panel-error-bar-icon"]').click();
  await page.locator('[data-test="dashboard-panel-query-editor"]').getByRole("textbox", { name: "Editor content;Press Alt+F1" }).fill('select * from "e2e_automate"');
  await page.locator('[data-test="dashboard-apply"]').click();
  await page.waitForTimeout(3000);
  await expect(page.getByText("Unsafe code detected: Access to 'document' is not allowed")).toBeVisible();

  
});

test("Add line JSON in Monaco Editor", async ({ page }) => {
    if (!lineJSON) {
      console.error("Skipping test: pictorial.json not found");
      return;
    }
    await dashboardPage.addCustomChart();
  // Type the content with raw modifier to bypass autocomplete
  await page.keyboard.insertText(lineJSON);
  
  await page.waitForTimeout(1000);
  await page.locator('[data-test="dashboard-panel-error-bar-icon"]').click();
  await page.locator('[data-test="dashboard-panel-query-editor"]').getByRole("textbox", { name: "Editor content;Press Alt+F1" }).fill('select * from "e2e_automate"');
  await page.waitForTimeout(3000);
  await page.locator('[data-test="dashboard-apply"]').click();
  await page.waitForTimeout(3000);
  // await expect(page.getByText("Unsafe code detected: Access to 'document' is not allowed")).toBeVisible();

  
});

});
