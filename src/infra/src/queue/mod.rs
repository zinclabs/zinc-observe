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

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use config::meta::meta_store::MetaStore;
use tokio::sync::{mpsc, OnceCell};

use crate::errors::{Error, Result};

pub mod nats;
pub mod nop;

static DEFAULT: OnceCell<Box<dyn Queue>> = OnceCell::const_new();
static SUPER_CLUSTER: OnceCell<Box<dyn Queue>> = OnceCell::const_new();

pub async fn get_queue() -> &'static Box<dyn Queue> {
    DEFAULT.get_or_init(default).await
}

pub async fn get_super_cluster() -> &'static Box<dyn Queue> {
    SUPER_CLUSTER.get_or_init(init_super_cluster).await
}

pub async fn init() -> Result<()> {
    nats::init().await
}

async fn default() -> Box<dyn Queue> {
    match config::get_config().common.queue_store.as_str().into() {
        MetaStore::Nats => Box::<nats::NatsQueue>::default(),
        _ => Box::<nop::NopQueue>::default(),
    }
}

async fn init_super_cluster() -> Box<dyn Queue> {
    if config::get_config().common.local_mode {
        panic!("super cluster is not supported in local mode");
    }
    Box::new(nats::NatsQueue::super_cluster())
}

#[async_trait]
pub trait Queue: Sync + Send + 'static {
    async fn create(&self, topic: &str) -> Result<()>;
    async fn publish(&self, topic: &str, value: Bytes) -> Result<()>;
    async fn consume(&self, topic: &str) -> Result<Arc<mpsc::Receiver<Message>>>;
    async fn purge(&self, topic: &str, sequence: usize) -> Result<()>;
}

pub enum Message {
    Nats(async_nats::jetstream::Message),
}

impl Message {
    pub fn message(&self) -> &Bytes {
        match self {
            Message::Nats(msg) => &msg.payload,
        }
    }

    pub async fn ack(&self) -> Result<()> {
        match self {
            Message::Nats(msg) => msg
                .ack()
                .await
                .map_err(|e| Error::Message(format!("ack error:{}", e)))?,
        }
        Ok(())
    }
}
