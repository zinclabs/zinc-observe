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

use std::fmt;

use datafusion::{
    common::{Column, DFSchemaRef, Result},
    logical_expr::{LogicalPlan, UserDefinedLogicalNodeCore},
    prelude::Expr,
};

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd)]
pub struct DeduplicationLogicalNode {
    pub input: LogicalPlan,
    pub deduplication_columns: Vec<Column>,
    #[allow(unused)]
    pub max_rows: usize,
}

impl DeduplicationLogicalNode {
    #[allow(unused)]
    pub fn new(input: LogicalPlan, deduplication_columns: Vec<Column>) -> Self {
        Self {
            input,
            deduplication_columns,
            max_rows: 1,
        }
    }
}

impl UserDefinedLogicalNodeCore for DeduplicationLogicalNode {
    fn name(&self) -> &str {
        "Deduplication"
    }

    fn inputs(&self) -> Vec<&LogicalPlan> {
        vec![&self.input]
    }

    fn schema(&self) -> &DFSchemaRef {
        self.input.schema()
    }

    fn expressions(&self) -> Vec<Expr> {
        vec![]
    }

    fn fmt_for_explain(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Deduplication")
    }

    fn with_exprs_and_inputs(&self, _exprs: Vec<Expr>, inputs: Vec<LogicalPlan>) -> Result<Self> {
        Ok(DeduplicationLogicalNode {
            input: inputs.into_iter().next().unwrap(),
            deduplication_columns: self.deduplication_columns.clone(),
            max_rows: self.max_rows,
        })
    }
}
