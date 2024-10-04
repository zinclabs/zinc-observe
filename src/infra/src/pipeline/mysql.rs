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

use std::sync::Arc;

use async_trait::async_trait;
use config::{
    meta::{
        pipeline::{components::PipelineSource, Pipeline},
        stream::StreamParams,
    },
    utils::json,
};
use tokio::sync::mpsc;

use crate::{
    db::{mysql::CLIENT, Event},
    errors::{DbError, Error, Result},
};

pub struct MySqlPipelineTable {}

impl MySqlPipelineTable {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for MySqlPipelineTable {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::PipelineTable for MySqlPipelineTable {
    async fn create_table(&self) -> Result<()> {
        let pool = CLIENT.clone();
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS pipeline
(
    id              VARCHAR(256) not null primary key,
    version         INT not null,
    enabled         BOOLEAN default true not null,
    name            VARCHAR(256) not null,
    description     TEXT,
    org             VARCHAR(100) not null,
    source_type     VARCHAR(50) not null,
    stream_org      VARCHAR(100),
    stream_name     VARCHAR(256),
    stream_type     VARCHAR(50),
    derived_stream  TEXT,
    nodes           TEXT,
    edges           TEXT,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
);
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(())
    }

    async fn create_table_index(&self) -> Result<()> {
        let pool = CLIENT.clone();

        let queries = vec![
            "CREATE INDEX pipeline_org_idx ON pipeline (org);",
            "CREATE INDEX pipeline_id_idx ON pipeline (id);",
            "CREATE UNIQUE INDEX pipeline_org_src_type_stream_params_idx ON pipeline (org, source_type, stream_org, stream_name, stream_type);",
        ];

        for query in queries {
            if let Err(e) = sqlx::query(query).execute(&pool).await {
                if e.to_string().contains("Duplicate key") {
                    // index already exists
                    return Ok(());
                }
                log::error!("[MYSQL] create index for pipeline table error: {}", e);
                return Err(e.into());
            }
        }
        Ok(())
    }

    async fn put(&self, pipeline: &Pipeline) -> Result<()> {
        let pool = CLIENT.clone();
        let mut tx = pool.begin().await?;

        if let Err(e) = match &pipeline.source {
            PipelineSource::Realtime(stream_params) => {
                let (source_type, stream_org, stream_name, stream_type): (&str, &str, &str, &str) = (
                    "realtime",
                    stream_params.org_id.as_str(),
                    stream_params.stream_name.as_str(),
                    stream_params.stream_type.as_str(),
                );
                sqlx::query(
                    r#"
INSERT IGNORE INTO pipeline (id, version, enabled, name, description, org, source_type, stream_org, stream_name, stream_type, nodes, edges)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
                    "#,
                )
                .bind(&pipeline.id)
                .bind(pipeline.version)
                .bind(pipeline.enabled)
                .bind(&pipeline.name)
                .bind(&pipeline.description)
                .bind(&pipeline.org)
                .bind(source_type)
                .bind(stream_org)
                .bind(stream_name)
                .bind(stream_type)
                .bind(json::to_string(&pipeline.nodes).expect("Serializing pipeline nodes error"))
                .bind(json::to_string(&pipeline.edges).expect("Serializing pipeline edges error"))
                .execute(&mut *tx)
                .await
            }
            PipelineSource::Scheduled(derived_stream) => {
                let (source_type, derived_stream_str) = (
                    "scheduled",
                    json::to_string(&derived_stream)
                        .expect("Serializing pipeline DerivedStream error"),
                );
                sqlx::query(
                    r#"
INSERT IGNORE INTO pipeline (id, version, enabled, name, description, org, source_type, derived_stream, nodes, edges)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
                    "#,
                )
                .bind(&pipeline.id)
                .bind(pipeline.version)
                .bind(pipeline.enabled)
                .bind(&pipeline.name)
                .bind(&pipeline.description)
                .bind(&pipeline.org)
                .bind(source_type)
                .bind(derived_stream_str)
                .bind(json::to_string(&pipeline.nodes).expect("Serializing pipeline nodes error"))
                .bind(json::to_string(&pipeline.edges).expect("Serializing pipeline edges error"))
                .execute(&mut *tx)
                .await
            }
        } {
            if let Err(e) = tx.rollback().await {
                log::error!("[MYSQL] rollback push pipeline error: {}", e);
            }
            return Err(e.into());
        }

        if let Err(e) = tx.commit().await {
            log::error!("[MYSQL] commit push pipeline error: {}", e);
            return Err(e.into());
        }

        Ok(())
    }

    async fn update(&self, pipeline: Pipeline) -> Result<()> {
        let pool = CLIENT.clone();
        let mut tx = pool.begin().await?;

        if let Err(e) = match pipeline.source {
            PipelineSource::Realtime(stream_params) => {
                let (source_type, stream_org, stream_name, stream_type): (&str, &str, &str, &str) = (
                    "realtime",
                    stream_params.org_id.as_str(),
                    stream_params.stream_name.as_str(),
                    stream_params.stream_type.as_str(),
                );
                sqlx::query(
                    r#"
UPDATE pipeline
    SET version = ?, enabled = ?, name = ?, description = ?, org = ?, source_type = ?, stream_org = ?, stream_name = ?, stream_type = ?, nodes = ?, edges = ?
    WHERE id =?;
                    "#,
                )
                .bind(pipeline.version)
                .bind(pipeline.enabled)
                .bind(pipeline.name)
                .bind(pipeline.description)
                .bind(pipeline.org)
                .bind(source_type)
                .bind(stream_org)
                .bind(stream_name)
                .bind(stream_type)
                .bind(json::to_string(&pipeline.nodes).expect("Serializing pipeline nodes error"))
                .bind(json::to_string(&pipeline.edges).expect("Serializing pipeline edges error"))
                .bind(pipeline.id)
                .execute(&mut *tx)
                .await
            }
            PipelineSource::Scheduled(derived_stream) => {
                let (source_type, derived_stream_str) = (
                    "scheduled",
                    json::to_string(&derived_stream)
                        .expect("Serializing pipeline DerivedStream error"),
                );
                sqlx::query(
                    r#"
UPDATE pipeline
    SET version = ?, enabled = ?, name = ?, description = ?, org = ?, source_type = ?, derived_stream = ?, nodes = ?, edges = ?
    WHERE id = ?;
                    "#,
                )
                .bind(pipeline.version)
                .bind(pipeline.enabled)
                .bind(pipeline.name)
                .bind(pipeline.description)
                .bind(pipeline.org)
                .bind(source_type)
                .bind(derived_stream_str)
                .bind(json::to_string(&pipeline.nodes).expect("Serializing pipeline nodes error"))
                .bind(json::to_string(&pipeline.edges).expect("Serializing pipeline edges error"))
                .bind(pipeline.id)
                .execute(&mut *tx)
                .await
            }
        } {
            if let Err(e) = tx.rollback().await {
                log::error!("[MYSQL] rollback push pipeline error: {}", e);
            }
            return Err(e.into());
        }

        if let Err(e) = tx.commit().await {
            log::error!("[MYSQL] commit push pipeline error: {}", e);
            return Err(e.into());
        }

        Ok(())
    }

    async fn get_by_stream(&self, org: &str, stream_params: &StreamParams) -> Result<Pipeline> {
        let pool = CLIENT.clone();
        let query = r#"
SELECT * FROM pipeline WHERE org = ? AND source_type = ? AND stream_org = ? AND stream_name = ? AND stream_type = ?;
        "#;
        let pipeline = match sqlx::query_as::<_, Pipeline>(query)
            .bind(org)
            .bind("realtime")
            .bind(stream_params.org_id.as_str())
            .bind(stream_params.stream_name.as_str())
            .bind(stream_params.stream_type.as_str())
            .fetch_one(&pool)
            .await
        {
            Ok(pipeline) => pipeline,
            Err(e) => {
                log::debug!("[MYSQL] get pipeline by stream error: {}", e);
                return Err(Error::from(DbError::KeyNotExists(format!(
                    "{org}/{stream_params}",
                ))));
            }
        };
        Ok(pipeline)
    }

    async fn get_by_id(&self, pipeline_id: &str) -> Result<Pipeline> {
        let pool = CLIENT.clone();
        let query = r#"SELECT * FROM pipeline WHERE id = ?;"#;
        let pipeline = match sqlx::query_as::<_, Pipeline>(query)
            .bind(pipeline_id)
            .fetch_one(&pool)
            .await
        {
            Ok(pipeline) => pipeline,
            Err(e) => {
                log::debug!("[MYSQL] get pipeline by id error: {}", e);
                return Err(Error::from(DbError::KeyNotExists(pipeline_id.to_string())));
            }
        };
        Ok(pipeline)
    }

    async fn get_with_same_source_stream(&self, pipeline: &Pipeline) -> Result<Pipeline> {
        let pool = CLIENT.clone();
        let similar_pipeline = match &pipeline.source {
            PipelineSource::Realtime(stream_params) => {
                sqlx::query_as::<_, Pipeline>(
                    r#"
SELECT * FROM pipeline 
    WHERE source_type = ? AND stream_org = ? AND stream_name = ? AND stream_type = ?;
                "#,
                )
                .bind("realtime")
                .bind(stream_params.org_id.as_str())
                .bind(stream_params.stream_name.as_str())
                .bind(stream_params.stream_type.as_str())
                .fetch_one(&pool)
                .await?
            }
            PipelineSource::Scheduled(_) => {
                // only checks for realtime pipelines
                return Err(Error::from(DbError::KeyNotExists("".to_string())));
            }
        };

        Ok(similar_pipeline)
    }

    async fn list(&self) -> Result<Vec<Pipeline>> {
        let pool = CLIENT.clone();
        let query = r#"SELECT * FROM pipeline ORDER BY id;"#;
        let pipelines = match sqlx::query_as::<_, Pipeline>(query).fetch_all(&pool).await {
            Ok(pipelines) => pipelines,
            Err(e) => {
                log::debug!("[MYSQL] list all pipelines error: {}", e);
                return Err(Error::from(DbError::KeyNotExists("".to_string())));
            }
        };
        Ok(pipelines)
    }

    async fn list_by_org(&self, org: &str) -> Result<Vec<Pipeline>> {
        let pool = CLIENT.clone();
        let query = r#"SELECT * FROM pipeline WHERE org = ? ORDER BY id;"#;
        let pipelines = match sqlx::query_as::<_, Pipeline>(query)
            .bind(org)
            .fetch_all(&pool)
            .await
        {
            Ok(pipelines) => pipelines,
            Err(e) => {
                log::debug!("[MYSQL] list pipelines by org error: {}", e);
                return Err(Error::from(DbError::KeyNotExists(org.to_string())));
            }
        };
        Ok(pipelines)
    }

    async fn list_streams_with_pipeline(&self, org: &str) -> Result<Vec<Pipeline>> {
        let pool = CLIENT.clone();
        let query = r#"
SELECT * FROM pipeline WHERE org = ? AND source_type = ? ORDER BY id;
        "#;
        let pipelines = match sqlx::query_as::<_, Pipeline>(query)
            .bind(org)
            .bind("realtime")
            .fetch_all(&pool)
            .await
        {
            Ok(pipelines) => pipelines,
            Err(e) => {
                log::debug!("[MYSQL] list streams with pipelines error: {}", e);
                return Err(Error::from(DbError::KeyNotExists(format!(
                    "{org}/realtime"
                ))));
            }
        };
        Ok(pipelines)
    }

    async fn delete(&self, pipeline_id: &str) -> Result<()> {
        let pool = CLIENT.clone();
        sqlx::query(r#"DELETE FROM pipeline WHERE id = ?;"#)
            .bind(pipeline_id)
            .execute(&pool)
            .await?;
        Ok(())
    }

    async fn watch(&self, _prefix: &str) -> Result<Arc<mpsc::Receiver<Event>>> {
        todo!("taiming")
    }
}
