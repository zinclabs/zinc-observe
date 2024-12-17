import { test, expect } from "./baseFixtures.js";
import { LoginPage } from '../pages/loginPage.js';
import { IamPage } from "../pages/iamPage.js";



test.describe("Service Account for API access", () => {
    let loginPage, iamPage;
    const emailName = `email${Date.now()}@gmail.com`;

    test.beforeEach(async ({ page }) => {
        loginPage = new LoginPage(page);
        iamPage = new IamPage(page);
        await loginPage.gotoLoginPage();
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
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.enterEmailServiceAccount(emailName);
        await iamPage.clickSaveServiceAccount();
        await iamPage.verifySuccessMessage('Service Account created successfully.');

    });

    test("Service Account not created if Email already exists", async ({ page }) => {

        await iamPage.gotoIamPage();
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.enterSameEmailServiceAccount();
        await iamPage.clickSaveServiceAccount();
        await iamPage.verifySuccessMessage('User already exists');

    });

    test("Service Account not created if Cancel clicked", async ({ page }) => {

        await iamPage.gotoIamPage();
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.enterEmailServiceAccount(emailName);
        await iamPage.enterFirstLastNameServiceAccount();
        await iamPage.clickCancelServiceAccount();

    });

    test("Service Account Token copied", async ({ page }) => {

        await iamPage.gotoIamPage();
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.enterEmailServiceAccount(emailName);
        await iamPage.clickSaveServiceAccount();
        await iamPage.verifySuccessMessage('Service Account created successfully.');
        await iamPage.clickCopyToken();


    });

    test("Service Account Download Token", async ({ page }) => {

        await iamPage.gotoIamPage();
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.enterEmailServiceAccount(emailName);
        await iamPage.clickSaveServiceAccount();
        await iamPage.verifySuccessMessage('Service Account created successfully.');
        await iamPage.clickDownloadToken();


    });
    test("Service Account Token Pop Up Closed", async ({ page }) => {

        await iamPage.gotoIamPage();
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.enterEmailServiceAccount(emailName);
        await iamPage.clickSaveServiceAccount();
        await iamPage.verifySuccessMessage('Service Account created successfully.');
        await iamPage.clickServiceAccountPopUpClosed();

    });

    test("Service Account Created and deleted", async ({ page }) => {

        await iamPage.gotoIamPage();
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.enterEmailServiceAccount(emailName);
        await iamPage.clickSaveServiceAccount();
        await iamPage.verifySuccessMessage('Service Account created successfully.');
        await iamPage.clickServiceAccountPopUpClosed();
        await iamPage.reloadServiceAccountPage();
        await iamPage.deletedServiceAccount(emailName);
        await iamPage.requestServiceAccountOk();
        await iamPage.verifySuccessMessage('Service Account deleted successfully.');

    });

    test("Service Account Created and not deleted if cancel clicked", async ({ page }) => {

        await iamPage.gotoIamPage();
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.enterEmailServiceAccount(emailName);
        await iamPage.clickSaveServiceAccount();
        await iamPage.verifySuccessMessage('Service Account created successfully.');
        await iamPage.clickServiceAccountPopUpClosed();
        await iamPage.reloadServiceAccountPage();
        await iamPage.deletedServiceAccount(emailName);
        await iamPage.requestServiceAccountCancel();

    });

    test("Service Account Created and updated details", async ({ page }) => {

        await iamPage.gotoIamPage();
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.enterEmailServiceAccount(emailName);
        await iamPage.clickSaveServiceAccount();
        await iamPage.verifySuccessMessage('Service Account created successfully.');
        await iamPage.clickServiceAccountPopUpClosed();
        await iamPage.reloadServiceAccountPage();
        await iamPage.updatedServiceAccount(emailName);
        await iamPage.enterFirstLastNameServiceAccount();
        await iamPage.clickSaveServiceAccount();
        await iamPage.verifySuccessMessage('Service Account updated successfully.');

    });

    test("Service Account Created and refresh token", async ({ page }) => {

        await iamPage.gotoIamPage();
        await iamPage.iamPageServiceAccountsTab();
        await iamPage.iamPageAddServiceAccount();
        await iamPage.enterEmailServiceAccount(emailName);
        await iamPage.clickSaveServiceAccount();
        await iamPage.verifySuccessMessage('Service Account created successfully.');
        await iamPage.clickServiceAccountPopUpClosed();
        await iamPage.reloadServiceAccountPage();
        await iamPage.refreshServiceAccount(emailName);
        await iamPage.requestServiceAccountOk();
        await iamPage.verifySuccessMessage('Service token refreshed successfully.');

    });



});
