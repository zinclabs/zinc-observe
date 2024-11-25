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

use std::sync::Arc;

use anyhow::Context;
use arrow_schema::Schema;
use config::{
    get_config, is_local_disk_storage,
    meta::{
        bitvec::BitVec,
        inverted_index::InvertedIndexTantivyMode,
        search::{ScanStats, StorageType},
        stream::FileKey,
    },
    utils::{
        file::is_exists, inverted_index::convert_parquet_idx_file_name_to_tantivy_file,
        time::BASE_TIME,
    },
    FILE_EXT_TANTIVY, FILE_EXT_TANTIVY_FOLDER, INDEX_FIELD_NAME_FOR_ALL,
};
use datafusion::execution::cache::cache_manager::FileStatisticsCache;
use futures::future::try_join_all;
use hashbrown::HashMap;
use infra::{
    cache::file_data,
    errors::{Error, ErrorCodes},
};
use itertools::Itertools;
use tantivy::Directory;
use tokio::sync::Semaphore;
use tracing::Instrument;

use crate::service::{
    db, file_list,
    search::{
        datafusion::exec,
        generate_search_schema_diff,
        index::IndexCondition,
        tantivy::puffin_directory::{
            caching_directory::CachingDirectory,
            convert_puffin_file_to_tantivy_dir,
            footer_cache::FooterCache,
            reader::{warm_up_terms, PuffinDirReader},
            reader_cache,
        },
    },
};

/// search in remote object storage
#[tracing::instrument(name = "service:search:grpc:storage", skip_all, fields(org_id = query.org_id, stream_name = query.stream_name))]
#[allow(clippy::too_many_arguments)]
pub async fn search(
    query: Arc<super::QueryParams>,
    schema: Arc<Schema>,
    file_list: &[FileKey],
    sorted_by_time: bool,
    file_stat_cache: Option<FileStatisticsCache>,
    mut index_condition: Option<IndexCondition>,
    mut fst_fields: Vec<String>,
) -> super::SearchTable {
    let enter_span = tracing::span::Span::current();
    log::info!("[trace_id {}] search->storage: enter", query.trace_id);
    // fetch all schema versions, group files by version
    let schema_versions = match infra::schema::get_versions(
        &query.org_id,
        &query.stream_name,
        query.stream_type,
        query.time_range,
    )
    .instrument(enter_span.clone())
    .await
    {
        Ok(versions) => versions,
        Err(err) => {
            log::error!("[trace_id {}] get schema error: {}", query.trace_id, err);
            return Err(Error::ErrorCode(ErrorCodes::SearchStreamNotFound(
                query.stream_name.clone(),
            )));
        }
    };
    log::info!(
        "[trace_id {}] search->storage: stream {}/{}/{}, get schema versions num {}",
        query.trace_id,
        query.org_id,
        query.stream_type,
        query.stream_name,
        schema_versions.len()
    );
    if schema_versions.is_empty() {
        return Ok((vec![], ScanStats::new()));
    }
    let schema_latest_id = schema_versions.len() - 1;

    // get file list
    let mut files = file_list.to_vec();
    if files.is_empty() {
        return Ok((vec![], ScanStats::default()));
    }
    log::info!(
        "[trace_id {}] search->storage: stream {}/{}/{}, load file_list num {}",
        query.trace_id,
        query.org_id,
        query.stream_type,
        query.stream_name,
        files.len(),
    );

    // check inverted index
    let cfg = get_config();
    let inverted_index_type = cfg.common.inverted_index_search_format.clone();
    let use_inverted_index = query.use_inverted_index && inverted_index_type == "tantivy";
    if use_inverted_index {
        log::info!(
            "[trace_id {}] flight->search: use_inverted_index with tantivy format {}",
            query.trace_id,
            use_inverted_index
        );
    }

    let mut idx_took = 0;
    let mut is_add_filter_back = false;
    if use_inverted_index {
        (idx_took, is_add_filter_back) =
            filter_file_list_by_tantivy_index(query.clone(), &mut files, index_condition.clone())
                .await?;
        log::info!(
            "[trace_id {}] search->storage: stream {}/{}/{}, {} inverted index reduced file_list num to {} in {} ms",
            query.trace_id,
            query.org_id,
            query.stream_type,
            query.stream_name,
            inverted_index_type,
            files.len(),
            idx_took
        );
    }

    if !is_add_filter_back {
        index_condition = None;
        fst_fields = vec![];
    }

    let cfg = get_config();
    let mut files_group: HashMap<usize, Vec<FileKey>> =
        HashMap::with_capacity(schema_versions.len());
    let mut scan_stats = ScanStats::new();
    if !cfg.common.widening_schema_evolution || schema_versions.len() == 1 {
        let files = files.to_vec();
        scan_stats = match file_list::calculate_files_size(&files).await {
            Ok(size) => size,
            Err(err) => {
                log::error!(
                    "[trace_id {}] calculate files size error: {}",
                    query.trace_id,
                    err
                );
                return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                    "calculate files size error".to_string(),
                )));
            }
        };
        files_group.insert(schema_latest_id, files);
    } else {
        scan_stats.files = files.len() as i64;
        for file in files.iter() {
            // calculate scan size
            scan_stats.records += file.meta.records;
            scan_stats.original_size += file.meta.original_size;
            scan_stats.compressed_size += file.meta.compressed_size;
            // check schema version
            let schema_ver_id = match db::schema::filter_schema_version_id(
                &schema_versions,
                file.meta.min_ts,
                file.meta.max_ts,
            ) {
                Some(id) => id,
                None => {
                    log::error!(
                        "[trace_id {}] search->storage: file {} schema version not found, will use the latest schema, min_ts: {}, max_ts: {}",
                        query.trace_id,
                        &file.key,
                        file.meta.min_ts,
                        file.meta.max_ts
                    );
                    // HACK: use the latest version if not found in schema versions
                    schema_latest_id
                }
            };
            let group = files_group.entry(schema_ver_id).or_default();
            group.push(file.clone());
        }
    }

    log::info!(
        "[trace_id {}] search->storage: stream {}/{}/{}, load files {}, scan_size {}, compressed_size {}",
        query.trace_id,
        query.org_id,
        query.stream_type,
        query.stream_name,
        scan_stats.files,
        scan_stats.original_size,
        scan_stats.compressed_size
    );

    if cfg.common.memory_circuit_breaker_enable {
        super::check_memory_circuit_breaker(&query.trace_id, &scan_stats)?;
    }

    // load files to local cache
    let cache_start = std::time::Instant::now();
    let (cache_type, deleted_files) = cache_files(
        &query.trace_id,
        &files.iter().map(|f| f.key.as_ref()).collect_vec(),
        &mut scan_stats,
    )
    .instrument(enter_span.clone())
    .await?;
    if !deleted_files.is_empty() {
        // remove deleted files from files_group
        for (_, g_files) in files_group.iter_mut() {
            g_files.retain(|f| !deleted_files.contains(&f.key));
        }
    }

    scan_stats.idx_took = idx_took as i64;
    scan_stats.querier_files = scan_stats.files;
    log::info!(
        "[trace_id {}] search->storage: stream {}/{}/{}, load files {}, memory cached {}, disk cached {}, download others into {:?} cache done, took: {} ms",
        query.trace_id,
        query.org_id,
        query.stream_type,
        query.stream_name,
        scan_stats.querier_files,
        scan_stats.querier_memory_cached_files,
        scan_stats.querier_disk_cached_files,
        cache_type,
        cache_start.elapsed().as_millis()
    );

    // set target partitions based on cache type
    let target_partitions = if cache_type == file_data::CacheType::None {
        cfg.limit.query_thread_num
    } else {
        cfg.limit.cpu_num
    };

    // construct latest schema map
    let schema_latest = Arc::new(
        schema
            .as_ref()
            .clone()
            .with_metadata(std::collections::HashMap::new()),
    );
    let mut schema_latest_map = HashMap::with_capacity(schema_latest.fields().len());
    for field in schema_latest.fields() {
        schema_latest_map.insert(field.name(), field);
    }

    let mut tables = Vec::new();
    for (ver, files) in files_group {
        if files.is_empty() {
            continue;
        }
        if files.is_empty() {
            continue;
        }
        let schema = schema_versions[ver].clone();
        let schema = schema.with_metadata(std::collections::HashMap::new());

        let session = config::meta::search::Session {
            id: format!("{}-{}-{ver}", query.trace_id, query.job_id),
            storage_type: StorageType::Memory,
            work_group: query.work_group.clone(),
            target_partitions,
        };

        let diff_fields = generate_search_schema_diff(&schema, &schema_latest_map)?;
        let table = exec::create_parquet_table(
            &session,
            schema_latest.clone(),
            &files,
            diff_fields,
            sorted_by_time,
            file_stat_cache.clone(),
            index_condition.clone(),
            fst_fields.clone(),
        )
        .await?;
        tables.push(table);
    }

    Ok((tables, scan_stats))
}

#[tracing::instrument(name = "service:search:grpc:storage:cache_files", skip_all)]
async fn cache_files(
    trace_id: &str,
    files: &[&str],
    scan_stats: &mut ScanStats,
) -> Result<(file_data::CacheType, Vec<String>), Error> {
    let cfg = get_config();
    let cache_type = if cfg.memory_cache.enabled
        && scan_stats.compressed_size < cfg.memory_cache.skip_size as i64
    {
        // if scan_compressed_size < ZO_MEMORY_CACHE_SKIP_SIZE, use memory cache
        file_data::CacheType::Memory
    } else if !is_local_disk_storage()
        && cfg.disk_cache.enabled
        && scan_stats.compressed_size < cfg.disk_cache.skip_size as i64
    {
        // if scan_compressed_size < ZO_DISK_CACHE_SKIP_SIZE, use disk cache
        file_data::CacheType::Disk
    } else {
        // no cache
        return Ok((file_data::CacheType::None, vec![]));
    };

    let mut tasks = Vec::new();
    let semaphore = std::sync::Arc::new(Semaphore::new(cfg.limit.query_thread_num));
    for file in files.iter() {
        let trace_id = trace_id.to_string();
        let file_name = file.to_string();
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let task: tokio::task::JoinHandle<(Option<String>, bool, bool)> =
            tokio::task::spawn(async move {
                let cfg = get_config();
                let ret = match cache_type {
                    file_data::CacheType::Memory => {
                        let mut disk_exists = false;
                        let mem_exists = file_data::memory::exist(&file_name).await;
                        if !mem_exists && !cfg.memory_cache.skip_disk_check {
                            // when skip_disk_check = false, need to check disk cache
                            disk_exists = file_data::disk::exist(&file_name).await;
                        }
                        if !mem_exists && (cfg.memory_cache.skip_disk_check || !disk_exists) {
                            (
                                file_data::memory::download(&trace_id, &file_name)
                                    .await
                                    .err(),
                                false,
                                false,
                            )
                        } else {
                            (None, mem_exists, disk_exists)
                        }
                    }
                    file_data::CacheType::Disk => {
                        if !file_data::disk::exist(&file_name).await {
                            (
                                file_data::disk::download(&trace_id, &file_name).await.err(),
                                false,
                                false,
                            )
                        } else {
                            (None, false, true)
                        }
                    }
                    _ => (None, false, false),
                };
                // return file_name if download failed
                let file_name = if let Some(e) = ret.0 {
                    log::warn!(
                        "[trace_id {trace_id}] search->storage: download file to cache err: {}",
                        e
                    );
                    Some(file_name)
                } else {
                    None
                };
                drop(permit);
                (file_name, ret.1, ret.2)
            });
        tasks.push(task);
    }

    let mut delete_files = Vec::new();
    for task in tasks {
        match task.await {
            Ok((file, mem_exists, disk_exists)) => {
                if mem_exists {
                    scan_stats.querier_memory_cached_files += 1;
                } else if disk_exists {
                    scan_stats.querier_disk_cached_files += 1;
                }
                if let Some(file) = file {
                    delete_files.push(file);
                }
            }
            Err(e) => {
                log::error!(
                    "[trace_id {trace_id}] search->storage: load file task err: {}",
                    e
                );
            }
        }
    }

    Ok((cache_type, delete_files))
}

/// Filter file list using inverted index
/// This function will load the index file corresponding to each file in the file list.
/// FSTs in those files are used to match the incoming query in `SearchRequest`.
/// If the query does not match any FST in the index file, the file will be filtered out.
/// If the query does match then the segment IDs for the file will be updated.
/// If the query not find corresponding index file, the file will *not* be filtered out.
async fn filter_file_list_by_tantivy_index(
    query: Arc<super::QueryParams>,
    file_list: &mut Vec<FileKey>,
    index_condition: Option<IndexCondition>,
) -> Result<(usize, bool), Error> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    // Cache the corresponding Index files
    let mut scan_stats = ScanStats::new();
    let mut file_list_map = file_list
        .drain(..)
        .map(|f| (f.key.clone(), f))
        .collect::<HashMap<_, _>>();
    let index_file_names = file_list_map
        .iter()
        .filter_map(|(_, f)| {
            scan_stats.compressed_size += f.meta.index_size;
            if f.meta.index_size > 0 {
                convert_parquet_idx_file_name_to_tantivy_file(&f.key)
                    .map(|v| (f.key.clone(), v, f.meta.records))
            } else {
                None
            }
        })
        .collect_vec();
    scan_stats.querier_files = index_file_names.len() as i64;
    let (cache_type, _) = cache_files(
        &query.trace_id,
        &index_file_names
            .iter()
            .map(|(_parquet_file, ttv_file, _)| ttv_file.as_str())
            .collect_vec(),
        &mut scan_stats,
    )
    .await?;

    log::info!(
        "[trace_id {}] search->storage: stream {}/{}/{}, load puffin index files {}, memory cached {}, disk cached {}, download others into {:?} cache done, took: {} ms",
        query.trace_id,
        query.org_id,
        query.stream_type,
        query.stream_name,
        scan_stats.querier_files,
        scan_stats.querier_memory_cached_files,
        scan_stats.querier_disk_cached_files,
        cache_type,
        start.elapsed().as_millis()
    );

    let semaphore = std::sync::Arc::new(Semaphore::new(cfg.limit.cpu_num));
    let mut tasks = Vec::new();

    for (parquet_file, _ttv_file, records) in index_file_names {
        let file = parquet_file;
        let file_size = file_list_map.get(&file).unwrap().meta.index_size as usize;
        let trace_id = query.trace_id.to_string();
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        // Spawn a task for each file, wherein full text search and
        // secondary index search queries are executed
        let index_condition_clone = index_condition.clone();
        let task = tokio::task::spawn(async move {
            let res =
                search_tantivy_index(&trace_id, &file, index_condition_clone, records, file_size)
                    .await;
            drop(permit);
            res
        });
        tasks.push(task)
    }

    let mut total_hits = 0;
    let mut is_add_filter_back = false;
    for result in try_join_all(tasks)
        .await
        .map_err(|e| Error::ErrorCode(ErrorCodes::ServerInternalError(e.to_string())))?
    {
        let result: anyhow::Result<(String, Option<BitVec>)> = result;
        // Each result corresponds to a file in the file list
        match result {
            Ok((file_name, bitvec)) => {
                if let Some(res) = bitvec {
                    // Replace the segment IDs in the existing `FileKey` with the new found segments
                    let file = file_list_map
                        .get_mut(&file_name)
                        // we expect each file name has atleast 1 file
                        .unwrap();
                    let hits_per_file = res.count_ones();
                    total_hits += hits_per_file;
                    file.with_segment_ids(res);
                    log::debug!(
                        "[trace_id {}] search->storage: hits for index_condition: {:?} found {} in {}",
                        query.trace_id,
                        index_condition,
                        hits_per_file,
                        file_name
                    );
                } else {
                    // if the bitmap is empty then we remove the file from the list
                    log::debug!(
                        "[trace_id {}] search->storage: no match found in index for file {}",
                        query.trace_id,
                        file_name
                    );
                    file_list_map.remove(&file_name);
                }
            }
            Err(e) => {
                log::warn!(
                    "[trace_id {}] search->storage: error filtering file via FST index. Keep file to search. error: {}",
                    query.trace_id,
                    e.to_string()
                );
                is_add_filter_back = true;
                continue;
            }
        }
    }
    log::info!(
        "[trace_id {}] search->storage: total hits for index_condition: {:?} found {}",
        query.trace_id,
        index_condition,
        total_hits
    );
    file_list.extend(file_list_map.into_values());
    Ok((start.elapsed().as_millis() as usize, is_add_filter_back))
}

pub async fn get_tantivy_directory(
    _trace_id: &str,
    file_name: &str,
    file_size: usize,
) -> anyhow::Result<PuffinDirReader> {
    let source = object_store::ObjectMeta {
        location: file_name.into(),
        last_modified: *BASE_TIME,
        size: file_size,
        e_tag: None,
        version: None,
    };
    Ok(PuffinDirReader::from_path(source).await?)
}

async fn search_tantivy_index(
    trace_id: &str,
    parquet_file_name: &str,
    index_condition: Option<IndexCondition>,
    records: i64,
    file_size: usize,
) -> anyhow::Result<(String, Option<BitVec>)> {
    let Some(ttv_file_name) = convert_parquet_idx_file_name_to_tantivy_file(parquet_file_name)
    else {
        return Err(anyhow::anyhow!(
            "[trace_id {trace_id}] search->storage: Unable to find tantivy index files for parquet file {}",
            parquet_file_name
        ));
    };

    // cache the indexer and reader
    let cfg = get_config();
    let indexer = if cfg.common.inverted_index_cache_enabled {
        reader_cache::GLOBAL_CACHE.get(&ttv_file_name)
    } else {
        None
    };
    let (tantivy_index, tantivy_reader) = match indexer {
        Some((indexer, reader)) => (indexer, reader),
        None => {
            log::debug!("init cache for puffin file: {}", ttv_file_name);
            let reader_directory: Box<dyn Directory> = if cfg.common.inverted_index_tantivy_mode
                == InvertedIndexTantivyMode::Mmap.to_string()
            {
                let puffin_dir_path = format!(
                    "{}{}",
                    cfg.common.data_cache_dir,
                    ttv_file_name.replace(FILE_EXT_TANTIVY, FILE_EXT_TANTIVY_FOLDER)
                );
                if !is_exists(&puffin_dir_path) {
                    let read_dir =
                        get_tantivy_directory(trace_id, &ttv_file_name, file_size).await?;
                    let _ = convert_puffin_file_to_tantivy_dir(read_dir, &puffin_dir_path).await?;
                }
                Box::new(tantivy::directory::MmapDirectory::open(&puffin_dir_path)?)
            } else {
                let puffin_dir =
                    Arc::new(get_tantivy_directory(trace_id, &ttv_file_name, file_size).await?);
                let footer_cache = FooterCache::from_directory(puffin_dir.clone()).await?;
                let cache_dir =
                    CachingDirectory::new_with_cacher(puffin_dir, Arc::new(footer_cache));
                Box::new(cache_dir)
            };

            let index = tantivy::Index::open(reader_directory)?;
            let reader = index
                .reader_builder()
                .reload_policy(tantivy::ReloadPolicy::Manual)
                .num_warming_threads(0)
                .try_into()?;
            let index = Arc::new(index);
            let reader = Arc::new(reader);
            if cfg.common.inverted_index_cache_enabled {
                reader_cache::GLOBAL_CACHE
                    .put(ttv_file_name.to_string(), (index.clone(), reader.clone()));
            }
            (index, reader)
        }
    };

    let tantivy_searcher = tantivy_reader.searcher();
    let tantivy_schema = tantivy_index.schema();
    let fts_field = tantivy_schema.get_field(INDEX_FIELD_NAME_FOR_ALL).unwrap();

    // generate the tantivy query
    let condition: IndexCondition = index_condition.ok_or(anyhow::anyhow!(
        "[trace_id {trace_id}] search->storage: IndexCondition not found"
    ))?;
    let query = condition.to_tantivy_query(tantivy_schema.clone(), fts_field);

    // warm up the terms in the query
    if cfg.common.inverted_index_tantivy_mode == InvertedIndexTantivyMode::Puffin.to_string() {
        let mut warm_terms: HashMap<tantivy::schema::Field, HashMap<tantivy::Term, bool>> =
            HashMap::new();
        query.query_terms(&mut |term, need_position| {
            let field = term.field();
            let entry = warm_terms.entry(field).or_default();
            entry.insert(term.clone(), need_position);
        });
        // if no terms are found in the query, warm up all fields
        if warm_terms.is_empty() {
            for field in condition.get_fields() {
                let field = tantivy_schema.get_field(&field).unwrap();
                warm_terms.insert(field, HashMap::new());
            }
        }
        warm_up_terms(&tantivy_searcher, &warm_terms).await?;
    }

    // search the index
    let matched_docs = tokio::task::spawn_blocking(move || {
        tantivy_searcher.search(&query, &tantivy::collector::DocSetCollector)
    })
    .await??;

    // return early if no matches in tantivy
    if matched_docs.is_empty() {
        return Ok((parquet_file_name.to_string(), None));
    }

    // Prepare a vec of segment offsets
    // this is useful when there are more than one segments
    let seg_metas = tantivy_index
        .searchable_segment_metas()
        .context("Count segments")?;
    let mut res = BitVec::with_capacity(records as usize + 1);
    // rewrite the empty bitvec
    res.resize((records + 1) as usize, false);
    // Return early if there is only one segment
    if seg_metas.len() == 1 {
        for doc in matched_docs {
            res.set(doc.doc_id as usize, true);
        }
        Ok((parquet_file_name.to_string(), Some(res)))
    } else {
        Err(anyhow::anyhow!(
            "[trace_id {trace_id}] search->storage: Multiple segments in tantivy index not supported"
        ))
    }
}
