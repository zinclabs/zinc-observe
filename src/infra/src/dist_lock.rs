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

use crate::{
    db::{etcd, nats},
    errors::Result,
};

pub struct Locker(LockerStore);

enum LockerStore {
    Etcd(etcd::Locker),
    Nats(nats::Locker),
}

impl Locker {
    pub fn key(&self) -> String {
        match self.0 {
            LockerStore::Etcd(ref locker) => locker.key.clone(),
            LockerStore::Nats(ref locker) => locker.key.clone(),
        }
    }
}

#[inline(always)]
pub async fn lock_with_trace_id(
    trace_id: &str,
    key: &str,
    wait_ttl: u64,
) -> Result<Option<Locker>> {
    let lock = lock(key, wait_ttl).await;
    match lock {
        Ok(lock) => {
            log::debug!("[trace_id {trace_id}] fetch lock with key: {key}");
            Ok(lock)
        }
        Err(e) => Err(e),
    }
}

#[inline(always)]
pub async fn unlock_with_trace_id(trace_id: &str, locker: &Option<Locker>) -> Result<()> {
    if locker.is_none() {
        return Ok(());
    }
    let unlock = unlock(locker).await;
    match unlock {
        Ok(_) => {
            log::debug!(
                "[trace_id {trace_id}] release lock with key: {:?}",
                locker.as_ref().map(|l| l.key()).unwrap_or_default()
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// lock key in etcd, wait_ttl is 0 means wait forever
#[inline(always)]
pub async fn lock(key: &str, wait_ttl: u64) -> Result<Option<Locker>> {
    let cfg = config::get_config();
    if cfg.common.local_mode {
        return Ok(None);
    }
    match cfg.common.cluster_coordinator.as_str() {
        "nats" => {
            let mut lock = nats::Locker::new(key);
            lock.lock(wait_ttl).await?;
            Ok(Some(Locker(LockerStore::Nats(lock))))
        }
        _ => {
            let mut lock = etcd::Locker::new(key);
            lock.lock(wait_ttl).await?;
            Ok(Some(Locker(LockerStore::Etcd(lock))))
        }
    }
}

pub async fn unlock(locker: &Option<Locker>) -> Result<()> {
    if let Some(locker) = locker {
        match &locker.0 {
            LockerStore::Etcd(locker) => locker.unlock().await,
            LockerStore::Nats(locker) => locker.unlock().await,
        }
    } else {
        Ok(())
    }
}
