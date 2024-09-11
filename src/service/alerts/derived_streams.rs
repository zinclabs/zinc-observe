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

use std::str::FromStr;

use async_trait::async_trait;
use chrono::Utc;
use config::{
    get_config,
    meta::{
        alerts::{FrequencyType, QueryType},
        pipeline::components::DerivedStream,
    },
    utils::json::{Map, Value},
};
use cron::Schedule;

use crate::service::{alerts::QueryConditionExt, db};

pub async fn save(
    mut derived_stream: DerivedStream,
    pipeline_name: &str,
    pipeline_id: &str,
) -> Result<(), anyhow::Error> {
    // 1. Start validate DerivedStream
    // checks for query type
    match derived_stream.query_condition.query_type {
        QueryType::SQL => {
            if derived_stream
                .query_condition
                .sql
                .as_ref()
                .map_or(false, |sql| sql.is_empty())
            {
                return Err(anyhow::anyhow!(
                    "DerivedStreams with SQL mode should have a query"
                ));
            }
        }
        QueryType::PromQL => {
            if derived_stream
                .query_condition
                .promql
                .as_ref()
                .map_or(false, |promql| promql.is_empty())
                || derived_stream.query_condition.promql_condition.is_none()
            {
                return Err(anyhow::anyhow!(
                    "DerivedStreams with SQL mode should have a query"
                ));
            }
        }
        _ => {}
    };
    // End input validation

    // 2. update the frequency
    if derived_stream.trigger_condition.frequency_type == FrequencyType::Cron {
        // Check the cron expression
        Schedule::from_str(&derived_stream.trigger_condition.cron)?;
    } else if derived_stream.trigger_condition.frequency == 0 {
        // default 3 mins, set min at 1 minutes
        derived_stream.trigger_condition.frequency =
            std::cmp::max(1, get_config().limit.derived_stream_schedule_interval / 60);
    }

    // test derived_stream
    if let Err(e) = &derived_stream.evaluate(None).await {
        return Err(anyhow::anyhow!(
            "DerivedStream not saved due to failed test run caused by {}",
            e.to_string()
        ));
    };

    // Save the trigger to db
    let next_run_at = Utc::now().timestamp_micros();
    let trigger = db::scheduler::Trigger {
        org: derived_stream.org_id.to_string(),
        module: db::scheduler::TriggerModule::DerivedStream,
        module_key: derived_stream.get_scheduler_module_key(pipeline_name, pipeline_id),
        next_run_at,
        is_realtime: false,
        is_silenced: false,
        ..Default::default()
    };

    match db::scheduler::get(&trigger.org, trigger.module.clone(), &trigger.module_key).await {
        Ok(_) => db::scheduler::update_trigger(trigger)
            .await
            .map_err(|_| anyhow::anyhow!("Trigger already exists, but failed to update")),
        Err(_) => db::scheduler::push(trigger)
            .await
            .map_err(|e| anyhow::anyhow!("Error save DerivedStream trigger: {}", e)),
    }
}

pub async fn delete(
    derived_stream: DerivedStream,
    pipeline_name: &str,
    pipeline_id: &str,
) -> Result<(), anyhow::Error> {
    db::scheduler::delete(
        &derived_stream.org_id,
        db::scheduler::TriggerModule::DerivedStream,
        &derived_stream.get_scheduler_module_key(pipeline_name, pipeline_id),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Error deleting derived stream trigger: {e}"))
}

#[async_trait]
pub trait DerivedStreamExt: Sync + Send + 'static {
    fn get_scheduler_module_key(&self, pipeline_name: &str, pipeline_id: &str) -> String;
    async fn evaluate(
        &self,
        start_time: Option<i64>,
    ) -> Result<(Option<Vec<Map<String, Value>>>, i64), anyhow::Error>;
}

#[async_trait]
impl DerivedStreamExt for DerivedStream {
    fn get_scheduler_module_key(&self, pipeline_name: &str, pipeline_id: &str) -> String {
        format!(
            "{}/{}/{}/{}",
            self.stream_type, self.org_id, pipeline_name, pipeline_id
        )
    }

    async fn evaluate(
        &self,
        start_time: Option<i64>,
    ) -> Result<(Option<Vec<Map<String, Value>>>, i64), anyhow::Error> {
        self.query_condition
            .evaluate_scheduled(
                &self.org_id,
                None,
                self.stream_type,
                &self.trigger_condition,
                &self.query_condition,
                start_time,
            )
            .await
    }
}
