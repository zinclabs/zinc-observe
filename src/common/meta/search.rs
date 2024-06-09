use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ResultMeta {
    pub start_time: i64,
    pub end_time: i64,
    pub is_aggregate: bool,
}
