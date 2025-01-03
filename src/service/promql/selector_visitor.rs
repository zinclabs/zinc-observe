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

use promql_parser::{parser::Expr, util::ExprVisitor};

pub struct MetricSelectorVisitor {
    pub(crate) exprs: Vec<Expr>,
}

impl MetricSelectorVisitor {
    pub fn new() -> Self {
        Self { exprs: vec![] }
    }

    pub fn exprs_to_string(&self) -> String {
        self.exprs
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .join(",")
    }
}

impl Default for MetricSelectorVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ExprVisitor for MetricSelectorVisitor {
    type Error = &'static str;

    fn pre_visit(&mut self, expr: &Expr) -> Result<bool, Self::Error> {
        match expr {
            Expr::VectorSelector(_) => {
                self.exprs.push(expr.clone());
            }
            Expr::MatrixSelector(_) => {
                self.exprs.push(expr.clone());
            }
            _ => {}
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {

    use promql_parser::parser;

    use super::*;

    #[test]
    fn test_selector_visitor() {
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
        let mut visitor = MetricSelectorVisitor::default();
        promql_parser::util::walk_expr(&mut visitor, &ast).unwrap();

        let expected = vec![
            "container_fs_reads_bytes_total{container!=\"\",device=~\"(/dev/)?(mmcblk[0-9]p[0-9]+|nvme.+|rbd.+|sd.+|vd.+|xvd.+|dm-.+|md.+|dasd.+)\"}[5m]",
            "container_fs_writes_bytes_total{container!=\"\",device=~\"(/dev/)?(mmcblk[0-9]p[0-9]+|nvme.+|rbd.+|sd.+|vd.+|xvd.+|dm-.+|md.+|dasd.+)\"}[5m]",
        ];
        assert_eq!(visitor.exprs_to_string(), expected.join(","));

        let promql = r#"http_requests_total{environment=~"staging|testing|development",method!="GET"} offset 5m"#;

        let ast = parser::parse(promql).unwrap();
        let mut visitor = MetricSelectorVisitor::default();
        promql_parser::util::walk_expr(&mut visitor, &ast).unwrap();
        let expected = vec![
            "http_requests_total{environment=~\"staging|testing|development\",method!=\"GET\"} offset 5m",
        ];
        assert_eq!(visitor.exprs_to_string(), expected.join(","));
    }
}
