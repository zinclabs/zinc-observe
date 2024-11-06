import { expect } from '@playwright/test';
import {
  dateTimeButtonLocator, relative30SecondsButtonLocator, absoluteTabLocator,
  Past30SecondsValue
} from '../pages/CommonLocator.js';
export class ReportsPage {
  constructor(page) {
    this.page = page;
    this.homeMenu = page.locator("[name ='home']");
    this.reportsMenu = page.locator("[name='reports']");
    this.scheduledTab = page.locator("[title='Scheduled']");
    this.addReportButton = page.locator('[data-test="report-list-add-report-btn"]');
    this.reportNameInput = page.locator("[aria-label='Name *']");
    this.folderInput = page.locator("[aria-label='Folder *']");
    this.dashboardInput = page.locator("[aria-label='Dashboard *']");
    this.dashboardTabInput = page.locator("[aria-label='Dashboard Tab *']");
    this.continueButtonStep1 = page.locator('[data-test="add-report-step1-continue-btn"]');
    this.continueButtonStep2 = page.locator('[data-test="add-report-step2-continue-btn"]');
    this.titleInput = page.locator("[aria-label='Title *']");
    this.recipientsInput = page.locator("[aria-label='Recipients *']");
    this.saveButton = page.locator('[data-test="add-report-save-btn"]');
    this.successAlert = page.getByRole('alert').nth(1);
    this.dateTimeButton = dateTimeButtonLocator;
    this.relative30SecondsButton = page.locator(relative30SecondsButtonLocator);
    this.absoluteTab = absoluteTabLocator;
    this.profileButton = page.locator('button').filter({ hasText: (process.env["ZO_ROOT_USER_EMAIL"]) });
    this.signOutButton = page.getByText('Sign Out');
  }
  async navigateToReports() {
    await this.homeMenu.hover();
    await this.reportsMenu.click();
    await this.scheduledTab.click();
  }
  async clickAddReportButton() {
    await this.page.waitForSelector('[data-test="report-list-add-report-btn"]');
    await this.addReportButton.click();
  }

    async createReport(dashboardName) {
    await this.page.waitForSelector("[aria-label='Name *']");
    await this.reportNameInput.fill("rreport1");
    await this.page.waitForTimeout(5000);
    await this.folderInput.dblclick();
    await this.page.waitForLoadState("networkidle");
    await this.folderInput.pressSequentially('de', { delay: 100 });
    await this.page.getByRole('option', { name: 'default' }).click();
    await this.dashboardInput.dblclick();
    await this.page.waitForLoadState("networkidle");
    await this.dashboardInput.pressSequentially(dashboardName, { delay: 100 });
    await this.page.getByRole('option', { name: dashboardName }).click();
    await this.dashboardTabInput.dblclick();
    await this.page.waitForLoadState("networkidle");
    await this.dashboardTabInput.pressSequentially('de', { delay: 100 });
    await this.page.getByRole('option', { name: 'default' }).click();
  }
  async createReportSetting() {
    await this.continueButtonStep1.click();
    await this.continueButtonStep2.click();
    await this.titleInput.fill("reporterTest");
    await this.recipientsInput.fill(process.env["ZO_ROOT_USER_EMAIL"]);
    await this.saveButton.click();
  }

  async verifyReportSaved() {
    await expect(this.successAlert).toContainText('Report saved successfully.');
  }
  async deleteReport(reportName) {
    await this.page.locator('[data-test="report-list-search-input"]').fill(reportName);
    await this.page
      .locator(`[data-test="report-list-${reportName}-delete-report"]`)
      .click();
    await this.page.locator('[data-test="confirm-button"]').click();
  }
  async setTimeToPast30Seconds() {
    // Set the time filter to the last 30 seconds
    await this.page.locator(this.dateTimeButton).click();
    await this.relative30SecondsButton.click();
    await this.page.waitForTimeout(5000);
  }

  async changeTimeZone() {
    // Set the time zone
    await this.page.locator('label').filter({ hasText: 'Timezonearrow_drop_down' }).locator('i').click();
    await this.page.getByRole('option', { name: 'Asia/Chita' }).locator('div').nth(2).click();
    await this.page.waitForTimeout(5000);
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
    await this.page.getByRole('button', { name: '1', exact: true }).click();
    await this.page.getByLabel('access_time').first().fill(startTime);
    await this.page.getByRole('button', { name: '1', exact: true }).click();
    await this.page.getByLabel('access_time').nth(1).fill(endTime);

  }

  async verifyDateTime(startTime, endTime) {
    await expect(this.page.locator(this.dateTimeButton)).toContainText(`${startTime} - ${endTime}`);
  }

  async signOut() {
    await this.profileButton.click();
    await this.signOutButton.click();
  }
}


