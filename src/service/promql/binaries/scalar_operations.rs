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

use datafusion::error::{DataFusionError, Result};
use promql_parser::parser::token;

/// Supported operation between two float type values.
pub fn scalar_binary_operations(
    token: u8,
    lhs: f64,
    rhs: f64,
    return_bool: bool,
    comparison_operator: bool,
) -> Result<f64> {
    let value = if comparison_operator {
        let val = match token {
            token::T_EQLC => lhs == rhs,
            token::T_NEQ => lhs != rhs,
            token::T_GTR => lhs > rhs,
            token::T_LSS => lhs < rhs,
            token::T_GTE => lhs >= rhs,
            token::T_LTE => lhs <= rhs,
            _ => {
                return Err(DataFusionError::NotImplemented(format!(
                    "Unsupported scalar comparison operation: {:?} {:?} {:?}",
                    token, lhs, rhs
                )));
            }
        };
        if return_bool {
            val as u32 as f64
        } else {
            // if the return value was true, that means our element
            // satisfies the comparison, hence return it
            if val {
                lhs
            } else {
                return Err(DataFusionError::NotImplemented(format!(
                    "Must filter this lhs value scalar operation: {:?} {:?} {:?}",
                    token, lhs, rhs
                )));
            }
        }
    } else {
        match token {
            token::T_ADD => lhs + rhs,
            token::T_SUB => lhs - rhs,
            token::T_MUL => lhs * rhs,
            token::T_DIV => lhs / rhs,
            token::T_POW => lhs.powf(rhs),
            token::T_MOD => lhs % rhs,
            token::T_ATAN2 => lhs.atan2(rhs),
            _ => {
                return Err(DataFusionError::NotImplemented(format!(
                    "Unsupported scalar operation: {:?} {:?} {:?}",
                    token, lhs, rhs
                )));
            }
        }
    };
    Ok(value)
}
