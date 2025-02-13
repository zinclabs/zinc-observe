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

use config::meta::dashboards::reports::{ListReportsParams, Report};
use infra::table;
use sea_orm::{ConnectionTrait, TransactionTrait};

use crate::service::db;

pub async fn get<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_snowflake_id: &str,
    name: &str,
) -> Result<Report, anyhow::Error> {
    match table::reports::get_by_name(conn, org_id, folder_snowflake_id, name).await? {
        Some((_, report)) => Ok(report),
        _ => Err(anyhow::anyhow!("Report not found")),
    }
}

pub async fn create<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    folder_snowflake_id: &str,
    report: Report,
) -> Result<(), anyhow::Error> {
    let org = report.org_id.clone();
    let schedule_key = report.name.clone();
    let next_run_at = report.start;

    create_without_updating_trigger(conn, folder_snowflake_id, report).await?;
    let trigger = db::scheduler::Trigger {
        org,
        module: db::scheduler::TriggerModule::Report,
        module_key: schedule_key,
        next_run_at,
        ..Default::default()
    };
    db::scheduler::push(trigger)
        .await
        .inspect_err(|e| log::error!("Failed to save trigger: {}", e))?;
    Ok(())
}

pub async fn update<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    folder_snowflake_id: &str,
    new_folder_snowflake_id: Option<&str>,
    report: Report,
) -> Result<(), anyhow::Error> {
    let org_id = report.org_id.clone();
    let schedule_key = report.name.clone();
    let next_run_at = report.start;

    update_without_updating_trigger(conn, folder_snowflake_id, new_folder_snowflake_id, report)
        .await?;
    let scheduler_exists =
        db::scheduler::exists(&org_id, db::scheduler::TriggerModule::Report, &schedule_key).await;

    let trigger = db::scheduler::Trigger {
        org: org_id.clone(),
        module: db::scheduler::TriggerModule::Report,
        module_key: schedule_key,
        next_run_at,
        ..Default::default()
    };
    if scheduler_exists {
        db::scheduler::update_trigger(trigger)
            .await
            .inspect_err(|e| log::error!("Failed to update trigger: {}", e))?;
    } else {
        db::scheduler::push(trigger)
            .await
            .inspect_err(|e| log::error!("Failed to save trigger: {}", e))?;
    }

    Ok(())
}

pub async fn create_without_updating_trigger<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    folder_snowflake_id: &str,
    report: Report,
) -> Result<(), anyhow::Error> {
    table::reports::create_report(conn, folder_snowflake_id, report, None).await?;
    // todo: emit supercluster event
    Ok(())
}

pub async fn update_without_updating_trigger<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    folder_snowflake_id: &str,
    new_folder_snowflake_id: Option<&str>,
    report: Report,
) -> Result<(), anyhow::Error> {
    table::reports::update_report(conn, folder_snowflake_id, new_folder_snowflake_id, report)
        .await?;
    // todo: emit supercluster event
    Ok(())
}

pub async fn delete<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_snowflake_id: &str,
    name: &str,
) -> Result<(), anyhow::Error> {
    // let key = format!("/reports/{org_id}/{name}");
    table::reports::delete_by_name(conn, org_id, folder_snowflake_id, name)
        .await
        .map_err(|e| anyhow::anyhow!("Error deleting report: {}", e))?;
    let _ = db::scheduler::delete(org_id, db::scheduler::TriggerModule::Report, name)
        .await
        .inspect_err(|e| log::error!("Failed to delete trigger: {}", e));
    // todo: emit supercluster event
    // todo: does deletion from scheduler need to happen in other cluster?;
    Ok(())
}

pub async fn list<C: ConnectionTrait>(
    conn: &C,
    params: &ListReportsParams,
) -> Result<Vec<table::reports::ListReportsQueryResult>, anyhow::Error> {
    let reports = table::reports::list_reports(conn, params).await?;
    Ok(reports)
}

pub async fn reset<C: ConnectionTrait>(conn: &C) -> Result<(), anyhow::Error> {
    table::reports::delete_all(conn).await?;
    // todo: emit supercluster event
    Ok(())
}
