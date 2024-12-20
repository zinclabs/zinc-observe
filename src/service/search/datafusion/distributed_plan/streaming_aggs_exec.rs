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
    collections::VecDeque,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use arrow::{array::RecordBatch, datatypes::SchemaRef};
use dashmap::DashMap;
use datafusion::{
    common::{Result, Statistics},
    execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext},
    physical_expr::EquivalenceProperties,
    physical_plan::{
        memory::MemoryStream, DisplayAs, DisplayFormatType, ExecutionMode, ExecutionPlan,
        ExecutionPlanProperties, Partitioning, PlanProperties,
    },
};
use futures::{Stream, StreamExt};
use once_cell::sync::Lazy;

pub static GLOBAL_CACHE: Lazy<Arc<StreamingAggsCache>> =
    Lazy::new(|| Arc::new(StreamingAggsCache::default()));

/// Collected statistics for files
/// Cache is invalided when file size or last modification has changed
pub struct StreamingAggsCache {
    data: DashMap<String, Vec<RecordBatch>>,
    cacher: parking_lot::Mutex<VecDeque<String>>,
    max_entries: usize,
}

impl StreamingAggsCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            data: DashMap::new(),
            cacher: parking_lot::Mutex::new(VecDeque::new()),
            max_entries,
        }
    }

    pub fn get(&self, k: &str) -> Option<Vec<RecordBatch>> {
        self.data.get(k).map(|v| v.value().clone())
    }

    pub fn insert(&self, k: String, v: RecordBatch) {
        let mut w = self.cacher.lock();
        if w.len() >= self.max_entries {
            if let Some(k) = w.pop_front() {
                self.data.remove(&k);
            }
        }
        w.push_back(k.clone());
        drop(w);
        let mut entry = self.data.entry(k).or_default();
        entry.push(v);
    }

    pub fn remove(&self, k: &str) {
        self.data.remove(k);
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Default for StreamingAggsCache {
    fn default() -> Self {
        Self::new(
            config::get_config()
                .limit
                .datafusion_file_stat_cache_max_entries,
        )
    }
}

#[allow(dead_code)]
pub fn remove_cache(id: &str) {
    GLOBAL_CACHE.remove(id);
}

#[derive(Debug)]
pub struct StreamingAggsExec {
    id: String,
    input: Arc<dyn ExecutionPlan>,
    /// Cache holding plan properties like equivalences, output partitioning etc.
    cache: PlanProperties,
}

impl StreamingAggsExec {
    /// Create a new UnionExec
    pub fn new(id: String, input: Arc<dyn ExecutionPlan>) -> Self {
        let partitions_num = input.output_partitioning().partition_count();
        let cache = Self::compute_properties(
            Arc::clone(&input.schema()),
            partitions_num + 1, // itself has one partition
        );
        Self { id, input, cache }
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

    fn data(&self) -> Result<Vec<RecordBatch>> {
        Ok(GLOBAL_CACHE.get(&self.id).unwrap_or_default())
    }
}

impl DisplayAs for StreamingAggsExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(f, "StreamingAggsExec: streaming_id={}", self.id)
            }
        }
    }
}

impl ExecutionPlan for StreamingAggsExec {
    fn name(&self) -> &'static str {
        "StreamingAggsExec"
    }

    /// Return a reference to Any that can be used for downcasting
    fn as_any(&self) -> &dyn Any {
        self
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
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        // self data
        if partition == 0 {
            return Ok(Box::pin(MemoryStream::try_new(
                self.data()?,
                Arc::clone(&self.schema()),
                None,
            )?));
        }
        // input data
        Ok(Box::pin(MonitorStream::new(
            self.id.clone(),
            self.input.schema(),
            self.input.execute(partition - 1, context)?,
        )))
    }

    fn statistics(&self) -> Result<Statistics> {
        Ok(Statistics::new_unknown(&self.schema()))
    }

    fn benefits_from_input_partitioning(&self) -> Vec<bool> {
        vec![false; self.children().len()]
    }

    fn supports_limit_pushdown(&self) -> bool {
        true
    }
}

struct MonitorStream {
    id: String,
    schema: SchemaRef,
    stream: SendableRecordBatchStream,
}

impl MonitorStream {
    fn new(id: String, schema: SchemaRef, stream: SendableRecordBatchStream) -> Self {
        Self { id, schema, stream }
    }
}

impl Stream for MonitorStream {
    type Item = Result<RecordBatch>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.stream.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(record_batch))) => {
                GLOBAL_CACHE.insert(self.id.clone(), record_batch.clone());
                Poll::Ready(Some(Ok(record_batch)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Err(e))) => {
                log::error!("Error in MonitorStream: {:?}", e);
                Poll::Ready(None)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

impl RecordBatchStream for MonitorStream {
    /// Get the schema
    fn schema(&self) -> SchemaRef {
        Arc::clone(&self.schema)
    }
}
