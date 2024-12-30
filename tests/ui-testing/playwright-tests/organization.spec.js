import { test, expect } from "./baseFixtures.js";
import { LoginPage } from '../pages/loginPage.js';
import { IamOrgPage } from "../pages/iamOrgPage.js";
import { IamUserPage } from "../pages/iamUserPage.js";




test.describe("Organizations Redesign", () => {
    let loginPage, iamOrgPage, iamUserPage;
    const orgName = `org${Date.now()}`;
    const emailName = `email${Date.now()}@gmail.com`;
    const password = "password123";
    const username = "username123";


    test.beforeEach(async ({ page }) => {
        // Login
        loginPage = new LoginPage(page);
        iamOrgPage = new IamOrgPage(page);
        iamUserPage = new IamUserPage(page);

        await loginPage.gotoLoginPage();
        await loginPage.loginAsInternalUser(); // Login as root user
        await loginPage.login();

    });


    test("Add an organization successfully", async ({ page }) => {

        // Navigate to Organizations
        await iamOrgPage.navigateToOrganizations();

        // Add Organization
        await iamOrgPage.addOrganization(orgName);

        // Verify success message
        await iamOrgPage.verifySuccessMessage('Organization added successfully.');
        //await expect(iamOrgPage.alert).toContainText('Organization added successfully.');
    });


    test("Error when Add organization without name", async ({ page }) => {

        // Navigate to Organizations
        await iamOrgPage.navigateToOrganizations();

        // Attempt to add organization without a name
        await iamOrgPage.addOrgButton.click({ force: true });
        //await iamOrgPage.verifyError('Name is required');
        await iamOrgPage.orgNameInput.fill('');

        await iamOrgPage.checkSaveButton();




    });

    test("Cancel when Add organization with name", async ({ page }) => {

        // Navigate to Organizations
        await iamOrgPage.navigateToOrganizations();

        // Attempt to add organization without a name
        await iamOrgPage.addOrgButton.click({ force: true });
        await iamOrgPage.orgNameInput.fill('Org');

        // Cancel the operation
        await iamOrgPage.cancelAddOrganization();

    });

   test('Add a user successfully', async ({ page }) => {

    await iamUserPage.goToUsersTab();

    await iamUserPage.addBasicUser();

    await iamUserPage.enterEmailBasicUser(emailName);
    

    await iamUserPage.addBasicUser();

    await iamUserPage.addBasicUser();

    await iamUserPage.validateAlertMessage('User added successfully.');

  });






});
