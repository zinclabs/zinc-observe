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

use std::{
    fs::{File, Metadata},
    io::{Read, Write},
    path::Path,
};

#[inline(always)]
pub fn get_file_meta(file: &str) -> Result<Metadata, std::io::Error> {
    let file = File::open(file)?;
    file.metadata()
}

#[inline(always)]
pub fn get_file_contents(file: &str) -> Result<Vec<u8>, std::io::Error> {
    let mut file = File::open(file)?;
    let mut contents: Vec<u8> = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(contents)
}

#[inline(always)]
pub fn put_file_contents(file: &str, contents: &[u8]) -> Result<(), std::io::Error> {
    let mut file = File::create(file)?;
    file.write_all(contents)
}

#[inline(always)]
pub fn scan_files<P: AsRef<Path>>(
    root: P,
    ext: &str,
    limit: Option<usize>,
) -> Result<Vec<String>, std::io::Error> {
    let limit = limit.unwrap_or_default();
    let mut files = Vec::with_capacity(std::cmp::max(16, limit));
    let dir = std::fs::read_dir(root)?;
    for entry in dir {
        let path = entry?.path();
        if path.is_file() {
            let path_ext = path
                .extension()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();
            if path_ext == ext {
                files.push(path.to_str().unwrap().to_string());
                if limit > 0 && files.len() >= limit {
                    return Ok(files);
                }
            }
        } else {
            let fl = if limit > 0 { limit - files.len() } else { 0 };
            let ff = scan_files(path, ext, Some(fl))?;
            if !ff.is_empty() {
                files.extend(ff);
            };
            if limit > 0 && files.len() >= limit {
                return Ok(files);
            }
        }
    }
    Ok(files)
}

#[inline(always)]
pub fn scan_files_for_move<P: AsRef<Path>>(
    root: P,
    ext: &str,
    limit: Option<usize>,
    start_from: &str,
) -> Result<(Vec<String>, String), std::io::Error> {
    let mut last_scanned_dir = String::new();
    let mut dirs = get_dirs(root.as_ref())?;
    dirs.sort();

    if !start_from.is_empty() {
        let idx = dirs.iter().position(|x| x == start_from);
        if let Some(idx) = idx {
            dirs = dirs.split_at(idx).1.to_vec();
        }
    }
    let limit = limit.unwrap_or_default();
    let mut files = Vec::with_capacity(std::cmp::max(16, limit));

    for dir in dirs {
        last_scanned_dir = dir.to_string();
        let dir_files = scan_files(dir, ext, Some(limit))?;
        files.extend(dir_files);
        if limit > 0 && files.len() >= limit {
            break;
        }
    }
    Ok((files, last_scanned_dir.to_string()))
}

#[inline(always)]
pub fn get_dirs<P: AsRef<Path>>(root: P) -> Result<Vec<String>, std::io::Error> {
    let mut leaf_dirs = Vec::new();
    let mut dirs = Vec::new();
    dirs.push(root.as_ref().to_path_buf());

    while let Some(dir_path) = dirs.pop() {
        let mut has_sub_dir = false;
        let dir = std::fs::read_dir(&dir_path)?;
        for entry in dir {
            let path = entry?.path();
            if path.is_dir() {
                has_sub_dir = true;
                dirs.push(path);
            }
        }
        if !has_sub_dir {
            leaf_dirs.push(dir_path.to_str().unwrap().to_string());
        }
    }
    Ok(leaf_dirs)
}

#[cfg(unix)]
pub fn set_permission<P: AsRef<std::path::Path>>(path: P, mode: u32) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path.as_ref())?;
    std::fs::set_permissions(path.as_ref(), std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
pub fn set_permission<P: AsRef<std::path::Path>>(
    path: P,
    _mode: u32,
) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(path.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scan_files() {
        let content = b"Some Text";
        let file_name = "sample.parquet";

        put_file_contents(file_name, content).unwrap();
        assert_eq!(get_file_contents(file_name).unwrap(), content);
        assert!(get_file_meta(file_name).unwrap().is_file());
        assert!(!scan_files(".", "parquet", None).unwrap().is_empty());
        std::fs::remove_file(file_name).unwrap();
    }
}
