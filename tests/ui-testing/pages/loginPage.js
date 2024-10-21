// loginPage.js
import { expect } from '@playwright/test';

export class LoginPage {
  constructor(page) {
    this.page = page;
    this.userIdInput = page.locator('[data-cy="login-user-id"]');
    
    this.passwordInput = page.locator('[data-cy="login-password"]');
    //this.loginButton = this.page.getByRole('button', { name: 'Login' });
  }

  async gotoLoginPage() {
    await this.page.goto(process.env["ZO_BASE_URL"]);
  }

  async login(username, password) {
    // Fill in user credentials and login
    //await this.page.getByText('Login as internal user').click();
   // 
    await this.userIdInput.fill(username);
    await this.page.waitForTimeout(3000);
    await this.passwordInput.fill(password);
    //await this.loginButton.click();
    await this.page.locator('[data-cy="login-sign-in"]').click();

  }
}
