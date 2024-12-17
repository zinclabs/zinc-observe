import { test, expect } from "./baseFixtures.js";
import { LoginPage } from '../pages/loginPage.js';
import { IamPage } from "../pages/iamPage.js";



test.describe("Service Account for API access", () => {
    let loginPage, iamPage;

    test.beforeEach(async ({ page }) => {
        loginPage = new LoginPage(page);
        iamPage = new IamPage(page);
        await loginPage.gotoLoginPage();
        await loginPage.loginAsInternalUser();
        await loginPage.login();
    });

   
    test("Error Message displayed if Email Blank", async ({ page }) => {

        await iamPage.gotoIamPage();
        await iamPage.iamURLValidation();
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.iamPageAddServiceAccountEmailValidation();

    });

    test("Service Account created", async ({ page }) => {

        await iamPage.gotoIamPage();
        //await iamPage.iamURLValidation();
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.iamPageAddServiceAccountEmailValidation();

    });

   
   

});
