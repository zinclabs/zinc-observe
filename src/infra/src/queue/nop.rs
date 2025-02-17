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
use tokio::sync::mpsc;

use crate::errors::*;

pub async fn init() -> Result<()> {
    Ok(())
}

pub struct NopQueue {}

impl NopQueue {
    pub fn new(_prefix: &str) -> Self {
        Self {}
    }

    pub fn super_cluster() -> Self {
        Self::new("")
    }
}

impl Default for NopQueue {
    fn default() -> Self {
        Self::new("")
    }
}

#[async_trait]
impl super::Queue for NopQueue {
    async fn create(&self, _topic: &str) -> Result<()> {
        todo!()
    }

    async fn publish(&self, _topic: &str, _value: Bytes) -> Result<()> {
        todo!()
    }

    async fn consume(&self, _topic: &str) -> Result<Arc<mpsc::Receiver<super::Message>>> {
        todo!()
    }

    async fn purge(&self, _topic: &str, _sequence: usize) -> Result<()> {
        todo!()
    }
}
