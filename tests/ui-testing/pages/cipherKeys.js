// cipher keys.js
import { expect } from '@playwright/test';


export class CipherKeys {

    constructor(page) {

        this.page = page;

        this.settingsMenu = page.locator('[data-test="menu-link-settings-item"]');
        this.cipherKeyTab = page.locator('[data-test="management-cipher-key-tab"]');
        this.addCipherKeyButton = page.getByRole('button', { name: 'Add Cipher Key' });
        this.nameInput = page.locator('[data-test="add-cipher-key-name-input"]');
        this.secretInput = page.locator('[data-test="add-cipher-key-openobserve-secret-input"]');
        this.continueButton = page.locator('[data-test="add-report-step1-continue-btn"]');
        this.saveButton = page.locator('[data-test="add-cipher-key-save-btn"]');
        this.alert = page.getByRole('alert').first();

         // this.deleteButton = cipherName => `//td[contains(text(),'${cipherName}')]/following-sibling::td/button[@title='Delete Service Account']`;
         this.deleteButton = cipherName => `//td[contains(text(),'${cipherName}')]/following-sibling::td/button[@title='Delete']`;

         this.confirmOkButton = "//div[@class='q-card']/div[2]/button/span[text()='OK']";
         this.cancelButton = "//div[@class='q-card']/div[2]/button/span[text()='Cancel']";
        
         this.updateButton = cipherName => `//td[contains(text(),'${cipherName}')]/following-sibling::td/button[@title='Edit']`;
 
         this.profileButton = page.locator('[data-test="header-my-account-profile-icon"]');
         this.signOutButton = page.getByText('Sign Out');


    }

    async navigateToSettingsMenu() {
        await this.page.waitForSelector('[data-test="menu-link-settings-item"]');
        await this.settingsMenu.click();
      }
    
      async navigateToCipherKeyTab() {
        await this.page.waitForSelector('[data-test="management-cipher-key-tab"]');
        await this.cipherKeyTab.click();
      }

      async addCipherKey() {
        await this.page.waitForSelector('button', { name: 'Add Cipher Key' });
        await this.addCipherKeyButton.click();
       
      }

      async addCipherKeyName(name) {
    
        await this.page.waitForSelector('[data-test="add-cipher-key-name-input"]');
        await this.nameInput.fill(name);  
      }
    

      async addCipherKeyOO(secret) {
        await this.page.waitForSelector('[data-test="add-cipher-key-openobserve-secret-input"]');
        await this.secretInput.click();
        await this.secretInput.fill(secret);
      }

      async addCipherKeyContinue() {
        await this.page.waitForSelector('[data-test="add-report-step1-continue-btn"]');
        await this.continueButton.click();
      }

      async addCipherKeyTink() {
        await this.page.waitForSelector('[data-test="add-cipher-key-auth-method-input"]');
        await this.page.locator('[data-test="cipher-key-encryption-mechanism-step"] [data-test="add-cipher-key-auth-method-input"]').click();

        // await this.page.locator('[data-test="add-cipher-key-auth-method-input"]').click();
        await this.page.getByRole('option', { name: 'Tink KeySet' }).click();
      }

      async addCipherKeyType() {
        await this.page.waitForSelector('[data-test="add-cipher-key-type-input"]');
        await this.page.locator('[data-test="add-cipher-key-type-input"]').getByText('OpenObserve').click();
        await this.page.getByText('Akeyless').click();
      }

      async addCipherKeyTypeURL(abase) {
        await this.page.waitForSelector('[data-test="add-cipher-key-akeyless-baseurl-input"]');
        await this.page.locator('[data-test="add-cipher-key-akeyless-baseurl-input"]').click();
        await this.page.locator('[data-test="add-cipher-key-akeyless-baseurl-input"]').fill(abase);
      }

      async addCipherKeyTypeID(aid) {
        await this.page.waitForSelector('[data-test="add-cipher-key-akeyless-access-id-input"]');
        await this.page.locator('[data-test="add-cipher-key-akeyless-access-id-input"]').click();
        await this.page.locator('[data-test="add-cipher-key-akeyless-access-id-input"]').fill(aid);
      }

      async addCipherKeyTypeKey(akey) {
        await this.page.waitForSelector('[data-test="add-cipher-key-akeyless-access-key-input"]');
        await this.page.locator('[data-test="add-cipher-key-akeyless-access-key-input"]').click();
        await this.page.locator('[data-test="add-cipher-key-akeyless-access-key-input"]').fill(akey);
      }

      async addCipherKeyStatic() {
        await this.page.waitForSelector('[data-test="add-cipher-key-secret-type-input"]');
        await this.page.locator('[data-test="add-cipher-key-secret-type-input"]').click();
        await this.page.getByRole('option', { name: 'Static Secret' }).locator('span').click();
    }

      async addCipherKeyStaticName(astatic) {
        await this.page.waitForSelector('[data-test="add-cipher-key-akeyless-static-secret-name-input"]');
        await this.page.locator('[data-test="add-cipher-key-akeyless-static-secret-name-input"]').click();
        await this.page.locator('[data-test="add-cipher-key-akeyless-static-secret-name-input"]').fill(astatic);
    }


    async addCipherKeyDFC() {
        await this.page.waitForSelector('[data-test="add-cipher-key-secret-type-input"]');
        await this.page.locator('[data-test="add-cipher-key-secret-type-input"]').getByText('Static Secret').click();
        await this.page.getByRole('option', { name: 'DFC' }).click();
    }

    async addCipherKeyNameDFC(nameDFC) {
        await this.page.waitForSelector('[data-test="add-cipher-key-akeyless-dfc-name-input"]');
        await this.page.locator('[data-test="add-cipher-key-akeyless-dfc-name-input"]').click();
        await this.page.locator('[data-test="add-cipher-key-akeyless-dfc-name-input"]').fill(nameDFC);
    }

    async addCipherKeyEncryDataDFC(encryDFC) {
        await this.page.waitForSelector('[data-test="add-cipher-key-akeyless-dfc-encrypted-data-input"]');
        await this.page.locator('[data-test="add-cipher-key-akeyless-dfc-encrypted-data-input"]').click();
        await this.page.locator('[data-test="add-cipher-key-akeyless-dfc-encrypted-data-input"]').fill(encryDFC);
    }

    async addCipherKeySimple() {
        
        await this.page.getByRole('option', { name: 'Simple' }).locator('div').nth(2).click();
     }

    async addCipherKeyStore() {
        
                await this.page.getByText('Key Store Details (Type:').click();
    }

      async addCipherKeySave() {
        await this.page.waitForSelector('[data-test="add-cipher-key-save-btn"]');
        await this.saveButton.click();
      }

      async updateCipherKeys(cipherName) {
        const updateButtonLocator = this.page.locator(this.updateButton(cipherName));
        // Wait for the update button to be visible
        await updateButtonLocator.waitFor({ state: 'visible', timeout: 30000 });
        // Click the update button
        await updateButtonLocator.click({ force: true });
     
    }
    
    async updateCipherKeysSecret() {
        await this.page.waitForSelector('[data-test="add-cipher-key-openobserve-secret-input-update"]');
        await this.page.locator('[data-test="add-cipher-key-openobserve-secret-input-update"]').click();
     
    }

    async updateCipherKeysSecretCancel() {
        await this.page.waitForSelector('[data-test="add-cipher-key-openobserve-secret-input-cancel"]');
        await this.page.locator('[data-test="add-cipher-key-openobserve-secret-input-cancel"]').click();
     
    }

    async updateCipherKeysCancel() {
        await this.page.waitForSelector('[data-test="add-cipher-key-cancel-btn"]');
        await this.page.locator('[data-test="add-cipher-key-cancel-btn"]').click();
     
    }

    async deletedCipherKeys(cipherName) {
        const deleteButtonLocator = this.page.locator(this.deleteButton(cipherName));
        // Wait for the delete button to be visible
        await deleteButtonLocator.waitFor({ state: 'visible', timeout: 30000 });
        // Click the delete button
        await deleteButtonLocator.click({ force: true });

    }

    async requestCipherKeysOk() {
        // Wait for the confirmation button to be visible and click it
        await this.page.locator(this.confirmOkButton).waitFor({ state: 'visible', timeout: 10000 });
        await this.page.locator(this.confirmOkButton).click({ force: true });
        // Add some buffer wait, if necessary
        await this.page.waitForTimeout(2000);
    }

    async requestCipherKeysCancel() {
        // Wait for the cancel confirmation button to be visible and click it
        await this.page.locator(this.cancelButton).waitFor({ state: 'visible', timeout: 10000 });
        await this.page.locator(this.cancelButton).click({ force: true });

        // Add some buffer wait, if necessary
        await this.page.waitForTimeout(2000);


    }

      async verifyAlertMessage(expectedText) {
    
        await expect(this.alert).toContainText(expectedText);
      }
    
      async signOut() {
        await this.profileButton.click();
        await this.signOutButton.click();
      }


}
