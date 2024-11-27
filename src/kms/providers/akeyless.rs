use async_trait::async_trait;
use config::utils::json;

use super::super::{Error, KMS};

/// Main structure to interface with akeyless kms
#[derive(Debug)]
pub struct Akeyless {
    /// base url for the api endpoint of akeyless. Must not have terminating slash
    base_url: String,
    /// The auth info structure
    auth: AuthInfo,
    /// The key info structure
    key: KeyInfo,
}

/// Specified the key type from akeyless
/// we only support aes keys, rsa keys generate the certs, and cannot be
/// used via api endpoints for encryption/decryption
#[derive(Debug, Clone, Copy)]
pub enum KeyType {
    /// indicates AES128GCM / AES256GCM
    AesGcm,
    /// indicates AES128SIV / AES256SIV
    AesSiv,
    /// indicates AES128CBC / AES256CBC
    AesCbc,
}

/// This stores the key related info for akeyless
#[allow_unused]
#[derive(Debug, Clone)]
pub struct KeyInfo {
    /// name of the key item
    name: String,
    /// display id for the key item
    display_id: String,
    /// type of the key
    typ: KeyType,
    /// optional IV (initialization vector) for the key. This is only required for cbc keys
    iv: Option<String>,
}

/// This stores the auth related info for akeyless
#[allow_unused]
#[derive(Debug, Clone)]
pub struct AuthInfo {
    /// The access id for generating tokens
    access_id: String,
    /// The access key for generating tokens
    access_key: String,
    /// The actual generated token ; this is short-lived, and must be refreshed periodically
    token: String,
    /// Timestamp for when the token was last refreshed
    updated_at: i64,
}

/// Typestate trait for the builder
trait AuthState {}
struct Base;
struct Authenticated;
struct KeyConfigured;
impl AuthState for Base {}
impl AuthState for Authenticated {}
impl AuthState for KeyConfigured {}

/// The builder struct for the Akeyless KMS
pub struct AkeylessBuilder<S: AuthState> {
    /// api endpoint url, must be without trailing /
    base_url: String,
    /// The auth info structure, will be set internally
    auth: Option<AuthInfo>,
    /// The key info structure, will be set internally
    key: Option<KeyInfo>,
    /// marker so compiler does not complain about `S` type
    marker: std::marker::PhantomData<S>,
}

/// Helper function to refresh the short-lived token
async fn refresh_token(base: &str, id: &str, key: &str) -> Result<String, Error> {
    let client = reqwest::Client::builder().build()?;
    let payload = json::json!({
        "access-id":id,
        "access-key":key,
        "access-type":"access_key",
        "json": true
    });
    let res = client
        .post(format!("{}/auth", base))
        .json(&payload)
        .send()
        .await?;
    let status = res.status();
    let res_json = res.json::<json::Value>().await?;
    if !status.is_success() {
        // in case of error, the response always has error key with explanation
        return Err(Error::AuthenticationError(
            res_json.get("error").unwrap().as_str().unwrap().to_string(),
        ));
    }
    Ok(res_json.get("token").unwrap().as_str().unwrap().to_string())
}

/// public method to get the uninitialized akeyless km builder
pub fn new_akeyless_builder(base_url: &str) -> AkeylessBuilder<Base> {
    AkeylessBuilder {
        base_url: base_url.to_string(),
        auth: None,
        key: None,
        marker: Default::default(),
    }
}

/// This is for un-auth builder
impl AkeylessBuilder<Base> {
    /// Authenticates and generates the short-lived token
    pub async fn authenticate(
        self,
        access_id: &str,
        access_key: &str,
    ) -> Result<AkeylessBuilder<Authenticated>, Error> {
        let token = refresh_token(&self.base_url, access_id, access_key).await?;
        let info = AuthInfo {
            access_id: access_id.to_string(),
            access_key: access_key.to_string(),
            token,
            updated_at: chrono::Utc::now().timestamp_micros(),
        };
        Ok(AkeylessBuilder {
            base_url: self.base_url,
            auth: Some(info),
            key: None,
            marker: Default::default(),
        })
    }
}

/// This is for authenticated builder, without key configured
impl AkeylessBuilder<Authenticated> {
    /// validates and sets the key info for the key
    /// to be used for encryption/decryption.
    async fn set_key(
        self,
        key_name: &str,
        iv: Option<&str>,
    ) -> Result<AkeylessBuilder<KeyConfigured>, Error> {
        let client = reqwest::Client::builder().build()?;
        let payload = json::json!({
            "filter":key_name.to_string(),
            "json": true,
            "token":self.auth.as_ref().unwrap().token.clone(),
        });

        // we first fetch the items and check if the provided key exists or not
        let res = client
            .post(format!("{}/list-items", self.base_url))
            .json(&payload)
            .send()
            .await?;
        let status = res.status();
        let res_json = res.json::<json::Value>().await?;
        if !status.is_success() {
            return Err(Error::InvalidKey(
                res_json.get("error").unwrap().as_str().unwrap().to_string(),
            ));
        }

        // if no keys found with given name, the returned object does not have `items` key at all
        let items = match res_json.get("items") {
            Some(v) => v.as_array().unwrap(),
            None => {
                return Err(Error::InvalidKey(format!(
                    "no key found with name {key_name}"
                )))
            }
        };
        if items.len() != 1 {
            let key_names: Vec<String> = items
                .iter()
                .map(|i| i.get("display_id").unwrap().as_str().unwrap().to_string())
                .collect();
            return Err(Error::InvalidKey(format!(
                "multiple keys found with given name : {:?}",
                key_names
            )));
        }
        let key = items.get(0).unwrap();

        // Then we check if the given key is of type aes or not
        // rsa keys generate certs, and cannot be used via api to encrypt/decrypt
        let typ = key.get("item_type").unwrap().as_str().unwrap();
        if !typ.starts_with("AES") {
            return Err(Error::InvalidKey(format!(
                "invalid key type, only AES keys supported, found {typ}"
            )));
        }

        let typ = match typ.split_at(typ.len() - 3) {
            (_, "GCM") => KeyType::AesGcm,
            (_, "SIV") => KeyType::AesSiv,
            (_, "CBC") => KeyType::AesCbc,
            _ => {
                return Err(Error::InvalidKey(format!(
                    "invalid key type, only AES keys supported, found {typ}"
                )));
            }
        };

        // for cbc keys, we need IV, so we check if it present or not
        match (typ, iv) {
            (KeyType::AesCbc, None) => {
                return Err(Error::InvalidKey(format!(
                    "missing iv, AES CBC key must have IV"
                )));
            }
            _ => {}
        }

        let kinfo = KeyInfo {
            name: key_name.to_string(),
            display_id: key.get("display_id").unwrap().as_str().unwrap().to_string(),
            typ,
            iv: iv.map(|s| s.to_string()),
        };

        Ok(AkeylessBuilder {
            base_url: self.base_url,
            auth: self.auth,
            key: Some(kinfo),
            marker: Default::default(),
        })
    }
}

/// This is for converting the authenticated, key-configured builder
/// into the final struct which implements the kms interface
impl AkeylessBuilder<KeyConfigured> {
    fn build(self) -> Akeyless {
        Akeyless {
            base_url: self.base_url,
            auth: self.auth.unwrap(),
            key: self.key.unwrap(),
        }
    }
}

#[async_trait]
impl KMS for Akeyless {
    async fn refresh(&mut self) -> Result<(), Error> {
        let new_token =
            refresh_token(&self.base_url, &self.auth.access_id, &self.auth.access_key).await?;
        self.auth.token = new_token;
        self.auth.updated_at = chrono::Utc::now().timestamp_micros();
        Ok(())
    }
    async fn encrypt(&self, data: &str) -> Result<String, Error> {
        let client = reqwest::Client::builder().build()?;

        let ctx = match self.key.typ {
            KeyType::AesCbc => {
                json::json!({
                    "iv":self.key.iv.as_ref().unwrap().to_string()
                })
            }
            _ => {
                json::json!({})
            }
        };

        let payload = json::json!({
            "key-name": self.key.name.clone(),
            "encryption-context":ctx,
            "json": true,
            "plaintext":data.to_string(),
            "token":self.auth.token.clone(),
        });

        let res = client
            .post(format!("{}/encrypt", self.base_url))
            .json(&payload)
            .send()
            .await?;
        let status = res.status();
        let res_json = res.json::<json::Value>().await?;
        if !status.is_success() {
            return Err(Error::EncryptionError(
                res_json.get("error").unwrap().as_str().unwrap().to_string(),
            ));
        }
        Ok(res_json
            .get("result")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string())
    }

    async fn decrypt(&self, data: &str) -> Result<String, Error> {
        let client = reqwest::Client::builder().build()?;

        let ctx = match self.key.typ {
            KeyType::AesCbc => {
                json::json!({
                    "iv":self.key.iv.as_ref().unwrap().to_string()
                })
            }
            _ => {
                json::json!({})
            }
        };

        let payload = json::json!({
            "key-name": self.key.name.clone(),
            "encryption-context":ctx,
            "json": true,
            "ciphertext":data.to_string(),
            "token":self.auth.token.clone(),
        });

        let res = client
            .post(format!("{}/decrypt", self.base_url))
            .json(&payload)
            .send()
            .await?;
        let status = res.status();
        let res_json = res.json::<json::Value>().await?;
        if !status.is_success() {
            return Err(Error::DecryptionError(
                res_json.get("error").unwrap().as_str().unwrap().to_string(),
            ));
        }
        Ok(res_json
            .get("result")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string())
    }
}
