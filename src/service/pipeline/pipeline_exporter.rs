// Copyright 2025 OpenObserve Inc.
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

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use config::Config;
use infra::errors::Result;
use reqwest::ClientBuilder;

use crate::service::pipeline::{
    pipeline_entry::PipelineEntry,
    pipeline_http_exporter_client::PipelineHttpExporterClient,
    pipeline_offset_manager::{init_pipeline_offset_manager, PIPELINE_OFFSET_MANAGER},
};

const INITIAL_RETRY_DELAY_MS: u64 = 100;

#[async_trait]
pub trait PipelineRouter: Sync + Send {
    async fn export(&self, entry: PipelineEntry) -> Result<()>;
}

pub struct PipelineExporterClientBuilder {}

impl PipelineExporterClientBuilder {
    pub fn build() -> Result<Box<dyn PipelineRouter>> {
        let cfg = &config::get_config();
        // todo: each stream has own protocal in the futrue, we should use specif base on each
        // stream setting
        Ok(Self::build_http(cfg))
    }

    fn build_http(cfg: &Arc<Config>) -> Box<PipelineHttpExporterClient> {
        let builder = ClientBuilder::new()
            .timeout(Duration::from_secs(cfg.pipeline.remote_request_timeout))
            .pool_max_idle_per_host(cfg.pipeline.max_connections);

        if cfg.pipeline.tls_enable {
            // todo
        }

        Box::new(PipelineHttpExporterClient::new(builder.build().unwrap()))
    }
}

pub struct PipelineExporter {
    router: Box<dyn PipelineRouter>,
}

impl PipelineExporter {
    fn new(router: Box<dyn PipelineRouter>) -> Self {
        Self { router }
    }

    pub fn init() -> Result<Self> {
        let client = PipelineExporterClientBuilder::build()?;
        Ok(Self::new(client))
    }

    pub async fn export_entry(&self, entry: PipelineEntry) -> Result<()> {
        let mut attempts = 0;
        let mut delay = INITIAL_RETRY_DELAY_MS;
        let max_retry_attempts = config::get_config().pipeline.remote_request_retry;
        while attempts < max_retry_attempts {
            // todo: if endpoint reponse partial success, we need to resovle the issue?
            // we assume that all the data is received successfully when the endpoint response 200.
            match self.router.export(entry.clone()).await {
                Ok(_) => {
                    if attempts > 0 {
                        log::info!("Successfully exported entry after {} retries", attempts);
                    }

                    // update file position
                    let manager = PIPELINE_OFFSET_MANAGER
                        .get_or_init(init_pipeline_offset_manager)
                        .await;
                    manager
                        .write()
                        .await
                        .save(
                            entry.get_stream_path().to_str().unwrap(),
                            entry.get_entry_position(),
                        )
                        .await;
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    if attempts == max_retry_attempts {
                        log::error!(
                            "Failed to export pipeline entry after {} attempts, giving up: {:?}",
                            max_retry_attempts,
                            e
                        );

                        // return err will break the read_entry loop, so we do not read this file
                        // again no need to send
                        // WatcherEvent::StopWatchFileAndWait(path) to file watcher
                        return Err(e);
                    }

                    log::warn!(
                        "Export pipeline entry attempt {} failed, retrying in {}ms: {:?}",
                        attempts,
                        delay,
                        e
                    );

                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    delay *= 2;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::service::pipeline::pipeline_exporter::PipelineExporterClientBuilder;

    #[test]
    fn test_pipeline_exporter_client_builder() {
        PipelineExporterClientBuilder::build().unwrap();
    }
}
