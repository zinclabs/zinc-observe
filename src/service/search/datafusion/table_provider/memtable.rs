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

use std::{any::Any, sync::Arc};

use arrow::array::RecordBatch;
use arrow_schema::{DataType, SchemaRef};
use async_trait::async_trait;
use datafusion::{
    common::{project_schema, Constraints, Result},
    datasource::{MemTable, TableProvider},
    execution::context::SessionState,
    physical_plan::{
        expressions::{CastExpr, Column},
        projection::ProjectionExec,
        ExecutionPlan, PhysicalExpr,
    },
};
use datafusion_expr::{Expr, TableType};
use hashbrown::HashMap;

#[derive(Debug)]
pub(crate) struct NewMemTable {
    mem_table: MemTable,
    diff_rules: HashMap<String, DataType>,
}

impl NewMemTable {
    /// Create a new in-memory table from the provided schema and record batches
    pub fn try_new(
        schema: SchemaRef,
        partitions: Vec<Vec<RecordBatch>>,
        rules: HashMap<String, DataType>,
    ) -> Result<Self> {
        let mem = MemTable::try_new(schema, partitions)?;
        Ok(Self {
            mem_table: mem,
            diff_rules: rules,
        })
    }
}

#[async_trait]
impl TableProvider for NewMemTable {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.mem_table.schema()
    }

    fn constraints(&self) -> Option<&Constraints> {
        self.mem_table.constraints()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    async fn scan(
        &self,
        state: &SessionState,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let memory_exec = self
            .mem_table
            .scan(state, projection, filters, limit)
            .await?;

        // add the diff rules to the execution plan
        if self.diff_rules.is_empty() {
            return Ok(memory_exec);
        }
        let projected_schema = project_schema(&self.schema(), projection)?;
        let mut exprs: Vec<(Arc<dyn PhysicalExpr>, String)> =
            Vec::with_capacity(projected_schema.fields().len());
        for (idx, field) in projected_schema.fields().iter().enumerate() {
            let name = field.name().to_string();
            let col = Arc::new(Column::new(&name, idx));
            if let Some(data_type) = self.diff_rules.get(&name) {
                exprs.push((Arc::new(CastExpr::new(col, data_type.clone(), None)), name));
            } else {
                exprs.push((col, name));
            }
        }
        let projection_exec = ProjectionExec::try_new(exprs, memory_exec)?;
        Ok(Arc::new(projection_exec))
    }

    async fn insert_into(
        &self,
        state: &SessionState,
        input: Arc<dyn ExecutionPlan>,
        overwrite: bool,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        self.mem_table.insert_into(state, input, overwrite).await
    }

    fn get_column_default(&self, column: &str) -> Option<&Expr> {
        self.mem_table.get_column_default(column)
    }
}
