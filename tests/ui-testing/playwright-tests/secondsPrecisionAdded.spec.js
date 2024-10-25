// Implementation test.spec.js
import { test, expect } from '@playwright/test';
import { LoginPage } from '../pages/loginPage.js';
import { LogsPage } from '../pages/logsPage.js';
import { TracesPage } from '../pages/tracesPage.js';
import { ReportsPage } from '../pages/reportsPage.js';
import { DashboardPage } from '../pages/dashboardPage.js';
import { AlertPage } from '../pages/alertsPage.js';

import { MetricsPage } from '../pages/metricsPage.js';
import { RumPage } from '../pages/rumPage.js';

import { startTimeValue, endTimeValue, startDateTimeValue, endDateTimeValue } from '../pages/CommonLocator.js';


//console.log ('Login Started')

test('Relative Seconds on Logs page', async ({ page }) => {
  // Create page object instances
  const loginPage = new LoginPage(page);
  const logsPage = new LogsPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  // console.log ('URL Opened')

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  // Step 2: Navigate to Logs Page
  //await page.waitForTimeout(4000);  // Wait for login process 
  await page.waitForTimeout(4000); 
  await logsPage.navigateToLogs();
  await page.waitForTimeout(4000); 
  // Step 3: Select Index and Stream
  //await page.waitForTimeout(3000);  // Wait for logs page to load 

  // await logsPage.selectIndexAndStream();

  // Step 4: Set the time to past 30 seconds and verify
  await logsPage.setTimeToPast30Seconds();
  await page.waitForTimeout(4000); 
  await logsPage.verifyTimeSetTo30Seconds();
  await page.waitForTimeout(4000); 

  await logsPage.signOut();

});


test('Absolute Seconds on Logs page', async ({ page }) => {
  // Create page object instances
  const loginPage = new LoginPage(page);
  const logsPage = new LogsPage(page);

  // Step 1: Navigate to the application and login
  //await page.goto('https://alpha1.dev.zinclabs.dev/web/');
  await page.goto(process.env["ZO_BASE_URL"]);

  await loginPage.gotoLoginPage();

  //await loginPage.login('testing@alpha1.com', 'SecTest@500');

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  // Step 2: Navigate to Logs Page
 // await page.waitForTimeout(4000);  // Wait for login process 

  // await logsPage.selectOrganization();

  // Navigate to Logs page
  await page.waitForTimeout(4000); 
  await logsPage.navigateToLogs();
  await page.waitForTimeout(4000); 
  // Step 3: Select Index and Stream
  //await page.waitForTimeout(3000);  // Wait for logs page to load
  //await logsPage.selectIndexAndStream();

  // Step 4: Set the time to past 30 seconds and verify
  //await logsPage.adjustFilterParameters();
  //await logsPage.enableSQLMode();

  // Set the Date and Time Range
  await logsPage.setDateTime();
  await page.waitForTimeout(4000); 
  await logsPage.fillTimeRange(startTimeValue, endTimeValue);

  // Verify the time range is displayed correctly
  await logsPage.verifyDateTime(startDateTimeValue, endDateTimeValue);

  await logsPage.signOut();


});


test('Relative Seconds on Traces', async ({ page }) => {

  // Create page object instances
  const loginPage = new LoginPage(page);
  const tracesPage = new TracesPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  await page.waitForTimeout(4000);  // Wait for login process

  // Step 2: Navigate to the traces page and perform actions

  await tracesPage.navigateToTraces();

  await page.waitForTimeout(4000);  // Wait for login process

  // Step 4: Set the time to past 30 seconds and verify

  await tracesPage.setTimeToPast30Seconds();
  await tracesPage.verifyTimeSetTo30Seconds();

  await tracesPage.signOut();

});

test('Absolute Seconds on Traces', async ({ page }) => {

  // Create page object instances
  const loginPage = new LoginPage(page);
  const tracesPage = new TracesPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  await page.waitForTimeout(4000);  // Wait for login process 

  // Step 2: Navigate to the traces page and perform actions

  await tracesPage.navigateToTraces();
  await page.waitForTimeout(4000);  // Wait for login process
  // Step 4: // Set the Date and Time Range

  await tracesPage.setDateTime();
  await tracesPage.fillTimeRange(startTimeValue, endTimeValue);

  // Verify the time range is displayed correctly

  await tracesPage.verifyDateTime(startDateTimeValue, endDateTimeValue);

  await tracesPage.signOut();

});


test('Relative Seconds on Reports', async ({ page }) => {

  // Create page object instances
  const loginPage = new LoginPage(page);
  const reportsPage = new ReportsPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  await page.waitForTimeout(4000);  // Wait for login process 

  // Step 2: Navigate to the Reports page and click add report

  await reportsPage.navigateToReports();

  await page.waitForTimeout(4000);  // Wait for login process

  await reportsPage.addNewReport();
  await page.waitForTimeout(4000);  // Wait for login process
  // Step 4: Set the time to past 30 seconds and verify

  await reportsPage.setTimeToPast30Seconds();
  await reportsPage.verifyTimeSetTo30Seconds();

  await reportsPage.signOut();
});

test('Absolute Seconds on Reports', async ({ page }) => {

  // Create page object instances
  const loginPage = new LoginPage(page);
  const reportsPage = new ReportsPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  await page.waitForTimeout(4000);  // Wait for login process 

  // Step 2: Navigate to the Reports page and click add report

  await reportsPage.navigateToReports();

  await page.waitForTimeout(4000);  // Wait for login process

  await reportsPage.addNewReport();
  await page.waitForTimeout(4000);  // Wait for login process

  // Step 4: // Set the Date and Time Range

  await reportsPage.setDateTime();
  await reportsPage.fillTimeRange(startTimeValue, endTimeValue);

  // Verify the time range is displayed correctly
  await reportsPage.verifyDateTime(startDateTimeValue, endDateTimeValue);

  await reportsPage.signOut();

});


test('Relative Seconds on Dashboard', async ({ page }) => {

  // Create page object instances
  const loginPage = new LoginPage(page);
  const dashboardPage = new DashboardPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  await page.waitForTimeout(4000);  // Wait for login process 

  // Step 2: Navigate to the dashboard page and add a new dashboard
  await dashboardPage.navigateToDashboards();
  await page.waitForTimeout(4000);  // Wait for login process
  await dashboardPage.addDashboard('Relative D');
  await page.waitForTimeout(4000);  // Wait for login process
  // Step 4: Set the time to past 30 seconds and verify

  await dashboardPage.setTimeToPast30Seconds();
  await dashboardPage.verifyTimeSetTo30Seconds();

  await dashboardPage.signOut();
});

test('Absolute Seconds on Dashboard', async ({ page }) => {

  // Create page object instances
  const loginPage = new LoginPage(page);
  const dashboardPage = new DashboardPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  await page.waitForTimeout(4000);  // Wait for login process

  // Step 2: Navigate to the dashboard page and add a new dashboard
  await dashboardPage.navigateToDashboards();
  await page.waitForTimeout(4000);  // Wait for login process
  await dashboardPage.addDashboard('Absolute DB');
  await page.waitForTimeout(4000);  // Wait for login process

  // Step 3: // Set the Date and Time Range

  await dashboardPage.setDateTime();
  await dashboardPage.fillTimeRange(startTimeValue, endTimeValue);

  // Verify the time range is displayed correctly
  await dashboardPage.verifyDateTime(startDateTimeValue, endDateTimeValue);

  await dashboardPage.signOut();

});

test('Relative Seconds on Alert', async ({ page }) => {

  // Create page object instances
  const loginPage = new LoginPage(page);
  const alertPage = new AlertPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  await page.waitForTimeout(4000);  // Wait for login process 

  // Navigate to Alerts and Create Alert
  await alertPage.navigateToAlerts();
  await page.waitForTimeout(4000);  // Wait for login process
  await alertPage.createAlert();


  // Set the time to past 30 seconds and verify

  await alertPage.setTimeToPast30Seconds();
  await alertPage.verifyTimeSetTo30Seconds();

  await alertPage.signOut();
});


test('Relative Seconds on Metrics page', async ({ page }) => {
  // Create page object instances
  const loginPage = new LoginPage(page);
  const metricsPage = new MetricsPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  //console.log ('URL Opened')

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  // Step 2: Navigate to Logs Page
  await page.waitForTimeout(4000);  // Wait for login process 

  // Open metrics page and perform operations
  await metricsPage.openMetricsPage();
  await page.waitForTimeout(3000); // wait for the page to load
  await metricsPage.selectIndex('otelcol_http_server_duration_bucket');
  await page.waitForTimeout(3000); // wait for selection to complete

  // Set time filter and verify
  await metricsPage.setTimeToPast30Seconds();
  await page.waitForTimeout(3000); // wait for time selection
  await metricsPage.verifyTimeSetTo30Seconds();


});

test('Absolute Seconds on Metrics page', async ({ page }) => {
  // Create page object instances
  const loginPage = new LoginPage(page);
  const metricsPage = new MetricsPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  //console.log ('URL Opened')

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  // Step 2: Navigate to Logs Page
  await page.waitForTimeout(4000);  // Wait for login process 

  // Open metrics page and perform operations
  await metricsPage.openMetricsPage();
  await page.waitForTimeout(3000); // wait for the page to load
  await metricsPage.selectIndex('otelcol_http_server_duration_bucket');
  await page.waitForTimeout(3000); // wait for selection to complete

  // Set the Date and Time Range
  await metricsPage.setDateTime();
  await metricsPage.fillTimeRange(startTimeValue, endTimeValue);

  // Verify the time range is displayed correctly
  await metricsPage.verifyDateTime(startDateTimeValue, endDateTimeValue);


});
/*
test('Relative Seconds on RUM Performance page', async ({ page }) => {
  // Create page object instances
  const loginPage = new LoginPage(page);
  const rumPage = new RumPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  //console.log ('URL Opened')

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  // Step 2: Navigate to RUM Page
  await page.waitForTimeout(4000);  // Wait for login process 

  // Open RUM page and perform operations
  await rumPage.navigateToPerformanceOverview();

  // Set time filter and verify
  await rumPage.setTimeToPast30Seconds();
  await page.waitForTimeout(3000); // wait for time selection
  await rumPage.verifyTimeSetTo30Seconds();


});

test('Absolute Seconds on RUM Performance page', async ({ page }) => {
  // Create page object instances
  const loginPage = new LoginPage(page);
  const rumPage = new RumPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  // console.log ('URL Opened')

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  // Step 2: Navigate to RUM Page
  await page.waitForTimeout(4000);  // Wait for login process 

  // Open RUM page and perform operations
  await rumPage.navigateToPerformanceOverview();


  // Set the Date and Time Range
  await rumPage.setDateTime();
  await rumPage.fillTimeRange(startTimeValue, endTimeValue);

  // Verify the time range is displayed correctly
  await rumPage.verifyDateTime(startDateTimeValue, endDateTimeValue);


});

/*

test('Relative Seconds on RUM Sessions page', async ({ page }) => {
  // Create page object instances
  const loginPage = new LoginPage(page);
  const rumPage = new RumPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);



  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  // Step 2: Navigate to RUM Page
  await page.waitForTimeout(4000);  // Wait for login process 

  // Open RUM page and perform operations
  await rumPage.navigateToPerformanceOverview();
  await page.waitForTimeout(4000);
  await rumPage.navigateToSessionsTab();
  // Set time filter and verify
  await rumPage.setTimeToPast30Seconds();
  await page.waitForTimeout(3000); // wait for time selection
  await rumPage.verifyTimeSetTo30SecondsSchedule();


});

test('Absolute Seconds on RUM Sessions page', async ({ page }) => {
  // Create page object instances
  const loginPage = new LoginPage(page);
  const rumPage = new RumPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  // console.log ('URL Opened')

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  // Step 2: Navigate to RUM Page
  await page.waitForTimeout(4000);  // Wait for login process 

  // Open RUM page and perform operations
  await rumPage.navigateToPerformanceOverview();
  await page.waitForTimeout(4000);
  await rumPage.navigateToSessionsTab();
  // Set the Date and Time Range
  await rumPage.setDateTime();
  await rumPage.fillTimeRange(startTimeValue, endTimeValue);

  // Verify the time range is displayed correctly
  await rumPage.verifyDateTime(startDateTimeValue, endDateTimeValue);


});

test('Relative Seconds on RUM Tracking page', async ({ page }) => {
  // Create page object instances
  const loginPage = new LoginPage(page);
  const rumPage = new RumPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  // console.log ('URL Opened')

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  // Step 2: Navigate to RUM Page
  await page.waitForTimeout(4000);  // Wait for login process 

  // Open RUM page and perform operations
  await rumPage.navigateToPerformanceOverview();
  await page.waitForTimeout(4000);
  await rumPage.navigateToErrorTrackingTab();
  // Set time filter and verify
  await rumPage.setTimeToPast30Seconds();
  await page.waitForTimeout(3000); // wait for time selection
  await rumPage.verifyTimeSetTo30Seconds();


});

test('Absolute Seconds on RUM Tracking page', async ({ page }) => {
  // Create page object instances
  const loginPage = new LoginPage(page);
  const rumPage = new RumPage(page);

  // Step 1: Navigate to the application and login

  await page.goto(process.env["ZO_BASE_URL"]);

  // console.log ('URL Opened')

  await loginPage.gotoLoginPage();

  await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);

  // Step 2: Navigate to RUM Page
  await page.waitForTimeout(4000);  // Wait for login process 

  // Open RUM page and perform operations
  await rumPage.navigateToPerformanceOverview();
  await page.waitForTimeout(4000);
  await rumPage.navigateToErrorTrackingTab();
  // Set the Date and Time Range
  await rumPage.setDateTime();
  await rumPage.fillTimeRange(startTimeValue, endTimeValue);

  // Verify the time range is displayed correctly
  await rumPage.verifyDateTime(startDateTimeValue, endDateTimeValue);


});


*/