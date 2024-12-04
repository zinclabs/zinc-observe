// iamPage.js
import { expect } from '@playwright/test';


export class IamPage {
    constructor(page) {
        this.page = page;
        this.iamPageMenu = page.locator('[data-test="menu-link-\\/iam-item"]');

    }
    async gotoIamPage() {

        await this.iamPageMenu.click();


    }


    async iamPageDefaultOrg() {

        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();
        await this.page.getByText('default', { exact: true }).click();


    }

    async iamPageDefaultMultiOrg() {



        await this.page.locator('[data-test="navbar-organizations-select"]').getByText('arrow_drop_down').click();


        await this.page.getByRole('option', { name: 'defaulttestmulti' }).locator('div').nth(2).click();




    }

    async iamPageURLValidation() {

        await expect(this.page).toHaveURL(/defaulttestmulti/);

    }

    async iamURLValidation() {

        await expect(this.page).toHaveURL(/iam/);

    }

}
