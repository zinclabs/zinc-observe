import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages/loginPage';
import { DashboardPage } from '../pages/dashboardPage';
import { ReportsPage } from '../pages/reportsPage';
import { IngestionPage } from '../pages/ingestionPage';


test.describe("Report test cases Updated", () => {
    let loginPage, dashboardPage, reportsPage, ingestionPage;

    test.beforeEach(async ({ page }) => {
        loginPage = new LoginPage(page);
        ingestionPage = new IngestionPage(page);
        dashboardPage = new DashboardPage(page);
        reportsPage = new ReportsPage(page);
        await loginPage.gotoLoginPage();
        await loginPage.login();
        await ingestionPage.ingestion();
    });
    test("Create, use, and delete dashboard and report", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports");
        await reportsPage.clickAddReportButton();
        await reportsPage.createReport(dashboardPage.dashboardName);
        await reportsPage.createReportSetting();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and report for changing timezone", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports");
        await reportsPage.clickAddReportButton();
        await reportsPage.setTimeToPast30Seconds();
        await reportsPage.changeTimeZone();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports/create");
        await reportsPage.createReport(dashboardPage.dashboardName);
        await reportsPage.createReportSetting();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

});
