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

use config::cluster;

use crate::service::db::file_list::{broadcast, local::BROADCAST_QUEUE};

pub async fn run() -> Result<(), anyhow::Error> {
    loop {
        if cluster::is_offline() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let files = {
            let mut q = BROADCAST_QUEUE.write().await;
            if q.is_empty() {
                continue;
            }
            q.drain(..).collect::<Vec<_>>()
        };
        if let Err(e) = broadcast::send(&files, None).await {
            log::error!("[broadcast] local queue to nodes error: {}", e);
        }
    }
    log::info!("job::files::broadcast is stopped");
    Ok(())
}
