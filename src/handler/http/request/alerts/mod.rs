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

use actix_web::HttpResponse;

use crate::{
    common::meta::http::HttpResponse as MetaHttpResponse, service::alerts::alert::AlertError,
};

pub mod alert;
pub mod destinations;
pub mod templates;

impl From<AlertError> for HttpResponse {
    fn from(value: AlertError) -> Self {
        match &value {
            AlertError::InfraError(err) => MetaHttpResponse::internal_error(err),
            AlertError::CreateDefaultFolderError => MetaHttpResponse::internal_error(value),
            AlertError::AlertNameMissing => MetaHttpResponse::bad_request(value),
            AlertError::AlertNameOfgaUnsupported => MetaHttpResponse::bad_request(value),
            AlertError::AlertNameContainsForwardSlash => MetaHttpResponse::bad_request(value),
            AlertError::AlertDestinationMissing => MetaHttpResponse::bad_request(value),
            AlertError::CreateAlreadyExists => MetaHttpResponse::conflict(value),
            AlertError::AlertNotFound => MetaHttpResponse::not_found(value),
            AlertError::AlertDestinationNotFound { .. } => MetaHttpResponse::not_found(value),
            AlertError::StreamNotFound { .. } => MetaHttpResponse::not_found(value),
            AlertError::DecodeVrl(err) => MetaHttpResponse::bad_request(err),
            AlertError::ParseCron(err) => MetaHttpResponse::bad_request(err),
            AlertError::RealtimeMissingCustomQuery => MetaHttpResponse::bad_request(value),
            AlertError::SqlMissingQuery => MetaHttpResponse::bad_request(value),
            AlertError::SqlContainsSelectStar => MetaHttpResponse::bad_request(value),
            AlertError::PromqlMissingQuery => MetaHttpResponse::bad_request(value),
            AlertError::SendNotificationError { .. } => MetaHttpResponse::internal_error(value),
            AlertError::GetDestinationWithTemplateError(error) => {
                MetaHttpResponse::internal_error(error)
            }
            AlertError::PeriodExceedsMaxQueryRange { .. } => MetaHttpResponse::bad_request(value),
            AlertError::ResolveStreamNameError(_) => MetaHttpResponse::internal_error(value),
        }
    }
}
