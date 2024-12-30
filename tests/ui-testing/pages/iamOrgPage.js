// OrganizationPage.js
import { expect } from '@playwright/test';


export class IamOrgPage {
    constructor(page) {
        this.page = page;
        this.iamMenuLink = page.locator('[data-test="menu-link-\\/iam-item"]');
        this.organizationsTab = page.locator('[data-test="iam-organizations-tab"]');
        this.addOrgButton = page.getByRole('button', { name: 'Add Organization' });
        this.orgNameInput = page.locator('[data-test="org-name"]');
        this.submitButton = page.locator('[data-test="add-org"]');
        this.cancelButton = page.getByRole('button', { name: 'Cancel' });
        this.alert = this.page.getByRole('alert').first();
        this.alertMessage = this.page.getByRole('alert').first();
    }

    async navigateToOrganizations() {
        await this.iamMenuLink.click({force:true});
        await this.organizationsTab.click({force:true});
    }

    async addOrganization(orgName) {
        await this.addOrgButton.click({force:true});
        await this.orgNameInput.fill(orgName);
        await this.submitButton.click();
    }

    async cancelAddOrganization() {
        await this.cancelButton.click();
    }

    async getAlertText() {
        await expect(this.page.getByRole('alert')).toContainText('Organization added successfully.');
    }

    async verifySuccessMessage(expectedMessage) {
        await expect(this.page.locator('role=alert').first()).toBeVisible();
        await expect(this.alertMessage).toContainText(expectedMessage);
    }


    async checkSaveButton() {

        // Check if the button is enabled before clicking
        if (await this.submitButton.isEnabled()) {
            await this.submitButton.click();
            console.log('Button clicked successfully');
            // Verify error message
            await expect(this.page.getByRole('alert')).toContainText('Name is required');

        } else {
            console.log('Button is disabled');
        } 
    }


}


