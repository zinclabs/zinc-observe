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

use std::{
    cmp::min,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use bytes::Bytes;
use config::{cluster, get_config};
use etcd_client::{
    Certificate, DeleteOptions, EventType, GetOptions, Identity, SortOrder, SortTarget, TlsOptions,
};
use hashbrown::HashMap;
use tokio::{
    sync::{mpsc, OnceCell},
    task::JoinHandle,
    time,
};

use crate::{
    db::{Event, EventData},
    dist_lock,
    errors::*,
};

static ETCD_CLIENT: OnceCell<etcd_client::Client> = OnceCell::const_new();

pub async fn get_etcd_client() -> &'static etcd_client::Client {
    ETCD_CLIENT.get_or_init(connect).await
}

pub async fn init() {
    let cfg = get_config();
    if cfg.common.local_mode || cfg.common.cluster_coordinator.to_lowercase() == "nats" {
        return;
    }
    // enable keep alive for auth token
    tokio::task::spawn(async move { keepalive_connection().await });
}

pub struct Etcd {
    prefix: String,
}

impl Etcd {
    pub fn new(prefix: &str) -> Etcd {
        let prefix = prefix.trim_end_matches('/');
        Etcd {
            prefix: prefix.to_string(),
        }
    }

    async fn get_key_value(&self, key: &str) -> Result<(String, Bytes)> {
        let key = format!("{}{}", self.prefix, key);
        let mut client = get_etcd_client().await.clone();
        let opt = GetOptions::new()
            .with_prefix()
            .with_sort(SortTarget::Key, SortOrder::Descend)
            .with_limit(1);
        let ret = client.get(key.as_str(), Some(opt)).await?;
        if ret.kvs().is_empty() {
            return Err(Error::from(DbError::KeyNotExists(key)));
        }
        let item_key = ret.kvs()[0]
            .key_str()
            .unwrap()
            .strip_prefix(&self.prefix)
            .unwrap();
        Ok((
            item_key.to_string(),
            Bytes::from(ret.kvs()[0].value().to_vec()),
        ))
    }
}

impl Default for Etcd {
    fn default() -> Self {
        Self::new(&get_config().etcd.prefix)
    }
}

#[async_trait]
impl super::Db for Etcd {
    async fn create_table(&self) -> Result<()> {
        Ok(())
    }

    async fn stats(&self) -> Result<super::Stats> {
        let mut client = get_etcd_client().await.clone();
        let stats = client.status().await?;
        let bytes_len = stats.db_size();
        let resp = client
            .get(
                "",
                Some(GetOptions::new().with_all_keys().with_count_only()),
            )
            .await?;
        let keys_count = resp.count();
        Ok(super::Stats {
            bytes_len,
            keys_count,
        })
    }

    async fn get(&self, key: &str) -> Result<Bytes> {
        let key = format!("{}{}", self.prefix, key);
        let mut client = get_etcd_client().await.clone();
        let opt = GetOptions::new()
            .with_prefix()
            .with_sort(SortTarget::Key, SortOrder::Descend)
            .with_limit(1);
        let ret = client.get(key.as_str(), Some(opt)).await?;
        if ret.kvs().is_empty() {
            return Err(Error::from(DbError::KeyNotExists(key)));
        }
        Ok(Bytes::from(ret.kvs()[0].value().to_vec()))
    }

    async fn put(
        &self,
        key: &str,
        value: Bytes,
        _need_watch: bool,
        start_dt: Option<i64>,
    ) -> Result<()> {
        let key = if start_dt.is_some() {
            format!("{}{}/{}", self.prefix, key, start_dt.unwrap())
        } else {
            format!("{}{}", self.prefix, key)
        };
        let mut client = get_etcd_client().await.clone();
        let _ = client.put(key, value, None).await?;
        Ok(())
    }

    async fn get_for_update(
        &self,
        key: &str,
        need_watch: bool,
        start_dt: Option<i64>,
        update_fn: Box<super::UpdateFn>,
    ) -> Result<()> {
        // acquire lock and update
        let lock_key = format!("/meta{key}/{}", start_dt.unwrap_or_default());
        let locker = match dist_lock::lock(&lock_key, 0, None).await {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::Message(format!(
                    "dist_lock key: {}, acquire error: {}",
                    lock_key, e
                )));
            }
        };
        log::info!("Acquired lock for cluster key: {}", lock_key);

        // get value and update
        let value = self.get_key_value(key).await.ok();
        let old_key = value.as_ref().map(|v| v.0.clone());
        let old_value = value.map(|v| v.1);
        let ret = match update_fn(old_value) {
            Err(e) => Err(e),
            Ok(None) => Ok(()),
            Ok(Some((value, new_value))) => {
                if let Some(value) = value {
                    if let Err(e) = self.put(&old_key.unwrap(), value, need_watch, None).await {
                        if let Err(e) = dist_lock::unlock(&locker).await {
                            log::error!("dist_lock unlock err: {}", e);
                        }
                        log::info!("Released lock for cluster key: {}", lock_key);
                        return Err(e);
                    }
                }
                if let Some((new_key, new_value, new_start_dt)) = new_value {
                    if let Err(e) = self
                        .put(&new_key, new_value, need_watch, new_start_dt)
                        .await
                    {
                        if let Err(e) = dist_lock::unlock(&locker).await {
                            log::error!("dist_lock unlock err: {}", e);
                        }
                        log::info!("Released lock for cluster key: {}", lock_key);
                        return Err(e);
                    }
                }
                Ok(())
            }
        };

        // release lock
        if let Err(e) = dist_lock::unlock(&locker).await {
            log::error!("dist_lock unlock err: {}", e);
        }
        log::info!("Released lock for cluster key: {}", lock_key);
        ret
    }

    async fn delete(
        &self,
        key: &str,
        with_prefix: bool,
        _need_watch: bool,
        start_dt: Option<i64>,
    ) -> Result<()> {
        let mut key = format!("{}{}", self.prefix, key);
        if start_dt.is_some() {
            key = format!("{}/{}", key, start_dt.unwrap());
        }
        let mut client = get_etcd_client().await.clone();
        let opt = with_prefix.then(|| DeleteOptions::new().with_prefix());
        let _ = client.delete(key.as_str(), opt).await?.deleted();
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<HashMap<String, Bytes>> {
        let cfg = get_config();
        let mut result = HashMap::default();
        let key = format!("{}{}", self.prefix, prefix);
        let mut client = get_etcd_client().await.clone();
        let mut opt = GetOptions::new()
            .with_prefix()
            .with_sort(SortTarget::Key, SortOrder::Ascend)
            .with_limit(cfg.etcd.load_page_size);
        let mut resp = client.get(key.clone(), Some(opt.clone())).await?;
        let mut first_call = true;
        let mut have_next = true;
        let mut last_key = String::new();
        loop {
            let kvs_num = resp.kvs().len() as i64;
            if kvs_num < cfg.etcd.load_page_size {
                have_next = false;
            }
            for kv in resp.kvs() {
                let item_key = kv.key_str().unwrap();
                if !item_key.starts_with(&key) {
                    have_next = false;
                    break;
                }
                if item_key.eq(last_key.as_str()) {
                    continue;
                }
                let item_key = item_key.strip_prefix(&self.prefix).unwrap();
                result.insert(item_key.to_string(), Bytes::from(kv.value().to_vec()));
            }
            tokio::task::yield_now().await; // yield to other tasks

            if !have_next {
                break;
            }
            if first_call {
                first_call = false;
                opt = opt.with_from_key();
            }
            last_key = resp.kvs().last().unwrap().key_str().unwrap().to_string();
            resp = client.get(last_key.clone(), Some(opt.clone())).await?;
        }
        Ok(result)
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let cfg = get_config();
        let mut result = Vec::new();
        let key = format!("{}{}", self.prefix, prefix);
        let mut client = get_etcd_client().await.clone();
        let mut opt = GetOptions::new()
            .with_prefix()
            .with_sort(SortTarget::Key, SortOrder::Ascend)
            .with_limit(cfg.etcd.load_page_size);
        let mut resp = client.get(key.clone(), Some(opt.clone())).await?;
        let mut first_call = true;
        let mut have_next = true;
        let mut last_key = String::new();
        loop {
            let kvs_num = resp.kvs().len() as i64;
            if kvs_num < cfg.etcd.load_page_size {
                have_next = false;
            }
            for kv in resp.kvs() {
                let item_key = kv.key_str().unwrap();
                if !item_key.starts_with(&key) {
                    have_next = false;
                    break;
                }
                if item_key.eq(last_key.as_str()) {
                    continue;
                }
                let item_key = item_key.strip_prefix(&self.prefix).unwrap();
                result.push(item_key.to_string());
            }
            tokio::task::yield_now().await; // yield to other tasks

            if !have_next {
                break;
            }
            if first_call {
                first_call = false;
                opt = opt.with_from_key();
            }
            last_key = resp.kvs().last().unwrap().key_str().unwrap().to_string();
            resp = client.get(last_key.clone(), Some(opt.clone())).await?;
        }
        Ok(result)
    }

    async fn list_values(&self, prefix: &str) -> Result<Vec<Bytes>> {
        let cfg = get_config();
        let mut result = Vec::new();
        let key = format!("{}{}", self.prefix, prefix);
        let mut client = get_etcd_client().await.clone();
        let mut opt = GetOptions::new()
            .with_prefix()
            .with_sort(SortTarget::Key, SortOrder::Ascend)
            .with_limit(cfg.etcd.load_page_size);
        let mut resp = client.get(key.clone(), Some(opt.clone())).await?;
        let mut first_call = true;
        let mut have_next = true;
        let mut last_key = String::new();
        loop {
            let kvs_num = resp.kvs().len() as i64;
            if kvs_num < cfg.etcd.load_page_size {
                have_next = false;
            }
            for kv in resp.kvs() {
                let item_key = kv.key_str().unwrap();
                if !item_key.starts_with(&key) {
                    have_next = false;
                    break;
                }
                if item_key.eq(last_key.as_str()) {
                    continue;
                }
                result.push(Bytes::from(kv.value().to_vec()));
            }
            tokio::task::yield_now().await; // yield to other tasks

            if !have_next {
                break;
            }
            if first_call {
                first_call = false;
                opt = opt.with_from_key();
            }
            last_key = resp.kvs().last().unwrap().key_str().unwrap().to_string();
            resp = client.get(last_key.clone(), Some(opt.clone())).await?;
        }
        Ok(result)
    }

    async fn list_values_by_start_dt(
        &self,
        prefix: &str,
        start_dt: Option<(i64, i64)>,
    ) -> Result<Vec<(i64, Bytes)>> {
        let cfg = get_config();
        if start_dt.is_none() || start_dt == Some((0, 0)) {
            let vals = self.list_values(prefix).await?;
            return Ok(vals.into_iter().map(|v| (0, v)).collect());
        }

        let (min_dt, max_dt) = start_dt.unwrap();
        let mut result = Vec::new();
        let key = format!("{}{}", self.prefix, prefix);
        let mut client = get_etcd_client().await.clone();
        let mut opt = GetOptions::new()
            .with_prefix()
            .with_sort(SortTarget::Key, SortOrder::Ascend)
            .with_limit(cfg.etcd.load_page_size);
        let mut resp = client.get(key.clone(), Some(opt.clone())).await?;
        let mut first_call = true;
        let mut have_next = true;
        let mut last_key = String::new();
        loop {
            let kvs_num = resp.kvs().len() as i64;
            if kvs_num < cfg.etcd.load_page_size {
                have_next = false;
            }
            for kv in resp.kvs() {
                let item_key = kv.key_str().unwrap();
                if !item_key.starts_with(&key) {
                    have_next = false;
                    break;
                }
                if item_key.eq(last_key.as_str()) {
                    continue;
                }
                let start_dt = item_key
                    .split('/')
                    .last()
                    .unwrap()
                    .parse::<i64>()
                    .unwrap_or_default();
                if start_dt >= min_dt && start_dt <= max_dt {
                    result.push((start_dt, Bytes::from(kv.value().to_vec())));
                }
            }
            tokio::task::yield_now().await; // yield to other tasks

            if !have_next {
                break;
            }
            if first_call {
                first_call = false;
                opt = opt.with_from_key();
            }
            last_key = resp.kvs().last().unwrap().key_str().unwrap().to_string();
            resp = client.get(last_key.clone(), Some(opt.clone())).await?;
        }
        Ok(result)
    }

    async fn count(&self, prefix: &str) -> Result<i64> {
        let key = format!("{}{}", self.prefix, prefix);
        let mut client = get_etcd_client().await.clone();
        let opt = GetOptions::new().with_prefix().with_count_only();
        let resp = client.get(key.clone(), Some(opt)).await?;
        Ok(resp.count())
    }

    async fn watch(&self, prefix: &str) -> Result<Arc<mpsc::Receiver<Event>>> {
        let (tx, rx) = mpsc::channel(65535);
        let prefix = prefix.to_string();
        let key = format!("{}{}", &self.prefix, prefix);
        let self_prefix = self.prefix.to_string();
        let _task: JoinHandle<Result<()>> = tokio::task::spawn(async move {
            loop {
                if cluster::is_offline() {
                    break;
                }
                let mut client = get_etcd_client().await.clone();
                let opt = etcd_client::WatchOptions::new().with_prefix();
                log::debug!("[ETCD:watch] prefix: {}", prefix);
                let (mut _watcher, mut stream) =
                    match client.watch(key.clone(), Some(opt.clone())).await {
                        Ok((watcher, stream)) => (watcher, stream),
                        Err(e) => {
                            log::error!("[ETCD:watch] prefix: {}, error: {}", key, e);
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            continue;
                        }
                    };
                loop {
                    let resp = match stream.message().await {
                        Ok(resp) => resp,
                        Err(e) => {
                            log::error!("[ETCD:watch] prefix: {}, get message error: {}", key, e);
                            break;
                        }
                    };
                    if let Some(ev) = resp {
                        for ev in ev.events() {
                            let kv = ev.kv().unwrap();
                            let item_key = kv.key_str().unwrap();
                            let item_key = item_key.strip_prefix(&self_prefix).unwrap();
                            let ret = match ev.event_type() {
                                EventType::Put => tx.try_send(Event::Put(EventData {
                                    key: item_key.to_string(),
                                    value: Some(Bytes::from(kv.value().to_vec())),
                                    start_dt: None,
                                })),
                                EventType::Delete => tx.try_send(Event::Delete(EventData {
                                    key: item_key.to_string(),
                                    value: None,
                                    start_dt: None,
                                })),
                            };
                            if let Err(e) = ret {
                                log::warn!(
                                    "[ETCD:watch] prefix: {}, key: {}, send error: {}",
                                    prefix,
                                    item_key,
                                    e
                                );
                            }
                        }
                    }
                }
            }
            Ok(())
        });
        Ok(Arc::new(rx))
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
    async fn add_start_dt_column(&self) -> Result<()> {
        Ok(())
    }
}

pub async fn create_table() -> Result<()> {
    Ok(())
}

pub async fn connect() -> etcd_client::Client {
    let cfg = get_config();
    if cfg.common.print_key_config {
        log::info!("Etcd init cfg: {:?}", cfg.etcd);
    }

    let mut opts = etcd_client::ConnectOptions::new()
        .with_timeout(core::time::Duration::from_secs(cfg.etcd.command_timeout))
        .with_connect_timeout(core::time::Duration::from_secs(cfg.etcd.connect_timeout));
    if !&cfg.etcd.user.is_empty() {
        opts = opts.with_user(&cfg.etcd.user, &cfg.etcd.password);
    }
    if cfg.etcd.cert_auth {
        let server_root_ca_cert = tokio::fs::read(&cfg.etcd.ca_file).await.unwrap();
        let server_root_ca_cert = Certificate::from_pem(server_root_ca_cert);
        let client_cert = tokio::fs::read(&cfg.etcd.cert_file).await.unwrap();
        let client_key = tokio::fs::read(&cfg.etcd.key_file).await.unwrap();
        let client_identity = Identity::from_pem(client_cert, client_key);
        let tls = TlsOptions::new()
            .domain_name(&cfg.etcd.domain_name)
            .ca_certificate(server_root_ca_cert)
            .identity(client_identity);
        opts = opts.with_tls(tls);
    }
    let addrs = cfg.etcd.addr.split(',').collect::<Vec<&str>>();
    etcd_client::Client::connect(addrs, Some(opts))
        .await
        .expect("Etcd connect failed")
}

pub async fn keepalive_connection() -> Result<()> {
    loop {
        if cluster::is_offline() {
            break;
        }
        let mut client = get_etcd_client().await.clone();
        let key = format!("{}healthz", get_config().etcd.prefix);
        let key = key.as_str();
        client.put(key, "OK", None).await?;
        let mut interval = time::interval(time::Duration::from_secs(60));
        interval.tick().await; // trigger the first run
        loop {
            interval.tick().await;
            match client.get(key, None).await {
                Ok(ret) => for _item in ret.kvs() {},
                Err(e) => {
                    log::error!("keep alive connection error: {:?}", e);
                    break;
                }
            };
        }
    }

    Ok(())
}

pub async fn keepalive_lease_id<F>(id: i64, ttl: i64, stopper: F) -> Result<()>
where
    F: Fn() -> bool,
{
    let mut ttl_keep_alive = min(10, (ttl / 2) as u64);
    loop {
        if stopper() {
            break;
        }
        let mut client = get_etcd_client().await.clone();
        let (mut keeper, mut stream) = match client.lease_keep_alive(id).await {
            Ok((keeper, stream)) => (keeper, stream),
            Err(e) => {
                log::error!("lease {:?} keep alive error: {:?}", id, e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        };
        loop {
            if stopper() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(ttl_keep_alive)).await;
            match keeper.keep_alive().await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("lease {:?} keep alive do keeper error: {:?}", id, e);
                    ttl_keep_alive = 1;
                    break;
                }
            }
            match stream.message().await {
                Ok(v) => {
                    if v.unwrap().ttl() == 0 {
                        log::error!("lease {:?} keep alive ttl is 0", id);
                        return Err(Error::from(etcd_client::Error::LeaseKeepAliveError(
                            "lease expired or revoked".to_string(),
                        )));
                    }
                    ttl_keep_alive = min(10, (ttl / 2) as u64);
                }
                Err(e) => {
                    log::error!("lease {:?} keep alive receive message: {:?}", id, e);
                    ttl_keep_alive = 1;
                    break;
                }
            };
        }
    }

    Ok(())
}

pub(crate) struct Locker {
    pub key: String,
    lock_id: String,
    state: Arc<AtomicU8>, // 0: init, 1: locking, 2: release
}

impl Locker {
    pub(crate) fn new(key: &str) -> Self {
        Self {
            key: format!("{}locker{}", get_config().etcd.prefix, key),
            lock_id: "".to_string(),
            state: Arc::new(AtomicU8::new(0)),
        }
    }

    /// lock with timeout, 0 means use default timeout, unit: second
    pub(crate) async fn lock(&mut self, timeout: u64) -> Result<()> {
        let cfg = get_config();
        let mut client = get_etcd_client().await.clone();
        let mut last_err = None;
        let timeout = if timeout == 0 {
            cfg.etcd.lock_wait_timeout
        } else {
            timeout
        };
        let mut n = timeout / cfg.etcd.command_timeout;
        if n < 1 {
            n = 1;
        }
        for _ in 0..n {
            match client.lock(self.key.as_str(), None).await {
                Ok(resp) => {
                    self.lock_id = String::from_utf8_lossy(resp.key()).to_string();
                    self.state.store(1, Ordering::SeqCst);
                    last_err = None;
                    break;
                }
                Err(err) => {
                    last_err = Some(err.to_string());
                    if !err.to_string().contains("Timeout expired") {
                        break;
                    }
                }
            };
        }
        if let Some(err) = last_err {
            return Err(Error::Message(format!(
                "etcd lock for key: {}, error: {}",
                self.key, err
            )));
        }
        Ok(())
    }

    pub(crate) async fn unlock(&self) -> Result<()> {
        if self.state.load(Ordering::SeqCst) != 1 {
            return Ok(());
        }
        let mut client = get_etcd_client().await.clone();
        if let Err(err) = client.unlock(self.lock_id.as_str()).await {
            log::error!("etcd unlock for key: {}, error: {}", self.key, err);
            return Err(Error::Message("etcd unlock error".to_string()));
        };
        self.state.store(2, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::Db, *};

    #[tokio::test]
    async fn test_etcd_prefix() {
        let client = Etcd::default();
        assert_eq!(client.prefix, "/zinc/observe".to_string());
    }

    #[tokio::test]
    async fn test_etcd_count() {
        if get_config().common.local_mode {
            return;
        }
        let client = Etcd::default();
        client
            .put("/test/count/1", bytes::Bytes::from("1"), false, None)
            .await
            .unwrap();
        client
            .put("/test/count/2", bytes::Bytes::from("2"), false, None)
            .await
            .unwrap();
        client
            .put("/test/count/3", bytes::Bytes::from("3"), false, None)
            .await
            .unwrap();
        let count = client.count("/test/count").await.unwrap();
        assert_eq!(count, 3);
    }
}
