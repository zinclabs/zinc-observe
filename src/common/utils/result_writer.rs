use std::path;

use config::get_config;
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
