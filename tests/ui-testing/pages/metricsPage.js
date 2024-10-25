import { expect } from '@playwright/test';

import { dateTimeButtonLocator, relative30SecondsButtonLocator, absoluteTabLocator, Past30SecondsValue, currentMonth,
  oneDateMonthLocator, currentMon } from '../pages/CommonLocator.js';

export  class MetricsPage {

  constructor(page) {
    this.page = page;

    this.metricsMenuLink = page.locator('[data-test="menu-link-\\/metrics-item"]');
    	this.indexDropdown = page.locator('[data-test="logs-search-index-list"]').getByText('arrow_drop_down');
    	this.searchIndexInput = page.locator('[data-test="log-search-index-list-select-stream"]');


    this.dateTimeButton = dateTimeButtonLocator;
    this.relative30SecondsButton = page.locator(relative30SecondsButtonLocator);
    this.absoluteTab = absoluteTabLocator;

   
    this.profileButton = page.locator('button').filter({ hasText: (process.env["ZO_ROOT_USER_EMAIL"]) });
    this.signOutButton = page.getByText('Sign Out');

  }

  async openMetricsPage() {
    await this.page.waitForSelector('[data-test="menu-link-\\/metrics-item"]');
    	await this.metricsMenuLink.click({ force: true });
	}

	async selectIndex(indexName) {
    await this.page.waitForSelector('[data-test="logs-search-index-list"]');
    	await this.indexDropdown.click({ force: true });
      await this.page.waitForSelector('[data-test="log-search-index-list-select-stream"]');
    	await this.searchIndexInput.fill(indexName);
    	await this.page.getByText(indexName).click({ force: true });
	}


  async setTimeToPast30Seconds() {
    // Set the time filter to the last 30 seconds
    await this.page.locator(this.dateTimeButton).click({ force: true });
    await this.relative30SecondsButton.click({ force: true });
  }

  async verifyTimeSetTo30Seconds() {
    // Verify that the time filter displays "Past 30 Seconds"
    await expect(this.page.locator(this.dateTimeButton)).toContainText(Past30SecondsValue);
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
    await expect(this.page.locator(this.dateTimeButton)).toContainText(Past30SecondsValue);
  }


async setDateTime() {
   
    //await expect(this.page.locator(this.dateTimeButton)).toBeVisible();
    await this.page.waitForSelector(dateTimeButtonLocator);
    await this.page.locator(this.dateTimeButton).click({ force: true });
    await this.page.waitForSelector(absoluteTabLocator);
    await this.page.locator(this.absoluteTab).click({ force: true });


  }
  async fillTimeRange(startTime, endTime) {

    await this.page.getByText(currentMonth).click();
   
    await this.page.locator("//span[text() ='"+currentMon+"']").click();


    await this.page.locator(oneDateMonthLocator).dblclick();

    //await this.page.getByRole('button', { name: '1', exact: true }).click();

    await this.page.getByLabel('access_time').first().fill(startTime);

    //await this.page.getByRole('button', { name: '1', exact: true }).click();
    //await this.page.locator(oneDateMonthLocator).nth(1).click({ force: true });

    await this.page.getByLabel('access_time').nth(1).fill(endTime);

  }


  async verifyDateTime(startTime, endTime) {
    // await expect(this.page.locator(this.dateTimeButton)).toContainText(`${startTime} - ${endTime}`);
     await expect(this.page.locator(this.dateTimeButton)).toHaveText(new RegExp(`${startTime}.*${endTime}`));
   }
 
   async signOut() {
     await this.profileButton.click({ force: true });
     await this.signOutButton.click({ force: true });
   }

}

