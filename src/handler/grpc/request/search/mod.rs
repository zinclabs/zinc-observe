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

#[cfg(feature = "enterprise")]
use config::metrics;
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::search::{QueryManager, TaskStatus, WorkGroup};
use proto::cluster_rpc::{
    search_server::Search, CancelQueryRequest, CancelQueryResponse, QueryStatusRequest,
    QueryStatusResponse,
};
use tonic::{Request, Response, Status};

#[derive(Clone, Debug)]
#[cfg(feature = "enterprise")]
pub struct Searcher {
    pub query_manager: std::sync::Arc<QueryManager>,
}

#[cfg(feature = "enterprise")]
impl Searcher {
    pub fn new() -> Self {
        Self {
            query_manager: std::sync::Arc::new(QueryManager::new()),
        }
    }

    // check is the trace_id in the query_manager
    pub async fn contain_key(&self, trace_id: &str) -> bool {
        self.query_manager.contain_key(trace_id).await
    }

    // insert the trace_id and task_status into the query_manager
    pub async fn insert(&self, trace_id: String, task_status: TaskStatus) {
        self.query_manager.insert(trace_id, task_status).await;
    }

    // remove the trace_id from the query_manager
    pub async fn remove(
        &self,
        trace_id: &str,
        query_cancelled: bool,
    ) -> Option<Vec<(String, TaskStatus)>> {
        self.query_manager.remove(trace_id, query_cancelled).await
    }

    // check is the trace_id in the query_manager and is_leader
    pub async fn is_leader(&self, trace_id: &str) -> bool {
        self.query_manager.is_leader(trace_id).await
    }

    // insert the sender into the query_manager by trace_id
    pub async fn insert_sender(
        &self,
        trace_id: &str,
        sender: tokio::sync::oneshot::Sender<()>,
        query_start: bool,
    ) -> Result<(), infra::errors::Error> {
        self.query_manager
            .insert_sender(trace_id, sender, query_start)
            .await
    }

    // get all task status that is leader
    pub async fn get_task_status(&self) -> Vec<proto::cluster_rpc::QueryStatus> {
        self.query_manager.get_task_status().await
    }

    // add file stats
    pub async fn add_file_stats(
        &self,
        trace_id: &str,
        files: i64,
        records: i64,
        original_size: i64,
        compressed_size: i64,
    ) {
        self.query_manager
            .add_file_stats(trace_id, files, records, original_size, compressed_size)
            .await;
    }

    // add work group
    pub async fn add_work_group(&self, trace_id: &str, work_group: Option<WorkGroup>) {
        self.query_manager
            .add_work_group(trace_id, work_group)
            .await;
    }
}

#[derive(Clone, Debug)]
#[cfg(not(feature = "enterprise"))]
pub struct Searcher;

#[cfg(not(feature = "enterprise"))]
impl Searcher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl Search for Searcher {
    #[cfg(feature = "enterprise")]
    async fn query_status(
        &self,
        _req: Request<QueryStatusRequest>,
    ) -> Result<Response<QueryStatusResponse>, Status> {
        let status = self.get_task_status().await;
        Ok(Response::new(QueryStatusResponse { status }))
    }

    #[cfg(not(feature = "enterprise"))]
    async fn query_status(
        &self,
        _req: Request<QueryStatusRequest>,
    ) -> Result<Response<QueryStatusResponse>, Status> {
        Err(Status::unimplemented("Not Supported"))
    }

    #[cfg(feature = "enterprise")]
    async fn cancel_query(
        &self,
        req: Request<CancelQueryRequest>,
    ) -> Result<Response<CancelQueryResponse>, Status> {
        let trace_id = req.into_inner().trace_id;
        match self.remove(&trace_id, true).await {
            Some(cancelled) => {
                for (_, senders) in cancelled {
                    for sender in senders.abort_senders.into_iter().rev() {
                        let _ = sender.send(());
                    }
                    metrics::QUERY_CANCELED_NUMS
                        .with_label_values(&[&senders.org_id.unwrap_or_default()])
                        .inc();
                }
                Ok(Response::new(CancelQueryResponse { is_success: true }))
            }
            None => Ok(Response::new(CancelQueryResponse { is_success: false })),
        }
    }

    #[cfg(not(feature = "enterprise"))]
    async fn cancel_query(
        &self,
        _req: Request<CancelQueryRequest>,
    ) -> Result<Response<CancelQueryResponse>, Status> {
        Err(Status::unimplemented("Not Supported"))
    }

    #[cfg(feature = "enterprise")]
    async fn cluster_cancel_query(
        &self,
        req: Request<CancelQueryRequest>,
    ) -> Result<Response<CancelQueryResponse>, Status> {
        use crate::service::search as SearchService;

        let trace_id = req.into_inner().trace_id;
        match SearchService::cancel_query("", &trace_id).await {
            Ok(ret) => Ok(Response::new(CancelQueryResponse {
                is_success: ret.is_success,
            })),
            Err(_) => Ok(Response::new(CancelQueryResponse { is_success: false })),
        }
    }

    #[cfg(not(feature = "enterprise"))]
    async fn cluster_cancel_query(
        &self,
        _req: Request<CancelQueryRequest>,
    ) -> Result<Response<CancelQueryResponse>, Status> {
        Err(Status::unimplemented("Not Supported"))
    }
}
