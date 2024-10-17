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

use datafusion::error::Result;
use promql_parser::parser::{Expr as PromExpr, LabelModifier};

use crate::service::promql::{value::Value, Engine};

pub async fn topk(
    ctx: &mut Engine,
    param: Box<PromExpr>,
    modifier: &Option<LabelModifier>,
    data: &Value,
) -> Result<Value> {
    super::eval_top(ctx, param, data, modifier, false).await
}
