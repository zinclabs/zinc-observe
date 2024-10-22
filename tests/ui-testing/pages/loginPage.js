// loginPage.js
import { expect } from '@playwright/test';

export class LoginPage {
  constructor(page) {
    this.page = page;
    this.userIdInput = page.locator('[data-test="login-user-id"]');
    this.passwordInput = page.locator('[data-test="login-password"]');
    this.loginButton = this.page.getByRole('button', { name: 'Login' });
  }

  async gotoLoginPage() {
    await this.page.goto(process.env["ZO_BASE_URL"]);
  }

  async login(username, password) {
    // Fill in user credentials and login
    await this.userIdInput.fill(username);
    await this.passwordInput.fill(password);
    await this.loginButton.click();
  }
}
