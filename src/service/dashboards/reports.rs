// Copyright 2023 Zinc Labs Inc.
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

use actix_web::http;
use chromiumoxide::{browser::Browser, cdp::browser_protocol::page::PrintToPdfParams, Page};
use config::{get_chrome_launch_options, CONFIG, SMTP_CLIENT};
use cron::Schedule;
use futures::{future::try_join_all, StreamExt};
use lettre::{
    message::{header::ContentType, MultiPart, SinglePart},
    AsyncTransport, Message,
};
use reqwest::Client;

use crate::{
    common::{
        meta::{
            authz::Authz,
            dashboards::reports::{
                HttpReportPayload, Report, ReportDashboard, ReportDestination, ReportEmailDetails,
                ReportFrequencyType, ReportTimerangeType,
            },
        },
        utils::auth::{remove_ownership, set_ownership},
    },
    service::db,
};

pub async fn save(
    org_id: &str,
    name: &str,
    mut report: Report,
    create: bool,
) -> Result<(), anyhow::Error> {
    if CONFIG.common.report_server_url.is_empty() {
        // Check if SMTP is enabled, otherwise don't save the report
        if !CONFIG.smtp.smtp_enabled {
            return Err(anyhow::anyhow!("SMTP configuration not enabled"));
        }

        // Check if Chrome is enabled, otherwise don't save the report
        if !CONFIG.chrome.chrome_enabled || CONFIG.chrome.chrome_path.is_empty() {
            return Err(anyhow::anyhow!("Chrome not enabled"));
        }

        if CONFIG.common.report_user_name.is_empty()
            || CONFIG.common.report_user_password.is_empty()
        {
            return Err(anyhow::anyhow!("Report username and password ENVs not set"));
        }
    }

    if !name.is_empty() {
        report.name = name.to_string();
    }
    if report.name.is_empty() {
        return Err(anyhow::anyhow!("Report name is required"));
    }
    if report.name.contains('/') {
        return Err(anyhow::anyhow!("Report name cannot contain '/'"));
    }

    if report.frequency.frequency_type == ReportFrequencyType::Cron {
        // Check if the cron expression is valid
        Schedule::from_str(&report.frequency.cron)?;
    } else if report.frequency.interval == 0 {
        report.frequency.interval = 1;
    }

    match db::dashboards::reports::get(org_id, &report.name).await {
        Ok(_) => {
            if create {
                return Err(anyhow::anyhow!("Report already exists"));
            }
        }
        Err(_) => {
            if !create {
                return Err(anyhow::anyhow!("Report not found"));
            }
        }
    }

    // Atleast one `ReportDashboard` and `ReportDestination` needs to be present
    if report.dashboards.is_empty() || report.destinations.is_empty() {
        return Err(anyhow::anyhow!(
            "Atleast one dashboard/destination is required"
        ));
    }

    // Check if dashboards & tabs exist
    let mut tasks = Vec::with_capacity(report.dashboards.len());
    for dashboard in report.dashboards.iter() {
        let dash_id = &dashboard.dashboard;
        let folder = &dashboard.folder;
        if dashboard.tabs.is_empty() {
            return Err(anyhow::anyhow!("Atleast one tab is required"));
        }

        // Supports only one tab for now
        let tab_id = &dashboard.tabs[0];
        tasks.push(async move {
            let dashboard = db::dashboards::get(org_id, dash_id, folder).await?;
            // Check if the tab_id exists
            if let Some(dashboard) = dashboard.v3 {
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
        return Err(anyhow::anyhow!("Some dashboards/tabs not found"));
    }

    match db::dashboards::reports::set(org_id, &report, create).await {
        Ok(_) => {
            if name.is_empty() {
                set_ownership(org_id, "reports", Authz::new(&report.name)).await;
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub async fn get(org_id: &str, name: &str) -> Result<Report, anyhow::Error> {
    db::dashboards::reports::get(org_id, name)
        .await
        .map_err(|_| anyhow::anyhow!("Report not found"))
}

pub async fn list(
    org_id: &str,
    permitted: Option<Vec<String>>,
) -> Result<Vec<Report>, anyhow::Error> {
    match db::dashboards::reports::list(org_id).await {
        Ok(reports) => {
            let mut result = Vec::new();
            for report in reports {
                if permitted.is_none()
                    || permitted
                        .as_ref()
                        .unwrap()
                        .contains(&format!("report:{}", report.name))
                    || permitted
                        .as_ref()
                        .unwrap()
                        .contains(&format!("report:_all_{}", org_id))
                {
                    result.push(report);
                }
            }
            Ok(result)
        }
        Err(e) => Err(e),
    }
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), (http::StatusCode, anyhow::Error)> {
    if db::dashboards::reports::get(org_id, name).await.is_err() {
        return Err((
            http::StatusCode::NOT_FOUND,
            anyhow::anyhow!("Report not found {}", name),
        ));
    }

    match db::dashboards::reports::delete(org_id, name).await {
        Ok(_) => {
            remove_ownership(org_id, "reports", Authz::new(name)).await;
            Ok(())
        }
        Err(e) => Err((http::StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

pub async fn trigger(org_id: &str, name: &str) -> Result<(), (http::StatusCode, anyhow::Error)> {
    let report = match db::dashboards::reports::get(org_id, name).await {
        Ok(report) => report,
        _ => {
            return Err((
                http::StatusCode::NOT_FOUND,
                anyhow::anyhow!("Report not found"),
            ));
        }
    };
    report
        .send_subscribers()
        .await
        .map_err(|e| (http::StatusCode::INTERNAL_SERVER_ERROR, e))
}

pub async fn enable(
    org_id: &str,
    name: &str,
    value: bool,
) -> Result<(), (http::StatusCode, anyhow::Error)> {
    let mut report = match db::dashboards::reports::get(org_id, name).await {
        Ok(report) => report,
        _ => {
            return Err((
                http::StatusCode::NOT_FOUND,
                anyhow::anyhow!("Report not found"),
            ));
        }
    };
    report.enabled = value;
    db::dashboards::reports::set(org_id, &report, false)
        .await
        .map_err(|e| (http::StatusCode::INTERNAL_SERVER_ERROR, e))
}

impl Report {
    /// Sends the report to subscribers
    pub async fn send_subscribers(&self) -> Result<(), anyhow::Error> {
        if self.dashboards.is_empty() {
            return Err(anyhow::anyhow!("Atleast one dashboard is required"));
        }

        if !CONFIG.common.report_server_url.is_empty() {
            let mut recepients = vec![];
            for recepient in &self.destinations {
                match recepient {
                    ReportDestination::Email(email) => recepients.push(email.clone()),
                }
            }

            let report_data = HttpReportPayload {
                dashboards: self.dashboards.clone(),
                email_details: ReportEmailDetails {
                    title: self.title.clone(),
                    recepients,
                    name: self.name.clone(),
                    message: self.message.clone(),
                    dashb_url: format!("{}{}/web", CONFIG.common.web_url, CONFIG.common.base_uri),
                },
            };

            let url = url::Url::parse(&format!(
                "{}/{}/reports/{}/send",
                &CONFIG.common.report_server_url, &self.org_id, &self.name
            ))
            .unwrap();
            match Client::builder()
                .danger_accept_invalid_certs(CONFIG.common.report_server_skip_tls_verify)
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
                        return Err(anyhow::anyhow!(
                            "report send error status: {}, err: {:?}",
                            resp.status(),
                            resp.bytes().await
                        ));
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Error contacting report server: {e}"));
                }
            }
            Ok(())
        } else {
            // Currently only one `ReportDashboard` can be captured and sent
            let dashboard = &self.dashboards[0];
            let report = generate_report(
                dashboard,
                &self.org_id,
                &CONFIG.common.report_user_name,
                &CONFIG.common.report_user_password,
                &self.timezone,
            )
            .await?;
            self.send_email(&report.0, report.1).await
        }
    }

    /// Sends emails to the [`Report`] recepients. Currently only one pdf data is supported.
    async fn send_email(&self, pdf_data: &[u8], dashb_url: String) -> Result<(), anyhow::Error> {
        if !CONFIG.smtp.smtp_enabled {
            return Err(anyhow::anyhow!("SMTP configuration not enabled"));
        }

        let mut recepients = vec![];
        for recepient in &self.destinations {
            match recepient {
                ReportDestination::Email(email) => recepients.push(email),
            }
        }

        let mut email = Message::builder()
            .from(CONFIG.smtp.smtp_from_email.parse()?)
            .subject(format!("Openobserve Report - {}", &self.title));

        for recepient in recepients {
            email = email.to(recepient.parse()?);
        }

        if !CONFIG.smtp.smtp_reply_to.is_empty() {
            email = email.reply_to(CONFIG.smtp.smtp_reply_to.parse()?);
        }

        let email = email
            .multipart(
                MultiPart::mixed()
                    .singlepart(SinglePart::html(self.message.clone()))
                    .singlepart(SinglePart::html(format!(
                        "<p><a href='{dashb_url}' target='_blank'>Link to dashboard</a></p>"
                    )))
                    .singlepart(
                        // Only supports PDF for now, attach the PDF
                        lettre::message::Attachment::new(
                            self.title.clone(), // Attachment filename
                        )
                        .body(pdf_data.to_owned(), ContentType::parse("application/pdf")?),
                    ),
            )
            .unwrap();

        // Send the email
        match SMTP_CLIENT.as_ref().unwrap().send(email).await {
            Ok(_) => {
                log::info!("email sent successfully for the report {}", &self.name);
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("Error sending email: {e}")),
        }
    }
}

async fn generate_report(
    dashboard: &ReportDashboard,
    org_id: &str,
    user_id: &str,
    user_pass: &str,
    timezone: &str,
) -> Result<(Vec<u8>, String), anyhow::Error> {
    // Check if Chrome is enabled, otherwise don't save the report
    if !CONFIG.chrome.chrome_enabled {
        return Err(anyhow::anyhow!("Chrome not enabled"));
    }

    let dashboard_id = &dashboard.dashboard;
    let folder_id = &dashboard.folder;

    if dashboard.tabs.is_empty() {
        return Err(anyhow::anyhow!("Atleast one tab is required"));
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

    let web_url = format!("{}{}/web", CONFIG.common.web_url, CONFIG.common.base_uri);
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
    tokio::time::sleep(Duration::from_secs(5)).await;

    let timerange = &dashboard.timerange;

    // dashboard link in the email should contain data of the same period as the report
    let (dashb_url, email_dashb_url) = match timerange.range_type {
        ReportTimerangeType::Relative => {
            let period = &timerange.period;
            let (time_duration, time_unit) = period.split_at(period.len() - 1);
            let dashb_url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&period={period}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
            );

            let time_duration: i64 = time_duration.parse()?;
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
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&from={}&to={}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
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
    tokio::time::sleep(Duration::from_secs(2)).await;
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
        handle.await?;
        return Err(anyhow::anyhow!(
            "[REPORT] main element not rendered yet for dashboard {dashboard_id}: {e}"
        ));
    }
    if let Err(e) = page.find_element("div.displayDiv").await {
        browser.close().await?;
        handle.await?;
        return Err(anyhow::anyhow!(
            "[REPORT] div.displayDiv element not rendered yet for dashboard {dashboard_id}: {e}"
        ));
    }

    // Last two elements loaded means atleast the metric components have loaded.
    // Convert the page into pdf
    let pdf_data = page
        .pdf(PrintToPdfParams {
            landscape: Some(true),
            ..Default::default()
        })
        .await?;

    browser.close().await?;
    handle.await?;
    log::debug!("done with headless browser");
    Ok((pdf_data, email_dashb_url))
}

async fn wait_for_panel_data_load(page: &Page) -> Result<(), anyhow::Error> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(CONFIG.chrome.chrome_sleep_secs.into());
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
            return Err(anyhow::anyhow!(
                "span element indicator for data load not rendered yet"
            ));
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
