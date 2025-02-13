mod intermediate;
mod queries;
mod relations;

use chrono::Utc;
use config::meta::{
    dashboards::reports::{ListReportsParams, Report as MetaReport},
    folder::{Folder as MetaFolder, FolderType},
};
pub use queries::ListReportsQueryResult;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ConnectionTrait, EntityTrait, ModelTrait, Set,
    TransactionTrait,
};
use svix_ksuid::{Ksuid, KsuidLike};

use super::entity::{report_dashboards, reports};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    SeaOrm(#[from] sea_orm::error::DbErr),

    #[error("Report not found.")]
    ReportNotFound,

    #[error("Dashboard not found.")]
    DashboardNotFound,

    #[error("Report folder not found.")]
    ReportFolderNotFound,

    #[error("Frequency interval cannot be negative.")]
    NegativeFrequencyInterval,
}

/// Gets a report by its name.
pub async fn get_by_name<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_snowflake_id: &str,
    report_name: &str,
) -> Result<Option<(MetaFolder, MetaReport)>, Error> {
    let _lock = super::get_lock().await;
    let txn = conn.begin().await?;
    let Some(models) = queries::SelectReportAndJoinRelationsResult::get(
        conn,
        org_id,
        folder_snowflake_id,
        report_name,
    )
    .await?
    else {
        return Ok(None);
    };
    txn.commit().await?;
    let (folder, report) = models.try_into()?;
    Ok(Some((folder, report)))
}

/// Creates a new report.
///
/// The new report is created in the report folder with the given `folder_snowflake_id`.
///
/// If the optional `report_id` is given, then that will be used as the new report's ID. If none is
/// given then a random ID will be generated.
pub async fn create_report<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    folder_snowflake_id: &str,
    report: MetaReport,
    report_id: Option<Ksuid>,
) -> Result<MetaReport, Error> {
    let _lock = super::get_lock().await;
    let txn = conn.begin().await?;

    let report_id = report_id
        .unwrap_or_else(|| Ksuid::new(None, None))
        .to_string();
    let now = Utc::now().timestamp_micros();

    // Try to get the primary key of report folder.
    let Some(report_folder_model) = super::folders::get_model(
        &txn,
        &report.org_id,
        folder_snowflake_id,
        FolderType::Reports,
    )
    .await?
    else {
        return Err(Error::ReportFolderNotFound);
    };

    // Convert frequency into an intermediate type which can be serialized into a JSON schema that
    // the DB expects.
    let frequency_intermediate: intermediate::ReportFrequency = report
        .frequency
        .try_into()
        .map_err(|_| Error::NegativeFrequencyInterval)?;
    let frequency_json = serde_json::to_value(frequency_intermediate)?;

    // Convert destinations into an intermediate type which can be serialized into a JSON schema
    // that the DB expects.
    let destinations_intermediate: intermediate::ReportDestinations =
        report.destinations.clone().into();
    let destinations_json = serde_json::to_value(destinations_intermediate)?;

    // Create the new `report` record.
    let report_active_model = reports::ActiveModel {
        id: Set(report_id.clone()),
        folder_id: Set(report_folder_model.id.clone()),
        name: Set(report.name),
        title: Set(report.title),
        description: Set(Some(report.description).filter(|s| !s.is_empty())),
        enabled: Set(report.enabled),
        frequency: Set(frequency_json),
        destinations: Set(destinations_json),
        message: Set(Some(report.message).filter(|s| !s.is_empty())),
        chrome_driver_login_email: Set(Some(report.user).filter(|s| !s.is_empty())),
        chrome_driver_login_password: Set(Some(report.password).filter(|s| !s.is_empty())),
        timezone: Set(report.timezone),
        tz_offset: Set(report.tz_offset),
        owner: Set(Some(report.owner).filter(|s| !s.is_empty())),
        last_edited_by: Set(Some(report.last_edited_by).filter(|s| !s.is_empty())),
        created_at: Set(now),
        updated_at: Set(Some(now)),
        start_at: Set(report.start),
        last_triggered_at: Set(report.last_triggered_at),
    };
    let report_model = report_active_model.insert(&txn).await?;

    // Create new `report_dashboards` relation records.
    let existing_rltns = vec![];
    let desired_rltns = report.dashboards;
    let rltns_to_create = relations::relations_to_create(
        &txn,
        &report.org_id,
        &report_id,
        &existing_rltns,
        &desired_rltns,
    )
    .await?;
    report_dashboards::Entity::insert_many(rltns_to_create)
        .exec(&txn)
        .await?;

    // Convert the newly created records into a domain model
    let joined_models = queries::JoinReportDashboardFolderResults::get(&txn, &report_id).await?;
    let models = queries::SelectReportAndJoinRelationsResult {
        report: report_model,
        report_folder: report_folder_model,
        joined_dashboards: joined_models,
    };
    let (_folder, report) = models.try_into()?;

    txn.commit().await?;
    Ok(report)
}

/// Updates the report.
///
/// Uses the `folder_snowflake_id` and `report.name` to find the report to update.
///
/// Optionally moves the report to a new folder if `new_folder_snowflake_id` is given.
pub async fn update_report<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    folder_snowflake_id: &str,
    new_folder_snowflake_id: Option<&str>,
    report: MetaReport,
) -> Result<(), Error> {
    let _lock = super::get_lock().await;
    let txn = conn.begin().await?;

    let Some(models) = queries::SelectReportAndJoinRelationsResult::get(
        conn,
        &report.org_id,
        folder_snowflake_id,
        &report.name,
    )
    .await?
    else {
        return Err(Error::ReportNotFound);
    };

    // Try to get the primary key of the folder with the new folder snowflake ID if one is given.
    let folder_id = if let Some(new_folder_snowflake_id) = new_folder_snowflake_id {
        let maybe_folder_model = super::folders::get_model(
            &txn,
            &report.org_id,
            new_folder_snowflake_id,
            FolderType::Reports,
        )
        .await?;
        let folder_model = maybe_folder_model.ok_or(Error::ReportFolderNotFound)?;
        Set(folder_model.id)
    } else {
        NotSet
    };

    // Convert frequency into an intermediate type which can be serialized into a JSON schema that
    // the DB expects.
    let frequency_intermediate: intermediate::ReportFrequency = report
        .frequency
        .try_into()
        .map_err(|_| Error::NegativeFrequencyInterval)?;
    let frequency_json = serde_json::to_value(frequency_intermediate)?;

    // Convert destinations into an intermediate type which can be serialized into a JSON schema
    // that the DB expects.
    let destinations_intermediate: intermediate::ReportDestinations =
        report.destinations.clone().into();
    let destinations_json = serde_json::to_value(destinations_intermediate)?;

    // Update the `reports` record.
    let report_active_model = reports::ActiveModel {
        id: Set(models.report.id.clone()),
        folder_id,
        name: Set(report.name),
        title: Set(report.title),
        description: Set(Some(report.description).filter(|s| !s.is_empty())),
        enabled: Set(report.enabled),
        frequency: Set(frequency_json),
        destinations: Set(destinations_json),
        message: Set(Some(report.message).filter(|s| !s.is_empty())),
        chrome_driver_login_email: Set(Some(report.user).filter(|s| !s.is_empty())),
        chrome_driver_login_password: Set(Some(report.password).filter(|s| !s.is_empty())),
        timezone: Set(report.timezone),
        tz_offset: Set(report.tz_offset),
        owner: Set(Some(report.owner).filter(|s| !s.is_empty())),
        last_edited_by: Set(Some(report.last_edited_by).filter(|s| !s.is_empty())),
        created_at: NotSet, // Never updated after creation.
        updated_at: Set(Some(Utc::now().timestamp_micros())),
        start_at: Set(report.start),
        last_triggered_at: Set(report.last_triggered_at),
    };
    report_active_model.update(&txn).await?;

    // Determine which `report_dashboards` records to create, update, and delete.
    let existing_rltns = models.joined_dashboards;
    let desired_rltns = report.dashboards;
    let rltns_to_create = relations::relations_to_create(
        &txn,
        &report.org_id,
        &models.report.id,
        &existing_rltns,
        &desired_rltns,
    )
    .await?;
    let rltns_to_update = relations::relations_to_update(&existing_rltns, &desired_rltns)?;
    let rltns_to_delete = relations::relations_to_delete(&existing_rltns, &desired_rltns);

    report_dashboards::Entity::insert_many(rltns_to_create)
        .exec(&txn)
        .await?;
    for rltn_active_model in rltns_to_update {
        rltn_active_model.update(&txn).await?;
    }
    for rltn_id in rltns_to_delete {
        report_dashboards::Entity::delete_by_id(rltn_id)
            .exec(&txn)
            .await?;
    }

    txn.commit().await?;
    Ok(())
}

/// Deletes a report by its name.
pub async fn delete_by_name<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_snowflake_id: &str,
    report_name: &str,
) -> Result<(), Error> {
    let _lock = super::get_lock().await;
    let txn = conn.begin().await?;
    let Some((_folder_model, Some(report_model))) =
        queries::get_report_from_folder(conn, org_id, folder_snowflake_id, report_name).await?
    else {
        return Ok(());
    };
    report_model.delete(&txn).await?;
    txn.commit().await?;
    Ok(())
}

/// Delete all reports.
pub async fn delete_all<C: ConnectionTrait>(conn: &C) -> Result<(), Error> {
    let _lock = super::get_lock().await;
    reports::Entity::delete_many().exec(conn).await?;
    Ok(())
}

/// Lists the reports the satisfy the given parameters.
pub async fn list_reports<C: ConnectionTrait>(
    conn: &C,
    params: &ListReportsParams,
) -> Result<Vec<ListReportsQueryResult>, Error> {
    let _lock = super::get_lock().await;
    let reports = queries::ListReportsQueryResult::get(conn, params).await?;
    Ok(reports)
}
