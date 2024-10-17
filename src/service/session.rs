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

use super::db;

pub async fn get_session(session_id: &str) -> Option<String> {
    db::session::get(session_id).await.ok()
}

pub async fn set_session(session_id: &str, val: &str) -> Option<()> {
    db::session::set(session_id, val).await.ok()
}

pub async fn remove_session(session_id: &str) {
    let _ = db::session::delete(session_id).await;
}
