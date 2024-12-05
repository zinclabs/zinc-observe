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

use config::meta::alerts::{
    alert as meta_alerts, destinations as meta_destinations, templates as meta_templates,
};
use serde::Serialize;
use utoipa::ToSchema;

use super::{alert::Alert, destinations::Destination, templates::Template};

/// HTTP response body for `ListStreamAlerts` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ListStreamAlertsResponseBody {
    pub list: Vec<Alert>,
}

/// HTTP response body for `ListAlerts` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ListAlertsResponseBody {
    pub list: Vec<Alert>,
}

/// HTTP response body for `GetAlert` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct GetAlertResponseBody(pub Alert);

/// HTTP response body for `EnableAlert` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct EnableAlertResponseBody {
    pub enabled: bool,
}

/// HTTP response body for `GetDestination` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct GetDestinationResponseBody(pub Destination);

/// HTTP response body for `ListDestinations` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ListDestinationsResponseBody(pub Vec<Destination>);

/// HTTP response body for `GetTemplate` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct GetTemplateResponseBody(pub Template);

/// HTTP response body for `ListTemplates` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ListTemplatesResponseBody(pub Vec<Template>);

impl From<Vec<meta_alerts::Alert>> for ListStreamAlertsResponseBody {
    fn from(value: Vec<meta_alerts::Alert>) -> Self {
        Self {
            list: value.into_iter().map(|a| a.into()).collect(),
        }
    }
}

impl From<Vec<meta_alerts::Alert>> for ListAlertsResponseBody {
    fn from(value: Vec<meta_alerts::Alert>) -> Self {
        Self {
            list: value.into_iter().map(|a| a.into()).collect(),
        }
    }
}

impl From<meta_alerts::Alert> for GetAlertResponseBody {
    fn from(value: meta_alerts::Alert) -> Self {
        Self(value.into())
    }
}

impl From<meta_destinations::Destination> for GetDestinationResponseBody {
    fn from(value: meta_destinations::Destination) -> Self {
        Self(value.into())
    }
}

impl From<Vec<meta_destinations::Destination>> for ListDestinationsResponseBody {
    fn from(value: Vec<meta_destinations::Destination>) -> Self {
        Self(value.into_iter().map(|d| d.into()).collect())
    }
}

impl From<meta_templates::Template> for GetTemplateResponseBody {
    fn from(value: meta_templates::Template) -> Self {
        Self(value.into())
    }
}

impl From<Vec<meta_templates::Template>> for ListTemplatesResponseBody {
    fn from(value: Vec<meta_templates::Template>) -> Self {
        Self(value.into_iter().map(|t| t.into()).collect())
    }
}
