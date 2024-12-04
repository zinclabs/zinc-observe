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

use std::{cmp::max, sync::Arc, time::Duration};

use async_nats::jetstream::{self, consumer::DeliverPolicy};
use async_trait::async_trait;
use bytes::Bytes;
use config::{get_cluster_name, get_config};
use futures::TryStreamExt;
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{db::nats::get_nats_client, errors::*};

pub async fn init() -> Result<()> {
    Ok(())
}

pub struct NatsQueue {
    prefix: String,
}

impl NatsQueue {
    pub fn new(prefix: &str) -> Self {
        let prefix = prefix.trim_end_matches('/');
        Self {
            prefix: prefix.to_string(),
        }
    }

    pub fn super_cluster() -> Self {
        Self::new("super_cluster_queue_")
    }
}

impl Default for NatsQueue {
    fn default() -> Self {
        Self::new(&config::get_config().nats.prefix)
    }
}

#[async_trait]
impl super::Queue for NatsQueue {
    async fn create(&self, topic: &str) -> Result<()> {
        let cfg = config::get_config();
        let client = get_nats_client().await.clone();
        let jetstream = jetstream::new(client);
        let topic_name = format!("{}{}", self.prefix, topic);
        let config = jetstream::stream::Config {
            name: topic_name.to_string(),
            subjects: vec![topic_name.to_string(), format!("{}.*", topic_name)],
            retention: jetstream::stream::RetentionPolicy::Limits,
            max_age: Duration::from_secs(60 * 60 * 24 * max(1, cfg.nats.queue_max_age)),
            num_replicas: cfg.nats.replicas,
            ..Default::default()
        };
        _ = jetstream.get_or_create_stream(config).await?;
        Ok(())
    }

    /// you can pub message with the topic or topic.* to match the topic
    async fn publish(&self, topic: &str, value: Bytes) -> Result<()> {
        let client = get_nats_client().await.clone();
        let jetstream = jetstream::new(client);
        // Publish a message to the stream
        let topic_name = format!("{}{}", self.prefix, topic);
        let ack = jetstream.publish(topic_name, value).await?;
        ack.await?;
        Ok(())
    }

    async fn consume(&self, topic: &str) -> Result<Arc<mpsc::Receiver<super::Message>>> {
        let (tx, rx) = mpsc::channel(1024);
        let stream_name = format!("{}{}", self.prefix, topic);
        let _task: JoinHandle<Result<()>> = tokio::task::spawn(async move {
            let client = get_nats_client().await.clone();
            let jetstream = jetstream::new(client);
            let stream = jetstream.get_stream(&stream_name).await?;
            let consumer_name = get_cluster_name();
            let config = jetstream::consumer::pull::Config {
                name: Some(consumer_name.to_string()),
                durable_name: Some(consumer_name.to_string()),
                deliver_policy: get_deliver_policy(),
                ..Default::default()
            };
            let consumer = stream
                .get_or_create_consumer(&consumer_name, config)
                .await?;
            // Consume messages from the consumer
            let mut messages = consumer.messages().await.expect("consumer messages error");
            while let Ok(Some(message)) = messages.try_next().await {
                let message = super::Message::Nats(message);
                tx.send(message)
                    .await
                    .map_err(|e| Error::Message(format!("nats message send error: {e}")))?;
            }
            Ok(())
        });
        Ok(Arc::new(rx))
    }

    async fn purge(&self, _topic: &str, _sequence: usize) -> Result<()> {
        Ok(())
    }
}

fn get_deliver_policy() -> DeliverPolicy {
    match get_config().nats.deliver_policy.to_lowercase().as_str() {
        "all" | "deliverall" | "deliver_all" => DeliverPolicy::All,
        "last" | "deliverlast" | "deliver_last" => DeliverPolicy::Last,
        "new" | "delivernew" | "deliver_new" => DeliverPolicy::New,
        _ => DeliverPolicy::All,
    }
}
