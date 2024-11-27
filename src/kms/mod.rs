use async_trait::async_trait;
use thiserror::Error as ThisError;
pub mod providers;

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
