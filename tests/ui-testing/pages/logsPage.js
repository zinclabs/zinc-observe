// logsPage.js
import { expect } from '@playwright/test';
import{ dateTimeButtonLocator, relative30SecondsButtonLocator, absoluteTabLocator, Past30SecondsValue } from '../pages/CommonLocator.js';

export class LogsPage {
  constructor(page) {

    this.page = page;

    this.orgDropdown = '[data-test="navbar-organizations-select"]';
    this.defaultOrgOption = this.page.getByRole('option', { name: 'default', exact: true }).locator('div').nth(2);
    this.logsMenuItem = page.locator('[data-test="menu-link-\\/logs-item"]');
    this.indexDropDown = page.locator('[data-test="logs-search-index-list"]').getByText('arrow_drop_down');
    this.streamToggle = page.locator('[data-test="log-search-index-list-stream-toggle-default"] div');

    this.filterMessage = page.locator('div:has-text("info Adjust filter parameters and click \'Run query\'")');

    this.dateTimeButton = dateTimeButtonLocator;
   // this.dateTimeButton = process.env["dateTimeButtonLocator"];

    this.relative30SecondsButton = page.locator(relative30SecondsButtonLocator);

    this.sqlModeToggle = this.page.getByLabel('SQL Mode').locator('div').nth(2);

    this.absoluteTab = absoluteTabLocator;

    this.profileButton = page.locator('button').filter({ hasText: (process.env["ZO_ROOT_USER_EMAIL"]) });
    this.signOutButton = page.getByText('Sign Out');

  }

  async selectOrganization() {

    await this.page.locator(this.orgDropdown).getByText('arrow_drop_down').click();


    await this.defaultOrgOption.click();
  }


  async navigateToLogs() {
    // Click on Logs menu item
    await this.logsMenuItem.click();
    //await this.page.waitForTimeout(3000);

  }

  async selectIndexAndStream() {
    // Select index and stream
    await this.indexDropDown.click();
    await this.streamToggle.first().click();
    // await this.page.waitForTimeout(3000);

    await this.streamToggle.nth(2).click();
  }
  /*
    async adjustFilterParameters() {
      await this.page.filterMessage.first().click();
    }
  */

  async setTimeToPast30Seconds() {
    // Set the time filter to the last 30 seconds
    await expect(this.page.locator(this.dateTimeButton)).toBeVisible();
    await this.page.locator(this.dateTimeButton).click();
    await this.relative30SecondsButton.click();
  }

  async verifyTimeSetTo30Seconds() {
    // Verify that the time filter displays "Past 30 Seconds"
    await expect(this.page.locator(this.dateTimeButton)).toContainText(Past30SecondsValue);
  }

  async enableSQLMode() {
    await this.sqlModeToggle.click();
  }

  async setDateTime() {
    await expect(this.page.locator(this.dateTimeButton)).toBeVisible();
    await this.page.locator(this.dateTimeButton).click();
    await this.page.locator(this.absoluteTab).click();
    await this.page.waitForTimeout(1000);

  }

  async fillTimeRange(startTime, endTime) {
    await this.page.getByRole('button', { name: '1', exact: true }).click();
    await this.page.getByLabel('access_time').first().fill(startTime);
    await this.page.getByRole('button', { name: '1', exact: true }).click();
    await this.page.getByLabel('access_time').nth(1).fill(endTime);
    // await this.page.waitForTimeout(1000);
  }

  async verifyDateTime(startTime, endTime) {
   
    await expect(this.page.locator(this.dateTimeButton)).toHaveText(new RegExp(`${startTime}.*${endTime}`));
  }
    //await expect(this.page.locator(this.dateTimeButton)).toContainText(`${startTime} - ${endTime}`);


  async signOut() {
    await this.profileButton.click();
    await this.signOutButton.click();
  }
  
}
