// loginPage.js
import { expect } from '@playwright/test';
export class LoginPage {
  constructor(page) {
    this.page = page;
    this.userIdInput = page.locator('[data-test="login-user-id"]');
    this.passwordInput = page.locator('[data-test="login-password"]');
    this.loginButton = page.locator('[data-cy="login-sign-in"]');
  }
  async gotoLoginPage() {
    await this.page.goto(process.env["ZO_BASE_URL"]);
    console.log("ZO_BASE_URL", process.env["ZO_BASE_URL"]);
  }

  async loginAsInternalUser() {

    if (await this.page.getByText('Login as internal user').isVisible()) {

       await this.page.getByText('Login as internal user').click();
       await this.page.waitForURL(process.env["ZO_BASE_URL"] + "/web/login", {
        waitUntil: "networkidle",
        });
   
    }
  
    
  }

  async login() {
    await this.userIdInput.fill(process.env["ZO_ROOT_USER_EMAIL"]);
    const waitForLogin = this.page.waitForResponse(
      (response) =>
        response.url().includes("/auth/login") && response.status() === 200
    );
    await this.passwordInput.fill(process.env["ZO_ROOT_USER_PASSWORD"]);
    await this.waitForLogin;
    await this.loginButton.click();
    await this.page.waitForURL(process.env["ZO_BASE_URL"] + "/web/", {
      waitUntil: "networkidle",
    });   
  }

  async gotoLoginPageSC() {
    await this.page.goto(process.env["ZO_BASE_URL_SC_UI"]);
    console.log("ZO_BASE_URL_SC_UI", process.env["ZO_BASE_URL_SC_UI"]);
  }

  async loginAsInternalUserSC() {

    if (await this.page.getByText('Login as internal user').isVisible()) {

      await this.page.getByText('Login as internal user').click();
      await this.page.waitForURL(process.env["ZO_BASE_URL_SC_UI"] + "/web/login", {
        waitUntil: "networkidle",
        });
  
   }
  
    
    
    
  }

  async loginSC() {
    await this.userIdInput.fill(process.env["ZO_ROOT_USER_EMAIL"]);
    const waitForLogin = this.page.waitForResponse(
      (response) =>
        response.url().includes("/auth/login") && response.status() === 200
    );
    await this.passwordInput.fill(process.env["ZO_ROOT_USER_PASSWORD"]);
    await this.waitForLogin;
    await this.loginButton.click();
    await this.page.waitForURL(process.env["ZO_BASE_URL_SC_UI"] + "/web/", {
      waitUntil: "networkidle",
    });  
  }



}