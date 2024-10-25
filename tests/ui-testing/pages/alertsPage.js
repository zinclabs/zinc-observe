import { expect } from '@playwright/test';

import{ dateTimeButtonLocator, relative30SecondsButtonLocator } from '../pages/CommonLocator.js';

export  class AlertPage {
    constructor(page) {
      this.page = page;
      this.alertMenu = this.page.locator('[data-test="menu-link-\\/alerts-item"]');
      this.addAlertButton = this.page.locator('[data-test="alert-list-add-alert-btn"]');
      this.sqlOption = this.page.getByText('SQL');
      this.addTimeRangeButton = this.page.locator('[data-test="multi-time-range-alerts-add-btn"]');

    
      this.dateTimeButton = dateTimeButtonLocator;
      this.relative30SecondsButton = page.locator(relative30SecondsButtonLocator);
      

    

    this.profileButton = page.locator('button').filter({ hasText: (process.env["ZO_ROOT_USER_EMAIL"]) });
    this.signOutButton = page.getByText('Sign Out');

  }

  async navigateToAlerts() {

    await this.page.waitForSelector('[data-test="menu-link-\\/alerts-item"]');
    await this.alertMenu.click({ force: true });
  }

  async createAlert() {
    await this.page.waitForSelector('[data-test="alert-list-add-alert-btn"]');
    await this.addAlertButton.click({ force: true });
    // await this.page.waitForSelector
    await this.sqlOption.click({ force: true });
    await this.page.waitForSelector('[data-test="multi-time-range-alerts-add-btn"]');
    await this.addTimeRangeButton.click({ force: true });
    
  }

  async setTimeToPast30Seconds() {
    // Set the time filter to the last 30 seconds
   // await expect(this.page.locator(this.dateTimeButton)).toBeVisible();
    await this.page.waitForSelector(dateTimeButtonLocator);
    await this.page.locator(this.dateTimeButton).click({ force: true });
    await this.page.waitForSelector(relative30SecondsButtonLocator);

    await this.relative30SecondsButton.click({ force: true });
  }

  async verifyTimeSetTo30Seconds() {
    // Verify that the time filter displays "Past 30 Seconds"
   // await expect(this.page.locator(this.dateTimeButton)).toContainText(process.env["Past30SecondsValue"]);
    await expect(this.page.locator(this.dateTimeButton)).toContainText('schedule30 Seconds agoarrow_drop_down');
  }
 
 
  async signOut() {
    await this.profileButton.click({ force: true });
    await this.signOutButton.click({ force: true });
  }
  
}