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

use bytes::Bytes;
use config::{
    cluster::LOCAL_NODE, get_config, meta::cluster::get_internal_grpc_token, utils::util::zero_or,
};
use hashbrown::HashMap;
use once_cell::sync::Lazy;
use proto::cluster_rpc::{ingest_client::IngestClient, IngestionData, IngestionType, StreamType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    sync::{Mutex, RwLock},
    time::{self, Duration},
};
use tonic::{
    codec::CompressionEncoding,
    metadata::{MetadataKey, MetadataValue},
    Request,
};

use crate::{
    common::{self},
    service::{self, grpc::get_ingester_channel},
};

const ERROR_STREAM_NAME: &str = "oo_errors";
static ERROR_LIST: Mutex<Vec<ErrorEntry>> = Mutex::const_new(vec![]);

type ErrorChannel = (
    tokio::sync::mpsc::Sender<ErrorEntry>,
    RwLock<tokio::sync::mpsc::Receiver<ErrorEntry>>,
);

static ERROR_CHANNEL: Lazy<ErrorChannel> = Lazy::new(|| {
    let (tx, rx) = tokio::sync::mpsc::channel(500);
    (tx, RwLock::new(rx))
});

#[derive(Debug, Serialize, Deserialize)]
struct ErrorEntry {
    org: String,
    stream: String,
    _timestamp: i64,
    error: String,
    kind: String,
}

impl ErrorEntry {
    fn to_json(self) -> Value {
        let mut m = serde_json::Map::new();
        m.insert("stream".into(), self.stream.into());
        m.insert("_timestamp".into(), self._timestamp.into());
        m.insert("error".into(), self.error.into());
        m.insert("reporter".into(), LOCAL_NODE.name.clone().into());
        m.insert("kind".into(), self.kind.into());
        serde_json::Value::Object(m)
    }
}

async fn send_errors(
    config: &config::Config,
    errors: HashMap<String, Vec<ErrorEntry>>,
) -> Result<(), tonic::Status> {
    for (org, data) in errors.into_iter() {
        let data: Vec<Value> = data.into_iter().map(|e| e.to_json()).collect();
        let req = proto::cluster_rpc::IngestionRequest {
            org_id: org.to_owned(),
            stream_name: ERROR_STREAM_NAME.to_owned(),
            stream_type: StreamType::Logs.into(),
            data: Some(IngestionData::from(data)),
            ingestion_type: Some(IngestionType::Json.into()),
        };
        let org_header_key: MetadataKey<_> = config.grpc.org_header_key.parse().unwrap();
        let token: MetadataValue<_> = get_internal_grpc_token().parse().unwrap();
        let (_, channel) = get_ingester_channel().await?;
        let mut client = IngestClient::with_interceptor(channel, move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", token.clone());
            req.metadata_mut()
                .insert(org_header_key.clone(), org.parse().unwrap());
            Ok(req)
        });
        client = client
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
            .max_decoding_message_size(config.grpc.max_message_size * 1024 * 1024)
            .max_encoding_message_size(config.grpc.max_message_size * 1024 * 1024);
        client.ingest(req).await?;
    }
    Ok(())
}

// TODO deal with ingestion response stuff
async fn ingest_errors(errors: HashMap<String, Vec<ErrorEntry>>) -> anyhow::Result<()> {
    for (org, data) in errors.into_iter() {
        let data: Vec<Value> = data.into_iter().map(|e| e.to_json()).collect();
        let bytes = Bytes::from(serde_json::to_string(&data).unwrap());
        let req = common::meta::ingestion::IngestionRequest::JSON(&bytes);

        service::logs::ingest::ingest(0, &org, ERROR_STREAM_NAME, req, "", None).await?;
    }
    Ok(())
}

pub fn report_error(org: &str, stream: &str, kind: &str, error: String) {
    log::error!("Error: {org}/{stream} : {kind} {error}");
    // let config = get_config();

    let reporting_enabled = true;
    if !reporting_enabled {
        return;
    }
    let err = ErrorEntry {
        org: org.to_string(),
        stream: stream.to_string(),
        _timestamp: chrono::Utc::now().timestamp_micros(),
        error,
        kind: kind.to_string(),
    };
    ERROR_CHANNEL.0.try_send(err).unwrap();
}

pub async fn run() -> Result<(), anyhow::Error> {
    let config = get_config();
    // let org = config.common.usage_org.as_str();
    let reporting_enabled = true;

    log::debug!(
        "self-metrics consumption enabled status : {}",
        reporting_enabled
    );

    if !reporting_enabled {
        return Ok(());
    }

    tokio::task::spawn(async {
        let mut receiver = ERROR_CHANNEL.1.write().await;
        while let Some(item) = receiver.recv().await {
            ERROR_LIST.lock().await.push(item);
        }
    });

    // Set up the interval timer for periodic fetching
    let timeout = zero_or(60, 60);
    let mut interval = time::interval(Duration::from_secs(timeout));
    interval.tick().await; // Trigger the first run

    loop {
        // Wait for the interval before running the task again
        interval.tick().await;

        let mut content = ERROR_LIST.lock().await;
        let err_list = std::mem::take(&mut *content);
        drop(content);
        if err_list.is_empty() {
            continue;
        }
        let err_count = err_list.len();
        let mut errors: HashMap<String, Vec<ErrorEntry>> = HashMap::new();
        for err in err_list.into_iter() {
            errors.entry(err.org.clone()).or_default().push(err);
        }

        // ingester can ingest its own metrics, others need to send to one of the ingesters
        if LOCAL_NODE.is_ingester() {
            match ingest_errors(errors).await {
                Ok(_) => {
                    log::debug!("successfully ingested self-metrics");
                }
                Err(e) => {
                    log::error!(
                        "error in sending self-errors, potentially dropped {} records : {:?}",
                        err_count,
                        e
                    )
                }
            }
        } else {
            match send_errors(&config, errors).await {
                Ok(_) => {
                    log::debug!("successfully sent self-metrics for ingestion");
                }
                Err(e) => {
                    log::error!(
                        "error in sending self-errors, potentially dropped {} records : {:?}",
                        err_count,
                        e
                    );
                }
            }
        }
    }
}
