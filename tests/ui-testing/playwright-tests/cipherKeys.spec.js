import { test, expect } from "./baseFixtures.js";
import { LoginPage } from '../pages/loginPage.js';
import { CipherKeys } from "../pages/cipherKeys.js";
import { IngestionPage } from '../pages/ingestionPage.js';
import { LogsPage } from '../pages/logsPage.js';



test.describe("Cipher Keys for security", () => {

    let loginPage, ingestionPage, cipherKeys, logsPage;
    // const cipherName = `c${Date.now()}`;
    const cipherName = `c${Date.now()}-${Math.random().toString(36).substring(7)}`;
    const simpleSecret = process.env['SIMPLE_KEYS'];
    const simpleSecretUp = process.env['SIMPLE_KEYS_UP'];

    
    const tinkeySecret = `{
      "primaryKeyId": ${process.env['PRIMARY_KEY_ID']},
      "key": [{
        "keyData": {
          "typeUrl": "type.googleapis.com/google.crypto.tink.AesSivKey",
          "value": "${process.env['PRIMARY_KEY_ID_VAL']}",
          "keyMaterialType": "SYMMETRIC"
        },
        "status": "ENABLED",
        "keyId": ${process.env['PRIMARY_KEY_ID']},
        "outputPrefixType": "TINK"
      }]
    }`;
    
    
    const tinkeySecretUp = `{
      "primaryKeyId": ${process.env['PRIMARY_KEY_ID_UP']},
      "key": [{
        "keyData": {
          "typeUrl": "type.googleapis.com/google.crypto.tink.AesSivKey",
          "value": "${process.env['PRIMARY_KEY_ID_VAL_UP']}",
          "keyMaterialType": "SYMMETRIC"
        },
        "status": "ENABLED",
        "keyId": ${process.env['PRIMARY_KEY_ID_UP']},
        "outputPrefixType": "TINK"
      }]
    }`;
    
    const baseURL = `https://api.akeyless.io`;
    const accessID = `p-c7k3ogiwk1z9am`;
    const accessKey = `tT5/Q0SrSyL80E3g7tU7PSymsG2m24s3EaYiCRl5VFc=`;
    const staticSecretSimple = `SimpleOO`;
    const staticSecretSimpleUp = `SimpleUp`;
    const staticSecretTink = `Test`;
    const staticSecretTinkUp = `Testj`;
    const simpleNameDFC = `TeD`;
    const simpleEncryDataDFCInvalid = 'Invalid';
    const simpleEncryDataTedDFC = 'AQAAAAEIAUcEh+OcmRtihrzgoqFOsjEvf0DARDAaYoSW1ihdKi2jjaZH4F9Zdd0fhgb8vnt+jj4BVeYvIZidsKOPqvEmnWmsxf+5sv/6WLgN07f3g80TiKfKEkmmuddB7SUFwCabKiXdS2EIF+g6kyE6HqXsY/p+69977ws4yxoMtcQ=';
    
    const simpleUpNameDFC = `SimpleUpdatedDFC`;
    const simpleUpEncryDataTedDFC = 'AQAAAAEIAbJfEaOmJpfW+2SUcfjA1sNItdO03NbtFGY+R2wMWVndCSh61yUrauCW+7KX8L1m5PZX538Yk1ME8olTCb5lauz01jJzMa8eI5fWtTMcR8GMN5NqW6/Cm6xODSWr3n6rpPaMChNDPdj6Ek+5mLjFQhsyz7Dp17fGMC7bSbA=';
    
    const tinkDFC = `loc/tinkfolder`;
    const tinkEncryDataTedDFC = 'AQAAAAEIAUnK/tp8YfW+dOTV6LPj4b9xYj3aN3c1hEFKB3TxxwRqE8gBLZFN/vg+Q9zxoTD5UqXNRyX2WuAXh5Qwuh3fz9kNwM9wUP47/Ju/ZYMIhEM+ENRRfoMnV5txR0IBoU0k7H1KD59EPptx6S/+B1/bRO4h+HDjYhsPeZFLtP7JmV+lksLpbkia3OGHAtXqYIOnhRTkIuAF5ILbrNGruUm4xVNxuZFKltDcB+4X0DWW5IAUC45TXR2OOAO7skPKjiDAgCF493MjXrTYh6AaPTXN/zMIAvF1NzK8BOTe2KGidCKJ0oNKKarA5gyToAAkkKcWEqjW3RsWqkQF1Yip4xZYM+TQvMa/vhUQW1hWAumOJKqbWqvLifPfOERkFW3AgxVQH4RtMN9/gtXjI6OvmPYiEQIgK0c3GldtKYfQA+aAuzm6fVR0D6y0miH9+F1U0/o9tHIJTg==';
    const tinkUpDFC = `tinkUpDFC`;
    const tinkUpEncryDataTedDFC = 'AQAAAAEIAYJ44b2lLPUHw8RwCfILD9wr/u6xDvBN7yaO65c0iUcJpxB4kepFpOq1ehqmxsZIyeeqy+n9CZXRqub6tTd1J33nxqP0LOi6n/pZT4fu2OcOE55ikgQRCxaRonwSF4GowD/49RphA8YqCjlAePRLa6xWbnOntfgkWQ+wZ1+dlAePIrY2sH43wREvMG301C+bTYgl8KnlPtf0hDqFt1CAj3/Mzd2XNEWxE+lLax6nNS+SVM2kz3DyE7kNgoropH/+rfAxUFyckDBgIQf32ud62LZhA3qaXvKowgmrxlSKPxmEiJ7hJgmJYWGcyf2EqZHcPjKXvP19lyYU64lNsRMDjna63YtrO8bsrnj6qdFWGKuUIdmOZq9pdnH50c9TpCE3CcmiBnpVOZd2jevIPe1Cn4G4ws35weNDbACDAPe7WODRUWjaWFJD1U/zRZ/Exc8mFFSvJA==';



    test.beforeEach(async ({ page }) => {
        loginPage = new LoginPage(page);
        logsPage = new LogsPage(page);
        ingestionPage = new IngestionPage(page);
        cipherKeys = new CipherKeys(page);
        await loginPage.gotoLoginPage();
        await loginPage.loginAsInternalUser();
        await loginPage.login();
        await ingestionPage.ingestionJoin();

    });

    test("Tink cipher keys created, updated and deleted to store at OpenObserve", async ({ page }) => {
      
      await cipherKeys.navigateToSettingsMenu();
        // Navigate to Cipher Key Management
      await cipherKeys.navigateToCipherKeyTab();
  
        // Add Cipher Key
      
      await cipherKeys.addCipherKey();
      await cipherKeys.addCipherKeyName(cipherName);
      await cipherKeys.addCipherKeyOO(tinkeySecret);
      await cipherKeys.addCipherKeyContinue();
      await cipherKeys.addCipherKeyTink();
      await cipherKeys.addCipherKeySave();
      await cipherKeys.verifyAlertMessage('check_circleCipher key created successfully');
      await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
      await cipherKeys.navigateToSettingsMenu();
      await cipherKeys.navigateToCipherKeyTab();
      await cipherKeys.updateCipherKeys(cipherName);
      await cipherKeys.updateCipherKeysSecret();
      await cipherKeys.updateCipherKeysSecretCancel();
      await cipherKeys.updateCipherKeysCancel();
      await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
      await cipherKeys.navigateToSettingsMenu();
      await cipherKeys.navigateToCipherKeyTab();
      await cipherKeys.updateCipherKeys(cipherName);
      await cipherKeys.updateCipherKeysSecret();
      await cipherKeys.addCipherKeyOO(tinkeySecretUp); 
      await cipherKeys.addCipherKeyContinue();
      await cipherKeys.addCipherKeyTink();
      await cipherKeys.addCipherKeySave();
      await cipherKeys.verifyAlertMessage('check_circleCipher key updated successfully');
      
      await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
      await cipherKeys.navigateToSettingsMenu();
      await cipherKeys.navigateToCipherKeyTab();
      await cipherKeys.deletedCipherKeys(cipherName);
      await cipherKeys.requestCipherKeysCancel();
      await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
      await cipherKeys.navigateToSettingsMenu();
      await cipherKeys.navigateToCipherKeyTab();
      await cipherKeys.deletedCipherKeys(cipherName);
      await cipherKeys.requestCipherKeysOk();
      await cipherKeys.verifyAlertMessage('check_circleCipher Key deleted successfully');


  
      });
  

    test("Simple cipher keys created, updated and deleteded to store at OpenObserve", async ({ page }) => {
    await cipherKeys.navigateToSettingsMenu();
      // Navigate to Cipher Key Management
    await cipherKeys.navigateToCipherKeyTab();

    console.log('simpleSecret:', simpleSecret);
    console.log('simpleSecretUp:', simpleSecretUp);

      // Add Cipher Key
    
    await cipherKeys.addCipherKey();
    await cipherKeys.addCipherKeyName(cipherName);
  
    await cipherKeys.addCipherKeyOO(simpleSecret);
    await cipherKeys.addCipherKeyContinue();
    await cipherKeys.addCipherKeySave();
    await cipherKeys.verifyAlertMessage('check_circleCipher key created successfully');
    await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
    await cipherKeys.navigateToSettingsMenu();
    await cipherKeys.navigateToCipherKeyTab();
    await cipherKeys.updateCipherKeys(cipherName);
    await cipherKeys.updateCipherKeysSecret();
    await cipherKeys.updateCipherKeysSecretCancel();
    await cipherKeys.updateCipherKeysCancel();
    await cipherKeys.signOut();

    await loginPage.gotoLoginPageSc();
    await loginPage.loginAsInternalUserSc();
    await loginPage.loginSc();
    await cipherKeys.navigateToSettingsMenu();
    await cipherKeys.navigateToCipherKeyTab();
    await page.goto(process.env["ZO_BASE_URL_SC"] + "/web/settings/cipher_keys?org_identifier=default");
    await cipherKeys.navigateToSettingsMenu();
    await cipherKeys.navigateToCipherKeyTab();
    await cipherKeys.updateCipherKeys(cipherName);
    await cipherKeys.updateCipherKeysSecret();
    await cipherKeys.addCipherKeyOO(simpleSecretUp);
    await cipherKeys.addCipherKeyContinue();
    await cipherKeys.addCipherKeySave();
    await cipherKeys.verifyAlertMessage('check_circleCipher key updated successfully');

    await logsPage.navigateToLogs();
    await logsPage.selectIndexDefault();
    await logsPage.decryptLogSQL(cipherName);
    await logsPage.enableSQLMode();
    await logsPage.selectRunQuery();
    await logsPage.validateDecryResult(cipherName);
    


    await page.goto(process.env["ZO_BASE_URL_SC"] + "/web/settings/cipher_keys?org_identifier=default");
    await cipherKeys.navigateToSettingsMenu();
    await cipherKeys.navigateToCipherKeyTab();
    await cipherKeys.deletedCipherKeys(cipherName);
    await cipherKeys.requestCipherKeysCancel();
    await page.goto(process.env["ZO_BASE_URL_SC"] + "/web/settings/cipher_keys?org_identifier=default");
    await cipherKeys.navigateToSettingsMenu();
    await cipherKeys.navigateToCipherKeyTab();
    await cipherKeys.deletedCipherKeys(cipherName);
    await cipherKeys.requestCipherKeysOk();
    await cipherKeys.verifyAlertMessage('check_circleCipher Key deleted successfully');



    });

    // test("Tink cipher keys created, updated and deleted to store at OpenObserve", async ({ page }) => {
    //     await cipherKeys.navigateToSettingsMenu();
    //       // Navigate to Cipher Key Management
    //     await cipherKeys.navigateToCipherKeyTab();
    
    //       // Add Cipher Key
        
    //     await cipherKeys.addCipherKey();
    //     await cipherKeys.addCipherKeyName(cipherName);
    //     await cipherKeys.addCipherKeyOO(tinkeySecret);
    //     await cipherKeys.addCipherKeyContinue();
    //     await cipherKeys.addCipherKeyTink();
    //     await cipherKeys.addCipherKeySave();
    //     await cipherKeys.verifyAlertMessage('check_circleCipher key created successfully');
    //     await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
    //     await cipherKeys.navigateToSettingsMenu();
    //     await cipherKeys.navigateToCipherKeyTab();
    //     await cipherKeys.updateCipherKeys(cipherName);
    //     await cipherKeys.updateCipherKeysSecret();
    //     await cipherKeys.updateCipherKeysSecretCancel();
    //     await cipherKeys.updateCipherKeysCancel();
    //     await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
    //     await cipherKeys.navigateToSettingsMenu();
    //     await cipherKeys.navigateToCipherKeyTab();
    //     await cipherKeys.updateCipherKeys(cipherName);
    //     await cipherKeys.updateCipherKeysSecret();
    //     await cipherKeys.addCipherKeyOO(tinkeySecretUp); 
    //     await cipherKeys.addCipherKeyContinue();
    //     await cipherKeys.addCipherKeyTink();
    //     await cipherKeys.addCipherKeySave();
    //     await cipherKeys.verifyAlertMessage('check_circleCipher key updated successfully');
        
    //     await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
    //     await cipherKeys.navigateToSettingsMenu();
    //     await cipherKeys.navigateToCipherKeyTab();
    //     await cipherKeys.deletedCipherKeys(cipherName);
    //     await cipherKeys.requestCipherKeysCancel();
    //     await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
    //     await cipherKeys.navigateToSettingsMenu();
    //     await cipherKeys.navigateToCipherKeyTab();
    //     await cipherKeys.deletedCipherKeys(cipherName);
    //     await cipherKeys.requestCipherKeysOk();
    //     await cipherKeys.verifyAlertMessage('check_circleCipher Key deleted successfully');


    
    //     });
    
     test("Error Message displayed if Cipher Key Name Blank", async ({ page }) => {  
        await cipherKeys.navigateToSettingsMenu();
          // Navigate to Cipher Key Management
        await cipherKeys.navigateToCipherKeyTab();
    
          // Add Cipher Key
        
        await cipherKeys.addCipherKey();
        await cipherKeys.addCipherKeyContinue();
        await cipherKeys.verifyAlertMessage('Name is required');
    
    });

    test("Error Message displayed if Cipher Key Secret Blank", async ({ page }) => {  
        await cipherKeys.navigateToSettingsMenu();
          // Navigate to Cipher Key Management
        await cipherKeys.navigateToCipherKeyTab();
    
          // Add Cipher Key
        
        await cipherKeys.addCipherKey();
        await cipherKeys.addCipherKeyName(cipherName);
        await cipherKeys.addCipherKeyContinue();
        await cipherKeys.verifyAlertMessage('Secret is required');
    
    });

    test("Error Message displayed if Cipher Key Secret Invalid", async ({ page }) => {  
        await cipherKeys.navigateToSettingsMenu();
          // Navigate to Cipher Key Management
        await cipherKeys.navigateToCipherKeyTab();
    
          // Add Cipher Key
        
        await cipherKeys.addCipherKey();
        await cipherKeys.addCipherKeyName(cipherName);
        await cipherKeys.addCipherKeyOO('invalidSecret');
        await cipherKeys.addCipherKeyContinue();
        await cipherKeys.addCipherKeySave();
        await cipherKeys.verifyAlertMessage('warningfailed to decode the key, check if secret value and mechanism match');
    
    });

    test("Simple cipher keys created, updated and deleted to store at Akeyless with Static Secret", async ({ page }) => {
        await cipherKeys.navigateToSettingsMenu();
        // Navigate to Cipher Key Management
        await cipherKeys.navigateToCipherKeyTab();
    
        // Add Cipher Key
        
        await cipherKeys.addCipherKey();
        await cipherKeys.addCipherKeyName(cipherName);
        await cipherKeys.addCipherKeyType();
        await cipherKeys.addCipherKeyTypeURL(baseURL);
        await cipherKeys.addCipherKeyTypeID(accessID);
        await cipherKeys.addCipherKeyTypeKey(accessKey);
        await cipherKeys.addCipherKeyStaticName(staticSecretSimple);
        await cipherKeys.addCipherKeyContinue();
        await cipherKeys.addCipherKeySave();
        await cipherKeys.verifyAlertMessage('check_circleCipher key created successfully');
        await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
        await cipherKeys.navigateToSettingsMenu();
        await cipherKeys.navigateToCipherKeyTab();
        await cipherKeys.updateCipherKeys(cipherName);
        await cipherKeys.updateCipherKeysCancel();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
        await cipherKeys.navigateToSettingsMenu();
        await cipherKeys.navigateToCipherKeyTab();
        await cipherKeys.updateCipherKeys(cipherName);
        await cipherKeys.addCipherKeyStaticName(staticSecretSimpleUp);
        await cipherKeys.addCipherKeyContinue();
        await cipherKeys.addCipherKeySave();
        await cipherKeys.verifyAlertMessage('check_circleCipher key updated successfully');
        await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
        await cipherKeys.navigateToSettingsMenu();
        await cipherKeys.navigateToCipherKeyTab();
        await cipherKeys.deletedCipherKeys(cipherName);
        await cipherKeys.requestCipherKeysCancel();
        await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
        await cipherKeys.navigateToSettingsMenu();
        await cipherKeys.navigateToCipherKeyTab();
        await cipherKeys.deletedCipherKeys(cipherName);
        await cipherKeys.requestCipherKeysOk();
        await cipherKeys.verifyAlertMessage('check_circleCipher Key deleted successfully');


        

       

    
        });

        test("Tink cipher keys created, updated and deleted to store at Akeyless with Static Secret", async ({ page }) => {
          await cipherKeys.navigateToSettingsMenu();
          // Navigate to Cipher Key Management
          await cipherKeys.navigateToCipherKeyTab();
      
          // Add Cipher Keys 
          
          await cipherKeys.addCipherKey();
          await cipherKeys.addCipherKeyName(cipherName);
          await cipherKeys.addCipherKeyType();
          await cipherKeys.addCipherKeyTypeURL(baseURL);
          await cipherKeys.addCipherKeyTypeID(accessID);
          await cipherKeys.addCipherKeyTypeKey(accessKey);
          await cipherKeys.addCipherKeyStaticName(staticSecretTink);
          await cipherKeys.addCipherKeyContinue();
          await cipherKeys.addCipherKeyTink();
          await cipherKeys.addCipherKeySave();
          await cipherKeys.verifyAlertMessage('check_circleCipher key created successfully');
          await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
          await cipherKeys.navigateToSettingsMenu();
          await cipherKeys.navigateToCipherKeyTab();
          await cipherKeys.updateCipherKeys(cipherName);
          await cipherKeys.updateCipherKeysCancel();
          await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
          await cipherKeys.navigateToSettingsMenu();
          await cipherKeys.navigateToCipherKeyTab();
          await cipherKeys.updateCipherKeys(cipherName);
          await cipherKeys.addCipherKeyStaticName(staticSecretTinkUp);
          await cipherKeys.addCipherKeyContinue();
          await cipherKeys.addCipherKeyTink();
          await cipherKeys.addCipherKeySave();
          await cipherKeys.verifyAlertMessage('check_circleCipher key updated successfully');
          await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
          await cipherKeys.navigateToSettingsMenu();
          await cipherKeys.navigateToCipherKeyTab();
          await cipherKeys.deletedCipherKeys(cipherName);
          await cipherKeys.requestCipherKeysCancel();
          await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
          await cipherKeys.navigateToSettingsMenu();
          await cipherKeys.navigateToCipherKeyTab();
          await cipherKeys.deletedCipherKeys(cipherName);
          await cipherKeys.requestCipherKeysOk();
          await cipherKeys.verifyAlertMessage('check_circleCipher Key deleted successfully');


        

      
          });

        test("Error Message displayed if DFC Encrypted Data Invalid at Akeyless", async ({ page }) => {
          await cipherKeys.navigateToSettingsMenu();
          // Navigate to Cipher Key Management
          await cipherKeys.navigateToCipherKeyTab();
      
          // Add Cipher Key
          
          await cipherKeys.addCipherKey();
          await cipherKeys.addCipherKeyName(cipherName);
          await cipherKeys.addCipherKeyType();
          await cipherKeys.addCipherKeyTypeURL(baseURL);
          await cipherKeys.addCipherKeyTypeID(accessID);
          await cipherKeys.addCipherKeyTypeKey(accessKey);
          await cipherKeys.addCipherKeyDFC();
          await cipherKeys.addCipherKeyNameDFC(simpleNameDFC);
          await cipherKeys.addCipherKeyEncryDataDFC(simpleEncryDataDFCInvalid);
          await cipherKeys.addCipherKeyContinue();
          await cipherKeys.addCipherKeySave();
          await cipherKeys.verifyAlertMessage('warningfailed to decode the ciphertext: illegal base64 data at input byte 4');
      
          });

          test("Simple cipher keys created, updated and deleted Akeyless with DFC Secret", async ({ page }) => {
            await cipherKeys.navigateToSettingsMenu();
            // Navigate to Cipher Key Management
            await cipherKeys.navigateToCipherKeyTab();
        
            // Add Cipher Key
            
            await cipherKeys.addCipherKey();
            await cipherKeys.addCipherKeyName(cipherName);
            await cipherKeys.addCipherKeyType();
            await cipherKeys.addCipherKeyTypeURL(baseURL);
            await cipherKeys.addCipherKeyTypeID(accessID);
            await cipherKeys.addCipherKeyTypeKey(accessKey);
            await cipherKeys.addCipherKeyDFC();
            await cipherKeys.addCipherKeyNameDFC(simpleNameDFC);
            await cipherKeys.addCipherKeyEncryDataDFC(simpleEncryDataTedDFC);
            await cipherKeys.addCipherKeyContinue();
            await cipherKeys.addCipherKeySave();
            await cipherKeys.verifyAlertMessage('check_circleCipher key created successfully');
            await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
            await cipherKeys.navigateToSettingsMenu();
            await cipherKeys.navigateToCipherKeyTab();
            await cipherKeys.updateCipherKeys(cipherName);
            await cipherKeys.updateCipherKeysCancel();
            await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
            await cipherKeys.navigateToSettingsMenu();
            await cipherKeys.navigateToCipherKeyTab();
            await cipherKeys.updateCipherKeys(cipherName);
            await cipherKeys.addCipherKeyDFC();
            await cipherKeys.addCipherKeyNameDFC(simpleUpNameDFC);
            await cipherKeys.addCipherKeyEncryDataDFC(simpleUpEncryDataTedDFC);
            await cipherKeys.addCipherKeyContinue();
            await cipherKeys.addCipherKeySave();
            await cipherKeys.verifyAlertMessage('check_circleCipher key updated successfully');
            await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
            await cipherKeys.navigateToSettingsMenu();
            await cipherKeys.navigateToCipherKeyTab();
            await cipherKeys.deletedCipherKeys(cipherName);
            await cipherKeys.requestCipherKeysCancel();
            await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
            await cipherKeys.navigateToSettingsMenu();
            await cipherKeys.navigateToCipherKeyTab();
            await cipherKeys.deletedCipherKeys(cipherName);
            await cipherKeys.requestCipherKeysOk();
            await cipherKeys.verifyAlertMessage('check_circleCipher Key deleted successfully');

            
        

        
            }); 

            test("Tink cipher keys created, updated and deleted Akeyless with DFC Secret", async ({ page }) => {
              await cipherKeys.navigateToSettingsMenu();
              // Navigate to Cipher Key Management
              await cipherKeys.navigateToCipherKeyTab();
          
              // Add Cipher Key
              
              await cipherKeys.addCipherKey();
              await cipherKeys.addCipherKeyName(cipherName);
              await cipherKeys.addCipherKeyType();
              await cipherKeys.addCipherKeyTypeURL(baseURL);
              await cipherKeys.addCipherKeyTypeID(accessID);
              await cipherKeys.addCipherKeyTypeKey(accessKey);
              await cipherKeys.addCipherKeyDFC();
              await cipherKeys.addCipherKeyNameDFC(tinkDFC);
              await cipherKeys.addCipherKeyEncryDataDFC(tinkEncryDataTedDFC);
              await cipherKeys.addCipherKeyContinue();
              await cipherKeys.addCipherKeyTink();
              await cipherKeys.addCipherKeySave();
              await cipherKeys.verifyAlertMessage('check_circleCipher key created successfully');
              await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
              await cipherKeys.navigateToSettingsMenu();
              await cipherKeys.navigateToCipherKeyTab();
              await cipherKeys.updateCipherKeys(cipherName);
              await cipherKeys.updateCipherKeysCancel();
              await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
              await cipherKeys.navigateToSettingsMenu();
              await cipherKeys.navigateToCipherKeyTab();
              await cipherKeys.updateCipherKeys(cipherName);
              await cipherKeys.addCipherKeyDFC();
              await cipherKeys.addCipherKeyNameDFC(tinkUpDFC);
              await cipherKeys.addCipherKeyEncryDataDFC(tinkUpEncryDataTedDFC);
              await cipherKeys.addCipherKeyContinue();
              await cipherKeys.addCipherKeyTink();
              await cipherKeys.addCipherKeySave();
              await cipherKeys.verifyAlertMessage('check_circleCipher key updated successfully');
              await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
              await cipherKeys.navigateToSettingsMenu();
              await cipherKeys.navigateToCipherKeyTab();
              await cipherKeys.deletedCipherKeys(cipherName);
              await cipherKeys.requestCipherKeysCancel();
              await page.goto(process.env["ZO_BASE_URL"] + "/web/settings/cipher_keys?org_identifier=default");
              await cipherKeys.navigateToSettingsMenu();
              await cipherKeys.navigateToCipherKeyTab();
              await cipherKeys.deletedCipherKeys(cipherName);
              await cipherKeys.requestCipherKeysOk();
              await cipherKeys.verifyAlertMessage('check_circleCipher Key deleted successfully');

              
        
          
              }); 

});
