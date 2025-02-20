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

use config::{
    ider,
    meta::{
        search::{Query, Request, SearchEventType},
        self_reporting::usage::UsageEvent,
        stream::StreamType,
    },
    utils::json,
};
use o2_enterprise::enterprise::cloud::billings::{self, org_usage};

use super::search as SearchService;

pub async fn get_org_usage(
    org_id: &str,
    usage_range: org_usage::UsageRange,
) -> Result<org_usage::OrgUsage, billings::BillingError> {
    // fire the query
    let trace_id = ider::uuid();
    let (sql, start_time, end_time) =
        org_usage::create_usage_query_sql_and_time_range(org_id, &usage_range);
    let req = Request {
        query: Query {
            sql,
            from: 0,
            size: -1,
            start_time,
            end_time,
            quick_mode: false,
            query_type: "".to_string(),
            track_total_hits: false,
            uses_zo_fn: false,
            query_fn: None,
            skip_wal: false,
            streaming_output: false,
            streaming_id: None,
        },
        encoding: config::meta::search::RequestEncoding::Empty,
        regions: vec![],
        clusters: vec![],
        timeout: 0,
        search_type: Some(SearchEventType::Other),
        search_event_context: None,
        use_cache: None,
    };
    let resp = SearchService::search(&trace_id, org_id, StreamType::Logs, None, &req).await?;

    if resp.is_partial {
        return Err(billings::BillingError::PartialUsageResults(
            resp.function_error,
        ));
    }

    // parse the results
    let mut org_total_usage = org_usage::OrgUsage::default();
    for hit in resp.hits {
        match json::from_value::<org_usage::OrgUsageQueryResult>(hit) {
            Ok(usage_result) => match usage_result.event {
                UsageEvent::Search => org_total_usage.search += usage_result.total_size,
                UsageEvent::Ingestion => org_total_usage.ingestion += usage_result.total_size,
                UsageEvent::Functions => org_total_usage.functions += usage_result.total_size,
                UsageEvent::Other => org_total_usage.other += usage_result.total_size,
            },
            Err(e) => {
                log::debug!(
                    "[CLOUD:USAGE]: found aggregated usage data but failed to parse into results: {e}"
                );
            }
        }
    }

    Ok(org_total_usage)
}

pub async fn get_organization_usage_threshold(
    org_id: &str,
    user_email: &str,
) -> Result<org_usage::OrgUsageThreshold, billings::BillingError> {
    let subscription_type = billings::get_org_subscription_type(org_id, user_email)
        .await?
        .unwrap_or_default();

    if subscription_type.is_free_sub() {
        let org_sage = get_org_usage(org_id, org_usage::UsageRange::default()).await?;
        Ok(org_sage.into())
    } else {
        Ok(org_usage::OrgUsageThreshold::default())
    }
}
