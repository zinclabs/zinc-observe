// Copyright 2024 OpenObserve Inc.
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
    query_cache_server::QueryCache, DeleteResultCacheRequest, DeleteResultCacheResponse,
    QueryCacheRequest, QueryCacheRes, QueryCacheResponse, QueryDelta, QueryResponse,
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
            &req.trace_id,
            req.discard_interval,
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

                let res = QueryCacheRes {
                    cached_response: Some(QueryResponse {
                        data: serde_json::to_vec(&res.cached_response).unwrap(),
                    }),
                    deltas,
                    has_pre_cache_delta: res.has_pre_cache_delta,
                    has_cached_data: res.has_cached_data,
                    cache_query_response: res.cache_query_response,
                    cache_start_time: res.response_start_time,
                    cache_end_time: res.response_end_time,
                };

                Ok(Response::new(QueryCacheResponse {
                    response: Some(res),
                }))
            }
            None => Ok(Response::new(QueryCacheResponse { response: None })),
        }
    }

    async fn delete_result_cache(
        &self,
        request: Request<DeleteResultCacheRequest>,
    ) -> Result<Response<DeleteResultCacheResponse>, Status> {
        let req: DeleteResultCacheRequest = request.into_inner();
        let deleted = cacher::delete_cache(&req.path).await.is_ok();

        Ok(Response::new(DeleteResultCacheResponse { deleted }))
    }
}
