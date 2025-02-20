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

//! These models define the schemas of HTTP request and response JSON bodies in
//! billings API endpoints.

use o2_enterprise::enterprise::cloud::billings as cloud_billings;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize)]
pub struct CheckoutSessionDetailRequestQuery {
    pub session_id: String,
    pub status: String,
    pub plan: String,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ListInvoicesResponseBody {
    pub invoices: Vec<cloud_billings::StripeInvoice>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ListSubscriptionResponseBody {
    pub subscription_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
}

impl From<cloud_billings::CustomerBilling> for ListSubscriptionResponseBody {
    fn from(value: cloud_billings::CustomerBilling) -> Self {
        Self {
            subscription_type: value.subscription_type.to_string(),
            customer_id: value.customer_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct GetOrgUsageResponseBody {
    pub data: OrgUserData,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, ToSchema, Default)]
pub struct OrgUserData {
    ingestion: f64,
    search: f64,
    functions: f64,
    other: f64,
}

impl From<cloud_billings::org_usage::OrgUsage> for GetOrgUsageResponseBody {
    fn from(value: cloud_billings::org_usage::OrgUsage) -> Self {
        Self {
            data: OrgUserData {
                ingestion: value.ingestion,
                search: value.search,
                functions: value.functions,
                other: value.other,
            },
            message: "Data usage retried successfully".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct GetQuotaThresholdResponseBody {
    pub data: OrgQuotaThreshold,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, ToSchema, Default)]
pub struct OrgQuotaThreshold {
    ingestion: f64,
    search: f64,
    functions: f64,
    other: f64,
}

impl From<cloud_billings::org_usage::OrgUsageThreshold> for GetQuotaThresholdResponseBody {
    fn from(value: cloud_billings::org_usage::OrgUsageThreshold) -> Self {
        Self {
            data: OrgQuotaThreshold {
                ingestion: value.ingestion,
                search: value.search,
                functions: value.functions,
                other: value.other,
            },
            message: "Organization monthly quota pulled successfully".to_string(),
        }
    }
}
