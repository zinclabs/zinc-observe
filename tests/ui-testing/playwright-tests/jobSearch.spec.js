import { test, expect } from "./baseFixtures.js";
import { LoginPage } from '../pages/loginPage.js';
import { LogsPage } from '../pages/logsPage.js';
import { IngestionPage } from '../pages/ingestionPage.js';

test.describe.configure({ mode: 'parallel' });

test.describe("Job Search for schedule query", () => {
    let loginPage, logsPage, ingestionPage;
    const stream = "e2e_automate";
    test.beforeEach(async ({ page }) => {
        loginPage = new LoginPage(page);
        ingestionPage = new IngestionPage(page);
        logsPage = new LogsPage(page);
        await loginPage.gotoLoginPage();
        await loginPage.loginAsInternalUser();
        await loginPage.login(); // Login as root user
        await ingestionPage.ingestion();
        // await ingestionPage.ingestionJoin();
    });

    test("Create Job Search for schedule query", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectStreamDropDown();
        await logsPage.selectStreamAndStreamTypeForLogs(stream);
        await logsPage.enableSQLMode();
        await logsPage.queryJobSearch();
        await logsPage.selectRunQuery();
        await logsPage.searchSchedulerDropdown();
        await logsPage.searchSchedulerCreate();
        await logsPage.searchSchedulerSubmit();
        await logsPage.validateAddJob();

    });

    test("Create and Delete Job Search for schedule query", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectStreamDropDown();
        await logsPage.selectStreamAndStreamTypeForLogs(stream);
        await logsPage.enableSQLMode();
        await logsPage.queryJobSearch();
        await logsPage.selectRunQuery();
        await logsPage.searchSchedulerDropdown();
        await logsPage.searchSchedulerCreate();
        await logsPage.searchSchedulerSubmit();
        await page.goto(process.env["ZO_BASE_URL_SC_UI"] + "/web/logs?action=search_scheduler&org_identifier=default&type=search_scheduler_list");


    });

    test("Create Job Search for schedule query with invalid data", async ({ page }) => {

        await logsPage.navigateToLogs();
        await logsPage.selectStreamDropDown();
        await logsPage.selectStreamAndStreamTypeForLogs(stream);
        await logsPage.enableSQLMode();
        await logsPage.queryJobSearch();
        await logsPage.selectRunQuery();
        await logsPage.searchSchedulerDropdown();
        await logsPage.searchSchedulerCreate();
        await logsPage.searchSchedulerInvalid();
        await logsPage.validateInvalidData();
       

    });






    
});
