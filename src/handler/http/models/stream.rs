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

use config::meta::stream as meta_stream;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, ToSchema, Hash)]
#[serde(rename_all = "lowercase")]
pub enum StreamType {
    #[default]
    Logs,
    Metrics,
    Traces,
    #[serde(rename = "enrichment_tables")]
    EnrichmentTables,
    #[serde(rename = "file_list")]
    Filelist,
    Metadata,
    Index,
}

impl From<meta_stream::StreamType> for StreamType {
    fn from(value: meta_stream::StreamType) -> Self {
        match value {
            meta_stream::StreamType::Logs => Self::Logs,
            meta_stream::StreamType::Metrics => Self::Metrics,
            meta_stream::StreamType::Traces => Self::Traces,
            meta_stream::StreamType::EnrichmentTables => Self::EnrichmentTables,
            meta_stream::StreamType::Filelist => Self::Filelist,
            meta_stream::StreamType::Metadata => Self::Metadata,
            meta_stream::StreamType::Index => Self::Index,
        }
    }
}

impl From<StreamType> for meta_stream::StreamType {
    fn from(value: StreamType) -> Self {
        match value {
            StreamType::Logs => Self::Logs,
            StreamType::Metrics => Self::Metrics,
            StreamType::Traces => Self::Traces,
            StreamType::EnrichmentTables => Self::EnrichmentTables,
            StreamType::Filelist => Self::Filelist,
            StreamType::Metadata => Self::Metadata,
            StreamType::Index => Self::Index,
        }
    }
}
