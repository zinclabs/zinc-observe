// Copyright 2024 Zinc Labs Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use async_nats::{error::Error as NatsError, jetstream};
use config::utils::json;
use thiserror::Error as ThisError;
pub mod grpc;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("IoError# {0}")]
    IoError(#[from] std::io::Error),
    #[error("DbError# {0}")]
    DbError(#[from] DbError),
    #[error("EtcdError# {0}")]
    EtcdError(#[from] etcd_client::Error),
    #[error("SerdeJsonError# {0}")]
    SerdeJsonError(#[from] json::Error),
    #[error("ArrowError# {0}")]
    ArrowError(#[from] datafusion::arrow::error::ArrowError),
    #[error("WatchError# watcher is exists {0}")]
    WatcherExists(String),
    #[error("StringUTF8Error# {0}")]
    StringUTF8Error(#[from] std::string::FromUtf8Error),
    #[error("SqlxError# {0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("Error# {0}")]
    NatsKJetstreamContextRequestError(#[from] NatsError<jetstream::context::RequestErrorKind>),
    #[error("Error# {0}")]
    NatsJetstreamContextCreateKeyValueError(
        #[from] NatsError<jetstream::context::CreateKeyValueErrorKind>,
    ),
    #[error("Error# {0}")]
    NatsJetstreamKvEntryError(#[from] NatsError<jetstream::kv::EntryErrorKind>),
    #[error("Error# {0}")]
    NatsKJetstreamKvPutError(#[from] NatsError<jetstream::kv::PutErrorKind>),
    #[error("Error# {0}")]
    NatsKJetstreamKvUpdateError(#[from] NatsError<jetstream::kv::UpdateErrorKind>),
    #[error("Error# {0}")]
    NatsKJetstreamKvWatchError(#[from] NatsError<jetstream::kv::WatchErrorKind>),
    #[error("Error# {0}")]
    NatsKJetstreamKvWatcherError(#[from] NatsError<jetstream::kv::WatcherErrorKind>),
    #[error("Error# {0}")]
    NatsKJetstreamKvStatusError(#[from] NatsError<jetstream::kv::StatusErrorKind>),
    #[error("Error# {0}")]
    NatsKJetstreamCreateStreamError(#[from] NatsError<jetstream::context::CreateStreamErrorKind>),
    #[error("Error# {0}")]
    NatsKJetstreamGetStreamError(#[from] NatsError<jetstream::context::GetStreamErrorKind>),
    #[error("Error# {0}")]
    NatsKJetstreamPublishError(#[from] NatsError<jetstream::context::PublishErrorKind>),
    #[error("Error# {0}")]
    NatsKJetstreamStreamConsumerError(#[from] NatsError<jetstream::stream::ConsumerErrorKind>),
    #[error("Error# {0}")]
    NatsKJetstreamConsumerStreamError(#[from] NatsError<jetstream::consumer::StreamErrorKind>),
    #[error("Error# {0}")]
    Message(String),
    #[error("ErrorCode# {0}")]
    ErrorCode(ErrorCodes),
    #[error("Not implemented")]
    NotImplemented,
    #[error("Unknown error")]
    Unknown,
}

unsafe impl Send for Error {}

#[derive(ThisError, Debug)]
pub enum DbError {
    #[error("key {0} does not exist")]
    KeyNotExists(String),
    #[error("error {0} performing operation on key {1}")]
    DBOperError(String, String),
}

#[derive(ThisError, Debug)]
pub enum ErrorCodes {
    ServerInternalError(String),
    SearchSQLNotValid(String),
    SearchStreamNotFound(String),
    FullTextSearchFieldNotFound,
    SearchFieldNotFound(String),
    SearchFunctionNotDefined(String),
    SearchParquetFileNotFound,
    SearchFieldHasNoCompatibleDataType(String),
    SearchSQLExecuteError(String),
    SearchCancelQuery(String),
    SearchTimeout(String),
}

impl std::fmt::Display for ErrorCodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"{{"error_code": {}, "error_msg": "{}"}}"#,
            self.get_code(),
            self.get_message()
        )
    }
}

impl ErrorCodes {
    pub fn get_code(&self) -> u16 {
        match self {
            ErrorCodes::ServerInternalError(_) => 10001,
            ErrorCodes::SearchSQLNotValid(_) => 20001,
            ErrorCodes::SearchStreamNotFound(_) => 20002,
            ErrorCodes::FullTextSearchFieldNotFound => 20003,
            ErrorCodes::SearchFieldNotFound(_) => 20004,
            ErrorCodes::SearchFunctionNotDefined(_) => 20005,
            ErrorCodes::SearchParquetFileNotFound => 20006,
            ErrorCodes::SearchFieldHasNoCompatibleDataType(_) => 20007,
            ErrorCodes::SearchSQLExecuteError(_) => 20008,
            ErrorCodes::SearchCancelQuery(_) => 20009,
            ErrorCodes::SearchTimeout(_) => 20010,
        }
    }

    pub fn get_message(&self) -> String {
        match self {
            ErrorCodes::ServerInternalError(_) => "Server Internal Error".to_string(),
            ErrorCodes::SearchSQLNotValid(_) => "Search SQL not valid".to_string(),
            ErrorCodes::SearchStreamNotFound(stream) => {
                format!("Search stream not found: {stream}")
            }
            ErrorCodes::FullTextSearchFieldNotFound => {
                "Full text search field not found".to_string()
            }
            ErrorCodes::SearchFieldNotFound(field) => format!("Search field not found: {field}"),
            ErrorCodes::SearchFunctionNotDefined(func) => {
                format!("Search function not defined: {func}")
            }
            ErrorCodes::SearchParquetFileNotFound => "Search parquet file not found".to_string(),
            ErrorCodes::SearchFieldHasNoCompatibleDataType(field) => {
                format!("Search field has no compatible data type: {field}")
            }
            ErrorCodes::SearchSQLExecuteError(_) => "Search SQL execute error".to_string(),
            ErrorCodes::SearchCancelQuery(_) => "Search query was cancelled".to_string(),
            ErrorCodes::SearchTimeout(_) => "Search query timed out".to_string(),
        }
    }

    pub fn get_inner_message(&self) -> String {
        match self {
            ErrorCodes::ServerInternalError(msg) => msg.to_owned(),
            ErrorCodes::SearchSQLNotValid(sql) => sql.to_owned(),
            ErrorCodes::SearchStreamNotFound(stream) => stream.to_owned(),
            ErrorCodes::FullTextSearchFieldNotFound => "".to_string(),
            ErrorCodes::SearchFieldNotFound(field) => field.to_owned(),
            ErrorCodes::SearchFunctionNotDefined(func) => func.to_owned(),
            ErrorCodes::SearchParquetFileNotFound => "".to_string(),
            ErrorCodes::SearchFieldHasNoCompatibleDataType(field) => field.to_owned(),
            ErrorCodes::SearchSQLExecuteError(msg) => msg.to_owned(),
            ErrorCodes::SearchCancelQuery(msg) => msg.to_owned(),
            ErrorCodes::SearchTimeout(msg) => msg.to_owned(),
        }
    }

    pub fn get_error_detail(&self) -> String {
        match self {
            ErrorCodes::ServerInternalError(msg) => msg.to_owned(),
            ErrorCodes::SearchSQLNotValid(sql) => sql.to_owned(),
            ErrorCodes::SearchStreamNotFound(_) => "".to_string(),
            ErrorCodes::FullTextSearchFieldNotFound => "".to_string(),
            ErrorCodes::SearchFieldNotFound(_) => "".to_string(),
            ErrorCodes::SearchFunctionNotDefined(_) => "".to_string(),
            ErrorCodes::SearchParquetFileNotFound => "".to_string(),
            ErrorCodes::SearchFieldHasNoCompatibleDataType(_) => "".to_string(),
            ErrorCodes::SearchSQLExecuteError(msg) => msg.to_owned(),
            ErrorCodes::SearchCancelQuery(msg) => msg.to_string(),
            ErrorCodes::SearchTimeout(msg) => msg.to_owned(),
        }
    }

    pub fn to_json(&self) -> String {
        let mut map = json::Map::new();
        map.insert("code".to_string(), json::Value::from(self.get_code()));
        map.insert("message".to_string(), json::Value::from(self.get_message()));
        map.insert(
            "inner".to_string(),
            json::Value::from(self.get_inner_message()),
        );
        json::Value::Object(map).to_string()
    }

    pub fn from_json(json: &str) -> Result<ErrorCodes> {
        let val: json::Value = match json::from_str(json) {
            Ok(val) => val,
            Err(_) => return Ok(ErrorCodes::ServerInternalError(json.to_string())),
        };
        let map = match val.as_object() {
            Some(map) => map,
            None => return Ok(ErrorCodes::ServerInternalError(json.to_string())),
        };
        let code = match map.get("code") {
            Some(code) => match code.as_i64() {
                Some(code) => code as u16,
                None => return Ok(ErrorCodes::ServerInternalError(json.to_string())),
            },
            None => return Ok(ErrorCodes::ServerInternalError(json.to_string())),
        };
        let message = match map.get("inner") {
            Some(message) => match message {
                json::Value::String(message) => message.to_owned(),
                _ => message.to_string(),
            },
            None => return Ok(ErrorCodes::ServerInternalError(json.to_string())),
        };
        match code {
            10001 => Ok(ErrorCodes::ServerInternalError(message)),
            20001 => Ok(ErrorCodes::SearchSQLNotValid(message)),
            20002 => Ok(ErrorCodes::SearchStreamNotFound(message)),
            20003 => Ok(ErrorCodes::FullTextSearchFieldNotFound),
            20004 => Ok(ErrorCodes::SearchFieldNotFound(message)),
            20005 => Ok(ErrorCodes::SearchFunctionNotDefined(message)),
            20006 => Ok(ErrorCodes::SearchParquetFileNotFound),
            20007 => Ok(ErrorCodes::SearchFieldHasNoCompatibleDataType(message)),
            20008 => Ok(ErrorCodes::SearchSQLExecuteError(message)),
            20009 => Ok(ErrorCodes::SearchCancelQuery(message)),
            20010 => Ok(ErrorCodes::SearchTimeout(message)),
            _ => Ok(ErrorCodes::ServerInternalError(json.to_string())),
        }
    }
}

#[derive(ThisError, Debug)]
pub enum JwtError {
    #[error("No matching JWK found for the given kid")]
    KeyNotExists(),
    #[error("Token doesn't have a {0} field")]
    MissingAttribute(String),
    #[error("Token can't be verified")]
    ValidationFailed(),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error() {
        let err = Error::Message("Ni! Try again.".to_string());
        assert_eq!("Error# Ni! Try again.", &err.to_string());

        let err = Error::from(DbError::KeyNotExists("/another/shrubbery".to_string()));
        assert_eq!(
            "DbError# key /another/shrubbery does not exist",
            &err.to_string()
        );
    }
}
