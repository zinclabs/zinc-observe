use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultCacheMeta {
    pub start_time: i64,
    pub end_time: i64,
    pub is_aggregate: bool,
}
