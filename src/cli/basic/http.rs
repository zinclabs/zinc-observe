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

use hashbrown::HashSet;
use prettytable::{Cell, Row, Table};

pub async fn query(
    org: &str,
    sql: &str,
    time_range: &str,
    limit: i64,
) -> Result<(), anyhow::Error> {
    let time_range = config::utils::time::parse_milliseconds(time_range)?;
    let time_end = config::utils::time::now_micros();
    let time_start = time_end - (time_range as i64 * 1000);
    let query = config::meta::search::Query {
        sql: sql.to_string(),
        from: 0,
        size: limit,
        start_time: time_start,
        end_time: time_end,
        quick_mode: false,
        ..Default::default()
    };
    let search_req = config::meta::search::Request {
        query,
        ..Default::default()
    };
    let url = format!("/api/{}/_search", org);
    let body = serde_json::to_vec(&search_req)?;
    let response = request(&url, Some(body), reqwest::Method::POST).await?;
    let Some(body) = response else {
        return Err(anyhow::anyhow!("query failed"));
    };
    let response = serde_json::from_str::<config::meta::search::Response>(&body)?;
    if response.hits.is_empty() {
        println!("total: 0, took: {} ms", response.took);
        return Ok(());
    }

    let mut table = Table::new();
    let mut columns = Vec::new();
    let mut columns_set = HashSet::new();
    for hit in response.hits.iter() {
        let hit = hit.as_object().unwrap();
        for key in hit.keys() {
            if columns_set.contains(key) {
                continue;
            }
            columns_set.insert(key.to_string());
            columns.push(key.to_string());
        }
    }
    let headers = columns.iter().map(|c| Cell::new(c)).collect();
    table.add_row(Row::new(headers));
    for hit in response.hits {
        let hit = hit.as_object().unwrap();
        let mut row = Vec::new();
        for column in columns.iter() {
            row.push(Cell::new(
                &hit.get(column).map_or("".to_string(), |v| v.to_string()),
            ));
        }
        table.add_row(Row::new(row));
    }
    table.printstd();
    println!("total: {}, took: {} ms", response.total, response.took);
    Ok(())
}

pub async fn node_offline() -> Result<(), anyhow::Error> {
    let url = "/node/enable?value=false";
    let response = request(url, None, reqwest::Method::PUT).await?;
    if response.is_some() {
        println!("node offline successfully");
    } else {
        println!("node offline failed");
    }
    Ok(())
}
pub async fn node_online() -> Result<(), anyhow::Error> {
    let url = "/node/enable?value=true";
    let response = request(url, None, reqwest::Method::PUT).await?;
    if response.is_some() {
        println!("node online successfully");
    } else {
        println!("node online failed");
    }
    Ok(())
}

pub async fn node_flush() -> Result<(), anyhow::Error> {
    let url = "/node/flush";
    let response = request(url, None, reqwest::Method::POST).await?;
    if response.is_some() {
        println!("node flush successfully");
    } else {
        println!("node flush failed");
    }
    Ok(())
}

pub async fn node_list() -> Result<(), anyhow::Error> {
    let url = "/node/list";
    let response = request(url, None, reqwest::Method::GET).await?;
    let Some(body) = response else {
        return Err(anyhow::anyhow!("node list failed"));
    };
    let mut nodes: Vec<config::meta::cluster::Node> = serde_json::from_str(&body)?;
    nodes.sort_by_key(|node| node.id);

    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("ID"),
        Cell::new("UUID"),
        Cell::new("Name"),
        Cell::new("HTTP Address"),
        Cell::new("Status"),
        Cell::new("Scheduled"),
        Cell::new("CPU"),
        Cell::new("MEM"),
    ]));

    for node in nodes {
        table.add_row(Row::new(vec![
            Cell::new(&node.id.to_string()),
            Cell::new(&node.uuid),
            Cell::new(&node.name),
            Cell::new(&node.http_addr),
            Cell::new(&format!("{:?}", node.status)),
            Cell::new(&node.scheduled.to_string()),
            Cell::new(&node.metrics.cpu_total.to_string()),
            Cell::new(
                &config::utils::size::bytes_to_human_readable(node.metrics.memory_total as f64)
                    .to_string(),
            ),
        ]));
    }

    table.printstd();
    Ok(())
}

pub async fn node_list_with_metrics() -> Result<(), anyhow::Error> {
    let url = "/node/list";
    let response = request(url, None, reqwest::Method::GET).await?;
    let Some(body) = response else {
        return Err(anyhow::anyhow!("node list failed"));
    };
    let mut nodes: Vec<config::meta::cluster::Node> = serde_json::from_str(&body)?;
    nodes.sort_by_key(|node| node.id);

    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("ID"),
        Cell::new("UUID"),
        Cell::new("Name"),
        Cell::new("HTTP Address"),
        Cell::new("Status"),
        Cell::new("Scheduled"),
        Cell::new("CPU"),
        Cell::new("Usage"),
        Cell::new("MEM"),
        Cell::new("Usage"),
        Cell::new("TCP Conns"),
        Cell::new("Established"),
        Cell::new("Close Wait"),
        Cell::new("Time Wait"),
    ]));

    for node in nodes {
        table.add_row(Row::new(vec![
            Cell::new(&node.id.to_string()),
            Cell::new(&node.uuid),
            Cell::new(&node.name),
            Cell::new(&node.http_addr),
            Cell::new(&format!("{:?}", node.status)),
            Cell::new(&node.scheduled.to_string()),
            Cell::new(&node.metrics.cpu_total.to_string()),
            Cell::new(&format!("{:.2}%", node.metrics.cpu_usage)),
            Cell::new(
                &config::utils::size::bytes_to_human_readable(node.metrics.memory_total as f64)
                    .to_string(),
            ),
            Cell::new(
                &config::utils::size::bytes_to_human_readable(node.metrics.memory_usage as f64)
                    .to_string(),
            ),
            Cell::new(&node.metrics.tcp_conns.to_string()),
            Cell::new(&node.metrics.tcp_conns_established.to_string()),
            Cell::new(&node.metrics.tcp_conns_close_wait.to_string()),
            Cell::new(&node.metrics.tcp_conns_time_wait.to_string()),
        ]));
    }

    table.printstd();
    Ok(())
}

pub async fn local_node_metrics() -> Result<(), anyhow::Error> {
    let metrics = config::utils::sysinfo::get_node_metrics();

    // Create header row with all column names
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("Name"),
        Cell::new("CPU"),
        Cell::new("Usage"),
        Cell::new("MEM"),
        Cell::new("Usage"),
        Cell::new("TCP Conns"),
        Cell::new("Established"),
        Cell::new("Close Wait"),
        Cell::new("Time Wait"),
    ]));

    // Create second row with all values
    table.add_row(Row::new(vec![
        Cell::new(&config::utils::sysinfo::os::get_hostname()),
        Cell::new(&metrics.cpu_total.to_string()),
        Cell::new(&format!("{:.2}%", metrics.cpu_usage)),
        Cell::new(
            &config::utils::size::bytes_to_human_readable(metrics.memory_total as f64).to_string(),
        ),
        Cell::new(
            &config::utils::size::bytes_to_human_readable(metrics.memory_usage as f64).to_string(),
        ),
        Cell::new(&metrics.tcp_conns.to_string()),
        Cell::new(&metrics.tcp_conns_established.to_string()),
        Cell::new(&metrics.tcp_conns_close_wait.to_string()),
        Cell::new(&metrics.tcp_conns_time_wait.to_string()),
    ]));

    table.printstd();
    Ok(())
}

async fn request(
    url: &str,
    body: Option<Vec<u8>>,
    method: reqwest::Method,
) -> Result<Option<String>, anyhow::Error> {
    let cfg = config::get_config();
    let client = reqwest::Client::new();
    let local = config::cluster::load_local_node();
    let url = format!("{}{}", local.http_addr, url);
    let user = cfg.auth.root_user_email.clone();
    let password = cfg.auth.root_user_password.clone();
    let mut response = client.request(method, url).basic_auth(user, Some(password));
    if let Some(body) = body {
        response = response.body(body);
    }
    let response = response.send().await?;
    let status = response.status();
    let body = response.text().await?;
    if status.is_success() {
        Ok(Some(body))
    } else {
        Err(anyhow::anyhow!(
            "request failed, status: {}, body: {}",
            status,
            body
        ))
    }
}
