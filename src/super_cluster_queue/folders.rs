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

use config::meta::folder::Folder;
use infra::{errors::Result, table};
use o2_enterprise::enterprise::super_cluster::queue::{FolderMessage, Message};

pub(crate) async fn process(msg: Message) -> Result<()> {
    let msg = msg.try_into()?;
    process_msg(msg).await?;
    Ok(())
}

pub(crate) async fn process_msg(msg: FolderMessage) -> Result<()> {
    match msg {
        FolderMessage::Create {
            org_id,
            id,
            folder_id,
            folder_type,
            name,
            description,
        } => {
            table::folders::put(
                &org_id,
                Some(id),
                Folder {
                    folder_id,
                    name,
                    description: description.unwrap_or_default(),
                },
                folder_type,
            )
            .await?;
        }
        FolderMessage::Update {
            org_id,
            folder_id,
            folder_type,
            name,
            description,
        } => {
            table::folders::put(
                &org_id,
                None,
                Folder {
                    folder_id,
                    name,
                    description: description.unwrap_or_default(),
                },
                folder_type,
            )
            .await?;
        }
        FolderMessage::Delete {
            org_id,
            folder_id,
            folder_type,
        } => {
            table::folders::delete(&org_id, &folder_id, folder_type).await?;
        }
    };
    Ok(())
}
