use std::path;

use config::get_config;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

pub async fn cache_results_to_disk(file_name: &str, data: &str) -> std::io::Result<()> {
    let cfg = get_config();
    let path = path::Path::new(&cfg.common.result_cache_dir).join(file_name);
    std::fs::create_dir_all(&cfg.common.result_cache_dir).expect("create cache dir success");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .await?;

    file.write(data.as_bytes()).await?;
    file.flush().await?;
    Ok(())
}
