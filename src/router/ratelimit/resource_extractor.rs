// Copyright 2025 OpenObserve Inc.
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

use actix_web::dev::ServiceRequest;
use futures_util::future::BoxFuture;
use infra::table::ratelimit::RatelimitRuleType;
use o2_ratelimit::ratelimit::dataresource::db::{
    MatchRatelimitRule, RatelimitResourceKey, RATELIMIT_RULES_CACHE,
};
use regex::Regex;

use crate::{
    common::utils::auth::extract_auth_str,
    handler::http::auth::validator::get_user_email_from_auth_str,
};

pub fn default_extractor(req: &ServiceRequest) -> BoxFuture<'_, String> {
    let auth_str = extract_auth_str(req.request());
    let local_path = req.path().to_string();
    let path = match local_path
        .strip_prefix(format!("{}/api/", config::get_config().common.base_uri).as_str())
    {
        Some(path) => path,
        None => &local_path,
    };

    let path_columns = path.split('/').collect::<Vec<&str>>();
    let (path, org_id) = (path.to_string(), path_columns[0].to_string());

    Box::pin(async move {
        let user_email = get_user_email_from_auth_str(&auth_str)
            .await
            .unwrap_or_default();
        let resource = format!("{}:{}:{}", org_id, user_email, path);

        if let Some(matched_resource) = exact_match(&org_id, &resource).await {
            return matched_resource;
        }

        if let Some(matched_resource) = regex_match(&org_id, &resource).await {
            return matched_resource;
        }

        resource
    })
}

async fn exact_match(org: &str, resource: &str) -> Option<String> {
    let rules = RATELIMIT_RULES_CACHE.read().await;
    let key = format!("{}:{}:{}", org, RatelimitRuleType::Exact, resource);
    match rules.get(&RatelimitResourceKey(key)) {
        Some(MatchRatelimitRule::Exact(rule)) => Some(rule.resource.clone()),
        _ => None,
    }
}

async fn regex_match(org: &str, resource: &str) -> Option<String> {
    let rules = RATELIMIT_RULES_CACHE.read().await;
    let key = format!("{}:{}", org, RatelimitRuleType::Regex);
    match rules.get(&RatelimitResourceKey(key)) {
        Some(MatchRatelimitRule::Regx(rules)) => {
            for rule in rules.iter() {
                if let Ok(re) = Regex::new(&rule.resource) {
                    if re.is_match(resource) {
                        return Some(rule.resource.clone());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use actix_web::{dev::ServiceRequest, test::TestRequest};
    use infra::table::ratelimit::RatelimitRule;

    use super::*;

    fn create_test_request(path: &str, auth: Option<&str>) -> ServiceRequest {
        let mut req = TestRequest::get().uri(path);

        if let Some(auth_str) = auth {
            req = req.insert_header(("Authorization", auth_str));
        }

        req.to_srv_request()
    }

    async fn init_test_cache() {
        let mut cache = HashMap::new();

        let exact_rule = RatelimitRule {
            org: "test_org".to_string(),
            rule_type: Some(RatelimitRuleType::Exact.to_string()),
            rule_id: Some("1".to_string()),
            resource: "test_org:user@example.com:/api/test".to_string(),
            threshold: 100,
        };
        let exact_key = RatelimitResourceKey(format!(
            "{}:{}:{}",
            "test_org",
            RatelimitRuleType::Exact.to_string(),
            "test_org:user@example.com:/api/test"
        ));
        cache.insert(exact_key, MatchRatelimitRule::Exact(exact_rule));

        let regex_rules = vec![
            RatelimitRule {
                org: "test_org".to_string(),
                rule_type: Some(RatelimitRuleType::Regex.to_string()),
                rule_id: Some("2".to_string()),
                resource: r"test_org:.*:test_org/actions/.*".to_string(),
                threshold: 200,
            },
            RatelimitRule {
                org: "test_org".to_string(),
                rule_type: Some(RatelimitRuleType::Regex.to_string()),
                rule_id: Some("3".to_string()),
                resource: r"test_org:.*:test_org/users/.*".to_string(),
                threshold: 300,
            },
        ];
        let regex_key = RatelimitResourceKey(format!(
            "{}:{}",
            "test_org",
            RatelimitRuleType::Regex.to_string()
        ));
        cache.insert(regex_key, MatchRatelimitRule::Regx(regex_rules));

        *RATELIMIT_RULES_CACHE.write().await = cache;
    }

    #[tokio::test]
    async fn test_exact_match() {
        init_test_cache().await;

        let result = exact_match("test_org", "test_org:user@example.com:/api/test").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "test_org:user@example.com:/api/test");

        let result = exact_match("test_org", "test_org:other@example.com:/api/test").await;
        assert!(result.is_none());

        let result = exact_match("wrong_org", "test_org:user@example.com:/api/test").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_regex_match() {
        init_test_cache().await;

        let result =
            regex_match("test_org", "test_org:user@example.com:test_org/actions/123").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), r"test_org:.*:test_org/actions/.*");

        let result = regex_match("test_org", "test_org:user@example.com:test_org/users/456").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), r"test_org:.*:test_org/users/.*");

        let result = regex_match("test_org", "test_org:user@example.com:other/789").await;
        assert!(result.is_none());

        let result = regex_match("wrong_org", "test_org:user@example.com:/est_org/123").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_default_extractor() {
        init_test_cache().await;

        let req = create_test_request(
            "/api/test_org",
            Some("Basic cm9vdEBleGFtcGxlLmNvbTpDb21wbGV4cGFzcyMxMjM="),
        );
        let result = default_extractor(&req).await;
        assert_eq!(result, "test_org:root@example.com:test_org");

        let req = create_test_request(
            "/api/test_org/actions/123",
            Some("Basic cm9vdEBleGFtcGxlLmNvbTpDb21wbGV4cGFzcyMxMjM="),
        );
        let result = default_extractor(&req).await;
        assert_eq!(result, r"test_org:.*:test_org/actions/.*");

        let req = create_test_request(
            "/api/other/789",
            Some("Basic cm9vdEBleGFtcGxlLmNvbTpDb21wbGV4cGFzcyMxMjM="),
        );
        let result = default_extractor(&req).await;
        assert_eq!(result, "other:root@example.com:other/789");

        let req = create_test_request("/api/test", None);
        let result = default_extractor(&req).await;
        assert_eq!(result, "test::test");
    }

    #[tokio::test]
    async fn test_path_handling() {
        init_test_cache().await;

        let req = create_test_request(
            "/api/default/_search",
            Some("Basic cm9vdEBleGFtcGxlLmNvbTpDb21wbGV4cGFzcyMxMjM="),
        );
        let result = default_extractor(&req).await;
        assert_eq!(result, "default:root@example.com:default/_search");
    }

    #[tokio::test]
    async fn test_cache_behavior() {
        *RATELIMIT_RULES_CACHE.write().await = HashMap::new();

        let result = exact_match("test_org", "test_org:user@example.com:/api/test").await;
        assert!(result.is_none());

        let result = regex_match("test_org", "test_org:user@example.com:/api/test/123").await;
        assert!(result.is_none());

        init_test_cache().await;

        let result = exact_match("test_org", "test_org:user@example.com:/api/test").await;
        assert!(result.is_some());
    }
}
