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

use tempfile::tempdir;
use wal::{build_file_path, Reader, Writer};

#[test]
fn wal() {
    let entry_num = 100;
    let dir = tempdir().unwrap();
    let dir = dir.path();
    let mut writer = Writer::new(dir, "org", "stream", 1, 1024_1024, 8 * 1024).unwrap();
    for i in 0..entry_num {
        let data = format!("hello world {}", i);
        writer.write(data.as_bytes(), true).unwrap();
    }
    writer.close().unwrap();

    let path = build_file_path(dir, "org", "stream", 1);
    let mut reader = Reader::from_path(path).unwrap();
    for i in 0..entry_num {
        let data = format!("hello world {}", i);
        let entry = reader.read_entry().unwrap().unwrap();
        assert_eq!(entry, data.as_bytes());
    }
    assert!(reader.read_entry().unwrap().is_none());
}
