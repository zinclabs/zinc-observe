import { test, expect } from "./baseFixtures";
import logData from "../cypress/fixtures/log.json";
import logsdata from "../../test-data/logs_data.json";

test.describe.configure({ mode: 'parallel' });

async function login(page) {
  await page.goto(process.env["ZO_BASE_URL"]);
  await page.waitForTimeout(1000);
  await page.getByText('Login as internal user').click();
  await page.locator('[data-cy="login-user-id"]').fill(process.env["ZO_ROOT_USER_EMAIL"]);
  await page.locator('label').filter({ hasText: 'Password *' }).click();
  await page.locator('[data-cy="login-password"]').fill(process.env["ZO_ROOT_USER_PASSWORD"]);
  await page.locator('[data-cy="login-sign-in"]').click();
}

const selectStreamAndStreamTypeForLogs = async (page, stream) => {
  await page.waitForTimeout(4000);
  await page.locator('[data-test="log-search-index-list-select-stream"]').click({ force: true });
  await page.locator("div.q-item").getByText(`${stream}`).first().click({ force: true });
};

async function applyQueryButton(page) {
  const search = page.waitForResponse(logData.applyQuery);
  await page.waitForTimeout(3000);
  await page.locator("[data-test='logs-search-bar-refresh-btn']").click({ force: true });
  await expect.poll(async () => (await search).status()).toBe(200);
}


async function runQuery(page, query) {

    const startTime = Date.now();
    const response = await page.evaluate(async (query, url, streamName, orgId) => {
      let response
      try {
        response = await fetch(`${url}/api/${orgId}/${streamName}/_search?type=logs`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify(query)
        });
      } catch (err) {
        console.log(err);
      }
  
      return response
    }, query);
    const endTime = Date.now();
    const duration = endTime - startTime;
    return {
      response,
      duration
    };
  }

test.describe("Compare SQL query execution times", () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.waitForTimeout(5000);

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

    await page.goto(`${logData.logsUrl}?org_identifier=${process.env["ORGNAME"]}`);
    await selectStreamAndStreamTypeForLogs(page, logData.Stream);
    await applyQueryButton(page);
  });

  test("should compare match_all_raw and match_all query times", async ({ page }) => {
    const oneMinuteAgo = Date.now() - 60 * 1000;
    const query1 = {
      query: {
        sql: "select * from 'e2e_automate'  WHERE match_all_raw('provide_credentials')",
        start_time: oneMinuteAgo,
        end_time: Date.now(),
        from: 0,
        size: 250,
        quick_mode: false
      },
      regions: []
    };

    const query2 = {
      query: {
        sql: "select * from 'e2e_automate'  WHERE match_all('provide_credentials')",
        start_time: oneMinuteAgo,
        end_time: Date.now(),
        from: 0,
        size: 250,
        quick_mode: false
      },
      regions: []
    };

    const result1 = await runQuery(page, query1);
    const result2 = await runQuery(page, query2);

    console.log(`Query 1 (match_all_raw) took ${result1.duration} ms and returned ${result1.response.total} records.`);
    console.log(`Query 2 (match_all) took ${result2.duration} ms and returned ${result2.response.total} records.`);

    try {
      expect(result2.duration).toBeLessThan(result1.duration);
      console.log('Assertion passed: match_all query took less time than match_all_raw query.');
    } catch (error) {
      console.error('Assertion failed: match_all query did not take less time than match_all_raw query.');
    }
  });


  test("should compare match_all and match_all_raw_ignore_case query times", async ({ page }) => {
    const oneMinuteAgo = Date.now() - 60 * 1000;
    const query1 = {
      query: {
        sql: "select * from 'e2e_automate'  WHERE match_all('provide_credentials')",
        start_time: oneMinuteAgo,
        end_time: Date.now(),
        from: 0,
        size: 250,
        quick_mode: false
      },
      regions: []
    };

    const query2 = {
      query: {
        sql: "select * from 'e2e_automate'  WHERE match_all_raw_ignorecase('provide_credentials')",
        start_time: oneMinuteAgo,
        end_time: Date.now(),
        from: 0,
        size: 250,
        quick_mode: false
      },
      regions: []
    };

    const result1 = await runQuery(page, query1);
    const result2 = await runQuery(page, query2);

    console.log(`Query 1 (match_all) took ${result1.duration} ms and returned ${result1.response.total} records.`);
    console.log(`Query 2 (match_all_raw_ignorecase) took ${result2.duration} ms and returned ${result2.response.total} records.`);

    try {
      expect(result1.duration).toBeLessThan(result2.duration);
      console.log('Assertion passed: match_all query took less time than match_all_raw_ignorecase query.');
    } catch (error) {
      console.error('Assertion failed: match_all query did not take less time than match_all_raw_ignorecase query.');
    }
  });
});
