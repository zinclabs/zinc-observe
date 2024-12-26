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

use chrono::{DateTime, Datelike, TimeZone, Utc};
use config::{
    meta::{
        search::{self, Response, SearchPartitionRequest},
        stream::StreamType,
    },
    utils::json,
};
use infra::{
    errors::{Error, ErrorCodes},
    storage,
    table::entity::{search_job_partitions::Model as PartitionJob, search_jobs::Model as Job},
};
use o2_enterprise::enterprise::{
    common::infra::config::get_config as get_o2_config,
    super_cluster::{
        kv::cluster::get_grpc_addr,
        search::{get_cluster_node_by_name, get_cluster_nodes},
    },
};
use tokio::sync::mpsc;

use super::grpc::make_grpc_search_client;
use crate::service::{
    db::search_job::{search_job_partitions::*, search_job_results::*, search_jobs::*},
    search::grpc_search::{grpc_search, grpc_search_partition},
};

// 1. get the oldest job from `search_jobs` table
// 2. check if the job is previous running (get error then retry, be cancel then retry) (case 1) or
//    do not have previous run (case 2) in case 2, call search_partition to get all jobs, write to
//    database
// 3. get all partition jobs from `search_job_partitions` table
// 4. after run on partition, write result to s3
// 6. when all partition is done, write data to s3, then update `search_jobs` table
pub async fn run(id: i64) -> Result<(), anyhow::Error> {
    // 1. get the oldest job from `search_jobs` table, and set status to running
    let job = match get_job().await? {
        Some(job) => job,
        None => return Ok(()),
    };

    let start = std::time::Instant::now();
    log::info!(
        "[SEARCH JOB {id}] job_id: {}, start running, trace_id: {}",
        job.id,
        job.trace_id
    );

    // 2. similar to the compactor, we need to update the job status every 15 seconds
    let ttl = std::cmp::max(60, config::get_config().limit.search_job_run_timeout / 4) as u64;
    let job_id = job.id.clone();
    let (_tx, mut rx) = mpsc::channel::<()>(1);
    tokio::task::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(ttl)) => {}
                _ = rx.recv() => {
                    log::debug!("[SEARCH JOB {id}] job_id: {job_id}, update_running_jobs done");
                    return;
                }
            }

            if let Err(e) = update_running_job(&job_id).await {
                log::error!("[SEARCH JOB {id}] job_id: {job_id}, update_job_status failed: {e}",);
            }
        }
    });

    // 3. check if the job is previous running (get error then retry, be cancel then retry) (case 1)
    //    or do not have previous run (case 2)
    if job.partition_num.is_none() {
        let res = handle_search_partition(&job).await;
        if let Err(e) = res {
            set_job_error_message(&job.id, &e.to_string()).await?;
            log::error!(
                "[SEARCH JOB {id}] job_id: {}, handle_search_partition error: {e}",
                job.id
            );
            return Err(e);
        }
    }

    // 4. get all partition jobs from `search_job_partitions` table
    let req: search::Request = json::from_str(&job.payload)?;
    let limit = if req.query.size > 0 {
        req.query.size
    } else {
        config::get_config().limit.query_default_limit
    };
    let offset = req.query.from;
    let partition_jobs = get_partition_jobs(&job.id).await?;
    let (partition_jobs, mut need) = filter_partition_job(partition_jobs, limit, offset).await?;

    // if need <= 0, means all partition jobs are done, but not generate the final result
    if need > 0 {
        for partition_job in partition_jobs.iter() {
            // check if the job is still running
            check_status(id, &job.id, &job.org_id).await?;
            let res = run_partition_job(id, &job, partition_job, req.clone(), need).await;
            let total = match res {
                Ok(total) => total,
                Err(e) => {
                    set_job_error_message(&job.id, &e.to_string()).await?;
                    log::error!(
                        "[SEARCH JOB {id}] job_id: {}, run_partition_job error: {e}",
                        job.id
                    );
                    return Err(e);
                }
            };
            need -= total;
            if need <= 0 {
                break;
            }
        }
    }

    log::info!(
        "[SEARCH JOB {id}] job_id: {}, start merge all partition result",
        job.id,
    );

    // 5. after run on partition, write result to s3
    let partition_jobs = get_partition_jobs(&job.id).await?;
    let mut response = merge_response(partition_jobs, limit, offset).await?;
    response.set_trace_id(job.trace_id.clone());
    let buf = json::to_vec(&response)?;
    let path = generate_result_path(job.created_at, &job.trace_id, None);
    storage::put(&path, buf.into()).await?;

    // 6. update `search_jobs` table
    set_job_finish(&job.id, &path).await?;

    log::info!(
        "[SEARCH JOB {id}] finish running, job_id: {}, time_elapsed: {}ms",
        job.id,
        start.elapsed().as_millis()
    );

    Ok(())
}

// 1. call search_partition to get all time range
// 2. write to database
async fn handle_search_partition(job: &Job) -> Result<(), anyhow::Error> {
    let stream_type = StreamType::from(job.stream_type.as_str());
    let req: search::Request = json::from_str(&job.payload)?;
    let partition_req = SearchPartitionRequest::from(&req);
    let res = grpc_search_partition(
        &job.trace_id,
        &job.org_id,
        stream_type,
        &partition_req,
        true,
    )
    .await?;

    submit_partitions(&job.id, res.partitions.as_slice()).await?;

    set_partition_num(&job.id, res.partitions.len() as i64)
        .await
        .map_err(|e| e.into())
}

// 1. rewrite the start_time and end_time to the query
// 2. set the partition status to running
// 3. run the query
// 4. write the result to s3
// 5. set the partition status to finish
async fn run_partition_job(
    id: i64,
    job: &Job,
    partition_job: &PartitionJob,
    req: search::Request,
    need: i64,
) -> Result<i64, anyhow::Error> {
    log::info!(
        "[SEARCH JOB {id}] running job_id: {}, partition id: {}",
        job.id,
        partition_job.partition_id
    );

    // 1. rewrite the start_time and end_time to the query
    let start = std::time::Instant::now();
    let mut req = req;
    req.query.start_time = partition_job.start_time;
    req.query.end_time = partition_job.end_time;
    req.query.from = 0;
    req.query.size = need;
    let partition_id = partition_job.partition_id;

    // 2. set the partition status to running
    set_partition_job_start(&job.id, partition_id).await?;

    // 3. run the query
    let stream_type = StreamType::from(job.stream_type.as_str());
    let res = grpc_search(
        &job.trace_id,
        &job.org_id,
        stream_type,
        Some(job.user_id.clone()),
        &req,
    )
    .await;
    if let Err(e) = res {
        set_partition_job_error_message(&job.id, partition_id, &e.to_string()).await?;
        return Err(e.into());
    }
    let mut result = res.unwrap();
    let took = start.elapsed().as_millis();
    result.set_local_took(took as usize, 0);

    // 4. write the result to s3
    let hits = result.total;
    let buf = json::to_vec(&result)?;
    let path = generate_result_path(job.created_at, &job.trace_id, Some(partition_id));
    storage::put(&path, buf.into()).await?;

    // 5. set the partition status to finish
    set_partition_job_finish(&job.id, partition_id, path.as_str()).await?;

    log::info!(
        "[SEARCH JOB {id}] finish job_id: {}, partition id: {}",
        job.id,
        partition_job.partition_id
    );

    Ok(hits as i64)
}

// get all partition jobs that need run
async fn filter_partition_job(
    partition_jobs: Vec<PartitionJob>,
    limit: i64,
    offest: i64,
) -> Result<(Vec<PartitionJob>, i64), anyhow::Error> {
    let mut need = limit + offest;
    let mut needed_partitions_jobs = Vec::new();
    for partition_job in partition_jobs.iter() {
        // if the result_path is not none, means the partition job is done
        if partition_job.result_path.is_some() {
            let path = partition_job.result_path.as_ref().unwrap();
            let buf = storage::get(path).await?;
            let res: Response = json::from_str(String::from_utf8(buf.to_vec())?.as_str())?;
            need -= res.total as i64;
        } else {
            needed_partitions_jobs.push(partition_job.clone());
        }
    }

    Ok((needed_partitions_jobs, need))
}

fn generate_result_path(
    created_at: i64,           // the job's created_at
    trace_id: &str,            // the job's trace_id
    partition_id: Option<i64>, // None means it is the final result
) -> String {
    let datetime: DateTime<Utc> = Utc.timestamp_nanos(created_at * 1000);

    format!(
        "result/{year}/{month}/{day}/{trace_id}/{partition_id}.result.json",
        year = datetime.year(),
        month = datetime.month(),
        day = datetime.day(),
        trace_id = trace_id,
        partition_id = if partition_id.is_some() {
            partition_id.unwrap().to_string()
        } else {
            "final".to_string()
        }
    )
}

pub async fn delete_jobs() -> Result<(), anyhow::Error> {
    // 1. get deleted jobs from database
    let jobs = get_deleted_jobs().await?;
    if jobs.is_empty() {
        return Ok(());
    }

    // 2. delete the s3 result
    for job in jobs.iter() {
        let mut deleted_files = Vec::new();
        let partition_num = job.partition_num;
        if let Some(partition_num) = partition_num {
            for i in 0..partition_num {
                let path = generate_result_path(job.created_at, &job.trace_id, Some(i));
                deleted_files.push(path);
            }
        }
        if job.result_path.is_some() {
            deleted_files.push(job.result_path.clone().unwrap());
        }

        // add old result path
        let job_result = get_job_result(&job.id).await?;
        for result in job_result.iter() {
            if result.result_path.is_some() {
                deleted_files.push(result.result_path.clone().unwrap());
            }
            // the old running maybe have partition result
            if let Some(partition_num) = partition_num {
                for i in 0..partition_num {
                    let path = generate_result_path(job.created_at, &result.trace_id, Some(i));
                    deleted_files.push(path);
                }
            }
        }

        // delete all files
        if let Err(e) = delete_result(deleted_files).await {
            log::warn!("[SEARCH JOB] delete_jobs failed to delete files error: {e}",);
        }

        // 3. delete the partition jobs from database
        clean_deleted_partition_job(&job.id).await?;

        // 4. delete the job result from database
        clean_deleted_job_result(&job.id).await?;

        // 5. delete the job from database
        clean_deleted_job(&job.id).await?;
    }

    Ok(())
}

async fn check_status(id: i64, job_id: &str, org_id: &str) -> Result<(), anyhow::Error> {
    let job = get(job_id, org_id).await?;
    if job.status != 1 {
        let message = format!(
            "[SEARCH JOB {id}] job_id: {}, status is not running when running search job, current status: {}",
            job.id, job.status
        );
        set_job_error_message(&job.id, message.as_str()).await?;
        log::error!("{}", message);
        return Err(anyhow::anyhow!(message));
    }
    Ok(())
}

async fn merge_response(
    jobs: Vec<PartitionJob>,
    limit: i64,
    offset: i64,
) -> Result<Response, anyhow::Error> {
    let mut response = Vec::new();
    for job in jobs.iter() {
        if job.result_path.is_none() || job.cluster.is_none() {
            continue;
        }
        let cluster = job.cluster.as_ref().unwrap();
        let path = job.result_path.as_ref().unwrap();
        let res = get_result(path, cluster).await?;
        response.push(res);
    }

    // if only one partition job, return directly
    if response.len() == 1 {
        let mut resp = response.remove(0);
        resp.from = offset;
        resp.size = limit;
        resp.hits = resp
            .hits
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();
        resp.total = resp.hits.len();
        return Ok(resp);
    }

    // merge all response
    let mut resp = response.remove(0);
    for r in response {
        resp.took += r.took;
        resp.took_detail = match (resp.took_detail, r.took_detail.as_ref()) {
            (Some(mut a), Some(b)) => {
                a.add(b);
                Some(a)
            }
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };
        resp.hits.extend(r.hits);
        resp.total += r.total;
        resp.file_count += r.file_count;
        resp.scan_size += r.scan_size;
        resp.idx_scan_size += r.idx_scan_size;
        resp.scan_records += r.scan_records;
        if !r.function_error.is_empty() {
            resp.function_error = format!("{} \n {}", resp.function_error, r.function_error);
            resp.is_partial = true;
        }
    }
    resp.from = offset;
    resp.size = limit;

    resp.hits = resp
        .hits
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();
    resp.total = resp.hits.len();

    Ok(resp)
}

// get the response in this cluster or other cluster
pub async fn get_result(path: &str, cluster: &str) -> Result<Response, anyhow::Error> {
    if *cluster == config::get_cluster_name() {
        let buf = storage::get(path).await?;
        let res: Response = json::from_slice::<Response>(&buf)?;
        return Ok(res);
    }

    // super cluster
    if get_o2_config().super_cluster.enabled {
        let node = get_cluster_node_by_name(cluster).await?;
        let path = path.to_string();
        let task = tokio::task::spawn(async move {
            let mut request = tonic::Request::new(proto::cluster_rpc::GetResultRequest { path });
            let mut client = make_grpc_search_client(&mut request, &node).await?;
            let response = match client.get_result(request).await {
                Ok(res) => res.into_inner(),
                Err(err) => {
                    log::error!(
                        "search->grpc: node: {}, search err: {:?}",
                        &node.get_grpc_addr(),
                        err
                    );
                    if err.code() == tonic::Code::Internal {
                        let err = ErrorCodes::from_json(err.message())?;
                        return Err(Error::ErrorCode(err));
                    }
                    return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                        "search node error".to_string(),
                    )));
                }
            };
            Ok(response)
        });

        let response = task
            .await
            .map_err(|e| Error::ErrorCode(ErrorCodes::ServerInternalError(e.to_string())))??;
        let response = json::from_slice::<search::Response>(&response.response)?;

        return Ok(response);
    }

    Err(anyhow::anyhow!(format!(
        "cluster name: {cluster} in partition job equal to current cluster name: {}",
        config::get_cluster_name()
    )))
}

pub async fn delete_result(paths: Vec<String>) -> Result<(), anyhow::Error> {
    let local_paths: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
    storage::del(&local_paths).await?;

    if get_o2_config().super_cluster.enabled {
        let nodes = get_cluster_nodes("delete_result", vec![], vec![]).await?;
        // delete result in all cluster,
        // because for retry job, we don't know partition in which cluster
        for node in nodes {
            if node.get_grpc_addr() == get_grpc_addr() {
                continue;
            }
            let paths = paths.clone();
            let task = tokio::task::spawn(async move {
                let mut request =
                    tonic::Request::new(proto::cluster_rpc::DeleteResultRequest { paths });
                let mut client = make_grpc_search_client(&mut request, &node).await?;
                let response = match client.delete_result(request).await {
                    Ok(res) => res.into_inner(),
                    Err(err) => {
                        log::error!(
                            "search->grpc: node: {}, search err: {:?}",
                            &node.get_grpc_addr(),
                            err
                        );
                        if err.code() == tonic::Code::Internal {
                            let err = ErrorCodes::from_json(err.message())?;
                            return Err(Error::ErrorCode(err));
                        }
                        return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                            "search node error".to_string(),
                        )));
                    }
                };
                Ok(response)
            });

            let _ = task
                .await
                .map_err(|e| Error::ErrorCode(ErrorCodes::ServerInternalError(e.to_string())))??;
        }
    }
    Ok(())
}
