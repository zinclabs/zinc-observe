import { test, expect } from "../baseFixtures";
import logData from "../../cypress/fixtures/log.json";
import logsdata from "../../../test-data/logs_data.json";

const randomDashboardName =
  "Dashboard_" + Math.random().toString(36).substr(2, 9);

test.describe.configure({ mode: "parallel" });

async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"], { waitUntil: "networkidle" });
     await page.getByText('Login as internal user').click();
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

test.describe(" VRL UI testcases", () => {
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

    // just to make sure org is set
    const orgNavigation = page.goto(
      `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    );

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

    // const logsdata = {}; // Fill this with your actual data

    const fetchResponse = await fetch(
      `${process.env.INGESTION_URL}/api/${orgId}/${streamName}/_json`,
      {
        method: "POST",
        headers: headers,
        body: JSON.stringify(logsdata),
      }
    );
    const response = await fetchResponse.json();

    await orgNavigation;

    // Making a POST request using fetch API
    // const response = await page.evaluate(
    //     async ({ url, headers, orgId, streamName, logsdata }) => {
    //         const fetchResponse = await fetch(
    //             `${url}/api/${orgId}/${streamName}/_json`,
    //             {
    //                 method: "POST",
    //                 headers: headers,
    //                 body: JSON.stringify(logsdata),
    //             }
    //         );
    //         return await fetchResponse.json();
    //     },
    //     {
    //         url: process.env.INGESTION_URL,
    //         headers: headers,
    //         orgId: orgId,
    //         streamName: streamName,
    //         logsdata: logsdata,
    //     }
    // );

    console.log(response);
    //  });
    // const allorgs = page.waitForResponse("**/api/default/organizations**");
    // const functions = page.waitForResponse("**/api/default/functions**");
    //   await page.goto(
    //   `${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`
    //  );
    //  const allsearch = page.waitForResponse("**/api/default/_search**");
    //  await selectStreamAndStreamTypeForLogs(page, logData.Stream);
    //  await applyQueryButton(page);
    // const streams = page.waitForResponse("**/api/default/streams**");
  });

  test("should display the VRL function in the field list after it has been written.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubectl_kubernetes_io_default_container"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    // await page.getByText('arrow_rightQueryAutoPromQLCustom SQL').click();
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] img')
      .click();

    //  await page.locator('#fnEditor > .monaco-editor > .overflow-guard > div:nth-child(2) > .lines-content > .view-lines').click();
    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrl=123");
    await page.waitForTimeout(3000);

    await page.locator('[data-test="dashboard-apply"]').click();
    const locator = page.locator(
      '[data-test="field-list-item-logs-e2e_automate-vrl"]'
    );

    // Get the locator and ensure it is visible
    await page
      .locator('[data-test="field-list-item-logs-e2e_automate-vrl"]')
      .waitFor({ state: "visible" });
  });

  test("should be able to successfully add multiple VRL functions.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubectl_kubernetes_io_default_container"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-2-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.getByText("arrow_rightQueryAutoPromQLCustom SQL").click();
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] img')
      .click();

    //await page.locator('[data-test="dashboard-vrl-function-editor"]').getByLabel('Editor content;Press Alt+F1').fill('.percenteage1 ,err = .kubernetes_annotations_kubectl_kubernetes_io_default_container * .kubernetes_container_hash / 100\ .percenteage2 ,err = .kubernetes_annotations_kubectl_kubernetes_io_default_container / .kubernetes_container_hash * 100 \n');

    await page.waitForTimeout(2000);

    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(1000);

    // await expect(page.getByText('drag_indicatortext_fields percenteage1')).toBeVisible;
    // await expect(page.getByText('drag_indicatortext_fields percenteage2')).toBeVisible;

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-percenteage1"] [data-test="dashboard-add-y-data"]'
      )
      .click();

    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(3000);

    await page
      .locator(
        '[data-test="dashboard-b-item-kubernetes_annotations_kubectl_kubernetes_io_default_container-remove"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-percenteage2"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
  });

  test("The chart should be successfully saved by adding the VRL function.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("div")
      .filter({ hasText: /^arrow_drop_down$/ })
      .nth(2)
      .click();
    await page
      .getByRole("option", { name: "e2e_automate" })
      .locator("div")
      .nth(2)
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubectl_kubernetes_io_default_container"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(100);

    await page.getByText("arrow_rightQueryAutoPromQLCustom SQL").click();
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] img')
      .click();

    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill(
        ".percentage , err=.histogram *.kubernetes_annotations_kubectl_kubernetes_io_default_container/ 100"
      );
    await page.waitForTimeout(3000);

    await page.locator('[data-test="dashboard-apply"]').click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-percentage"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page
      .locator('[data-test="dashboard-panel-name"]')
      .fill("VRL_Dashboard");
    await page.locator('[data-test="dashboard-panel-save"]').click();
  });

  test("The VRL function query should not vanish after changing from custom SQL mode to auto mode.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubectl_kubernetes_io_default_container"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-y-data"]'
      )
      .click();

    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] img')
      .click();

    //  await page.getByText('arrow_rightQueryAutoPromQLCustom SQL').click();
    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".percentage , err = .histogram*.kubernetes_container_hash/100");
    await page.waitForTimeout(3000);

    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(200);
    //  await page.locator('[data-test="chart-renderer"] canvas').click
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-percentage"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-customSql"]').click();
    await page.locator('[data-test="dashboard-auto"]').click();
    await expect(
      page.getByText(
        "Are you sure you want to change the query mode? The data saved for X-Axis, Y-Axis and Filters will be wiped off."
      )
    ).toBeVisible();

    await page.locator('[data-test="confirm-button"]').click();

    await page
      .getByText(".percentage , err = .histogram*.kubernetes_container_hash/")
      .click();
    await expect(
      page.getByText(
        ".percentage , err = .histogram*.kubernetes_container_hash/"
      )
    ).toBeVisible();
  });

  test("should display an error message when changing the VRL function if the existing VRL function is not updated or changed", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubectl_kubernetes_io_default_container"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="datetime-timezone-select"]').click();
    await page.getByText("Asia/Dhaka").click();

    await page.locator('[data-test="dashboard-apply"]').click();
    //  await page.getByText('arrow_rightQueryAutoPromQLCustom SQL').click();
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] img')
      .click();

    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill('.vrl="Hello"');
    await page.waitForTimeout(2000);

    await page.locator('[data-test="dashboard-apply"]').click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-y-data"]'
      )
      .click();

    //  await page.locator('[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-y-data"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.getByText('.vrl="Hello"').click();
    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .press("ArrowLeft");
    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .press("ArrowLeft");
    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill('.Test="Hello"');
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.waitForTimeout(2000);

    //await page.getByText('drag_indicatortext_fields test').click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-test"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    //await page.locator('#q-notify').getByRole('button').click();

    await page.getByText(
      "There are some errors, please fix them and try again"
    );
    // await page.locator('#q-notify').getByRole('button').click();

    await page.waitForTimeout(1000);

    //await expect(page.getByText('Please update Y-Axis Selection. Current Y-Axis field as is invalid for selected stream')).toBeVisible();

    await page.locator('[data-test="dashboard-y-item-vrl-remove"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("VRL");
    await page.locator('[data-test="dashboard-panel-save"]').click();
    await page
      .locator('[data-test="dashboard-edit-panel-VRL-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("should display an error message if an invalid VRL function is added.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] img')
      .click();
    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".VRL=Hello");
    await page.locator('[data-test="dashboard-apply"]').click();

    await expect(
      page
        .getByRole("list")
        .getByText(
          'Function error: error[E202]: syntax error ┌─ :1:1 │ 1 │ ╭ .VRL="Hello 2 │ │ . │ ╰──^ unexpected error: invalid string literal │ = see language documentation at https://vrl.dev = try your code in the VRL REPL, learn more at https://vrl.dev/examples'
        )
    ).toBeVisible();

    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill('.VRL="Hello"');
    await page.locator('[data-test="dashboard-apply"]').click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("VRL");
    await page.locator('[data-test="dashboard-panel-save"]').click();
  });

  test("function should not disappear when changing the chart type.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubectl_kubernetes_io_default_container"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_image"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] img')
      .click();

    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".Vrl=123");
    await page.locator('[data-test="dashboard-apply"]').click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="selected-chart-line-item"] img').click();
    await expect(
      page
        .locator("div")
        .filter({ hasText: /^\.Vrl=123$/ })
        .nth(2)
    ).toBeVisible();
    await page.locator('[data-test="selected-chart-area-item"] img').click();
    await expect(
      page
        .locator("div")
        .filter({ hasText: /^\.Vrl=123$/ })
        .nth(2)
    ).toBeVisible();
    await expect(
      page
        .locator("div")
        .filter({ hasText: /^\.Vrl=123$/ })
        .nth(2)
    ).toBeVisible();
    page.once("dialog", (dialog) => {
      console.log(`Dialog message: ${dialog.message()}`);
      dialog.dismiss().catch(() => {});
    });
    await page.locator('[data-test="dashboard-panel-discard"]').click();
  });

  test("should not show an error if the VRL is null on the older panel.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubectl_kubernetes_io_default_container"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_hash"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page
      .locator('[data-test="dashboard-panel-name"]')
      .fill("Dashboared ");
    await page.locator('[data-test="dashboard-panel-save"]').click();
    await page
      .locator('[data-test="dashboard-edit-panel-Dashboared -dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-edit-panel"]').click();
    await page.getByText("arrow_rightQueryAutoPromQLCustom SQL").click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
  });

  test("should display the VRL filed in table chart", async ({ page }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page.locator('[data-test="selected-chart-table-item"] img').click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] div')
      .nth(2)
      .click();
    await page
      .locator(
        "#fnEditor > .monaco-editor > .overflow-guard > div:nth-child(2) > .lines-content > .view-lines > .view-line"
      )
      .click();
    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrl ,err =.y_axis_1/.y_axis_2*100");
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="date-time-btn"]').click();

    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-x-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await expect(
      page
        .locator('[data-test="dashboard-panel-table"]')
        .getByRole("cell", { name: "Vrl" })
    ).toBeVisible();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("VRL");
    await page.locator('[data-test="dashboard-panel-save"]').click();
  });

  test("should able to select the VRL from saved Function list ", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();

    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] img')
      .click();
    await page
      .locator('[data-test="dashboard-query"]')
      .getByText("arrow_drop_down")
      .click();
    await page
      .locator('[data-test="dashboard-use-saved-vrl-function"]')
      .fill("VRL_01");
    await page.getByText("VRL_01").click();

    await expect(page.getByText("VRL_01 function applied")).toBeVisible();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await expect(
      page.getByText("drag_indicatortext_fields percentage")
    ).toBeVisible();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-percentage"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("VRL");
    await page.locator('[data-test="dashboard-panel-save"]').click();
    await page
      .locator('[data-test="dashboard-edit-panel-VRL-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("should be  VRL function is  updated in the function editor when a different function is selected from the saved function list.", async ({
    page,
  }) => {
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] div')
      .nth(2)
      .click();
    await page
      .locator('[data-test="dashboard-query"]')
      .getByText("arrow_drop_down")
      .click();
    await page.getByText("vrltest").click();
    await expect(page.getByText("vrltest function applied")).toBeVisible();
    await expect(page.getByText(".TEst=")).toBeVisible();
    await page
      .locator('[data-test="dashboard-query"]')
      .getByText("arrow_drop_down")
      .click();
    await page.getByText("VRL_01").click();
    await expect(page.getByText("VRL_01 function applied")).toBeVisible();
    await expect(page.getByText(".percentage, err=.y_axis_1 /")).toBeVisible();
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("VRL");
    await page.locator('[data-test="dashboard-panel-save"]').click();
    await page
      .locator('[data-test="dashboard-edit-panel-VRL-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await page.locator('[data-test="confirm-button"]').click();
  });
  test("should update the VRL function in the function editor when a different function is selected from the saved function list", async ({
    page,
  }) => {
    // Navigate to the Dashboards page
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Add a new dashboard
    const randomDashboardName = "Test Dashboard";
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add a new panel to the dashboard
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();

    // Set the date-time filter
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    // Apply a VRL function and verify
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] div')
      .nth(2)
      .click();
    await page
      .locator('[data-test="dashboard-query"]')
      .getByText("arrow_drop_down")
      .click();

    // Select the VRL function from the dropdown using the XPath approach
    await page
      .getByRole("option", { name: "vrltest" })
      .locator("div")
      .nth(2)
      .click();

    // Verify the VRL function is applied
    const vrlTestAppliedText = await page
      .locator("text=vrltest function applied")
      .textContent();
    expect(vrlTestAppliedText).toContain("vrltest function applied");

    const vrlTestText = await page.locator("text=.TEst=").textContent();
    expect(vrlTestText).toContain(".TEst=");

    // // Change the VRL function and verify
    // await page.locator('[data-test="dashboard-query"]').getByText('arrow_drop_down').click();

    // // Select the next VRL function using the XPath approach
    // await page.getByRole('option', { name: 'VRL_01' }).locator('div').nth(2).click();

    // // Verify the new VRL function is applied
    // const vrl01AppliedText = await page.locator('text=VRL_01 function applied').textContent();
    // expect(vrl01AppliedText).toContain('VRL_01 function applied');

    // const vrl01Text = await page.locator('text=.percentage, err=.y_axis_1 /').textContent();
    // expect(vrl01Text).toContain('.percentage, err=.y_axis_1 /');

    // Save the dashboard panel
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page.locator('[data-test="dashboard-panel-name"]').fill("VRL");
    await page.locator('[data-test="dashboard-panel-save"]').click();

    // Delete the dashboard panel
    await page
      .locator('[data-test="dashboard-edit-panel-VRL-dropdown"]')
      .click();
    await page.locator('[data-test="dashboard-delete-panel"]').click();
    await page.locator('[data-test="confirm-button"]').click();
  });

  test("should display the VRL function field when changing the chart type", async ({
    page,
  }) => {
    // Navigate to dashboards page
    await page.locator('[data-test="menu-link-\\/dashboards-item"]').click();
    await waitForDashboardPage(page);

    // Create a new dashboard
    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').click();
    await page
      .locator('[data-test="add-dashboard-name"]')
      .fill(randomDashboardName);
    await page.locator('[data-test="dashboard-add-submit"]').click();

    // Add a new panel
    await page
      .locator('[data-test="dashboard-if-no-panel-add-panel-btn"]')
      .click();
    await page
      .locator("label")
      .filter({ hasText: "Streamarrow_drop_down" })
      .locator("i")
      .click();
    await page.getByText("e2e_automate").click();

    // Set up Y-axis data
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_annotations_kubernetes_io_psp"] [data-test="dashboard-add-y-data"]'
      )
      .click();
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-kubernetes_container_name"] [data-test="dashboard-add-y-data"]'
      )
      .click();

    // Change to custom SQL and add VRL function
    await page.getByText("arrow_rightQueryAutoPromQLCustom SQL").click();
    await page
      .locator('[data-test="logs-search-bar-show-query-toggle-btn"] img')
      .click();
    await page
      .locator(
        "#fnEditor > .monaco-editor > .overflow-guard > .monaco-scrollable-element > .lines-content > .view-lines > .view-line"
      )
      .click();
    await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .fill(".vrl=100");
    await page.locator('[data-test="dashboard-apply"]').click();

    // Set date and time range
    await page.locator('[data-test="date-time-btn"]').click();
    await page.locator('[data-test="date-time-relative-6-w-btn"]').click();
    await page.locator('[data-test="dashboard-apply"]').click();

    // Add additional Y-axis data
    await page
      .locator(
        '[data-test="field-list-item-logs-e2e_automate-vrl"] [data-test="dashboard-add-b-data"]'
      )
      .click();
    await page.locator('[data-test="dashboard-apply"]').click();

    // Verify VRL function field visibility after changing chart types
    await page.locator('[data-test="selected-chart-area-item"]').click();
    await expect(
      page.locator('[data-test="dashboard-b-item-vrl"]')
    ).toBeVisible();

    await page.locator(".q-list > div:nth-child(2)").click();
    await expect(
      page.locator('[data-test="dashboard-b-item-vrl"]')
    ).toBeVisible();

    await page.locator("div:nth-child(4)").first().click();
    await expect(
      page.locator('[data-test="dashboard-b-item-vrl"]')
    ).toBeVisible();

    await page.locator('[data-test="selected-chart-scatter-item"]').click();
    await expect(
      page.locator('[data-test="dashboard-b-item-vrl"]')
    ).toBeVisible();

    await page.locator('[data-test="selected-chart-h-stacked-item"]').click();
    await expect(
      page.locator('[data-test="dashboard-b-item-vrl"]')
    ).toBeVisible();

    // Verify VRL function content is correct
    const vrlFunctionContent = await page
      .locator('[data-test="dashboard-vrl-function-editor"]')
      .getByLabel("Editor content;Press Alt+F1")
      .inputValue();
    expect(vrlFunctionContent).toBe(".vrl=100");

    // Save the dashboard panel
    await page.locator('[data-test="dashboard-panel-name"]').click();
    await page
      .locator('[data-test="dashboard-panel-name"]')
      .fill("VRL_Dashboard");
    await page.locator('[data-test="dashboard-panel-save"]').click();
  });


});
