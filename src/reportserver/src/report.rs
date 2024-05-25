use chromiumoxide::{
    browser::{Browser, BrowserConfig},
    cdp::browser_protocol::page::PrintToPdfParams,
    detection::{default_executable, DetectionOptions},
    fetcher::{BrowserFetcher, BrowserFetcherOptions},
    handler::viewport::Viewport,
    Page,
};
use config::CONFIG;
use futures::StreamExt;
use lettre::{
    message::{header::ContentType, MultiPart, SinglePart},
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use once_cell::sync::Lazy;
use tokio::time::{sleep, Duration};

use crate::models;

static CHROME_LAUNCHER_OPTIONS: tokio::sync::OnceCell<BrowserConfig> =
    tokio::sync::OnceCell::const_new();

pub async fn get_chrome_launch_options() -> &'static BrowserConfig {
    CHROME_LAUNCHER_OPTIONS
        .get_or_init(init_chrome_launch_options)
        .await
}

async fn init_chrome_launch_options() -> BrowserConfig {
    let mut browser_config = BrowserConfig::builder()
        .window_size(
            CONFIG.chrome.chrome_window_width,
            CONFIG.chrome.chrome_window_height,
        )
        .viewport(Viewport {
            width: CONFIG.chrome.chrome_window_width,
            height: CONFIG.chrome.chrome_window_height,
            device_scale_factor: Some(1.0),
            ..Viewport::default()
        });

    if CONFIG.chrome.chrome_with_head {
        browser_config = browser_config.with_head();
    }

    if CONFIG.chrome.chrome_no_sandbox {
        browser_config = browser_config.no_sandbox();
    }

    if !CONFIG.chrome.chrome_path.is_empty() {
        browser_config = browser_config.chrome_executable(CONFIG.chrome.chrome_path.as_str());
    } else {
        let mut should_download = false;

        if !CONFIG.chrome.chrome_check_default {
            should_download = true;
        } else {
            // Check if chrome is available on default paths
            // 1. Check the CHROME env
            // 2. Check usual chrome file names in user path
            // 3. (Windows) Registry
            // 4. (Windows & MacOS) Usual installations paths
            if let Ok(exec_path) = default_executable(DetectionOptions::default()) {
                browser_config = browser_config.chrome_executable(exec_path);
            } else {
                should_download = true;
            }
        }
        if should_download && !CONFIG.chrome.chrome_auto_download {
            should_download = false;
            log::error!("Chrome binary could not be detected");
        }

        if should_download {
            // Download known good chrome version
            let download_path = &CONFIG.chrome.chrome_download_path;
            log::info!("fetching chrome at: {download_path}");
            tokio::fs::create_dir_all(download_path).await.unwrap();
            let fetcher = BrowserFetcher::new(
                BrowserFetcherOptions::builder()
                    .with_path(download_path)
                    .build()
                    .unwrap(),
            );

            // Fetches the browser revision, either locally if it was previously
            // installed or remotely. Returns error when the download/installation
            // fails. Since it doesn't retry on network errors during download,
            // if the installation fails, it might leave the cache in a bad state
            // and it is advised to wipe it.
            // Note: Does not work on LinuxArm platforms.
            match fetcher.fetch().await {
                Ok(info) => {
                    log::info!(
                        "chrome fetched at path {:#?}",
                        info.executable_path.as_path()
                    );
                    browser_config = browser_config.chrome_executable(info.executable_path);
                }
                Err(e) => {
                    log::error!("chrome binary could not be fetched: {e}");
                }
            }
        }
    }
    browser_config.build().unwrap()
}

pub static SMTP_CLIENT: Lazy<AsyncSmtpTransport<Tokio1Executor>> = Lazy::new(|| {
    let tls_parameters = TlsParameters::new(CONFIG.smtp.smtp_host.clone()).unwrap();
    let mut transport_builder =
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&CONFIG.smtp.smtp_host)
            .port(CONFIG.smtp.smtp_port);

    let option = &CONFIG.smtp.smtp_encryption;
    transport_builder = if option == "starttls" {
        transport_builder.tls(Tls::Required(tls_parameters))
    } else if option == "ssltls" {
        transport_builder.tls(Tls::Wrapper(tls_parameters))
    } else {
        transport_builder
    };

    if !CONFIG.smtp.smtp_username.is_empty() && !CONFIG.smtp.smtp_password.is_empty() {
        transport_builder = transport_builder.credentials(Credentials::new(
            CONFIG.smtp.smtp_username.clone(),
            CONFIG.smtp.smtp_password.clone(),
        ));
    }
    transport_builder.build()
});

pub async fn generate_report(
    dashboard: &models::ReportDashboard,
    org_id: &str,
    user_id: &str,
    user_pass: &str,
    web_url: &str,
    timezone: &str,
) -> Result<(Vec<u8>, String), anyhow::Error> {
    let dashboard_id = &dashboard.dashboard;
    let folder_id = &dashboard.folder;

    let mut dashb_vars = "".to_string();
    for variable in dashboard.variables.iter() {
        dashb_vars = format!("{}&var-{}={}", dashb_vars, variable.key, variable.value);
    }

    if dashboard.tabs.is_empty() {
        return Err(anyhow::anyhow!("Atleast one tab is required"));
    }
    // Only one tab is supported for now
    let tab_id = &dashboard.tabs[0];

    log::info!("launching browser for dashboard {dashboard_id}");
    let (mut browser, mut handler) =
        Browser::launch(get_chrome_launch_options().await.clone()).await?;
    log::info!("browser launched");

    let handle = tokio::task::spawn(async move {
        while let Some(h) = handler.next().await {
            match h {
                Ok(_) => continue,
                Err(_) => break,
            }
        }
    });

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
    sleep(Duration::from_secs(5)).await;

    let timerange = &dashboard.timerange;

    // dashboard link in the email should contain data of the same period as the report
    let (dashb_url, email_dashb_url) = match timerange.range_type {
        models::ReportTimerangeType::Relative => {
            let period = &timerange.period;
            let (time_duration, time_unit) = period.split_at(period.len() - 1);
            let dashb_url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&period={period}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
            );
            log::debug!("dashb_url for dashboard {folder_id}/{dashboard_id}: {dashb_url}");

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
        models::ReportTimerangeType::Absolute => {
            let url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&from={}&to={}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
                &timerange.from, &timerange.to
            );
            log::debug!("dashb_url for dashboard {folder_id}/{dashboard_id}: {url}");

            (url.clone(), url)
        }
    };

    log::debug!("headless: going to dash url");
    // First navigate to the correct org
    page.goto(&format!("{web_url}/?org_identifier={org_id}"))
        .await?;
    page.wait_for_navigation().await?;
    sleep(Duration::from_secs(2)).await;
    log::debug!("headless: navigated to the org_id: {org_id}");

    page.goto(&dashb_url).await?;
    log::debug!("headless: going to dash url");

    // Wait for navigation does not really wait until it is fully loaded
    page.wait_for_navigation().await?;

    log::debug!("waiting for data to load for dashboard {dashboard_id}");

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

/// Sends emails to the [`Report`] recepients. Currently only one pdf data is supported.
pub async fn send_email(
    pdf_data: &[u8],
    email_details: models::EmailDetails,
    config: models::SmtpConfig,
) -> Result<(), anyhow::Error> {
    let mut recepients = vec![];
    for recepient in &email_details.recepients {
        recepients.push(recepient);
    }

    let mut email = Message::builder()
        .from(config.from_email.parse()?)
        .subject(format!("Openobserve Report - {}", &email_details.title));

    for recepient in recepients {
        email = email.to(recepient.parse()?);
    }

    if !config.reply_to.is_empty() {
        email = email.reply_to(config.reply_to.parse()?);
    }

    let email = email
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::html(email_details.message))
                .singlepart(SinglePart::html(format!(
                    "<p><a href='{}' target='_blank'>Link to dashboard</a></p>",
                    email_details.dashb_url
                )))
                .singlepart(
                    // Only supports PDF for now, attach the PDF
                    lettre::message::Attachment::new(
                        email_details.title, // Attachment filename
                    )
                    .body(pdf_data.to_owned(), ContentType::parse("application/pdf")?),
                ),
        )
        .unwrap();

    // Send the email
    match config.client.send(email).await {
        Ok(_) => {
            log::info!(
                "email sent successfully for the report {}",
                &email_details.name
            );
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Error sending email: {e}")),
    }
}

pub async fn wait_for_panel_data_load(page: &Page) -> Result<(), anyhow::Error> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(CONFIG.chrome.chrome_sleep_secs.into());
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
