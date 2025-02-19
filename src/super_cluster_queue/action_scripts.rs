// Copyright (c) 2025. OpenObserve Inc.
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

use infra::errors::Result;
use o2_enterprise::enterprise::super_cluster::queue::{ActionScriptsMessage, Message};

pub(crate) async fn process(msg: Message) -> Result<()> {
    let action_scripts_message: ActionScriptsMessage = msg.try_into()?;
    match action_scripts_message {
        ActionScriptsMessage::Put { action } => {
            if let Ok(current_cluster_action) =
                infra::table::action_scripts::get(&action.id.unwrap().to_string(), &action.org_id)
                    .await
            {
                // Delete zip file if origin cluster changed and we're the original cluster
                if current_cluster_action.origin_cluster_url != action.origin_cluster_url
                    && current_cluster_action.origin_cluster_url
                        == config::get_config().common.web_url
                {
                    if let Some(zip_path) = &current_cluster_action.zip_file_path {
                        if let Err(e) = infra::storage::del(&[zip_path]).await {
                            log::error!(
                                "failed to delete the zip file from s3 bucket {}: {}",
                                zip_path,
                                e
                            );
                        }
                    }
                }
                infra::table::action_scripts::update(&action).await?;
            } else {
                infra::table::action_scripts::add(&action).await?;
            }
        }
        ActionScriptsMessage::Delete { org_id, action_id } => {
            if let Ok(current_cluster_action) =
                infra::table::action_scripts::get(&action_id, &org_id).await
            {
                // Delete zip file if we're the original cluster
                if current_cluster_action.origin_cluster_url == config::get_config().common.web_url
                {
                    if let Some(zip_path) = &current_cluster_action.zip_file_path {
                        if let Err(e) = infra::storage::del(&[zip_path]).await {
                            log::error!(
                                "failed to delete the zip file from s3 bucket {}: {}",
                                zip_path,
                                e
                            );
                        }
                    }
                }
            }
            infra::table::action_scripts::remove(&org_id, &action_id).await?;
        }
    }
    Ok(())
}
