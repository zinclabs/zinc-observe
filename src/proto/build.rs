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

use std::io::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=proto");

    tonic_build::configure()
        .type_attribute("FileList", "#[derive(Eq)]")
        .type_attribute("FileList", "#[derive(serde::Serialize)]")
        .type_attribute("FileKey", "#[derive(Eq)]")
        .type_attribute("FileKey", "#[derive(serde::Serialize)]")
        .type_attribute("FileDescriptor", "#[derive(Eq)]")
        .type_attribute("FileDescriptor", "#[derive(serde::Serialize)]")
        .type_attribute("FileMeta", "#[derive(Eq)]")
        .type_attribute("FileMeta", "#[derive(serde::Serialize)]")
        .type_attribute("Job", "#[derive(Eq)]")
        .type_attribute("Job", "#[derive(serde::Serialize)]")
        .type_attribute("Partition", "#[derive(Eq)]")
        .type_attribute("SearchQuery", "#[derive(Eq)]")
        .type_attribute("SearchRequest", "#[derive(Eq)]")
        .type_attribute("SearchResponse", "#[derive(Eq)]")
        .type_attribute("SearchResponse", "#[derive(serde::Serialize)]")
        .type_attribute("SearchAggRequest", "#[derive(Eq)]")
        .type_attribute("SearchAggRequest", "#[derive(serde::Serialize)]")
        .type_attribute("SearchAggResponse", "#[derive(Eq)]")
        .type_attribute("SearchAggResponse", "#[derive(serde::Serialize)]")
        .type_attribute("Series", "#[derive(serde::Serialize)]")
        .type_attribute("Label", "#[derive(serde::Serialize)]")
        .type_attribute("Sample", "#[derive(serde::Serialize)]")
        .type_attribute("MetricsQueryStmt", "#[derive(serde::Serialize)]")
        .type_attribute("MetricsQueryRequest", "#[derive(serde::Serialize)]")
        .type_attribute("MetricsQueryResponse", "#[derive(serde::Serialize)]")
        .type_attribute("ScanStats", "#[derive(Eq)]")
        .type_attribute("ScanStats", "#[derive(serde::Serialize)]")
        .compile(
            &[
                "proto/cluster/common.proto",
                "proto/cluster/event.proto",
                "proto/cluster/filelist.proto",
                "proto/cluster/metrics.proto",
                "proto/cluster/search.proto",
                "proto/cluster/usage.proto",
                "proto/cluster/querycache.proto",
            ],
            &["proto"],
        )
        .unwrap();

    let mut config = prost_build::Config::new();
    config
        .type_attribute(
            "MetricMetadata",
            "#[derive(serde::Deserialize,serde::Serialize)]",
        )
        .type_attribute(
            "MetricType",
            "#[derive(serde::Deserialize,serde::Serialize)]",
        )
        .type_attribute("Sample", "#[derive(serde::Deserialize,serde::Serialize)]")
        .type_attribute("Exemplar", "#[derive(serde::Deserialize,serde::Serialize)]")
        .type_attribute(
            "Histogram",
            "#[derive(serde::Deserialize,serde::Serialize)]",
        )
        .type_attribute(
            "BucketSpan",
            "#[derive(serde::Deserialize,serde::Serialize)]",
        )
        .type_attribute(
            "TimeSeries",
            "#[derive(serde::Deserialize,serde::Serialize)]",
        )
        .type_attribute("Label", "#[derive(serde::Deserialize,serde::Serialize)]")
        .type_attribute("Labels", "#[derive(serde::Deserialize,serde::Serialize)]")
        .type_attribute(
            "LabelMatcher",
            "#[derive(serde::Deserialize,serde::Serialize)]",
        )
        .type_attribute(
            "ReadHints",
            "#[derive(serde::Deserialize,serde::Serialize)]",
        )
        .type_attribute("Chunk", "#[derive(serde::Deserialize,serde::Serialize)]")
        .type_attribute(
            "ChunkedSeries",
            "#[derive(serde::Deserialize,serde::Serialize)]",
        )
        .type_attribute(
            "zero_count",
            "#[derive(serde::Deserialize,serde::Serialize)]",
        )
        .type_attribute("count", "#[derive(serde::Deserialize,serde::Serialize)]");
    config.compile_protos(
        &[
            "proto/prometheus/remote.proto",
            "proto/prometheus/types.proto",
        ],
        &["proto"],
    )?;

    Ok(())
}
