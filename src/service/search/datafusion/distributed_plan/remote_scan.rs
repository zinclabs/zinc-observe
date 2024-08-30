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
use config::meta::{cluster::NodeInfo, stream::FileKey};
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
use prost::Message;
use proto::cluster_rpc::{self, FlightSearchRequest, PartitionKeys};
use tonic::{
    codec::CompressionEncoding,
    metadata::{MetadataKey, MetadataValue},
    transport::Channel,
    Streaming,
};

use super::codec::{ComposedPhysicalExtensionCodec, EmptyExecPhysicalExtensionCodec};
use crate::service::search::request::Request;

/// Execution plan for empty relation with produce_one_row=false
#[derive(Debug)]
pub struct RemoteScanExec {
    input: Arc<dyn ExecutionPlan>,
    file_list: Vec<Vec<FileKey>>,
    partition_keys: Vec<PartitionKeys>,
    match_all_keys: Vec<String>,
    is_leader: bool,
    req: Request,
    nodes: Vec<Arc<dyn NodeInfo>>,
    partitions: usize,
    cache: PlanProperties,
}

impl RemoteScanExec {
    /// Create a new RemoteScanExec
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        file_list: Vec<Vec<FileKey>>,
        partition_keys: Vec<PartitionKeys>,
        match_all_keys: Vec<String>,
        is_leader: bool,
        req: Request,
        nodes: Vec<Arc<dyn NodeInfo>>,
    ) -> Self {
        let output_partitions = nodes.len();
        let cache = Self::compute_properties(Arc::clone(&input.schema()), output_partitions);
        RemoteScanExec {
            input,
            req,
            file_list,
            partition_keys,
            match_all_keys,
            is_leader,
            nodes,
            partitions: output_partitions,
            cache,
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
        let fut = get_remote_batch(
            self.input.clone(),
            partition,
            self.nodes[partition].clone(),
            self.file_list[partition].clone(),
            self.partition_keys.clone(),
            self.match_all_keys.clone(),
            self.is_leader,
            req,
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
    file_list: Vec<FileKey>,
    partition_keys: Vec<PartitionKeys>,
    match_all_keys: Vec<String>,
    is_leader: bool,
    req: Request,
) -> Result<SendableRecordBatchStream> {
    let proto = ComposedPhysicalExtensionCodec {
        codecs: vec![Arc::new(EmptyExecPhysicalExtensionCodec {})],
    };
    let physical_plan_bytes = physical_plan_to_bytes_with_extension_codec(input, &proto)?;
    let (start_time, end_time) = req.time_range.unwrap_or((0, 0));
    let request = FlightSearchRequest {
        trace_id: req.trace_id.clone(),
        partition: partition as u32,
        org_id: req.org_id.clone(),
        stream_type: req.stream_type.to_string(),
        plan: physical_plan_bytes.to_vec(),
        file_list: file_list.iter().map(cluster_rpc::FileKey::from).collect(),
        partition_keys,
        match_all_keys,
        is_leader,
        start_time,
        end_time,
        timeout: req.timeout,
        work_group: req.work_group.clone(),
        user_id: req.user_id.clone(),
        search_event_type: req.search_event_type,
    };

    let mut buf: Vec<u8> = Vec::new();
    request
        .encode(&mut buf)
        .map_err(|e| datafusion::common::DataFusionError::Internal(format!("{e:?}")))?;

    let request = tonic::Request::new(Ticket {
        ticket: buf.clone().into(),
    });

    let cfg = config::get_config();
    let org_id: MetadataValue<_> = req
        .org_id
        .parse()
        .map_err(|_| DataFusionError::Internal("invalid org_id".to_string()))?;

    let org_header_key: MetadataKey<_> = cfg
        .grpc
        .org_header_key
        .parse()
        .map_err(|_| DataFusionError::Internal("invalid org_header_key".to_string()))?;
    let token: MetadataValue<_> = node
        .get_auth_token()
        .parse()
        .map_err(|_| DataFusionError::Internal("invalid token".to_string()))?;
    let channel = Channel::from_shared(node.get_grpc_addr().clone())
        .unwrap()
        .connect_timeout(std::time::Duration::from_secs(cfg.grpc.connect_timeout))
        .connect()
        .await
        .map_err(|err| {
            log::error!(
                "[trace_id {}] search->grpc: node: {}, connect err: {:?}",
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

    // the schema should be the first message returned, else client should error
    let flight_data = stream.message().await.unwrap().unwrap();
    // convert FlightData to a stream
    let schema = Arc::new(Schema::try_from(&flight_data)?);

    Ok(Box::pin(FlightStream::new(schema, stream)))
}

struct FlightStream {
    schema: SchemaRef,
    stream: Streaming<FlightData>,
}

impl FlightStream {
    fn new(schema: SchemaRef, stream: Streaming<FlightData>) -> Self {
        Self { schema, stream }
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
                Poll::Ready(Some(Err(DataFusionError::Internal(e.to_string()))))
            }
        }
    }

    // fn size_hint(&self) -> (usize, Option<usize>) {
    //     // TODO: check this
    //     (1, None)
    // }
}

impl RecordBatchStream for FlightStream {
    /// Get the schema
    fn schema(&self) -> SchemaRef {
        Arc::clone(&self.schema)
    }
}
