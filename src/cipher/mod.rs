use async_trait::async_trait;
use providers::local;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;
use utoipa::ToSchema;

pub mod providers;
pub mod cipher;
pub mod registry;
pub mod algorithms;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("ReqwestError# {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("AuthenticationError# {0}")]
    AuthenticationError(String),
    #[error("InvalidKey# {0}")]
    InvalidKey(String),
    #[error("EncryptionError# {0}")]
    EncryptionError(String),
    #[error("DecryptionError# {0}")]
    DecryptionError(String),
}

/// A trait to provide interface for all kms implementations
#[async_trait]
pub trait KMS {
    /// calling this method should refresh the internal state, if applicable
    /// For example, it can refresh a short-lived token
    async fn refresh(&mut self) -> Result<(), Error>;
    /// Encrypts the given data string as per the configured key
    async fn encrypt(&self, data: &str) -> Result<String, Error>;
    /// Decrypts the given (encrypted) data string as per the configured key
    async fn decrypt(&self, data: &str) -> Result<String, Error>;
}

#[derive(Hash, Debug, Clone, Serialize, Deserialize)]
pub enum Credentials {
    Akeyless {
        access_id: String,
        access_key: String,
        name: String,
        iv: Option<String>,
    },
    Local {
        key: String,
        algorithm: local::Algorithm,
    },
}

impl Credentials {
    pub fn get_type(&self) -> String {
        match self {
            Self::Akeyless { .. } => "akeyless".to_string(),
            Self::Local { .. } => "local".to_string(),
        }
    }
}

#[derive(Hash, Debug, Serialize, Deserialize)]
pub struct Key {
    pub created_by: String,
    pub created_at: i64,
    pub name: String,
    pub credentials: Credentials,
}

impl From<infra::table::keys::Model> for Key {
    fn from(value: infra::table::keys::Model) -> Self {
        Self {
            created_by: value.created_by,
            created_at: value.created_ts,
            name: value.name,
            credentials: serde_json::from_str(&value.credentials).unwrap(),
        }
    }
}

impl From<infra::table::keys::KeyInfo> for Key {
    fn from(value: infra::table::keys::KeyInfo) -> Self {
        Self {
            created_by: value.created_by,
            created_at: value.created_ts,
            name: value.name,
            credentials: serde_json::from_str(&value.credentials).unwrap(),
        }
    }
}

impl Into<infra::table::keys::KeyInfo> for Key {
    fn into(self) -> infra::table::keys::KeyInfo {
        let kt = match &self.credentials {
            Credentials::Akeyless { .. } => infra::table::keys::KeyType::Akeyless,
            Credentials::Local { .. } => infra::table::keys::KeyType::Local,
        };
        infra::table::keys::KeyInfo {
            created_ts: self.created_at,
            created_by: self.created_by,
            key_type: kt,
            name: self.name,
            credentials: serde_json::to_string(&self.credentials).unwrap(),
        }
    }
}
#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct KeyAddRequest {
    pub name: String,
    pub key_type: String,
    pub credentials: Credentials,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct KeyGetResponse {
    pub name: String,
    pub key_type: String,
    pub credentials: Credentials,
}
