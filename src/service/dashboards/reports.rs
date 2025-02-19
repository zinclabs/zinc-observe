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

use std::{str::FromStr, time::Duration};

use async_trait::async_trait;
use chromiumoxide::{browser::Browser, cdp::browser_protocol::page::PrintToPdfParams, Page};
use chrono::Timelike;
use config::{
    get_chrome_launch_options, get_config,
    meta::dashboards::{
        datetime_now,
        reports::{
            HttpReportPayload, Report, ReportDashboard, ReportDestination, ReportEmailDetails,
            ReportFrequencyType, ReportListFilters, ReportTimerangeType,
        },
    },
    SMTP_CLIENT,
};
use cron::Schedule;
use futures::{future::try_join_all, StreamExt};
use infra::{
    db::{connect_to_orm, ORM_CLIENT},
    table,
};
use itertools::Itertools;
use lettre::{
    message::{header::ContentType, MultiPart, SinglePart},
    AsyncTransport, Message,
};
use reqwest::Client;

use crate::{
    common::{
        meta::authz::Authz,
        utils::auth::{is_ofga_unsupported, remove_ownership, set_ownership},
    },
    service::{db, short_url},
};

/// Errors that can occur when interacting with reports.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("SMTP configuration not enabled")]
    SmtpNotEnabled,

    #[error("Chrome not enabled")]
    ChromeNotEnabled,

    #[error("Report username and password ENVs not set")]
    ReportUsernamePasswordNotSet,

    #[error("Report name cannot contain ':', '#', '?', '&', '%', quotes and space characters")]
    NameContainsOpenFgaUnsupportedCharacters,

    #[error("Report name is required")]
    NameIsEmpty,

    #[error("Report name cannot contain '/'")]
    NameContainsForwardSlash,

    #[error("Report already exists")]
    CreateReportNameAlreadyUsed,

    #[error("Report not found")]
    ReportNotFound,

    #[error("Atleast one dashboard is required")]
    NoDashboards,

    #[error("Atleast one tab is required")]
    NoDashboardTabs,

    #[error("Atleast one destination is required")]
    NoDestinations,

    #[error("Some dashboards/tabs not found")]
    DashboardTabNotFound,

    #[error(transparent)]
    ParseCronError(#[from] cron::error::Error),

    #[error(transparent)]
    DbError(anyhow::Error),

    #[error(transparent)]
    SendReportError(#[from] SendReportError),
}

pub async fn save(
    org_id: &str,
    name: &str,
    mut report: Report,
    create: bool,
) -> Result<(), ReportError> {
    let conn = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let cfg = get_config();
    if cfg.common.report_server_url.is_empty() {
        // Check if SMTP is enabled, otherwise don't save the report
        if !cfg.smtp.smtp_enabled {
            return Err(ReportError::SmtpNotEnabled);
        }

        // Check if Chrome is enabled, otherwise don't save the report
        if !cfg.chrome.chrome_enabled || cfg.chrome.chrome_path.is_empty() {
            return Err(ReportError::ChromeNotEnabled);
        }

        if cfg.common.report_user_name.is_empty() || cfg.common.report_user_password.is_empty() {
            return Err(ReportError::ReportUsernamePasswordNotSet);
        }
    }

    if !name.is_empty() {
        report.name = name.to_string();
    }

    // Don't allow the characters not supported by ofga
    if is_ofga_unsupported(&report.name) {
        return Err(ReportError::NameContainsOpenFgaUnsupportedCharacters);
    }
    if report.name.is_empty() {
        return Err(ReportError::NameIsEmpty);
    }
    if report.name.contains('/') {
        return Err(ReportError::NameContainsForwardSlash);
    }

    if report.frequency.frequency_type == ReportFrequencyType::Cron {
        let cron_exp = report.frequency.cron.clone();
        if cron_exp.starts_with("* ") {
            let (_, rest) = cron_exp.split_once(" ").unwrap();
            let now = chrono::Utc::now().second().to_string();
            report.frequency.cron = format!("{now} {rest}");
            log::debug!(
                "New cron expression for report {}: {}",
                report.name,
                report.frequency.cron
            );
        }
        // Check if the cron expression is valid
        Schedule::from_str(&report.frequency.cron)?;
    } else if report.frequency.interval == 0 {
        report.frequency.interval = 1;
    }

    match db::dashboards::reports::get(conn, org_id, "default", &report.name).await {
        Ok(old_report) => {
            if create {
                return Err(ReportError::CreateReportNameAlreadyUsed);
            }
            report.last_triggered_at = old_report.last_triggered_at;
            report.owner = old_report.owner;
            report.updated_at = Some(datetime_now());
        }
        Err(_) => {
            if !create {
                return Err(ReportError::ReportNotFound);
            } else {
                report.last_triggered_at = None;
            }
        }
    }

    // Atleast one `ReportDashboard` needs to be present
    if report.dashboards.is_empty() {
        return Err(ReportError::NoDashboards);
    }

    // Check if dashboards & tabs exist
    let mut tasks = Vec::with_capacity(report.dashboards.len());
    for dashboard in report.dashboards.iter() {
        let dash_id = &dashboard.dashboard;
        let folder = &dashboard.folder;
        if dashboard.tabs.is_empty() {
            return Err(ReportError::NoDashboardTabs);
        }

        // Supports only one tab for now
        let tab_id = &dashboard.tabs[0];
        tasks.push(async move {
            let maybe_dashboard =
                table::dashboards::get_from_folder(org_id, folder, dash_id).await?;
            // Check if the tab_id exists
            if let Some(dashboard) = maybe_dashboard.and_then(|d| d.v3) {
                let mut tab_found = false;
                for tab in dashboard.tabs {
                    if &tab.tab_id == tab_id {
                        tab_found = true;
                    }
                }
                if tab_found {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Tab not found"))
                }
            } else {
                Ok(())
            }
        });
    }
    if try_join_all(tasks).await.is_err() {
        return Err(ReportError::DashboardTabNotFound);
    }

    if create {
        let report_name = report.name.clone();
        db::dashboards::reports::create(conn, "default", report)
            .await
            .map_err(ReportError::DbError)?;
        if !name.is_empty() {
            set_ownership(org_id, "reports", Authz::new(&report_name)).await;
            // todo: set parent folder
        }
    } else {
        db::dashboards::reports::update(conn, "default", None, report)
            .await
            .map_err(ReportError::DbError)?;
    }

    Ok(())
}

pub async fn get(org_id: &str, name: &str) -> Result<Report, ReportError> {
    let conn = ORM_CLIENT.get_or_init(connect_to_orm).await;
    db::dashboards::reports::get(conn, org_id, "default", name)
        .await
        .map_err(|_| ReportError::ReportNotFound)
}

pub async fn list(
    org_id: &str,
    filters: ReportListFilters,
    permitted: Option<Vec<String>>,
) -> Result<Vec<table::reports::ListReportsQueryResult>, ReportError> {
    let conn = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let params = filters.into_parmas(org_id);
    let reports = db::dashboards::reports::list(conn, &params)
        .await
        .map_err(ReportError::DbError)?;
    let result = reports
        .into_iter()
        .filter(|report| {
            permitted.is_none()
                || permitted
                    .as_ref()
                    .unwrap()
                    .contains(&format!("report:{}", report.report_name))
                || permitted
                    .as_ref()
                    .unwrap()
                    .contains(&format!("report:_all_{}", org_id))
        })
        .collect_vec();
    Ok(result)
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), ReportError> {
    let conn = ORM_CLIENT.get_or_init(connect_to_orm).await;

    // TODO: If we are going to perform both the "get" and "delete" operations then they should be
    // in a transaction.
    if db::dashboards::reports::get(conn, org_id, "default", name)
        .await
        .is_err()
    {
        return Err(ReportError::ReportNotFound);
    }

    match db::dashboards::reports::delete(conn, org_id, "default", name).await {
        Ok(_) => {
            remove_ownership(org_id, "reports", Authz::new(name)).await;
            Ok(())
        }
        Err(e) => Err(ReportError::DbError(e)),
    }
}

pub async fn trigger(org_id: &str, name: &str) -> Result<(), ReportError> {
    let conn = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let report = match db::dashboards::reports::get(conn, org_id, "default", name).await {
        Ok(report) => report,
        _ => {
            return Err(ReportError::ReportNotFound);
        }
    };
    report.send_subscribers().await?;
    Ok(())
}

pub async fn enable(org_id: &str, name: &str, value: bool) -> Result<(), ReportError> {
    let conn = ORM_CLIENT.get_or_init(connect_to_orm).await;

    // TODO: The "get" and "update" operations should be in a transaction.
    let mut report = match db::dashboards::reports::get(conn, org_id, "default", name).await {
        Ok(report) => report,
        _ => {
            return Err(ReportError::ReportNotFound);
        }
    };
    report.enabled = value;
    db::dashboards::reports::update(conn, "default", None, report)
        .await
        .map_err(ReportError::DbError)
}

#[derive(Debug, thiserror::Error)]
pub enum SendReportError {
    #[error("Atleast one dashboard is required")]
    NoDashboards,

    #[error("Error contacting report server: {0}")]
    ReportServerClientError(#[from] reqwest::Error),

    #[error("report send error status: {0}, err: {1:?}")]
    ReportServerErrorRepsponse(reqwest::StatusCode, Result<bytes::Bytes, reqwest::Error>),

    #[error("SMTP configuration not enabled")]
    SmtpNotEnabled,

    #[error(transparent)]
    ParseAddressError(#[from] lettre::address::AddressError),

    #[error(transparent)]
    ParseContentTypeError(#[from] lettre::message::header::ContentTypeErr),

    #[error("Error sending email: {0}")]
    SendEmailError(#[from] lettre::transport::smtp::Error),

    #[error(transparent)]
    GenerateReportError(#[from] GenerateReportError),
}

#[async_trait]
pub trait SendReport {
    /// Sends the report to subscribers
    async fn send_subscribers(&self) -> Result<(), SendReportError>;
}

#[async_trait]
impl SendReport for Report {
    /// Sends the report to subscribers
    async fn send_subscribers(&self) -> Result<(), SendReportError> {
        if self.dashboards.is_empty() {
            return Err(SendReportError::NoDashboards);
        }

        let cfg = get_config();
        let mut recipients = vec![];
        for recipient in &self.destinations {
            match recipient {
                ReportDestination::Email(email) => recipients.push(email.clone()),
            }
        }
        let no_of_recipients = recipients.len();
        if !cfg.common.report_server_url.is_empty() {
            let report_data = HttpReportPayload {
                dashboards: self.dashboards.clone(),
                email_details: ReportEmailDetails {
                    title: self.title.clone(),
                    recipients,
                    name: self.name.clone(),
                    message: self.message.clone(),
                    dashb_url: format!("{}{}/web", cfg.common.web_url, cfg.common.base_uri),
                },
            };

            let url = url::Url::parse(&format!(
                "{}/api/{}/reports/{}/send",
                &cfg.common.report_server_url, &self.org_id, &self.name
            ))
            .unwrap();
            match Client::builder()
                .danger_accept_invalid_certs(cfg.common.report_server_skip_tls_verify)
                .build()
                .unwrap()
                .put(url)
                .query(&[("timezone", &self.timezone)])
                .header("Content-Type", "application/json")
                .json(&report_data)
                .send()
                .await
            {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        return Err(SendReportError::ReportServerErrorRepsponse(
                            resp.status(),
                            resp.bytes().await,
                        ));
                    }
                }
                Err(e) => {
                    return Err(SendReportError::ReportServerClientError(e));
                }
            }
            Ok(())
        } else {
            // Currently only one `ReportDashboard` can be captured and sent
            let dashboard = &self.dashboards[0];
            let report = generate_report(
                dashboard,
                &self.org_id,
                &cfg.common.report_user_name,
                &cfg.common.report_user_password,
                &self.timezone,
                no_of_recipients,
                &self.name,
            )
            .await?;
            send_email(self, &report.0, report.1).await
        }
    }
}

/// Sends emails to the [`Report`] recipients. Currently only one pdf data is supported.
async fn send_email(
    report: &Report,
    pdf_data: &[u8],
    dashb_url: String,
) -> Result<(), SendReportError> {
    let cfg = get_config();
    if !cfg.smtp.smtp_enabled {
        return Err(SendReportError::SmtpNotEnabled);
    }

    let mut recipients = vec![];
    for recipient in &report.destinations {
        match recipient {
            ReportDestination::Email(email) => recipients.push(email),
        }
    }

    if recipients.is_empty() {
        return Ok(());
    }

    let mut email = Message::builder()
        .from(cfg.smtp.smtp_from_email.parse()?)
        .subject(report.title.to_string());

    for recipient in recipients {
        email = email.to(recipient.parse()?);
    }

    if !cfg.smtp.smtp_reply_to.is_empty() {
        email = email.reply_to(cfg.smtp.smtp_reply_to.parse()?);
    }

    let email = email
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::html(format!(
                    "{}\n\n<p><a href='{dashb_url}' target='_blank'>Link to dashboard</a></p>",
                    report.message
                )))
                .singlepart(
                    // Only supports PDF for now, attach the PDF
                    lettre::message::Attachment::new(format!(
                        "{}.pdf",
                        sanitize_filename(&report.title)
                    ))
                    .body(pdf_data.to_owned(), ContentType::parse("application/pdf")?),
                ),
        )
        .unwrap();

    // Send the email
    match SMTP_CLIENT.as_ref().unwrap().send(email).await {
        Ok(_) => {
            log::info!("email sent successfully for the report {}", &report.name);
            Ok(())
        }
        Err(e) => Err(SendReportError::SendEmailError(e)),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GenerateReportError {
    #[error("Chrome not enabled")]
    ChromeNotEnabled,

    #[error("Atleast one tab is required")]
    NoDashboardTabs,

    #[error(transparent)]
    ChromiumOxideError(#[from] chromiumoxide::error::CdpError),

    #[error(transparent)]
    ParseTimeDurationError(std::num::ParseIntError),

    #[error(transparent)]
    BrowserCloseWaitError(std::io::Error),

    #[error(transparent)]
    JoinHandlerError(#[from] tokio::task::JoinError),

    #[error("[REPORT] main element not rendered yet for dashboard {dashboard_id}: {e}")]
    MainElementNotRendered {
        dashboard_id: String,
        e: chromiumoxide::error::CdpError,
    },

    #[error("[REPORT] div.displayDiv element not rendered yet for dashboard {dashboard_id}: {e}")]
    DisplayDivElementNotRendered {
        dashboard_id: String,
        e: chromiumoxide::error::CdpError,
    },

    #[error("span element indicator for data load not rendered yet")]
    DataLoadElementNotRendered,
}

async fn generate_report(
    dashboard: &ReportDashboard,
    org_id: &str,
    user_id: &str,
    user_pass: &str,
    timezone: &str,
    no_of_recipients: usize,
    report_name: &str,
) -> Result<(Vec<u8>, String), GenerateReportError> {
    let cfg = get_config();
    // Check if Chrome is enabled, otherwise don't save the report
    if !cfg.chrome.chrome_enabled {
        return Err(GenerateReportError::ChromeNotEnabled);
    }

    let dashboard_id = &dashboard.dashboard;
    let folder_id = &dashboard.folder;

    if dashboard.tabs.is_empty() {
        return Err(GenerateReportError::NoDashboardTabs);
    }
    // Only one tab is supported for now
    let tab_id = &dashboard.tabs[0];
    let mut dashb_vars = "".to_string();
    for variable in dashboard.variables.iter() {
        dashb_vars = format!("{}&var-{}={}", dashb_vars, variable.key, variable.value);
    }

    log::info!("launching browser for dashboard {dashboard_id}");
    let (mut browser, mut handler) =
        Browser::launch(get_chrome_launch_options().await.as_ref().unwrap().clone()).await?;
    log::info!("browser launched");

    let handle = tokio::task::spawn(async move {
        while let Some(h) = handler.next().await {
            match h {
                Ok(_) => continue,
                Err(_) => break,
            }
        }
    });

    let web_url = format!("{}{}/web", cfg.common.web_url, cfg.common.base_uri);
    log::debug!("Navigating to web url: {}", &web_url);
    let page = browser
        .new_page(&format!("{web_url}/login?login_as_internal_user=true"))
        .await?;
    page.disable_log().await?;
    log::debug!("headless: new page created");

    page.find_element("input[type='email']")
        .await?
        .click()
        .await?
        .type_str(user_id)
        .await?;
    log::debug!("headless: email input filled");

    page.find_element("input[type='password']")
        .await?
        .click()
        .await?
        .type_str(user_pass)
        .await?
        .press_key("Enter")
        .await?;
    log::debug!("headless: password input filled");

    // Does not seem to work for single page client application
    page.wait_for_navigation().await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    let timerange = &dashboard.timerange;
    let search_type_params = if no_of_recipients == 0 {
        "search_type=ui".to_string()
    } else {
        format!("search_type=reports&report_id={org_id}-{report_name}")
    };

    // dashboard link in the email should contain data of the same period as the report
    let (dashb_url, email_dashb_url) = match timerange.range_type {
        ReportTimerangeType::Relative => {
            let period = &timerange.period;
            let (time_duration, time_unit) = period.split_at(period.len() - 1);
            let dashb_url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&{search_type_params}&period={period}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
            );

            let time_duration: i64 = time_duration
                .parse()
                .map_err(GenerateReportError::ParseTimeDurationError)?;
            let end_time = chrono::Utc::now().timestamp_micros();
            let start_time = match time_unit {
                "m" => {
                    end_time
                        - chrono::Duration::try_minutes(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                "h" => {
                    end_time
                        - chrono::Duration::try_hours(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                "d" => {
                    end_time
                        - chrono::Duration::try_days(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                "w" => {
                    end_time
                        - chrono::Duration::try_weeks(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                _ => {
                    end_time
                        - chrono::Duration::try_days(30 * time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
            };

            let email_dashb_url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&from={start_time}&to={end_time}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
            );
            (dashb_url, email_dashb_url)
        }
        ReportTimerangeType::Absolute => {
            let url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&{search_type_params}&from={}&to={}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
                &timerange.from, &timerange.to
            );
            (url.clone(), url)
        }
    };

    log::debug!("headless: going to dash url");
    // First navigate to the correct org
    page.goto(&format!("{web_url}/?org_identifier={org_id}"))
        .await?;
    page.wait_for_navigation().await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    log::debug!("headless: navigated to the org_id: {org_id}");

    page.goto(&dashb_url).await?;
    log::debug!("headless: going to dash url");

    // Wait for navigation does not really wait until it is fully loaded
    page.wait_for_navigation().await?;

    log::info!("waiting for data to load for dashboard {dashboard_id}");

    // If the span element is not rendered yet, capture whatever is loaded till now
    if let Err(e) = wait_for_panel_data_load(&page).await {
        log::error!(
            "[REPORT] error occurred while finding the span element for dashboard {dashboard_id}:{e}"
        );
    } else {
        log::info!("[REPORT] all panel data loaded for report dashboard: {dashboard_id}");
    }

    if let Err(e) = page.find_element("main").await {
        browser.close().await?;
        browser
            .wait()
            .await
            .map_err(GenerateReportError::BrowserCloseWaitError)?;
        handle.await?;
        return Err(GenerateReportError::MainElementNotRendered {
            dashboard_id: dashboard_id.clone(),
            e,
        });
    }
    if let Err(e) = page.find_element("div.displayDiv").await {
        browser.close().await?;
        browser
            .wait()
            .await
            .map_err(GenerateReportError::BrowserCloseWaitError)?;
        handle.await?;
        return Err(GenerateReportError::DisplayDivElementNotRendered {
            dashboard_id: dashboard_id.clone(),
            e,
        });
    }

    // Last two elements loaded means atleast the metric components have loaded.
    // Convert the page into pdf
    let pdf_data = if no_of_recipients != 0 {
        page.pdf(PrintToPdfParams {
            landscape: Some(true),
            ..Default::default()
        })
        .await?
    } else {
        // No need to capture pdf
        vec![]
    };

    browser.close().await?;
    browser
        .wait()
        .await
        .map_err(GenerateReportError::BrowserCloseWaitError)?;
    handle.await?;
    log::debug!("done with headless browser");

    // convert to short_url
    let email_dashb_url = match short_url::shorten(org_id, &email_dashb_url).await {
        Ok(short_url) => short_url,
        Err(e) => {
            log::error!("Error shortening email dashboard url: {e}");
            email_dashb_url
        }
    };
    Ok((pdf_data, email_dashb_url))
}

async fn wait_for_panel_data_load(page: &Page) -> Result<(), GenerateReportError> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(get_config().chrome.chrome_sleep_secs.into());
    log::info!("waiting for headless data to load");
    loop {
        if page
            .find_element("span#dashboardVariablesAndPanelsDataLoaded")
            .await
            .is_ok()
        {
            return Ok(());
        }

        if start.elapsed() >= timeout {
            return Err(GenerateReportError::DataLoadElementNotRendered);
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
