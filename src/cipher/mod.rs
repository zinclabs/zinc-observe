use o2_enterprise::enterprise::cipher::CipherData;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod registry;

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct KeyAddRequest {
    pub name: String,
    pub key: CipherData,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct KeyGetResponse {
    pub name: String,
    pub key_type: CipherData,
}
