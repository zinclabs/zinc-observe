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

use std::{str::FromStr, sync::Arc};

use arrow_schema::Field;
use config::{
    get_config,
    meta::{
        search::{Session as SearchSession, StorageType},
        stream::{FileKey, FileMeta, StreamType},
    },
    utils::{parquet::new_parquet_writer, schema_ext::SchemaExt},
    PARQUET_BATCH_SIZE,
};
use datafusion::{
    arrow::{
        datatypes::{DataType, Schema},
        record_batch::RecordBatch,
    },
    catalog::TableProvider,
    common::Column,
    datasource::{
        file_format::{json::JsonFormat, parquet::ParquetFormat},
        listing::{ListingOptions, ListingTableConfig, ListingTableUrl},
        object_store::{DefaultObjectStoreRegistry, ObjectStoreRegistry},
    },
    error::{DataFusionError, Result},
    execution::{
        cache::cache_manager::{CacheManagerConfig, FileStatisticsCache},
        context::SessionConfig,
        memory_pool::{FairSpillPool, GreedyMemoryPool},
        runtime_env::{RuntimeConfig, RuntimeEnv},
        session_state::SessionStateBuilder,
    },
    logical_expr::AggregateUDF,
    optimizer::OptimizerRule,
    prelude::{Expr, SessionContext},
};
use hashbrown::HashMap;
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::{
    common::infra::config::get_config as get_o2_config, search::WorkGroup,
};

use super::{
    file_type::{FileType, GetExt},
    optimizer::join_reorder::JoinReorderRule,
    storage::file_list,
    table_provider::NewListingTable,
    udf::transform_udf::get_all_transform,
};

const DATAFUSION_MIN_MEM: usize = 1024 * 1024 * 256; // 256MB
const DATAFUSION_MIN_PARTITION: usize = 2; // CPU cores

pub async fn convert_parquet_file(
    trace_id: &str,
    buf: &mut Vec<u8>,
    schema: Arc<Schema>,
    bloom_filter_fields: &[String],
    rules: HashMap<String, DataType>,
    file_type: FileType,
) -> Result<()> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    let query_sql = format!(
        "SELECT * FROM tbl ORDER BY {} DESC",
        cfg.common.column_timestamp
    ); //  select_wildcard -> without_optimizer

    // query data
    let ctx = prepare_datafusion_context(None, vec![], false, 0).await?;

    // Configure listing options
    let listing_options = match file_type {
        FileType::PARQUET => {
            let file_format = ParquetFormat::default();
            ListingOptions::new(Arc::new(file_format))
                .with_file_extension(FileType::PARQUET.get_ext())
                .with_target_partitions(ctx.state().config().target_partitions())
        }
        FileType::JSON => {
            let file_format = JsonFormat::default();
            ListingOptions::new(Arc::new(file_format))
                .with_file_extension(FileType::JSON.get_ext())
                .with_target_partitions(ctx.state().config().target_partitions())
        }
        _ => {
            return Err(DataFusionError::Execution(format!(
                "Unsupported file type scheme {file_type:?}",
            )));
        }
    };

    let prefix = match ListingTableUrl::parse(format!("tmpfs:///{trace_id}/")) {
        Ok(url) => url,
        Err(e) => {
            return Err(datafusion::error::DataFusionError::Execution(format!(
                "ListingTableUrl error: {e}"
            )));
        }
    };

    let config = ListingTableConfig::new(prefix)
        .with_listing_options(listing_options)
        .with_schema(schema.clone());

    let table = NewListingTable::try_new(config, rules)?;
    ctx.register_table("tbl", Arc::new(table))?;

    // get all sorted data
    let df = match ctx.sql(&query_sql).await {
        Ok(df) => df,
        Err(e) => {
            log::error!(
                "convert sql execute failed, sql: {}, err: {:?}",
                query_sql,
                e
            );
            return Err(e);
        }
    };
    let schema: Schema = df.schema().into();
    let schema = Arc::new(schema);
    let batches = df.collect().await?;
    let file_meta = FileMeta::default();
    let mut writer = new_parquet_writer(buf, &schema, bloom_filter_fields, &file_meta);
    for batch in batches {
        writer.write(&batch).await?;
    }
    writer.close().await?;
    ctx.deregister_table("tbl")?;
    drop(ctx);

    log::info!(
        "convert_parquet_file took {} ms",
        start.elapsed().as_millis()
    );

    Ok(())
}

pub async fn merge_parquet_files(
    trace_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    schema: Arc<Schema>,
) -> Result<(Arc<Schema>, Vec<RecordBatch>)> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    // get all sorted data
    let query_sql = if stream_type == StreamType::Index {
        format!(
            "SELECT * FROM tbl WHERE file_name NOT IN (SELECT file_name FROM tbl WHERE deleted IS TRUE) ORDER BY {} DESC",
            cfg.common.column_timestamp
        )
    } else if cfg.limit.distinct_values_hourly
        && stream_type == StreamType::Metadata
        && stream_name == "distinct_values"
    {
        format!(
            "SELECT MIN({}) AS {}, SUM(count) as count, field_name, field_value, filter_name, filter_value, stream_name, stream_type FROM tbl GROUP BY field_name, field_value, filter_name, filter_value, stream_name, stream_type ORDER BY {} DESC",
            cfg.common.column_timestamp, cfg.common.column_timestamp, cfg.common.column_timestamp
        )
    } else {
        format!(
            "SELECT * FROM tbl ORDER BY {} DESC",
            cfg.common.column_timestamp
        )
    };

    // create datafusion context
    let ctx = prepare_datafusion_context(None, vec![], false, 0).await?;

    // Configure listing options
    let file_format = ParquetFormat::default();
    let listing_options = ListingOptions::new(Arc::new(file_format))
        .with_file_extension(FileType::PARQUET.get_ext())
        .with_target_partitions(ctx.state().config().target_partitions());
    let prefix = ListingTableUrl::parse(format!("tmpfs:///{trace_id}/"))?;
    let config = ListingTableConfig::new(prefix)
        .with_listing_options(listing_options)
        .with_schema(schema.clone());
    let table = Arc::new(NewListingTable::try_new(config, HashMap::default())?);
    ctx.register_table("tbl", table.clone())?;

    let df = ctx.sql(&query_sql).await?;
    let schema: Schema = df.schema().into();
    let schema = Arc::new(schema);
    let batches = df.collect().await?;

    ctx.deregister_table("tbl")?;
    drop(ctx);

    log::info!(
        "merge_parquet_files took {} ms",
        start.elapsed().as_millis()
    );

    Ok((schema, batches))
}

pub fn create_session_config(
    sorted_by_time: bool,
    target_partitions: usize,
) -> Result<SessionConfig> {
    let cfg = get_config();
    let mut target_partitions = if target_partitions == 0 {
        cfg.limit.cpu_num
    } else {
        std::cmp::max(cfg.limit.datafusion_min_partition_num, target_partitions)
    };
    if target_partitions == 0 {
        target_partitions = DATAFUSION_MIN_PARTITION;
    }
    let mut config = SessionConfig::from_env()?
        .with_batch_size(PARQUET_BATCH_SIZE)
        .with_target_partitions(target_partitions)
        .with_information_schema(true);
    config
        .options_mut()
        .execution
        .listing_table_ignore_subdirectory = false;
    config.options_mut().sql_parser.dialect = "PostgreSQL".to_string();

    // based on data distributing, it only works for the data on a few records
    // config = config.set_bool("datafusion.execution.parquet.pushdown_filters", true);
    // config = config.set_bool("datafusion.execution.parquet.reorder_filters", true);

    if cfg.common.bloom_filter_enabled {
        config = config.set_bool("datafusion.execution.parquet.bloom_filter_on_read", true);
    }
    if cfg.common.bloom_filter_disabled_on_search {
        config = config.set_bool("datafusion.execution.parquet.bloom_filter_on_read", false);
    }
    if sorted_by_time {
        config = config.set_bool("datafusion.execution.split_file_groups_by_statistics", true);
    }
    Ok(config)
}

pub async fn create_runtime_env(memory_limit: usize) -> Result<RuntimeEnv> {
    let object_store_registry = DefaultObjectStoreRegistry::new();

    let memory = super::storage::memory::FS::new();
    let memory_url = url::Url::parse("memory:///").unwrap();
    object_store_registry.register_store(&memory_url, Arc::new(memory));

    let wal = super::storage::wal::FS::new();
    let wal_url = url::Url::parse("wal:///").unwrap();
    object_store_registry.register_store(&wal_url, Arc::new(wal));

    let tmpfs = super::storage::tmpfs::Tmpfs::new();
    let tmpfs_url = url::Url::parse("tmpfs:///").unwrap();
    object_store_registry.register_store(&tmpfs_url, Arc::new(tmpfs));

    let cfg = get_config();
    let mut rn_config =
        RuntimeConfig::new().with_object_store_registry(Arc::new(object_store_registry));
    if cfg.limit.datafusion_file_stat_cache_max_entries > 0 {
        let cache_config = CacheManagerConfig::default();
        let cache_config = cache_config.with_files_statistics_cache(Some(
            super::storage::file_statistics_cache::GLOBAL_CACHE.clone(),
        ));
        rn_config = rn_config.with_cache_manager(cache_config);
    }

    let memory_size = std::cmp::max(DATAFUSION_MIN_MEM, memory_limit);
    let mem_pool = super::MemoryPoolType::from_str(&cfg.memory_cache.datafusion_memory_pool)
        .map_err(|e| {
            DataFusionError::Execution(format!("Invalid datafusion memory pool type: {}", e))
        })?;
    match mem_pool {
        super::MemoryPoolType::Greedy => {
            rn_config = rn_config.with_memory_pool(Arc::new(GreedyMemoryPool::new(memory_size)))
        }
        super::MemoryPoolType::Fair => {
            rn_config = rn_config.with_memory_pool(Arc::new(FairSpillPool::new(memory_size)))
        }
        super::MemoryPoolType::None => {}
    };
    RuntimeEnv::new(rn_config)
}

pub async fn prepare_datafusion_context(
    _work_group: Option<String>,
    optimizer_rules: Vec<Arc<dyn OptimizerRule + Send + Sync>>,
    sorted_by_time: bool,
    target_partitions: usize,
) -> Result<SessionContext, DataFusionError> {
    let cfg = get_config();
    #[cfg(not(feature = "enterprise"))]
    let (memory_size, target_partition) = (cfg.memory_cache.datafusion_max_size, target_partitions);
    #[cfg(feature = "enterprise")]
    let (mut memory_size, mut target_partition) =
        (cfg.memory_cache.datafusion_max_size, target_partitions);
    #[cfg(feature = "enterprise")]
    if let Some(wg) = _work_group {
        if let Ok(wg) = WorkGroup::from_str(&wg) {
            let (cpu, mem) = wg.get_dynamic_resource().await.map_err(|e| {
                DataFusionError::Execution(format!("Failed to get dynamic resource: {}", e))
            })?;
            if get_o2_config().search_group.cpu_limit_enabled {
                target_partition = target_partition * cpu as usize / 100;
            }
            memory_size = memory_size * mem as usize / 100;
            log::debug!(
                "[datafusion:{}] target_partition: {}, memory_size: {}",
                wg,
                target_partition,
                memory_size
            );
        }
    }

    let session_config = create_session_config(sorted_by_time, target_partition)?;
    let runtime_env = Arc::new(create_runtime_env(memory_size).await?);
    if !optimizer_rules.is_empty() {
        let state = SessionStateBuilder::new()
            .with_config(session_config)
            .with_runtime_env(runtime_env)
            .with_default_features()
            .with_optimizer_rules(optimizer_rules)
            .with_physical_optimizer_rule(Arc::new(JoinReorderRule::new()))
            .build();
        Ok(SessionContext::new_with_state(state))
    } else {
        Ok(SessionContext::new_with_config_rt(
            session_config,
            runtime_env,
        ))
    }
}

pub fn register_udf(ctx: &SessionContext, org_id: &str) -> Result<()> {
    ctx.register_udf(super::udf::match_udf::MATCH_UDF.clone());
    ctx.register_udf(super::udf::match_udf::MATCH_IGNORE_CASE_UDF.clone());
    ctx.register_udf(super::udf::regexp_udf::REGEX_MATCH_UDF.clone());
    ctx.register_udf(super::udf::regexp_udf::REGEX_NOT_MATCH_UDF.clone());
    ctx.register_udf(super::udf::regexp_udf::REGEXP_MATCH_TO_FIELDS_UDF.clone());
    ctx.register_udf(super::udf::time_range_udf::TIME_RANGE_UDF.clone());
    ctx.register_udf(super::udf::date_format_udf::DATE_FORMAT_UDF.clone());
    ctx.register_udf(super::udf::string_to_array_v2_udf::STRING_TO_ARRAY_V2_UDF.clone());
    ctx.register_udf(super::udf::arrzip_udf::ARR_ZIP_UDF.clone());
    ctx.register_udf(super::udf::arrindex_udf::ARR_INDEX_UDF.clone());
    ctx.register_udf(super::udf::arr_descending_udf::ARR_DESCENDING_UDF.clone());
    ctx.register_udf(super::udf::arrjoin_udf::ARR_JOIN_UDF.clone());
    ctx.register_udf(super::udf::arrcount_udf::ARR_COUNT_UDF.clone());
    ctx.register_udf(super::udf::arrsort_udf::ARR_SORT_UDF.clone());
    ctx.register_udf(super::udf::cast_to_arr_udf::CAST_TO_ARR_UDF.clone());
    ctx.register_udf(super::udf::spath_udf::SPATH_UDF.clone());
    ctx.register_udf(super::udf::to_arr_string_udf::TO_ARR_STRING.clone());
    ctx.register_udf(super::udf::histogram_udf::HISTOGRAM_UDF.clone());
    ctx.register_udf(super::udf::match_all_udf::MATCH_ALL_RAW_UDF.clone());
    ctx.register_udf(super::udf::match_all_udf::MATCH_ALL_RAW_IGNORE_CASE_UDF.clone());
    ctx.register_udf(super::udf::match_all_udf::MATCH_ALL_UDF.clone());
    ctx.register_udaf(AggregateUDF::from(
        super::udaf::percentile_cont::PercentileCont::new(),
    ));

    let udf_list = get_all_transform(org_id)?;
    for udf in udf_list {
        ctx.register_udf(udf.clone());
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn register_table(
    session: &SearchSession,
    schema: Arc<Schema>,
    table_name: &str,
    files: &[FileKey],
    rules: HashMap<String, DataType>,
    sort_key: &[(String, bool)],
) -> Result<SessionContext> {
    let cfg = get_config();
    // only sort by timestamp desc
    let sorted_by_time =
        sort_key.len() == 1 && sort_key[0].0 == cfg.common.column_timestamp && sort_key[0].1;

    let ctx = prepare_datafusion_context(
        session.work_group.clone(),
        vec![],
        sorted_by_time,
        session.target_partitions,
    )
    .await?;

    let table = create_parquet_table(
        session,
        schema.clone(),
        files,
        rules.clone(),
        sorted_by_time,
        ctx.runtime_env().cache_manager.get_file_statistic_cache(),
    )
    .await?;
    ctx.register_table(table_name, table)?;

    Ok(ctx)
}

pub async fn create_parquet_table(
    session: &SearchSession,
    schema: Arc<Schema>,
    files: &[FileKey],
    rules: HashMap<String, DataType>,
    sorted_by_time: bool,
    file_stat_cache: Option<FileStatisticsCache>,
) -> Result<Arc<dyn TableProvider>> {
    let cfg = get_config();
    let mut target_partitions = if session.target_partitions == 0 {
        cfg.limit.cpu_num
    } else {
        std::cmp::max(
            cfg.limit.datafusion_min_partition_num,
            session.target_partitions,
        )
    };
    if target_partitions == 0 {
        target_partitions = DATAFUSION_MIN_PARTITION;
    }

    // Configure listing options
    let file_format = ParquetFormat::default();
    let mut listing_options = ListingOptions::new(Arc::new(file_format))
        .with_file_extension(FileType::PARQUET.get_ext())
        .with_target_partitions(target_partitions)
        .with_collect_stat(true);

    if sorted_by_time {
        // specify sort columns for parquet file
        listing_options =
            listing_options.with_file_sort_order(vec![vec![datafusion::logical_expr::SortExpr {
                expr: Expr::Column(Column::new_unqualified(cfg.common.column_timestamp.clone())),
                asc: false,
                nulls_first: false,
            }]]);
    }

    let schema_key = schema.hash_key();
    let prefix = if session.storage_type == StorageType::Memory {
        file_list::set(&session.id, &schema_key, files).await;
        format!("memory:///{}/schema={}/", session.id, schema_key)
    } else if session.storage_type == StorageType::Wal {
        file_list::set(&session.id, &schema_key, files).await;
        format!("wal:///{}/schema={}/", session.id, schema_key)
    } else if session.storage_type == StorageType::Tmpfs {
        format!("tmpfs:///{}/", session.id)
    } else {
        return Err(DataFusionError::Execution(format!(
            "Unsupported storage_type {:?}",
            session.storage_type,
        )));
    };
    let prefix = match ListingTableUrl::parse(prefix) {
        Ok(url) => url,
        Err(e) => {
            return Err(datafusion::error::DataFusionError::Execution(format!(
                "ListingTableUrl error: {e}",
            )));
        }
    };

    let mut config = ListingTableConfig::new(prefix).with_listing_options(listing_options);
    let timestamp_field = schema.field_with_name(&cfg.common.column_timestamp);
    let schema = if timestamp_field.is_ok() && timestamp_field.unwrap().is_nullable() {
        let new_fields = schema
            .fields()
            .iter()
            .map(|x| {
                if x.name() == &cfg.common.column_timestamp {
                    Arc::new(Field::new(
                        cfg.common.column_timestamp.clone(),
                        DataType::Int64,
                        false,
                    ))
                } else {
                    x.clone()
                }
            })
            .collect::<Vec<_>>();
        Arc::new(Schema::new(new_fields))
    } else {
        schema
    };
    config = config.with_schema(schema);
    let mut table = NewListingTable::try_new(config, rules)?;
    if session.storage_type != StorageType::Tmpfs && file_stat_cache.is_some() {
        table = table.with_cache(file_stat_cache);
    }
    Ok(Arc::new(table))
}
