// action.js
import { expect } from '@playwright/test';


export class ActionPage {

    constructor(page) {
      this.page = page;
      this.menuLink = '[data-test="menu-link-\\/action-scripts-item"]';
      this.addActionButton = '[data-test="alert-list-add-alert-btn"]';
      this.nameInput = 'label:has-text("Name *")';
      this.descriptionInput = 'label:has-text("Description")';
      this.uploadInput = 'label:has-text("attachmentUpload Code Zip")';
      this.continueStep1Button = '[data-test="add-action-script-step1-continue-btn"]';
      this.continueStep2Button = '[data-test="add-action-script-step2-continue-btn"]';
      this.serviceAccountDropdown = '[data-test="add-action-script-select-serice-account-step"] text=arrow_drop_down';
      this.headerKeyInput = '[data-test="add-action-script-header--key-input"]';
      this.headerValueInput = '[data-test="add-action-script-header-KE-value-input"]';
      this.saveButton = '[data-test="add-action-script-save-btn"]';
      this.alertMessage = this.page.getByRole('alert').first();
      this.deleteAlertButton = '[data-test="alert-list-Ca-delete-alert"]';
      this.cancelButton = '[data-test="cancel-button"]';
      this.confirmButton = '[data-test="confirm-button"]';
    }
  
    async openMenu() {
      await this.page.click(this.menuLink);
    }
  

    async addActionScript(name, description, filePath) {
      await this.page.click(this.addActionButton);
      await this.page.fill(this.nameInput, name);
      await this.page.fill(this.descriptionInput, description);
      await this.page.setInputFiles(this.uploadInput, filePath);
      await this.page.click(this.continueStep1Button);
      await this.page.click(this.continueStep2Button);
    }
  

    async selectServiceAccount(accountEmail) {
      await this.page.click(this.serviceAccountDropdown);
      await this.page.click(`role=option[name="${accountEmail}"] >> nth=2`);
    }
  

    async addHeaders(key, value) {
      await this.page.fill(this.headerKeyInput, key);
      await this.page.fill(this.headerValueInput, value);
      await this.page.click(this.saveButton);
    }
  

    async validateAlert(text) {
      await expect(this.page.locator(this.alert)).toContainText(text);
    }
  

    async deleteAction() {
      await this.page.click(this.deleteAlertButton);
      await this.page.click(this.confirmButton);
    }


  }
  

  