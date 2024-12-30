
// Basic User Page.js
import { expect } from '@playwright/test';

export class IamUserPage {
    constructor(page) {
      this.iamMenuLink = page.locator('[data-test="menu-link-\\/iam-item"]');
      this.usersTab = page.locator('[data-test="iam-users-tab"]');
      this.addBasicUserButton = page.locator('button:has-text("Add Basic User")');
      this.emailField = page.locator('label:has-text("Email *")');
      this.passwordField = page.locator('label:has-text("Password *")');
      this.roleDropdown = page.locator('div:has-text("Role *errorarrow_drop_down")');
      this.saveButton = page.locator('button:has-text("Save")');
      this.alert = page.getByRole('alert').first();
      this.alertMessage = 'role=alert';
      this.form = 'form';
    }
  
    async goToUsersTab() {
      await this.iamMenuLink.click({force:true});
      await this.usersTab.click();
    }
  
    async addBasicUser() {
      await this.addBasicUserButton.click();
      // await this.emailField.fill(email);
      // await this.page.locator('label:nth-child(2) > .q-field__inner > .q-field__control > .q-field__control-container > .q-field__native').click();
      // // await this.page.getByRole('option', { name: 'Admin' }).locator('div').nth(2).click();
      // await this.page.getByRole('option', { name: role }).locator('div').nth(2).click();
      // await this.saveButton.click();
      // await this.page.getByLabel('Password *').fill(password);
      // await this.saveButton.click();

    }
  
    async validateAlertMessage(expectedMessage) {
      await expect(this.page.locator(this.alertMessage)).toContainText(expectedMessage);
    }

  
    async iamPageAddBasicUser() {

      await this.page.getByRole('button', { name: 'Add Basic User' }).click();

  }


  async iamPageAddBasicUserEmailValidation() {

      await this.page.getByRole('button', { name: 'Save' }).click();

      await expect(this.page.getByRole('alert').nth(2)).toContainText('Please enter a valid email address');


  }

  async enterEmailBasicUser(emailName) {

      await this.emailField.click();
      await this.emailField.fill(emailName);

  }

  async enterSameEmailBasicUser() {

      await this.emailField.click();
      await this.emailField.fill(process.env["ZO_ROOT_USER_EMAIL"]);

  }

  async enterEmailBasicUser(emailName) {

   // await this.page.locator('label:nth-child(2) > .q-field__inner > .q-field__control > .q-field__control-container > .q-field__native').click();
      // // await this.page.getByRole('option', { name: 'Admin' }).locator('div').nth(2).click();

}

  async enterFirstLastNameBasicUser() {


      await this.page.getByLabel('First Name').click();
      await this.page.getByLabel('First Name').fill('Test');
      await this.page.getByLabel('Last Name').click();
      await this.page.getByLabel('Last Name').fill('Auto');

  }

  async clickCancelBasicUser() {

      await this.page.getByRole('button', { name: 'Cancel' }).click();


  }

  async clickSaveBasicUser() {
      await this.page.getByRole('button', { name: 'Save' }).click();
  }


  async verifySuccessMessage(expectedMessage) {
      await expect(this.page.locator('role=alert').first()).toBeVisible();
      await expect(this.alertMessage).toContainText(expectedMessage);

  }

  async validateBasicUserToken() {

      await expect(this.page.getByRole('dialog')).toBeVisible();
      await expect(this.page.getByRole('dialog')).toContainText('Basic User Token');

  }

  async waitResEmailBasicUser(emailName) {
      await this.page.waitForResponse(
          (response) =>
              response.url().includes("/api/default/service_accounts/${emailName}") && response.status() === 200
      );

  }

  async clickCopyToken() {

      await this.page.locator('#q-portal--dialog--2').getByRole('button', { name: 'Copy Token' }).click();
      // await this.page.locator('#q-portal--dialog--3').getByRole('button', { name: 'Copy Token' }).click();

  }



  async clickDownloadToken() {

      const downloadPromise = this.page.waitForEvent('download');

      await this.page.getByRole('button', { name: 'Download Token' }).click();

  }

  async clickBasicUserPopUpClosed() {

      await this.page.locator('button').filter({ hasText: 'close' }).click();

  }

  async reloadBasicUserPage(emailName) {

      await this.page.reload(); // Optional, if necessary
     // await this.page.locator('[data-test="iam-page"]').getByText('arrow_drop_down').click({ force: true });
     // await this.page.getByText('100').click({ force: true });

  }

  async deletedBasicUser(emailName) {
      const deleteButtonLocator = this.page.locator(this.deleteButton(emailName));
      // Wait for the delete button to be visible
      await deleteButtonLocator.waitFor({ state: 'visible', timeout: 30000 });
      // Click the delete button
      await deleteButtonLocator.click({ force: true });

  }

  async requestBasicUserOk(emailName) {
      // Wait for the confirmation button to be visible and click it
      await this.page.locator(this.confirmOkButton).waitFor({ state: 'visible', timeout: 10000 });
      await this.page.locator(this.confirmOkButton).click({ force: true });
      // Add some buffer wait, if necessary
      await this.page.waitForTimeout(2000);
  }



  async requestBasicUserCancel() {
      // Wait for the cancel confirmation button to be visible and click it
      await this.page.locator(this.cancelButton).waitFor({ state: 'visible', timeout: 10000 });
      await this.page.locator(this.cancelButton).click({ force: true });

      // Add some buffer wait, if necessary
      await this.page.waitForTimeout(2000);


  }

  async updatedBasicUser(emailName) {
      const updateButtonLocator = this.page.locator(this.updateButton(emailName));
      // Wait for the update button to be visible
      await updateButtonLocator.waitFor({ state: 'visible', timeout: 30000 });
      // Click the update button
      await updateButtonLocator.click({ force: true });
      await expect(this.page.getByRole('dialog')).toContainText('Update Basic User');
  }

  
  


  }
  

  