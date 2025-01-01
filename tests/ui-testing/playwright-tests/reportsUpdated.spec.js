import { test, expect } from "./baseFixtures";
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
      //  await loginPage.loginAsInternalUser();
        await loginPage.login();
        await ingestionPage.ingestion();
    });
    test("Create, use, and delete dashboard and report", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and once Schedule Now type report", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportOnce();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });


    test("Create, use, and delete dashboard and hours Schedule Now  type report", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportHours();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and days Schedule Now type report", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportDays();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and weeks Schedule Now type report", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportWeeks();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and months Schedule Now type report", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportMonths();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and custom Schedule Now type report", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportCustom();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and custom Schedule Later type report ", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportCustom();
        await reportsPage.createReportScheduleLater();
        await reportsPage.createReportDateTime();
        await reportsPage.createReportZone();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and months Schedule Later type report ", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportMonths();
        await reportsPage.createReportScheduleLater();
        await reportsPage.createReportDateTime();
        await reportsPage.createReportZone();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and weeks Schedule Later type report ", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportWeeks();
        await reportsPage.createReportScheduleLater();
        await reportsPage.createReportDateTime();
        await reportsPage.createReportZone();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and days Schedule Later type report ", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportDays();
        await reportsPage.createReportScheduleLater();
        await reportsPage.createReportDateTime();
        await reportsPage.createReportZone();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and hours Schedule Later type report ", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportHours();
        await reportsPage.createReportScheduleLater();
        await reportsPage.createReportDateTime();
        await reportsPage.createReportZone();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });

    test("Create, use, and delete dashboard and once Schedule Later type report ", async ({ page }) => {
        const TEST_REPORT_NAME = "rreport1";
        await dashboardPage.navigateToDashboards();
        await dashboardPage.createDashboard();
        await expect(page).toHaveURL(/.*\/dashboards/);
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await reportsPage.createReportAddReportButton();
        await reportsPage.createReportReportNameInput();
        await reportsPage.createReportFolderInput();
        await reportsPage.createReportDashboardInput(dashboardPage.dashboardName);
        await reportsPage.createReportDashboardTabInput();
        await reportsPage.createReportContinueButtonStep1();
        await reportsPage.createReportOnce();
        await reportsPage.createReportScheduleLater();
        await reportsPage.createReportDateTime();
        await reportsPage.createReportZone();
        await reportsPage.createReportContinueButtonStep2();
        await reportsPage.createReportFillDetail();
        await reportsPage.createReportSaveButton();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/reports?org_identifier=default");
        await expect(page).toHaveURL(/.*\/reports/);
        await reportsPage.deleteReport(TEST_REPORT_NAME);
        await dashboardPage.navigateToDashboards();
        await dashboardPage.deleteDashboard();
    });


});