import { test, expect } from "../baseFixtures";
import path from "path";
import fs from "fs";
import logsdata from "../../../test-data/logs_data.json";

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

test.describe("Sanity Tests", () => {
  let pictorialJSON, lineJSON;

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
    
    await page.locator('[data-test="menu-link-\/dashboards-item"]').click();
    await page.waitForTimeout(3000);

    await page.locator('[data-test="dashboard-add"]').click();
    await page.locator('[data-test="add-dashboard-name"]').fill("Customcharts");
    await page.locator('[data-test="dashboard-add-submit"]').click();
    await page.waitForTimeout(3000);

    await page.locator('[data-test="dashboard-if-no-panel-add-panel-btn"]').click();
    await page.waitForTimeout(3000);
    await page.locator('[data-test="selected-chart-custom_chart-item"]').click();

    await page.locator(".view-lines").first().click();
    await page.waitForTimeout(500);

    await page.keyboard.press("Control+A");
    await page.keyboard.press("Backspace");
    await page.waitForTimeout(500);
    console.log("Pictorial JSON", pictorialJSON)
//     const pictorialChart = `
//     document.cookie
//     option ={
//   "title": {
//     "text": "Container Restart Count with Pictorial Bar Chart",
//     "subtext": "Using Images as Bars",
//     "left": "center"
//   },
//   "tooltip": {
//     "trigger": "item",
//     "formatter": "<img src=x onerror=alert('XSS')>"
//   },
//   "xAxis": {
//     "type": "category",
//     "data": ["node1", "node2", "node3"],
//     "axisLine": {
//       "show": false
//     },
//     "axisTick": {
//       "show": false
//     }
//   },
//   "yAxis": {
//     "type": "value",
//     "axisLine": {
//       "show": false
//     },
//     "axisTick": {
//       "show": false
//     }
//   },
//   "series": [
//     {
//       "type": "pictorialBar",
//       "symbol": "image://https://via.placeholder.com/100",
//       "symbolSize": [50, 30],
//       "data": [
//         { "value": 5, "name": "node1" },
//         { "value": 3, "name": "node2" },
//         { "value": 7, "name": "node3" }
//       ],
//       "label": {
//         "show": true,
//         "position": "top",
//         "formatter": "{c}"
//       },
//       "itemStyle": {
//         "normal": {
//           "opacity": 0.7
//         }
//       }
//     },
//     {
//       "type": "bar",
//       "barWidth": 20,
//       "data": [5, 3, 7],
//       "itemStyle": {
//         "normal": {
//           "color": "rgba(255, 140, 0, 0.5)"
//         }
//       }
//     }
//   ]
// }
// `
// ;


  // First clear any existing content
  await page.locator('[data-test="dashboard-markdown-editor-query-editor"]').click();
  await page.keyboard.press('Control+A');
  await page.keyboard.press('Delete');

  // Type the content with raw modifier to bypass autocomplete
  await page.keyboard.insertText(pictorialJSON);
  
  await page.waitForTimeout(1000);

  await page.locator('[data-test="dashboard-panel-error-bar-icon"]').click();
  await page.locator('[data-test="dashboard-panel-query-editor"]').getByRole("textbox", { name: "Editor content;Press Alt+F1" }).fill('select * from "e2e_automate"');
  
  await page.locator('[data-test="dashboard-apply"]').click();
  await page.waitForTimeout(3000);
  // await page.getByText('arrow_drop_downErrors (1)').click();
  await page.getByText('Unsafe code detected').click();
});

//   test("Add Line JSON in Monaco Editor", async ({ page }) => {
//     if (!lineJSON) {
//       console.error("Skipping test: line.json not found");
//       return;
//     }
    
//     await page.locator('[data-test="menu-link-\/dashboards-item"]').click();
//     await page.waitForTimeout(3000);

//     await page.locator('[data-test="dashboard-add"]').click();
//     await page.locator('[data-test="add-dashboard-name"]').fill("LineChart");
//     await page.locator('[data-test="dashboard-add-submit"]').click();
//     await page.waitForTimeout(3000);

//     await page.locator('[data-test="dashboard-if-no-panel-add-panel-btn"]').click();
//     await page.waitForTimeout(3000);
//     await page.locator('[data-test="selected-chart-custom_chart-item"]').click();

//     await page.locator(".view-lines").first().click();
//     await page.waitForTimeout(500);

//     await page.keyboard.press("Control+A");
//     await page.keyboard.press("Backspace");
//     await page.keyboard.type(lineJSON, { delay: 10 });
//     await page.waitForTimeout(500);

//     await page.locator('[data-test="dashboard-panel-error-bar-icon"]').click();
//     await page.locator('[data-test="dashboard-panel-query-editor"]').getByRole("textbox", { name: "Editor content;Press Alt+F1" }).fill('select * from "e2e_automate"');
    
//     await page.locator('[data-test="dashboard-apply"]').click();
//   });
});
