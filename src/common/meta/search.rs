use config::meta::search::Response;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ResultCacheMeta {
    pub start_time: i64,
    pub end_time: i64,
    pub is_aggregate: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CachedQueryResponse {
    pub cached_response: Response,
    pub deltas: Vec<QueryDelta>,
    pub has_pre_cache_delta: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct QueryDelta {
    pub delta_start_time: i64,
    pub delta_end_time: i64,
    pub delta_removed_hits: bool,
}
