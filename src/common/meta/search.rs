use config::meta::search::Response;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, Default)]
pub struct CachedQueryResponse {
    pub cached_response: Response,
    pub deltas: Vec<QueryDelta>,
    pub has_pre_cache_delta: bool,
    pub has_cached_data: bool,
    pub cache_query_response: bool,
    pub response_start_time: i64,
    pub response_end_time: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, Default)]
pub struct QueryDelta {
    pub delta_start_time: i64,
    pub delta_end_time: i64,
    pub delta_removed_hits: bool,
}
