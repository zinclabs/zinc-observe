use infra::{
    db::{connect_to_orm, ORM_CLIENT},
    table::entity::alerts,
};
use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;
use sea_orm::{EntityTrait, PaginatorTrait};

use crate::common::{
    meta::authz::Authz,
    utils::auth::{remove_ownership, set_ownership},
};

/// Generates a neww ID in OpenFGA for each alert using the alert's KSUID.
pub async fn run() -> Result<(), anyhow::Error> {
    if !get_o2_config().openfga.enabled {
        return Ok(());
    }

    o2_enterprise::enterprise::openfga::authorizer::authz::init_open_fga().await;
    let _ = crate::common::infra::ofga::init().await;
    let conn = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let mut pages = alerts::Entity::find().paginate(conn, 100);
    while let Some(alerts) = pages.fetch_and_next().await? {
        for a in alerts {
            remove_ownership(&a.org, "alerts", Authz::new(&a.name)).await;
            set_ownership(&a.org, "alerts", Authz::new(&a.id)).await;
        }
    }

    Ok(())
}
