// Copyright 2024 Zinc Labs Inc.
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

use infra::{errors::Result, table};
use o2_enterprise::enterprise::super_cluster::queue::{DashboardMessage, Message};

pub(crate) async fn process(msg: Message) -> Result<()> {
    let msg = msg.try_into()?;
    process_msg(msg).await?;
    Ok(())
}

pub(crate) async fn process_msg(msg: DashboardMessage) -> Result<()> {
    match msg {
        DashboardMessage::Put {
            org_id,
            folder_id,
            dashboard,
        } => {
            table::dashboards::put(&org_id, &folder_id, dashboard).await?;
        }
        DashboardMessage::Delete {
            org_id,
            folder_id,
            dashboard_id,
        } => {
            table::dashboards::delete_from_folder(&org_id, &folder_id, &dashboard_id).await?;
        }
    };
    Ok(())
}
