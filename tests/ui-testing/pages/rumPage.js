// rumPage.js
import { expect } from '@playwright/test';

import { dateTimeButtonLocator, relative30SecondsButtonLocator, absoluteTabLocator, Past30SecondsValue, oneDateMonthLocator } from './CommonLocator.js';


export class RumPage {
  constructor(page) {
    this.page = page;

    this.accessTime = page.getByLabel('access_time');
    this.tabSessions = page.locator('[data-test="tab-sessions"]');
    this.tabErrorTracking = page.locator('[data-test="tab-error_tracking"]');

    this.dateTimeButton = dateTimeButtonLocator;
    this.relative30SecondsButton = page.locator(relative30SecondsButtonLocator);
    this.absoluteTab = absoluteTabLocator;


    this.profileButton = page.locator('button').filter({ hasText: (process.env["ZO_ROOT_USER_EMAIL"]) });
    this.signOutButton = page.getByText('Sign Out');


  }

  async navigateToPerformanceOverview() {
    await this.page.locator('[data-test="menu-link-\\/rum\\/performance\\/overview-item"]').click();
    await this.page.waitForTimeout(3000);  // Wait for the page to load
  }


  async setTimeToPast30Seconds() {
    // Set the time filter to the last 30 seconds
    await this.page.locator(this.dateTimeButton).click();
    await this.relative30SecondsButton.click();
  }

  async verifyTimeSetTo30Seconds() {
    // Verify that the time filter displays "Past 30 Seconds"
    await expect(this.page.locator(this.dateTimeButton)).toContainText(Past30SecondsValue);
  }

  async setDateTime() {
    await expect(this.page.locator(this.dateTimeButton)).toBeVisible();
    await this.page.locator(this.dateTimeButton).click();
    await this.page.locator(this.absoluteTab).click();
    await this.page.waitForTimeout(1000);

  }

  async fillTimeRange(startTime, endTime) {

    await this.page.locator(oneDateMonthLocator).click();
    await this.page.waitForTimeout(3000);

    await this.page.getByLabel('access_time').first().fill(String(startTime));

    // await this.page.getByRole('button', { name: '1', exact: true }).click();
    await this.page.locator(oneDateMonthLocator).click();
    await this.page.waitForTimeout(3000);
    // await this.page.locator('//*[@id="f_e7860c12-4d74-484c-8f83-5e8a16c6adcc"]').fill(String(endTime));
    await this.page.getByLabel('access_time').nth(1).fill(String(endTime));

  }

  async verifyDateTime(startTime, endTime) {
    // await expect(this.page.locator(this.dateTimeButton)).toContainText(`${startTime} - ${endTime}`);
    await expect(this.page.locator(this.dateTimeButton)).toHaveText(new RegExp(`${startTime}.*${endTime}`));
  }

  async navigateToSessionsTab() {
    await this.tabSessions.click();
    await this.page.waitForTimeout(3000);
  }

  async navigateToErrorTrackingTab() {
    await this.tabErrorTracking.click();
    await this.page.waitForTimeout(3000);
  }

  async signOut() {
    await this.profileButton.click();
    await this.signOutButton.click();
  }

}