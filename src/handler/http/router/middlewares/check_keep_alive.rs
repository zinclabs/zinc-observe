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

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::{ConnectionType, StatusCode},
};
use actix_web_lab::middleware::Next;

pub async fn check_keep_alive(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let req_conn_type = req.head().connection_type();
    let mut resp = next.call(req).await?;
    if resp.status() >= StatusCode::BAD_REQUEST || req_conn_type == ConnectionType::Close {
        resp.response_mut()
            .head_mut()
            .set_connection_type(ConnectionType::Close);
    }
    Ok(resp)
}
