import { expect } from '@playwright/test';

export const path = require('path');
export
    class LogoManagementPage {

    constructor(page) {
        this.page = page;

        this.homeIcon = page.locator("[name ='home']");
        this.managementMenuItem = page.locator('[data-test="menu-link-settings-item"]');
        this.submitButton = page.locator('[data-test="dashboard-add-submit"]'); // Add appropriate data-test attribute
        this.editLogoTextButton = page.locator('[data-test="settings_ent_logo_custom_text_edit_btn"]');
        this.customLogoTextBox = page.locator('[data-test="settings_ent_logo_custom_text"]');
        this.customLogoText = page.locator("[aria-label ='Custom Logo Text']");
        this.saveButton = page.locator('[data-test="settings_ent_logo_custom_text_save_btn"]');
        this.deleteLogoButton = page.locator('[data-test="setting_ent_custom_logo_img_delete_btn"]');
        this.confirmDeleteButton = page.locator('[data-test="logs-search-bar-confirm-dialog-ok-btn"]');
        this.fileUploadInput = page.locator('input[data-test="setting_ent_custom_logo_img_file_upload"]');
    }

    async navigateToManagement() {
        await this.page.waitForSelector("[name ='home']");
        await this.homeIcon.hover();
        await this.page.waitForSelector('[data-test="menu-link-settings-item"]');
        await this.managementMenuItem.click({ force: true });
    }

    async goToManagement() {
        await this.managementMenuItem.click({ force: true });
        await expect(this.page.getByRole('main')).toContainText('Management');
    }

    async managementPageDefaultMultiOrg() {
        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();
    }

    async managementPageURLValidation() {
        await expect(this.page).toHaveURL(/defaulttestmulti/);
    }

    async managementURLValidation() {
        await expect(this.page).toHaveURL(/settings/);
    }

    async clickSaveSubmit() {
        await this.submitButton.waitFor({ state: 'visible' });
        await this.submitButton.click({ force: true });   
    }


    async updateCustomLogoText(text) {
        const isVisible = await this.editLogoTextButton.isVisible();
        if (isVisible) {
        
        
        await this.page.locator('[data-test="settings_ent_logo_custom_text_edit_btn"]').click();
        
        }
    
    await expect(this.page.getByText('Unauthorized Access')).not.toBeVisible();
    await this.customLogoTextBox.click();
    await expect(this.page.getByText('Unauthorized Access')).not.toBeVisible();
    await this.customLogoTextBox.fill(text);
    await this.page.waitForSelector('[data-test="settings_ent_logo_custom_text_save_btn"]');
    await this.saveButton.click({ force: true });

}
    async uploadLogo(filePath) {
        const isVisible = await this.deleteLogoButton.isVisible();
        if (isVisible) {
            await this.page.waitForSelector('[data-test="setting_ent_custom_logo_img_delete_btn"]');
            await this.deleteLogoButton.click({ force: true });
            await this.page.waitForSelector('[data-test="logs-search-bar-confirm-dialog-ok-btn"]');
            await this.confirmDeleteButton.click({ force: true });
        }
        await this.page.goto(process.env["ZO_BASE_URL"] + "/web/settings/general?org_identifier=default");
        await this.page.waitForSelector('[data-test="setting_ent_custom_logo_img_file_upload"]');
        await this.page.setInputFiles('input[ data-test="setting_ent_custom_logo_img_file_upload"]', filePath);
        await expect(this.page.getByText("Logo updated successfully")).toBeVisible({
            timeout: 30000,
        });


    }
}
