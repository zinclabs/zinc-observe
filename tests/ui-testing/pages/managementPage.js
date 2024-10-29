// managementPage.js
import { expect } from '@playwright/test';

export const path = require('path');
export
    class ManagementPage {

    constructor(page) {
        this.page = page;

        this.homeIcon = page.locator("[name ='home']");
        //this.managementMenuItem = page.getByText('Management');
        this.managementMenuItem = page.locator('[data-test="menu-link-/settings/-item"]');
        //this.submitButton = page.locator("[type='submit']").nth(1);
        this.submitButton = page.locator("[data-test='settings_ent_logo_custom_text_edit_btn']");
        this.customLogoText = page.locator("[aria-label ='Custom Logo Text']");
        this.saveButton = page.locator('[data-test="settings_ent_logo_custom_text_save_btn"]');
        this.deleteLogoButton = page.locator('[data-test="setting_ent_custom_logo_img_delete_btn"]');
        this.confirmDeleteButton = page.locator('[data-test="logs-search-bar-confirm-dialog-ok-btn"]');
        this.fileUploadInput = page.locator('input[data-test="setting_ent_custom_logo_img_file_upload"]');
    }


    async navigateToManagement() {
        await this.page.waitForSelector("[name ='home']");
        await this.homeIcon.hover();
        await this.page.waitForSelector('[data-test="menu-link-/settings/-item"]');
        await this.managementMenuItem.click({ force: true });
        await this.page.waitForTimeout(5000);
        //await this.page.goto(process.env["ZO_BASE_URL"]/web/settings/general);
        await this.page.goto(process.env["ZO_BASE_URL"] + "/web/settings/general/");

    }

    async updateCustomLogoText(text) {
        await this.page.waitForTimeout(5000);
        await this.page.waitForSelector("[data-test='settings_ent_logo_custom_text_edit_btn']");
        await this.submitButton.click({ force: true });
        await this.page.waitForSelector("[aria-label ='Custom Logo Text']");
        await this.customLogoText.fill(text);
        await this.page.waitForSelector('[data-test="settings_ent_logo_custom_text_save_btn"]');
        await this.saveButton.click({ force: true });
        await this.page.waitForTimeout(5000);

    }

    async uploadLogo(filePath) {
        const isVisible = await this.deleteLogoButton.isVisible();
        if (isVisible) {
            await this.page.waitForSelector('[data-test="setting_ent_custom_logo_img_delete_btn"]');
            await this.deleteLogoButton.click({ force: true });
            await this.page.waitForSelector('[data-test="logs-search-bar-confirm-dialog-ok-btn"]');
            await this.confirmDeleteButton.click({ force: true });
            
            
        }
        //  await this.fileUploadInput.setInputFiles(filePath);
        await this.page.waitForTimeout(5000);
        await this.page.waitForSelector('[data-test="setting_ent_custom_logo_img_file_upload"]');
        await this.page.setInputFiles('input[ data-test="setting_ent_custom_logo_img_file_upload"]', filePath);
        await this.page.waitForTimeout(5000);


    }
}