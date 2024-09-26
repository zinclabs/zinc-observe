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

use chrono::Utc;
use config::{
    get_config,
    utils::json::{Map, Value},
};
use cron::Schedule;
use hashbrown::HashMap;

use crate::{
    common::{
        meta::{
            // authz::Authz,
            alerts::{derived_streams::DerivedStreamMeta, FrequencyType, QueryType},
        },
        // utils::auth::{remove_ownership, set_ownership},
    },
    service::db,
};

pub async fn save(
    mut derived_stream: DerivedStreamMeta,
    pipeline_name: &str,
) -> Result<(), anyhow::Error> {
    derived_stream.name = derived_stream.name.trim().to_string();
    // 1. Start validate DerivedStream
    if !derived_stream.is_valid() {
        return Err(anyhow::anyhow!(
            "DerivedStream Name, destination, and Trigger period required"
        ));
    }

    // query_type for realtime
    if derived_stream.is_real_time && derived_stream.query_condition.query_type != QueryType::Custom
    {
        return Err(anyhow::anyhow!(
            "Realtime DerivedStream should use Custom query type"
        ));
    }

    // other checks for query type
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
    }

    // check source stream schema
    let schema = infra::schema::get(
        &derived_stream.source.org_id,
        &derived_stream.source.stream_name,
        derived_stream.source.stream_type,
    )
    .await?;
    if schema.fields().is_empty() {
        return Err(anyhow::anyhow!(
            "Source Stream {}/{} schema is empty.",
            derived_stream.source.org_id,
            derived_stream.source.stream_name,
        ));
    }
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

    // 3. clean up DerivedStream context attributes
    if let Some(attrs) = &derived_stream.context_attributes {
        let mut new_attrs = HashMap::with_capacity(attrs.len());
        for (key, val) in attrs.iter() {
            new_attrs.insert(key.trim().to_string(), val.to_string());
        }
        derived_stream.context_attributes = Some(new_attrs);
    }

    // test derived_stream
    if let Err(e) = &derived_stream.evaluate(None, None).await {
        return Err(anyhow::anyhow!(
            "DerivedStream not saved due to failed test run caused by {}",
            e.to_string()
        ));
    };

    // Save the trigger to db
    let next_run_at = Utc::now().timestamp_micros();
    let trigger = db::scheduler::Trigger {
        org: derived_stream.source.org_id.to_string(),
        module: db::scheduler::TriggerModule::DerivedStream,
        module_key: derived_stream.get_scheduler_module_key(pipeline_name),
        next_run_at,
        is_realtime: derived_stream.is_real_time,
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
    derived_stream: DerivedStreamMeta,
    pipeline_name: &str,
) -> Result<(), anyhow::Error> {
    db::scheduler::delete(
        &derived_stream.source.org_id,
        db::scheduler::TriggerModule::DerivedStream,
        &derived_stream.get_scheduler_module_key(pipeline_name),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Error deleting derived stream trigger: {e}"))
}

impl DerivedStreamMeta {
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty()
            && !self.is_real_time // TODO(taiming): support realtime DerivedStream
            && self.source.is_valid()
            && self.destination.is_valid()
            && self.trigger_condition.period != 0
    }

    pub fn get_scheduler_module_key(&self, pipeline_name: &str) -> String {
        format!(
            "{}/{}/{}/{}",
            self.source.stream_type, self.source.stream_name, pipeline_name, self.name
        )
    }

    pub async fn evaluate(
        &self,
        row: Option<&Map<String, Value>>,
        start_time: Option<i64>,
    ) -> Result<(Option<Vec<Map<String, Value>>>, i64), anyhow::Error> {
        if self.is_real_time {
            self.query_condition.evaluate_realtime(row).await
        } else {
            self.query_condition
                .evaluate_scheduled(&self.source, &self.trigger_condition, start_time)
                .await
        }
    }
}
