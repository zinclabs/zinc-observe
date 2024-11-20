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

pub use sea_orm_migration::prelude::*;

mod m20241114_000001_create_folders_table;
mod m20241115_150000_populate_folders_table;
mod m20241116_000001_delete_metas;
mod m20241119_000001_create_dashboards_table;
mod m20241119_000002_populate_dashboards_table;
mod m20241119_000003_delete_metas;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241114_000001_create_folders_table::Migration),
            Box::new(m20241115_150000_populate_folders_table::Migration),
            Box::new(m20241116_000001_delete_metas::Migration),
            Box::new(m20241119_000001_create_dashboards_table::Migration),
            // Box::new(m20241119_000002_populate_dashboards_table::Migration),
            // Box::new(m20241119_000003_delete_metas::Migration),
        ]
    }
}
