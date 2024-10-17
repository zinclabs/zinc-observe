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

pub mod cluster;
pub mod config;
pub mod ider;
pub mod meta;
pub mod metrics;
pub mod utils;

pub use config::*;

pub async fn init() -> Result<(), anyhow::Error> {
    // init ider
    ider::init();

    // initialize chrome launch options, so that if chrome download is
    // needed, it will happen now and not during serving report API
    if cluster::LOCAL_NODE.is_alert_manager() {
        let _ = get_chrome_launch_options().await;
    }
    Ok(())
}
