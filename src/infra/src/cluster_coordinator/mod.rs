pub mod alerts;

use crate::db::Db;

pub async fn get_coordinator() -> &'static Box<dyn Db> {
    super::db::get_coordinator().await
}
