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

use std::collections::HashSet;

use promql_parser::{
    parser::{self, Expr},
    util::ExprVisitor,
};

pub struct MetricNameVisitor {
    pub(crate) name: HashSet<String>,
}

fn get_name_from_expr(vector_selector: &parser::VectorSelector) -> String {
    vector_selector.name.as_ref().unwrap().to_string()
}

impl ExprVisitor for MetricNameVisitor {
    type Error = &'static str;

    fn pre_visit(&mut self, expr: &Expr) -> Result<bool, Self::Error> {
        match expr {
            Expr::VectorSelector(vector_selector) => {
                self.name.insert(get_name_from_expr(vector_selector));
                return Ok(true);
            }
            Expr::MatrixSelector(matrix_selector) => {
                self.name.insert(get_name_from_expr(&matrix_selector.vs));
                return Ok(true);
            }
            Expr::NumberLiteral(_) | Expr::StringLiteral(_) => return Ok(false),
            Expr::Unary(uni) => {
                let _ = self.pre_visit(&uni.expr);
            }
            Expr::Binary(bin) => {
                let _ = self.pre_visit(&bin.lhs);
                let _ = self.pre_visit(&bin.rhs);
            }
            Expr::Paren(_) => return Ok(true),
            Expr::Subquery(sub) => {
                let _ = self.pre_visit(&sub.expr);
            }
            Expr::Call(call) => {
                for expr in &call.args.args {
                    let _ = self.pre_visit(expr);
                }
            }
            Expr::Extension(_) => {
                return Ok(false);
            }
            Expr::Aggregate(aggr) => {
                let _ = self.pre_visit(&aggr.expr);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use promql_parser::parser;

    use crate::service::promql::name_visitor::MetricNameVisitor;

    #[test]
    fn name_test() {
        let promql = r#"sum by(k8s_node_name)(
                            rate(container_fs_reads_bytes_total{
                                container!= "",
                                device=~"(/dev/)?(mmcblk[0-9]p[0-9]+|nvme.+|rbd.+|sd.+|vd.+|xvd.+|dm-.+|md.+|dasd.+)"
                            }[5m])
                            +
                            rate(container_fs_writes_bytes_total{
                                container!= "",
                                device=~"(/dev/)?(mmcblk[0-9]p[0-9]+|nvme.+|rbd.+|sd.+|vd.+|xvd.+|dm-.+|md.+|dasd.+)"
                            }[5m])
                            )"#;

        let ast = parser::parse(promql).unwrap();
        let mut visitor = MetricNameVisitor {
            name: HashSet::new(),
        };
        promql_parser::util::walk_expr(&mut visitor, &ast).unwrap();

        assert_eq!(visitor.name.len(), 2);

        let promql = r#"http_requests_total{environment=~"staging|testing|development",method!="GET"} offset 5m"#;

        let ast = parser::parse(promql).unwrap();
        let mut visitor = MetricNameVisitor {
            name: HashSet::new(),
        };
        promql_parser::util::walk_expr(&mut visitor, &ast).unwrap();
        assert_eq!(visitor.name.len(), 1);
    }
}
