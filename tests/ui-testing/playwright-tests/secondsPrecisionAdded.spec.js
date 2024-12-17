// Implementation test.spec.js
import { test, expect } from "./baseFixtures";
import { LoginPage } from '../pages/loginPage.js';
import { LogsPage } from '../pages/logsPage.js';


import { startTimeValue, endTimeValue } from '../pages/CommonLocator.js';

test('Relative Seconds on Logs page', async ({ page }) => {
  // Create page object instances
  const loginPage = new LoginPage(page);
  const logsPage = new LogsPage(page);
  // Step 1: Navigate to the application and login
  await loginPage.gotoLoginPage();
  await loginPage.login();
  // Step 2: Navigate to Logs Page
 // await page.waitForTimeout(4000);  // Wait for login process 
  await logsPage.navigateToLogs();
  // Step 3: Set the time to past 30 seconds and verify
  await logsPage.setTimeToPast30Seconds();
  await logsPage.verifyTimeSetTo30Seconds();
  await logsPage.signOut();
});



