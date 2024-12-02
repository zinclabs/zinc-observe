import { test, expect } from "./baseFixtures";

import { ManagementPage } from "../pages/managementPage .js";

import { LoginPage } from '../pages/loginPage.js';


const path = require('path');

test('Logo Upload on Management ', async ({ page }) => {
    // Create page object instances
    const filePath = path.resolve(__dirname, './attachment/imagesAuto.png');

    const loginPage = new LoginPage(page);
    const managementPage = new ManagementPage(page);
 
    // Step 1: Navigate to the application and login
 
    await page.goto(process.env["ZO_BASE_URL"]);
 
    // console.log ('URL Opened')
 
    await loginPage.gotoLoginPage();
 
    await loginPage.login(process.env["ZO_ROOT_USER_EMAIL"], process.env["ZO_ROOT_USER_PASSWORD"]);
 
    // Step 2: Navigate to Logs Page


    await managementPage.navigateToManagement();
    await managementPage.updateCustomLogoText("logo Auto");

    console.log(`Uploading file from path: ${filePath}`);
    //
    await managementPage.uploadLogo(filePath);


    await page.waitForTimeout(5000); // Adjust or remove as needed
});
