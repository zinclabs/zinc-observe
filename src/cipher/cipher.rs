use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("InvalidKey# {0}")]
    InvalidKey(String),
    #[error("EncryptionError# {0}")]
    EncryptionError(String),
    #[error("DecryptionError# {0}")]
    DecryptionError(String),
}

pub trait Cipher: Sync + Send {
    fn clone_self(&self) -> Box<dyn Cipher>;
    fn encrypt(&mut self, value: &str) -> Result<String, Error>;
    fn decrypt(&mut self, value: &str) -> Result<String, Error>;
}

#[derive(Clone)]
pub struct Key {
    key: String,
}

impl Key {
    pub fn new(k: String) -> Self {
        Self { key: k }
    }
}

impl Cipher for Key {
    fn clone_self(&self) -> Box<dyn Cipher> {
        Box::new(Self {
            key: self.key.clone(),
        })
    }
    fn encrypt(&mut self, value: &str) -> Result<String, Error> {
        Ok(value.to_string())
    }
    fn decrypt(&mut self, value: &str) -> Result<String, Error> {
        Ok(value.to_string())
    }
}
