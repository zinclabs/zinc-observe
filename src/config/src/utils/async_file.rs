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

use std::{fs::Metadata, ops::Range, path::Path, time::SystemTime};

use async_walkdir::WalkDir;
use futures::StreamExt;
use tokio::{
    fs::{read_dir, remove_dir, File},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};

#[inline(always)]
pub async fn get_file_meta(path: impl AsRef<Path>) -> Result<Metadata, std::io::Error> {
    let file = File::open(path).await?;
    file.metadata().await
}

#[inline(always)]
pub async fn get_file_len(path: impl AsRef<Path>) -> Result<u64, std::io::Error> {
    let file = File::open(path).await?;
    file.metadata().await.map(|m| m.len())
}

#[inline(always)]
pub async fn is_exists(path: impl AsRef<Path>) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

#[inline(always)]
pub async fn get_file_contents(
    path: impl AsRef<Path>,
    range: Option<Range<usize>>,
) -> Result<Vec<u8>, std::io::Error> {
    let mut file = File::open(path).await?;
    let data = if let Some(range) = range {
        let to_read = range.end - range.start;
        let mut buf = Vec::with_capacity(to_read);
        file.seek(std::io::SeekFrom::Start(range.start as u64))
            .await?;
        let read = file.take(to_read as u64).read_to_end(&mut buf).await?;
        if read != to_read {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "Expected to read {} bytes, but read {} bytes",
                    to_read, read
                ),
            ));
        }
        buf
    } else {
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).await?;
        buf
    };
    Ok(data)
}

#[inline(always)]
pub async fn put_file_contents(
    path: impl AsRef<Path>,
    contents: &[u8],
) -> Result<(), std::io::Error> {
    let mut file = File::create(path).await?;
    file.write_all(contents).await
}

pub async fn clean_empty_dirs(
    dir: &str,
    last_updated: Option<SystemTime>,
) -> Result<(), std::io::Error> {
    let mut dirs = Vec::new();
    let mut entries = WalkDir::new(dir);
    loop {
        match entries.next().await {
            Some(Ok(entry)) => {
                if entry.path().display().to_string() == dir {
                    continue;
                }
                if let Ok(f) = entry.file_type().await {
                    if f.is_dir() {
                        match last_updated {
                            None => {
                                dirs.push(entry.path().to_str().unwrap().to_string());
                            }
                            Some(last_updated) => {
                                if let Ok(meta) = entry.metadata().await {
                                    if meta.modified().unwrap() < last_updated {
                                        dirs.push(entry.path().to_str().unwrap().to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Some(Err(e)) => {
                log::error!("clean_empty_dirs, err: {}", e);
                break;
            }
            None => break,
        }
    }
    dirs.sort_by_key(|b| std::cmp::Reverse(b.len()));
    for dir in dirs {
        if let Ok(mut entries) = read_dir(&dir).await {
            if let Ok(None) = entries.next_entry().await {
                if let Err(e) = remove_dir(&dir).await {
                    log::error!("Failed to remove empty dir: {}, err: {}", dir, e);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_file_contents_with_range() {
        let content = b"Hello World";
        let file_name = "range_test_async.txt";

        put_file_contents(file_name, content).await.unwrap();

        // Test valid range
        assert_eq!(
            get_file_contents(file_name, Some(0..5)).await.unwrap(),
            b"Hello"
        );

        // Test invalid range should error
        assert!(get_file_contents(file_name, Some(3..5)).await.is_ok());

        // Test out of bounds should error
        assert!(get_file_contents(file_name, Some(0..100)).await.is_err());

        std::fs::remove_file(file_name).unwrap();
    }
}
