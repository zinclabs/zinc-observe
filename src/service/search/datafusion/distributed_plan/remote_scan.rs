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

use std::{
    any::Any,
    collections::HashMap,
    sync::Arc,
    task::{Context, Poll},
};

use arrow::record_batch::RecordBatch;
use arrow_flight::{
    flight_service_client::FlightServiceClient, utils::flight_data_to_arrow_batch, FlightData,
    Ticket,
};
use arrow_schema::{Schema, SchemaRef};
use config::{
    meta::{cluster::NodeInfo, search::ScanStats, stream::FileKey},
    utils::json,
};
use datafusion::{
    common::{DataFusionError, Result, Statistics},
    execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext},
    physical_expr::{EquivalenceProperties, Partitioning},
    physical_plan::{
        stream::RecordBatchStreamAdapter, DisplayAs, DisplayFormatType, ExecutionMode,
        ExecutionPlan, PlanProperties,
    },
};
use datafusion_proto::bytes::physical_plan_to_bytes_with_extension_codec;
use futures::{Stream, StreamExt, TryStreamExt};
use parking_lot::Mutex;
use prost::Message;
use proto::cluster_rpc::{self, FlightSearchRequest, KvItem};
use tonic::{
    codec::CompressionEncoding,
    metadata::{MetadataKey, MetadataValue},
    Streaming,
};

use super::codec::{ComposedPhysicalExtensionCodec, EmptyExecPhysicalExtensionCodec};
use crate::service::{
    grpc::get_cached_channel,
    search::{index::IndexCondition, request::Request, MetadataMap},
};

/// Execution plan for empty relation with produce_one_row=false
#[derive(Debug)]
pub struct RemoteScanExec {
    input: Arc<dyn ExecutionPlan>,
    file_id_list: Vec<Vec<i64>>,
    idx_file_list: Vec<FileKey>,
    equal_keys: Vec<KvItem>,
    match_all_keys: Vec<String>,
    index_condition: Option<IndexCondition>,
    is_super_cluster: bool,
    req: Request,
    nodes: Vec<Arc<dyn NodeInfo>>,
    partitions: usize,
    cache: PlanProperties,
    pub scan_stats: Arc<Mutex<ScanStats>>,
    pub partial_err: Arc<Mutex<String>>,
    pub context: opentelemetry::Context,
}

impl RemoteScanExec {
    /// Create a new RemoteScanExec
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        file_id_list: Vec<Vec<i64>>,
        idx_file_list: Vec<FileKey>,
        equal_keys: Vec<KvItem>,
        match_all_keys: Vec<String>,
        index_condition: Option<IndexCondition>,
        is_super_cluster: bool,
        req: Request,
        nodes: Vec<Arc<dyn NodeInfo>>,
        context: opentelemetry::Context,
    ) -> Self {
        let output_partitions = nodes.len();
        let cache = Self::compute_properties(Arc::clone(&input.schema()), output_partitions);
        RemoteScanExec {
            input,
            req,
            file_id_list,
            idx_file_list,
            equal_keys,
            match_all_keys,
            index_condition,
            is_super_cluster,
            nodes,
            partitions: output_partitions,
            cache,
            scan_stats: Arc::new(Mutex::new(ScanStats::default())),
            partial_err: Arc::new(Mutex::new(String::new())),
            context,
        }
    }

    fn output_partitioning_helper(n_partitions: usize) -> Partitioning {
        Partitioning::UnknownPartitioning(n_partitions)
    }

    /// This function creates the cache object that stores the plan properties such as schema,
    /// equivalence properties, ordering, partitioning, etc.
    fn compute_properties(schema: SchemaRef, n_partitions: usize) -> PlanProperties {
        let eq_properties = EquivalenceProperties::new(schema);
        let output_partitioning = Self::output_partitioning_helper(n_partitions);
        PlanProperties::new(
            eq_properties,
            // Output Partitioning
            output_partitioning,
            // Execution Mode
            ExecutionMode::Bounded,
        )
    }
}

impl DisplayAs for RemoteScanExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(
                    f,
                    "RemoteScanExec: input_partitions=output_partitions={}",
                    self.partitions,
                )
            }
        }
    }
}

impl ExecutionPlan for RemoteScanExec {
    fn name(&self) -> &'static str {
        "RemoteScanExec"
    }

    /// Return a reference to Any that can be used for downcasting
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        Arc::clone(&self.input.schema())
    }

    fn properties(&self) -> &PlanProperties {
        &self.cache
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        _: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let req = self.req.clone();
        let file_id_list = if self.file_id_list.is_empty() {
            vec![]
        } else {
            self.file_id_list[partition].clone()
        };
        let fut = get_remote_batch(
            self.input.clone(),
            partition,
            self.nodes[partition].clone(),
            file_id_list,
            self.idx_file_list.clone(),
            self.equal_keys.clone(),
            self.match_all_keys.clone(),
            self.index_condition.clone(),
            self.is_super_cluster,
            req,
            self.scan_stats.clone(),
            self.partial_err.clone(),
            self.context.clone(),
        );
        let stream = futures::stream::once(fut).try_flatten();
        Ok(Box::pin(RecordBatchStreamAdapter::new(
            self.schema().clone(),
            stream,
        )))
    }

    fn statistics(&self) -> Result<Statistics> {
        Ok(Statistics::new_unknown(&self.schema()))
    }
}

#[allow(clippy::too_many_arguments)]
async fn get_remote_batch(
    input: Arc<dyn ExecutionPlan>,
    partition: usize,
    node: Arc<dyn NodeInfo>,
    file_id_list: Vec<i64>,
    idx_file_list: Vec<FileKey>,
    equal_keys: Vec<KvItem>,
    match_all_keys: Vec<String>,
    index_condition: Option<IndexCondition>,
    is_super_cluster: bool,
    req: Request,
    scan_stats: Arc<Mutex<ScanStats>>,
    partial_err: Arc<Mutex<String>>,
    context: opentelemetry::Context,
) -> Result<SendableRecordBatchStream> {
    let proto = ComposedPhysicalExtensionCodec {
        codecs: vec![Arc::new(EmptyExecPhysicalExtensionCodec {})],
    };
    let physical_plan_bytes = physical_plan_to_bytes_with_extension_codec(input, &proto)?;
    let (start_time, end_time) = req.time_range.unwrap_or((0, 0));

    let is_querier = !file_id_list.is_empty();
    let idx_file_list = if is_querier {
        idx_file_list
            .iter()
            .map(|f| cluster_rpc::IdxFileName {
                key: f.key.clone(),
                segment_ids: f.segment_ids.clone(),
            })
            .collect()
    } else {
        vec![]
    };

    let index_condition = match index_condition {
        Some(index_condition) => json::to_string(&index_condition).unwrap(),
        None => "".to_string(),
    };

    let request = FlightSearchRequest {
        trace_id: req.trace_id.clone(),
        partition: partition as u32,
        org_id: req.org_id.clone(),
        stream_type: req.stream_type.to_string(),
        plan: physical_plan_bytes.to_vec(),
        file_id_list: file_id_list.to_vec(),
        idx_file_list,
        equal_keys,
        match_all_keys,
        is_super_cluster,
        start_time,
        end_time,
        timeout: req.timeout,
        work_group: req.work_group.clone(),
        user_id: req.user_id.clone(),
        search_event_type: req.search_event_type,
        use_inverted_index: req.use_inverted_index,
        index_condition,
    };

    log::info!(
        "[trace_id {}] flight->search: request node: {}, is_querier: {}, files: {}, idx_files: {}",
        req.trace_id,
        &node.get_grpc_addr(),
        is_querier,
        request.file_id_list.len(),
        request.idx_file_list.len(),
    );

    let mut buf: Vec<u8> = Vec::new();
    request
        .encode(&mut buf)
        .map_err(|e| datafusion::common::DataFusionError::Internal(format!("{e:?}")))?;

    let mut request = tonic::Request::new(Ticket {
        ticket: buf.clone().into(),
    });

    let cfg = config::get_config();
    let org_id: MetadataValue<_> = req
        .org_id
        .parse()
        .map_err(|_| DataFusionError::Internal("invalid org_id".to_string()))?;

    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut MetadataMap(request.metadata_mut()))
    });

    let org_header_key: MetadataKey<_> = cfg
        .grpc
        .org_header_key
        .parse()
        .map_err(|_| DataFusionError::Internal("invalid org_header_key".to_string()))?;
    let token: MetadataValue<_> = node
        .get_auth_token()
        .parse()
        .map_err(|_| DataFusionError::Internal("invalid token".to_string()))?;
    let channel = get_cached_channel(&node.get_grpc_addr())
        .await
        .map_err(|err| {
            log::error!(
                "[trace_id {}] flight->search: node: {}, connect err: {:?}",
                req.trace_id.clone(),
                &node.get_grpc_addr(),
                err
            );
            DataFusionError::Internal("connect search node error".to_string())
        })?;
    let mut client =
        FlightServiceClient::with_interceptor(channel, move |mut req: tonic::Request<()>| {
            req.metadata_mut().insert("authorization", token.clone());
            req.metadata_mut()
                .insert(org_header_key.clone(), org_id.clone());
            Ok(req)
        });
    client = client
        .send_compressed(CompressionEncoding::Gzip)
        .accept_compressed(CompressionEncoding::Gzip)
        .max_decoding_message_size(cfg.grpc.max_message_size * 1024 * 1024)
        .max_encoding_message_size(cfg.grpc.max_message_size * 1024 * 1024);

    let mut stream = client
        .do_get(request)
        .await
        .map_err(|e| DataFusionError::Execution(e.to_string()))?
        .into_inner();

    let start = std::time::Instant::now();

    // the schema should be the first message returned, else client should error
    let flight_data = stream.message().await.unwrap().unwrap();
    // convert FlightData to a stream
    let schema = Arc::new(Schema::try_from(&flight_data)?);

    let mut files = 0;
    let mut scan_size = 0;
    if let Some(stats) = schema.metadata().get("scan_stats") {
        let stats: ScanStats = serde_json::from_str(stats).unwrap_or_default();
        files = stats.files;
        scan_size = stats.original_size;
        scan_stats.lock().add(&stats);
    }

    Ok(Box::pin(FlightStream::new(
        req.trace_id,
        schema,
        stream,
        node.get_grpc_addr(),
        is_querier,
        files,
        scan_size,
        partial_err,
        start,
    )))
}

struct FlightStream {
    trace_id: String,
    schema: SchemaRef,
    stream: Streaming<FlightData>,
    node_addr: String,
    is_querier: bool,
    files: i64,
    scan_size: i64,
    partial_err: Arc<Mutex<String>>,
    start: std::time::Instant,
}

impl FlightStream {
    #[allow(clippy::too_many_arguments)]
    fn new(
        trace_id: String,
        schema: SchemaRef,
        stream: Streaming<FlightData>,
        node_addr: String,
        is_querier: bool,
        files: i64,
        scan_size: i64,
        partial_err: Arc<Mutex<String>>,
        start: std::time::Instant,
    ) -> Self {
        Self {
            trace_id,
            schema,
            stream,
            node_addr,
            is_querier,
            files,
            scan_size,
            partial_err,
            start,
        }
    }
}

impl Stream for FlightStream {
    type Item = Result<RecordBatch>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let dictionaries_by_field = HashMap::new();

        match self.stream.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(flight_data))) => {
                let record_batch = flight_data_to_arrow_batch(
                    &flight_data,
                    self.schema.clone(),
                    &dictionaries_by_field,
                )?;
                Poll::Ready(Some(Ok(record_batch)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Err(e))) => {
                {
                    let mut guard = self.partial_err.lock();
                    let partial_err = guard.clone();
                    if partial_err.is_empty() {
                        guard.push_str(e.to_string().as_str());
                    } else {
                        guard.push_str(format!(" \n {}", e).as_str());
                    }
                }
                Poll::Ready(None)
            }
        }
    }
}

impl Drop for FlightStream {
    fn drop(&mut self) {
        log::info!(
            "[trace_id {}] flight->search: response node: {}, is_querier: {}, took: {} ms, files: {}, scan_size: {}",
            self.trace_id,
            self.node_addr,
            self.is_querier,
            self.start.elapsed().as_millis(),
            self.files,
            self.scan_size / 1024 / 1024,
        );
    }
}

impl RecordBatchStream for FlightStream {
    /// Get the schema
    fn schema(&self) -> SchemaRef {
        Arc::clone(&self.schema)
    }
}
