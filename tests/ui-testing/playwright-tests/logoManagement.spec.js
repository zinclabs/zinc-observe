import { test, expect } from "./baseFixtures";


import { LogoManagementPage } from "../pages/logoManagementPage.js";

import { LoginPage } from '../pages/loginPage.js';


const path = require('path');

// Function to generate a random 5-character alphabetic name
function generateRandomLogoName() {
    const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';
    let randomName = '';
    for (let i = 0; i < 5; i++) {
        randomName += alphabet.charAt(Math.floor(Math.random() * alphabet.length));
    }
    return randomName;
}


test('Logo Upload on Management ', async ({ page }) => {
    // Create page object instances
    const filePath = path.resolve(__dirname, './attachment/imagesAuto.png');

    const loginPage = new LoginPage(page);
    const logoManagementPage = new LogoManagementPage(page);

    // Example usage in Playwright POM
const logoName = generateRandomLogoName();
console.log(`Generated logo name: ${logoName}`);
 
    // Step 1: Navigate to the application and login
 
    await page.goto(process.env["ZO_BASE_URL"]);
 
    // console.log ('URL Opened')
 
    await loginPage.gotoLoginPage();

    await loginPage.loginAsInternalUser();
 
    await loginPage.login();
 
    // Step 2: Navigate to Logs Page


    await logoManagementPage.navigateToManagement();

    await logoManagementPage.clickSaveSubmit();

    await logoManagementPage.updateCustomLogoText(logoName );

    console.log(`Uploading file from path: ${filePath}`);
    //
    await logoManagementPage.uploadLogo(filePath);


    await page.waitForTimeout(5000); // Adjust or remove as needed
});
