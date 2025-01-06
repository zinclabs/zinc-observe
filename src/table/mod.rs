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

use config::get_config;
use infra::db::{connect_to_orm, sqlite::CLIENT_RW, ORM_CLIENT, SQLITE_STORE};
use migration::Migrator;
use sea_orm_migration::MigratorTrait;

pub mod alerts;
pub mod dashboards;
pub mod distinct_values;
#[allow(unused_imports)]
pub mod entity;
pub mod folders;
mod migration;
pub mod search_job;
pub mod search_queue;
pub mod short_urls;

/// Runs old migrations that are not managed by SeaORM.
///
/// Includes migrations for the `distinct_values` and `short_urls` tables.
pub async fn run_unmanaged_migrations() -> Result<(), anyhow::Error> {
    distinct_values::init().await?;
    short_urls::init().await?;
    Ok(())
}

pub async fn migrate() -> Result<(), anyhow::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Migrator::up(client, None).await?;
    Ok(())
}

pub async fn down(steps: Option<u32>) -> Result<(), anyhow::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Migrator::down(client, steps).await?;
    Ok(())
}

/// Acquires a lock on the SQLite client if SQLite is configured as the meta store.
///
/// # Returns
/// - `Some(MutexGuard)` if SQLite is configured
/// - `None` if a different store is configured
pub async fn get_lock() -> Option<tokio::sync::MutexGuard<'static, sqlx::Pool<sqlx::Sqlite>>> {
    if get_config()
        .common
        .meta_store
        .eq_ignore_ascii_case(SQLITE_STORE)
    {
        Some(CLIENT_RW.lock().await)
    } else {
        None
    }
}

#[macro_export]
macro_rules! orm_err {
    ($e:expr) => {
        Err(infra::errors::Error::DbError(
            infra::errors::DbError::SeaORMError($e.to_string()),
        ))
    };
}
