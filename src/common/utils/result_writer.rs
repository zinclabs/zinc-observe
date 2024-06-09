use std::path;

use config::{get_config, utils::file::get_file_contents};
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
    let path = path::Path::new(&get_config().common.result_cache_dir).join(file_path);
    let mut file = std::fs::File::open(path);
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;

    Ok(contents)
}
