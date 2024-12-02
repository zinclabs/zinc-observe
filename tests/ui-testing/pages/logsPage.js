// logsPage.js
import { expect } from '@playwright/test';
const { chromium } = require('playwright');
import { dateTimeButtonLocator, relative30SecondsButtonLocator, absoluteTabLocator, Past30SecondsValue } from '../pages/CommonLocator.js';

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
    this.relative30SecondsButton = page.locator(relative30SecondsButtonLocator);

    this.sqlModeToggle = this.page.getByLabel('SQL Mode').locator('div').nth(2);

    this.absoluteTab = absoluteTabLocator;

    this.homeButton = "[name ='home']";


    this.timeZoneDropdown = "//label[contains(@class,'timezone-select')]";
    this.timeZoneOption = (zone) => `role=option[name="${zone}"]`;
    this.dateSelector = (day) => `//div[contains(@class,'q-date__calendar-item--in')]/button/span[contains(@class,'q-btn__content')]/span[text()='${day}']`;
    this.monthSelector = (month) => `//span[text() ='${month}']`;
    this.yearSelector = (year) => `//span[text() ='${year}']`;
    this.startTimeField = "input[class = 'q-field__native q-placeholder']:nth-of-type(2)";
    this.endTimeField = "input[class = 'q-field__native q-placeholder']:nth-of-type(3)";
    this.scheduleText = "//i[text() ='schedule']/following-sibling::span";
    this.calendarDate = "//div[contains(@class,'q-date__calendar-item--in')]/button/span[contains(@class,'q-btn__content')]/span[text()='";
    this.startTimeInput = "input[class = 'q-field__native q-placeholder']:nth-child(2)";
    this.endTimeInput = "input[class = 'q-field__native q-placeholder']:nth-child(3)";
    this.timezoneSelector = "//label[contains(@class,'timezone-select')]";

    this.streamDropdown = '[data-test="log-search-index-list-select-stream"]';
    this.queryButton = "[data-test='logs-search-bar-refresh-btn']";
    this.queryEditor = '[data-test="logs-search-bar-query-editor"]';

    this.profileButton = page.locator('button').filter({ hasText: (process.env["ZO_ROOT_USER_EMAIL"]) });
    this.signOutButton = page.getByText('Sign Out');

  }

  async selectOrganization() {
    await this.page.locator(this.orgDropdown).getByText('arrow_drop_down').click();
    await this.defaultOrgOption.click();
  }


  async navigateToLogs() {
    // Click on Logs menu item
    await this.page.locator(this.homeButton).hover();
    await this.logsMenuItem.click();
    await this.page.waitForTimeout(3000);
    await expect(this.page.locator('[data-test="logs-search-no-stream-selected-text"]')).toContainText('info Select a stream and press \'Run query\' to continue. Additionally, you can apply additional filters and adjust the date range to enhance search.');
  }

  async selectIndexAndStream() {
    // Select index and stream
    await this.indexDropDown.click();
    await this.streamToggle.first().click();
    // await this.page.waitForTimeout(3000);
    await this.streamToggle.nth(2).click();
  }

  async selectIndexAndStreamJoin() {
    // Select index and stream
    //await this.page.locator('[data-test="logs-search-field-list-collapse-btn"]').click();
    //await this.page.locator('[data-test="logs-search-field-list-collapse-btn"]').click();

    await this.page.locator('[data-test="logs-search-index-list"]').getByText('arrow_drop_down').click();
    await this.page.waitForTimeout(3000);
    await this.page.locator('[data-test="log-search-index-list-stream-toggle-default"] div').first().click();
    await this.page.waitForTimeout(3000);
    await this.page.locator('[data-test="log-search-index-list-stream-toggle-e2e_automate"] div').first().click();
    await this.page.waitForTimeout(3000);
  }

  async selectRunQuery() {

    await this.page.locator(this.queryButton).click();
    await this.page.waitForTimeout(5000);
  }

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

  async setAbsoluteDate(year, month, day, currentMonth, currentYear) {
    // await this.page.locator(this.dateTimeButton).click();
    // await this.page.getByText(" Absolute ").click();

    if (currentMonth.includes(month) && year === currentYear.toString()) {
      await this.page.locator(this.calendarDate + day + "']").dblclick();
    } else if (year === currentYear.toString()) {
      await this.page.getByText(currentMonth).click();
      await this.page.locator("//span[text() ='" + month + "']").click();
      await this.page.locator("//span[text() ='" + day + "']").dblclick();
    } else {
      await this.page.getByText(currentMonth).click();
      await this.page.locator("//span[text() ='" + month + "']").click();
      await this.page.locator("//span[text() ='" + currentYear + "']").click();
      await this.page.locator("//span[text() ='" + year + "']").click();
      await this.page.locator("//span[text() ='" + day + "']").dblclick();
    }
  }

  async setStartAndEndTime(startTime, endTime) {
    await this.page.locator(this.startTimeInput).fill(startTime);
    await this.page.locator(this.endTimeInput).fill(endTime);
  }

  async setTimeRange(startTime, endTime) {
    await this.page.locator(this.startTimeField).fill(startTime);
    await this.page.locator(this.endTimeField).fill(endTime);
  }

  async verifySchedule(expectedTime) {
    const schedule = await this.page.locator(this.scheduleText).textContent();
    expect(schedule).toEqual(expectedTime);
  }

  async setTimeZone(zone) {
    await this.page.locator(this.timeZoneDropdown).click();
    await this.page.getByRole(this.timeZoneOption(zone)).click();
  }

  async changeTimeZone() {
    await this.page.locator("//label[contains(@class,'timezone-select')]").click()
    await this.page.getByRole('option', { name: 'Asia/Dubai' }).click()
  }

  async verifyDateTime(startTime, endTime) {
    await expect(this.page.locator(this.dateTimeButton)).toContainText(`${startTime} - ${endTime}`);
  }

  async copyShare() {
    const browser = await chromium.launch();

    const context = await browser.newContext({
      permissions: ['clipboard-read', 'clipboard-write']
    });
    const page = await context.newPage();
    await page.locator("[title ='Share Link']").click();
    const copiedUrl = await page.evaluate(() => navigator.clipboard.readText())
    console.log(copiedUrl);
    // console.log(Copied text: ${copiedUrl})
    const newpage = await context.newPage()
    await newpage.waitForTimeout(5000)
    await newpage.goto(copiedUrl)
    console.log(newpage.url())
    expect(copiedUrl).toContain('short')
  }

  async getScheduleText() {
    return await this.page.locator("//button[(@data-test ='date-time-btn')]/span[contains(@class,'q-btn__content')]/span").textContent();
  }

  async selectStream(stream) {
    await this.page.waitForTimeout(4000);
    await this.page.locator(this.streamDropdown).click({ force: true });
    await this.page.locator("div.q-item").getByText(`${stream}`).first().click({ force: true });
  }

  async applyQuery() {
    const search = this.page.waitForResponse("**/api/default/_search**");
    await this.page.waitForTimeout(3000);
    await this.page.locator(this.queryButton).click({ force: true });
    await expect.poll(async () => (await search).status()).toBe(200);
  }

  async clearAndRunQuery() {
    await this.page.locator(this.queryButton).click();
    await this.page.getByLabel("SQL Mode").locator("div").nth(2).click();
    await this.page.locator(this.queryEditor).click();
    await this.page.keyboard.press(process.platform === "darwin" ? "Meta+A" : "Control+A");
    await this.page.keyboard.press("Backspace");
    await this.page.waitForTimeout(3000);
    await this.page.locator(this.queryButton).click();
    await this.page.getByText("No column found in selected stream.").click();
  }


  async kubernetesContainerName() {
    await this.page.getByLabel('Expand "kubernetes_container_name"').click();
    await this.page.waitForTimeout(5000);
    await this.page.locator('[data-test="logs-search-subfield-add-kubernetes_container_name-ziox"] [data-test="log-search-subfield-list-equal-kubernetes_container_name-field-btn"]').click();

  }
  
  async kubernetesContainerNameJoin() {
    await this.page
    .locator('[data-test="logs-search-bar-query-editor"]')
    .getByLabel("Editor content;Press Alt+F1")
    .fill('SELECT a.kubernetes_container_name , b.kubernetes_container_name  FROM "default" as a join "e2e_automate" as b on a.kubernetes_container_name  = b.kubernetes_container_name');
    await this.page.waitForTimeout(5000);
  }

  async kubernetesContainerNameJoinLimit() {
    await this.page
    .locator('[data-test="logs-search-bar-query-editor"]')
    .getByLabel("Editor content;Press Alt+F1")
    .fill('SELECT a.kubernetes_container_name , b.kubernetes_container_name  FROM "default" as a join "e2e_automate" as b on a.kubernetes_container_name  = b.kubernetes_container_name LIMIT 10');
    await this.page.waitForTimeout(5000);
  }


  async kubernetesContainerNameJoinLike() {
    await this.page
    .locator('[data-test="logs-search-bar-query-editor"]')
    .getByLabel("Editor content;Press Alt+F1")
    .fill('SELECT a.kubernetes_container_name , b.kubernetes_container_name  FROM "default" as a join "e2e_automate" as b on a.kubernetes_container_name  = b.kubernetes_container_name WHERE a.kubernetes_container_name LIKE "%ziox%"');
    await this.page.waitForTimeout(5000);
  }

  async kubernetesContainerNameLeftJoin() {
    await this.page
    .locator('[data-test="logs-search-bar-query-editor"]')
    .getByLabel("Editor content;Press Alt+F1")
    .fill('SELECT a.kubernetes_container_name , b.kubernetes_container_name  FROM "default" as a LEFT JOIN "e2e_automate" as b on a.kubernetes_container_name  = b.kubernetes_container_name');
    await this.page.waitForTimeout(5000);
  }

  async kubernetesContainerNameRightJoin() {
    await this.page
    .locator('[data-test="logs-search-bar-query-editor"]')
    .getByLabel("Editor content;Press Alt+F1")
    .fill('SELECT a.kubernetes_container_name , b.kubernetes_container_name  FROM "default" as a RIGHT JOIN "e2e_automate" as b on a.kubernetes_container_name  = b.kubernetes_container_name');
    await this.page.waitForTimeout(5000);
  }

  async kubernetesContainerNameFullJoin() {
    await this.page
    .locator('[data-test="logs-search-bar-query-editor"]')
    .getByLabel("Editor content;Press Alt+F1")
    .fill('SELECT a.kubernetes_container_name , b.kubernetes_container_name  FROM "default" as a FULL JOIN "e2e_automate" as b on a.kubernetes_container_name  = b.kubernetes_container_name');
    await this.page.waitForTimeout(5000);
  }


  async signOut() {
    await this.profileButton.click();
    await this.signOutButton.click();
  }

}
