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
            new_folder_id,
            dashboard,
            ..
        } => {
            // `clone` is always true for super cluster
            table::dashboards::put(
                &org_id,
                &folder_id,
                new_folder_id.as_deref(),
                dashboard,
                true,
            )
            .await?;
        }
        DashboardMessage::Delete {
            org_id,
            folder_id,
            dashboard_id,
        } => {
            table::dashboards::delete_from_folder(&org_id, &folder_id, &dashboard_id).await?;
        }
        DashboardMessage::TimedAnnotationCreate {
            dashboard_id,
            timed_annotation,
        } => {
            table::timed_annotations::add(
                &dashboard_id,
                timed_annotation,
                true, // always true for super cluster
            )
            .await?;
        }
        DashboardMessage::TimedAnnotationUpdate {
            dashboard_id,
            timed_annotation,
        } => {
            table::timed_annotations::update(&dashboard_id, timed_annotation).await?;
        }
        DashboardMessage::TimedAnnotationDelete {
            dashboard_id,
            timed_annotation_id,
        } => {
            table::timed_annotations::delete(&dashboard_id, &timed_annotation_id).await?;
        }
        DashboardMessage::TimedAnnotationPanelsCreate {
            timed_annotation_id,
            panels,
        } => {
            table::timed_annotation_panels::insert_many_panels(&timed_annotation_id, panels)
                .await?;
        }
        DashboardMessage::TimedAnnotationPanelsDelete {
            timed_annotation_id,
            panels,
        } => {
            table::timed_annotation_panels::delete_many_panels(&timed_annotation_id, panels)
                .await?;
        }
        _ => {
            // Temporarily do catch-all message handling so that we can add new message types to
            // o2_enterprise without breaking the build.
            log::warn!("Unsupported dashboard super cluster message: {:?}", msg);
        }
    };
    Ok(())
}
