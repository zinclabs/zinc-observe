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
    db::{
        sqlite::{CLIENT_RO, CLIENT_RW},
        Event,
    },
    errors::{DbError, Error, Result},
};

pub struct SqlitePipelineTable {}

impl SqlitePipelineTable {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SqlitePipelineTable {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::PipelineTable for SqlitePipelineTable {
    async fn create_table(&self) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS pipeline
(
    id           VARCHAR(256) NOT NULL PRIMARY KEY,
    version      INT NOT NULL,
    name         VARCHAR(256) NOT NULL,
    description  TEXT,
    org          VARCHAR(100) NOT NULL,
    source_type  VARCHAR(50) NOT NULL,
    stream_org   VARCHAR(100),
    stream_name  VARCHAR(256),
    stream_type  VARCHAR(50),
    query_inner  TEXT,
    nodes        TEXT,
    edges        TEXT,
    created_at   TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
);
            "#,
        )
        .execute(&*client)
        .await?;
        Ok(())
    }

    async fn create_table_index(&self) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let queries = vec![
            "CREATE INDEX IF NOT EXISTS pipeline_org_idx ON pipeline (org);",
            "CREATE INDEX IF NOT EXISTS pipeline_id_idx ON pipeline (id);",
            "CREATE UNIQUE INDEX IF NOT EXISTS pipeline_org_src_type_stream_params_idx ON pipeline (org, source_type, stream_org, stream_name, stream_type);",
        ];

        for query in queries {
            sqlx::query(query).execute(&*client).await?;
        }
        Ok(())
    }

    async fn put(&self, pipeline: Pipeline) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let mut tx = client.begin().await?;

        if let Err(e) = match pipeline.source {
            PipelineSource::Stream(stream_params) => {
                let (source_type, stream_org, stream_name, stream_type): (&str, &str, &str, &str) = (
                    "stream",
                    stream_params.org_id.as_str(),
                    stream_params.stream_name.as_str(),
                    stream_params.stream_type.as_str(),
                );
                sqlx::query(
                    r#"
INSERT INTO pipeline (id, version, name, description, org, source_type, stream_org, stream_name, stream_type, nodes, edges)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    ON CONFLICT DO NOTHING;
                    "#,
                )
                .bind(pipeline.id)
                .bind(pipeline.version)
                .bind(pipeline.name)
                .bind(pipeline.description)
                .bind(pipeline.org)
                .bind(source_type)
                .bind(stream_org)
                .bind(stream_name)
                .bind(stream_type)
                .bind(json::to_string(&pipeline.nodes).expect("Serializing pipeline nodes error"))
                .bind(json::to_string(&pipeline.edges).expect("Serializing pipeline edges error"))
                .execute(&mut *tx)
                .await
            }
            PipelineSource::Query(query_inner) => {
                let (source_type, query_inner) = (
                    "query",
                    json::to_string(&query_inner).expect("Serializing pipeline QueryInner error"),
                );
                sqlx::query(
                    r#"
INSERT INTO pipeline (id, version, name, description, org, source_type, query_inner, nodes, edges)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
    ON CONFLICT DO NOTHING;
                    "#,
                )
                .bind(pipeline.id)
                .bind(pipeline.version)
                .bind(pipeline.name)
                .bind(pipeline.description)
                .bind(pipeline.org)
                .bind(source_type)
                .bind(query_inner)
                .bind(json::to_string(&pipeline.nodes).expect("Serializing pipeline nodes error"))
                .bind(json::to_string(&pipeline.edges).expect("Serializing pipeline edges error"))
                .execute(&mut *tx)
                .await
            }
        } {
            if let Err(e) = tx.rollback().await {
                log::error!("[SQLITE] rollback push pipeline error: {}", e);
            }
            return Err(e.into());
        }

        if let Err(e) = tx.commit().await {
            log::error!("[SQLITE] commit push pipeline error: {}", e);
            return Err(e.into());
        }

        // release lock
        drop(client);
        Ok(())
    }

    async fn get_by_stream(
        &self,
        org: &str,
        stream_params: &StreamParams,
    ) -> Result<Vec<Pipeline>> {
        let pool = CLIENT_RO.clone();
        let query = r#"
SELECT * FROM pipeline WHERE org = $1 AND source_type = $2 AND stream_org = $3 AND stream_name = $4 AND stream_type = $5;
        "#;
        let pipelines = match sqlx::query_as::<_, Pipeline>(query)
            .bind(org)
            .bind("stream")
            .bind(stream_params.org_id.as_str())
            .bind(stream_params.stream_name.as_str())
            .bind(stream_params.stream_type.as_str())
            .fetch_all(&pool)
            .await
        {
            Ok(pipelines) => pipelines,
            Err(e) => {
                log::error!("[SQLITE] get pipeline by stream error: {}", e);
                return Err(Error::from(DbError::KeyNotExists(format!(
                    "{org}/{stream_params}",
                ))));
            }
        };
        Ok(pipelines)
    }

    async fn get_by_id(&self, pipeline_id: &str) -> Result<Pipeline> {
        let pool = CLIENT_RO.clone();
        let query = "SELECT * FROM pipeline WHERE id = $1;";
        let pipeline = match sqlx::query_as::<_, Pipeline>(query)
            .bind(pipeline_id)
            .fetch_one(&pool)
            .await
        {
            Ok(pipeline) => pipeline,
            Err(e) => {
                log::error!("[SQLITE] get pipeline by id error: {}", e);
                return Err(Error::from(DbError::KeyNotExists(pipeline_id.to_string())));
            }
        };
        Ok(pipeline)
    }

    async fn list(&self) -> Result<Vec<Pipeline>> {
        let client = CLIENT_RO.clone();
        let query = "SELECT * FROM pipeline ORDER BY id;";
        let pipelines = match sqlx::query_as::<_, Pipeline>(query)
            .fetch_all(&client)
            .await
        {
            Ok(pipelines) => pipelines,
            Err(e) => {
                log::debug!("[SQLITE] list all pipelines error: {}", e);
                return Err(Error::from(DbError::KeyNotExists("".to_string())));
            }
        };
        Ok(pipelines)
    }

    async fn list_by_org(&self, org: &str) -> Result<Vec<Pipeline>> {
        let client = CLIENT_RO.clone();
        let query = "SELECT * FROM pipeline WHERE org = $1 ORDER BY id;";
        let pipelines = match sqlx::query_as::<_, Pipeline>(query)
            .bind(org)
            .fetch_all(&client)
            .await
        {
            Ok(pipelines) => pipelines,
            Err(e) => {
                log::debug!("[SQLITE] list pipeline by org error: {}", e);
                return Err(Error::from(DbError::KeyNotExists(org.to_string())));
            }
        };
        Ok(pipelines)
    }

    async fn delete(&self, pipeline_id: &str) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        sqlx::query(r#"DELETE FROM pipeline WHERE id = $2;"#)
            .bind(pipeline_id)
            .execute(&*client)
            .await?;

        drop(client);
        Ok(())
    }

    async fn watch(&self, _prefix: &str) -> Result<Arc<mpsc::Receiver<Event>>> {
        todo!("taiming")
    }
}
