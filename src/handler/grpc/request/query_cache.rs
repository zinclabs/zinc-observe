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

use async_trait::async_trait;
use proto::cluster_rpc::{
    query_cache_server::QueryCache, QueryCacheRequest, QueryCacheResponse, QueryDelta,
};
use tonic::{Request, Response, Status};

use crate::service::search::cache::cacher;

#[derive(Debug, Default)]
pub struct QueryCacheServerImpl;

#[async_trait]
impl QueryCache for QueryCacheServerImpl {
    async fn get_cached_result(
        &self,
        request: Request<QueryCacheRequest>,
    ) -> Result<Response<QueryCacheResponse>, Status> {
        let req: QueryCacheRequest = request.into_inner();
        match cacher::get_cached_results(
            req.start_time,
            req.end_time,
            req.is_aggregate,
            &req.file_path,
            &req.timestamp_col,
        )
        .await
        {
            Some(res) => {
                let deltas = res
                    .deltas
                    .iter()
                    .map(|d| QueryDelta {
                        delta_start_time: d.delta_start_time,
                        delta_end_time: d.delta_end_time,
                        delta_removed_hits: d.delta_removed_hits,
                    })
                    .collect();

                let response: Vec<proto::cluster_rpc::RangeCacheResponse> = res
                    .cached_response
                    .iter()
                    .map(|r| proto::cluster_rpc::RangeCacheResponse {
                        data: serde_json::to_vec(&r.cached_response).unwrap(),
                        has_cached_data: r.has_cached_data,
                        cache_start_time: r.response_start_time,
                        cache_end_time: r.response_end_time,
                    })
                    .collect();

                let res = proto::cluster_rpc::CacheResponse {
                    cached_response: response,
                    deltas,
                    has_pre_cache_delta: res.has_pre_cache_delta,
                    cache_query_response: res.cache_query_response,
                    ts_column: res.ts_column,
                };

                Ok(Response::new(QueryCacheResponse {
                    response: Some(res),
                }))
            }
            None => Ok(Response::new(QueryCacheResponse { response: None })),
        }
    }
}

// converter for CacheResponse to proto::cluster_rpc::CacheResponse
impl From<crate::common::meta::search::CacheResponse> for proto::cluster_rpc::CacheResponse {
    fn from(res: crate::common::meta::search::CacheResponse) -> Self {
        let deltas = res
            .deltas
            .iter()
            .map(|d| QueryDelta {
                delta_start_time: d.delta_start_time,
                delta_end_time: d.delta_end_time,
                delta_removed_hits: d.delta_removed_hits,
            })
            .collect();

        let response: Vec<proto::cluster_rpc::RangeCacheResponse> = res
            .cached_response
            .iter()
            .map(|r| proto::cluster_rpc::RangeCacheResponse {
                data: serde_json::to_vec(&r.cached_response).unwrap(),
                has_cached_data: r.has_cached_data,
                cache_start_time: r.response_start_time,
                cache_end_time: r.response_end_time,
            })
            .collect();

        proto::cluster_rpc::CacheResponse {
            cached_response: response,
            deltas,
            has_pre_cache_delta: res.has_pre_cache_delta,
            cache_query_response: res.cache_query_response,
            ts_column: res.ts_column,
        }
    }
}
