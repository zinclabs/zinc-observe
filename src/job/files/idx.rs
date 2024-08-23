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

use config::{
    ider,
    meta::stream::{FileMeta, StreamType},
    utils::parquet::new_parquet_writer,
    FILE_EXT_PARQUET,
};
use infra::storage;
use tokio::task;

use crate::common::utils::stream::populate_file_meta;

fn generate_index_file_name_from_compacted_file(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    compacted_file_name: &str,
) -> String {
    // eg: files/default/logs/quickstart1/2024/02/16/16/7164299619311026293.parquet
    let file_columns = compacted_file_name.split('/').collect::<Vec<&str>>();
    let stream_key = format!("{}/{}/{}", org_id, stream_type, stream_name);
    let file_date = format!(
        "{}/{}/{}/{}",
        file_columns[4], file_columns[5], file_columns[6], file_columns[7]
    );
    let file_name = ider::generate();
    format!("files/{stream_key}/{file_date}/{file_name}{FILE_EXT_PARQUET}")
}

pub(crate) async fn write_parquet_index_to_disk(
    batches: Vec<arrow::record_batch::RecordBatch>,
    file_size: u64,
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    file_name: &str,
    caller: &str,
) -> Result<(String, FileMeta, StreamType), anyhow::Error> {
    let schema = if let Some(first_batch) = batches.first() {
        first_batch.schema()
    } else {
        return Err(anyhow::anyhow!("No record batches found"));
    };

    // write metadata
    let mut file_meta = FileMeta {
        min_ts: 0,
        max_ts: 0,
        records: 0,
        original_size: file_size as i64,
        compressed_size: 0,
        flattened: false,
    };
    populate_file_meta(
        schema.clone(),
        vec![batches.to_vec()],
        &mut file_meta,
        Some("min_ts"),
        Some("max_ts"),
    )
    .await?;

    // write parquet file
    let mut buf_parquet = Vec::new();
    let mut writer = new_parquet_writer(&mut buf_parquet, &schema, &[], &[], &file_meta);
    for batch in batches {
        writer.write(&batch).await?;
    }
    writer.close().await?;
    file_meta.compressed_size = buf_parquet.len() as i64;

    let new_idx_file_name =
        generate_index_file_name_from_compacted_file(org_id, stream_type, stream_name, file_name);
    log::info!(
        "[JOB] IDX: write_to_disk: {}/{}/{} {} {} {}",
        org_id,
        stream_name,
        stream_type,
        new_idx_file_name,
        file_name,
        caller,
    );

    let store_file_name = new_idx_file_name.clone();
    match task::spawn_blocking(move || async move {
        storage::put(&store_file_name, bytes::Bytes::from(buf_parquet)).await
    })
    .await
    {
        Ok(output) => match output.await {
            Ok(_) => {
                log::info!("[JOB] disk file upload succeeded: {}", &new_idx_file_name);
                Ok((new_idx_file_name, file_meta, stream_type))
            }
            Err(err) => {
                log::error!("[JOB] disk file upload error: {:?}", err);
                Err(anyhow::anyhow!(err))
            }
        },
        Err(err) => {
            log::error!("[JOB] disk file upload error: {:?}", err);
            Err(anyhow::anyhow!(err))
        }
    }
}
