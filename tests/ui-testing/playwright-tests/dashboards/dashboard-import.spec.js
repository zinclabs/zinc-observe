import { test, expect } from "../baseFixtures";
import logData from "../../cypress/fixtures/log.json";
import logsdata from "../../../test-data/logs_data.json";
import importdata from "../../../test-data/dashboards-import.json";
import importdata1 from "../../../test-data/dashboard1-import.json";


const randomDashboardName =
  "Dashboard_" + Math.random().toString(36).substr(2, 9);

test.describe.configure({ mode: "parallel" });

async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"], { waitUntil: "networkidle" });

  if (await page.getByText("Login as internal user").isVisible()) {
    await page.getByText("Login as internal user").click();
  }

  await page.waitForTimeout(1000);
  await page
    .locator('[data-cy="login-user-id"]')
    .fill(process.env["ZO_ROOT_USER_EMAIL"]);

  // wait for login api response
  const waitForLogin = page.waitForResponse(
    (response) =>
      response.url().includes("/auth/login") && response.status() === 200
  );

  await page
    .locator('[data-cy="login-password"]')
    .fill(process.env["ZO_ROOT_USER_PASSWORD"]);
  await page.locator('[data-cy="login-sign-in"]').click();

  await waitForLogin;

  await page.waitForURL(process.env["ZO_BASE_URL"] + "/web/", {
    waitUntil: "networkidle",
  });
  await page
    .locator('[data-test="navbar-organizations-select"]')
    .getByText("arrow_drop_down")
    .click();
  await page.getByRole("option", { name: "default", exact: true }).click();
}

async function ingestion(page) {
  const orgId = process.env["ORGNAME"];
  const streamName = "e2e_automate";
  const basicAuthCredentials = Buffer.from(
    `${process.env["ZO_ROOT_USER_EMAIL"]}:${process.env["ZO_ROOT_USER_PASSWORD"]}`
  ).toString("base64");

  const headers = {
    Authorization: `Basic ${basicAuthCredentials}`,
    "Content-Type": "application/json",
  };
  const fetchResponse = await fetch(
    `${process.env.INGESTION_URL}/api/${orgId}/${streamName}/_json`,
    {
      method: "POST",
      headers: headers,
      body: JSON.stringify(logsdata),
    }
  );
  const response = await fetchResponse.json();
  console.log(response);
}

async function waitForDashboardPage(page) {
  const dashboardListApi = page.waitForResponse(
    (response) =>
      /\/api\/.+\/dashboards/.test(response.url()) && response.status() === 200
  );

  await page.waitForURL(process.env["ZO_BASE_URL"] + "/web/dashboards**");

  await page.waitForSelector(`text="Please wait while loading dashboards..."`, {
    state: "hidden",
  });
  await dashboardListApi;
  await page.waitForTimeout(500);
}

test.describe("dashboard UI testcases", () => {
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
    console.log("running before each");
    await login(page);
    await page.waitForTimeout(1000);
    await ingestion(page);
    await page.waitForTimeout(2000);

    // just to make sure org is set
    const orgNavigation = page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );

    await orgNavigation;
  });

  test("should import the dashbaord", async ({ page }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-import"]').click();

    //file name to be used for import
    const fileContentPath = "../test-data/dashboards-import.json";

    // Locate the file input field and set the JSON file
    const inputFile = await page.locator('input[type="file"]');
    //is used for setting the file to be imported
    await inputFile.setInputFiles(fileContentPath);

    await page.getByRole("button", { name: "Import" }).click();

    await page.waitForTimeout(2000);

    await expect(
      page.getByRole("cell", { name: "Cloudfront to OpenObserve" }).first()
    ).toBeVisible();
    await page
      .getByRole("row", { name: "01 Cloudfront to OpenObserve" })
      .locator('[data-test="dashboard-delete"]')
      .click();
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("Should cancel the file upload action when clicking the Cancel button in the 'Drop your file' dialog", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-import"]').click();

    //file name to be used for import
    const fileContentPath = "../test-data/dashboards-import.json";

    // Locate the file input field and set the JSON file
    const inputFile = await page.locator('input[type="file"]');
    //is used for setting the file to be imported
    await inputFile.setInputFiles(fileContentPath);

    await page.getByText("close").click();
    await expect(page.getByLabel("cloud_uploadDrop your file")).toBeVisible();
    await page.getByRole("button", { name: "Import" }).click();
    await expect(page.getByText("Please Enter a JSON object")).toBeVisible();
    await page.getByRole("button", { name: "Cancel" }).click();
  });

  test("Side scrollsbdhser", async ({ page }) => {
    ///////////////////////// NOt working
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-import"]').click();

    //file name to be used for import
    const fileContentPath = "../test-data/dashboards-import.json";

    // Locate the file input field and set the JSON file
    const inputFile = await page.locator('input[type="file"]');
    //is used for setting the file to be imported
    await inputFile.setInputFiles(fileContentPath);

    await page.locator(".visible > .slider").click();
    await page.keyboard.press("ArrowDown"); // Scrolls down
  });

  test("Verify that the .json data is editable in the UI file editor.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-import"]').click();

    //file name to be used for import
    const fileContentPath = "../test-data/dashboards-import.json";

    // Locate the file input field and set the JSON file
    const inputFile = await page.locator('input[type="file"]');
    //is used for setting the file to be imported
    await inputFile.setInputFiles(fileContentPath);

    await page.getByText('"Cloudfront Distribution to').click();
    await page
      .getByLabel("Editor content;Press Alt+F1")
      .fill(
        '[\n  {\n    "version": 5,\n    "dashboardId": "7300465175298072125",\n    "title": "Cloudfront to OpenObserve",\n    "description": "Cloudfront Distribution test to Kinesis Streams to Amazon Data Firehose to OpenObserve",\n    "role": "",\n    "owner": "",\n    "created": "2025-02-26T10:45:04.098Z",\n'
      );

    await page.getByRole("button", { name: "Import" }).click();

    await expect(
      page.getByRole("cell", { name: "Cloudfront to OpenObserve" }).first()
    ).toBeVisible();
    await page
      .getByRole("row", { name: "01 Cloudfront to OpenObserve" })
      .locator('[data-test="dashboard-delete"]')
      .click();
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("Should display an error validation message when clicking the Import button if the 'Title' field is missing in the .json data.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-import"]').click();

    //file name to be used for import
    const fileContentPath = "../test-data/dashboard1-import.json";

    // Locate the file input field and set the JSON file
    const inputFile = await page.locator('input[type="file"]');
    //is used for setting the file to be imported
    await inputFile.setInputFiles(fileContentPath);

    await page.getByRole("button", { name: "Import" }).click();

    await expect(page.getByText("Title is required for")).toBeVisible();
  });

  test("Should auto-update the .json data at the correct location when the Dashboard title is added from the error validation dialog.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-import"]').click();

    //file name to be used for import
    const fileContentPath = "../test-data/dashboard1-import.json";

    // Locate the file input field and set the JSON file
    const inputFile = await page.locator('input[type="file"]');
    //is used for setting the file to be imported
    await inputFile.setInputFiles(fileContentPath);

    await page.getByRole("button", { name: "Import" }).click();

    // Assert that the error message is displayed

    await expect(page.getByText("Title is required for")).toBeVisible();

    await page.getByLabel("Dashboard Title").click();

    const title = "Cloudfront to OpenObserve";

    // Fill in the dashboard title
    await page.getByLabel("Dashboard Title").fill(title);

    // Assert that the title is correctly displayed on the UI
    await expect(page.getByText(`"${title}"`)).toBeVisible();

    // Fetch the title from the JSON data
    let jsonTitle = await page
      .getByText("Cloudfront to OpenObserve")
      .innerText();

    // need to remove double quote, before comparing
    jsonTitle = jsonTitle?.substring?.(1, jsonTitle?.length - 1);

    // Normalize: Trim spaces, remove newlines, and replace multiple spaces with a single space
    const normalizedJsonTitle = jsonTitle.trim().replace(/\s+/g, " ");

    expect(normalizedJsonTitle).toBe(title);

    await page.getByRole("button", { name: "Import" }).click();
    
  //  delete the dashboard
    await page
      .getByRole("row", { name: "01 Cloudfront to OpenObserve" })
      .locator('[data-test="dashboard-delete"]')
      .click();
    await page.locator('[data-test="confirm-button"]').click();


  });



  test("Should display an error validation message if the 'Stream type' field is missing in the .json data when clicking the Import button.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-import"]').click();

    //file name to be used for import
    const fileContentPath = "../test-data/dashboard2-import.json";

    // Locate the file input field and set the JSON file
    const inputFile = await page.locator('input[type="file"]');
    //is used for setting the file to be imported
    await inputFile.setInputFiles(fileContentPath);

    await page.getByRole("button", { name: "Import" }).click();

    await expect(page.getByText('Dashboard(s) Failed to Import')).toBeVisible();
    await expect(page.getByText('Cloudfront to OpenObserve.dashboard.json : AxiosError: Request failed with status code')).toBeVisible();


  });

});
