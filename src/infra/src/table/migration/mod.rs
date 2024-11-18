pub use sea_orm_migration::prelude::*;

mod m20241114_000001_create_folder_table;
mod m20241115_150000_populate_folder_table;
mod m20241116_000001_delete_metas;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241114_000001_create_folder_table::Migration),
            Box::new(m20241115_150000_populate_folder_table::Migration),
            Box::new(m20241116_000001_delete_metas::Migration),
        ]
    }
}
