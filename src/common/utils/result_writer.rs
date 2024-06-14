use std::{io::Read, path};

use bytes::Bytes;
use config::{get_config, utils::file};
use infra::cache::file_data::disk;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

pub async fn cache_results_to_disk(
    file_path: &str,
    file_name: &str,
    data: &str,
) -> std::io::Result<()> {
    let cfg = get_config();
    let path = path::Path::new(&cfg.common.result_cache_dir).join(file_path);
    std::fs::create_dir_all(&path).expect("create cache dir success");
    let file_path = path.join(file_name);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&file_path)
        .await?;

    file.write(data.as_bytes()).await?;
    file.flush().await?;
    Ok(())
}

pub async fn get_results(file_path: &str, file_name: &str) -> std::io::Result<String> {
    let path = path::Path::new(&get_config().common.result_cache_dir)
        .join(format!("{}/{}", file_path, file_name));
    let file = std::fs::File::open(path);
    let mut contents = String::new();
    file.unwrap().read_to_string(&mut contents)?;

    Ok(contents)
}

pub async fn cache_results_to_disk_v1(
    trace_id: &str,
    file_path: &str,
    file_name: &str,
    data: String,
) -> std::io::Result<()> {
    let file = format!("{}/{}", file_path, file_name);
    let _ = disk::set(trace_id, &file, Bytes::from(data)).await;
    Ok(())
}
