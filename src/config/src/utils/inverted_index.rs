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

use std::borrow::Cow;

use itertools::Itertools;

use crate::{meta::stream::StreamType, FILE_EXT_PARQUET, FILE_EXT_TANTIVY, INDEX_MIN_CHAR_LEN};

/// Split a string into tokens based on a delimiter. if delimiter is empty, split by whitespace and
/// punctuation. also filter out tokens that are less than INDEX_MIN_CHAR_LEN characters long.
pub fn split_token(s: &str, delimiter: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| {
            if delimiter.is_empty() {
                c.is_whitespace() || c.is_ascii_punctuation()
            } else {
                delimiter.contains(c)
            }
        })
        .filter_map(|s| {
            let s = s.trim().trim_matches(|c: char| c.is_ascii_punctuation());
            // Question (Uddhav) : This is problematic if user is looking for a single character.
            // If the idea is to skip small tokens, then we shoula also check if the input string is
            // a single character. Is that allowed?
            if s.len() >= INDEX_MIN_CHAR_LEN {
                Some(s.to_string())
            } else {
                None
            }
        })
        .unique()
        .collect()
}

/// Packs two u32 values into a single u64 value.
/// Used to cast (offset: u32, size: u32) to u64 that's acceptable by FSTMap
pub fn pack_u32_pair(offset: u32, size: u32) -> u64 {
    let packed: u64 = (offset as u64) | ((size as u64) << 32);
    packed
}

/// Unpacks a u64 value read from FSTMap to (offset: u32, size: u32)
pub fn unpack_u32_pair(packed: u64) -> (u32, u32) {
    let offset = (packed & 0xFFFFFFFF) as u32;
    let size = ((packed >> 32) & 0xFFFFFFFF) as u32;
    (offset, size)
}

/// FST inverted index solution has a 1:1 mapping between parquet and idx files.
/// This is a helper function to convert the paruqet file name to idx file name.
/// e.g.
/// from: files/default/logs/quickstart1/2024/02/16/16/7164299619311026293.parquet
/// to:   files/default/index/quickstart1_logs/2024/02/16/16/7164299619311026293.ttv
pub fn convert_parquet_idx_file_name_to_tantivy_file(from: &str) -> Option<String> {
    let mut parts: Vec<Cow<str>> = from.split('/').map(Cow::Borrowed).collect();

    if parts.len() < 4 {
        return None;
    }

    // Replace the stream_type part
    let stream_type_pos = 2;
    let stream_type = match parts[stream_type_pos].as_ref() {
        "logs" => StreamType::Logs,
        "metrics" => StreamType::Metrics,
        "traces" => StreamType::Traces,
        _ => return None,
    };
    parts[stream_type_pos] = Cow::Borrowed("index");

    // Replace the stream_name part
    let stream_name_pos = stream_type_pos + 1;
    parts[stream_name_pos] = Cow::Owned(format!("{}_{}", parts[stream_name_pos], stream_type));

    // Replace the file extension
    let file_name_pos = parts.len() - 1;
    if !parts[file_name_pos].ends_with(FILE_EXT_PARQUET) {
        return None;
    }
    parts[file_name_pos] =
        Cow::Owned(parts[file_name_pos].replace(FILE_EXT_PARQUET, FILE_EXT_TANTIVY));

    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        let result = split_token("", "");
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_min_character() {
        let result = split_token("t", "");
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_empty_delimiter() {
        let result = split_token("Hello, world! This is a test.", "");
        assert_eq!(
            result,
            vec![
                "hello".to_string(),
                "world".to_string(),
                "this".to_string(),
                "test".to_string()
            ]
        );
    }

    #[test]
    fn test_non_empty_delimiter() {
        let result = split_token("Hello,world,This,is,a,test", ",");
        assert_eq!(
            result,
            vec![
                "hello".to_string(),
                "world".to_string(),
                "this".to_string(),
                "test".to_string()
            ]
        );
    }

    #[test]
    fn test_mixed_whitespace_and_punctuation() {
        let result = split_token("Hello, world! This - is; a: test.", "");
        assert_eq!(
            result,
            vec![
                "hello".to_string(),
                "world".to_string(),
                "this".to_string(),
                "test".to_string()
            ]
        );
    }

    #[test]
    fn test_all_punctuation() {
        let result = split_token("!!!,,,;;;...???", "");
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_min_char_length_filter() {
        let result = split_token("a an and", "");
        assert_eq!(result, vec!["and".to_string()]);
    }

    #[test]
    fn test_with_numeric_characters() {
        let result = split_token("123 4567 89", "");
        assert_eq!(result, vec!["123".to_string(), "4567".to_string()]);
    }

    #[test]
    fn test_leading_and_trailing_punctuation() {
        let result = split_token("!!!Hello!!! !!!world!!!", "");
        assert_eq!(result, vec!["hello".to_string(), "world".to_string()]);
    }

    #[test]
    fn test_duplicate_tokens() {
        let result = split_token("hello, hello, world, world", ",");
        assert_eq!(result, vec!["hello".to_string(), "world".to_string()]);
    }

    #[test]
    fn test_mixed_case_sensitivity() {
        let result = split_token("Hello, HeLLo, WORLD, world", ",");
        assert_eq!(result, vec!["hello".to_string(), "world".to_string()]);
    }

    #[test]
    fn test_delimiter_as_whitespace() {
        let result = split_token("Hello world This is a test", " ");
        assert_eq!(
            result,
            vec![
                "hello".to_string(),
                "world".to_string(),
                "this".to_string(),
                "test".to_string()
            ]
        );
    }

    #[test]
    fn test_complex_delimiter() {
        let result = split_token("Hello||world||This||is||a||test", "||");
        assert_eq!(
            result,
            vec![
                "hello".to_string(),
                "world".to_string(),
                "this".to_string(),
                "test".to_string()
            ]
        );
    }

    #[test]
    fn test_pack_unpack_u32_pair() {
        // Test with random values
        let offset = 123456;
        let size = 789012;
        let packed = pack_u32_pair(offset, size);
        let (unpacked_offset, unpacked_size) = unpack_u32_pair(packed);
        assert_eq!(offset, unpacked_offset);
        assert_eq!(size, unpacked_size);

        // Test with 0 values
        let packed_zero = pack_u32_pair(0, 0);
        let (unpacked_zero_offset, unpacked_zero_size) = unpack_u32_pair(packed_zero);
        assert_eq!(0, unpacked_zero_offset);
        assert_eq!(0, unpacked_zero_size);

        // Test with maximum values
        let max_offset = u32::MAX;
        let max_size = u32::MAX;
        let packed_max = pack_u32_pair(max_offset, max_size);
        let (unpacked_max_offset, unpacked_max_size) = unpack_u32_pair(packed_max);
        assert_eq!(max_offset, unpacked_max_offset);
        assert_eq!(max_size, unpacked_max_size);
    }

    #[test]
    fn test_pack_unpack_u32_pair_overflow() {
        // Test with values that would cause overflow
        let offset = u32::MAX;
        let size = 1;
        let packed = pack_u32_pair(offset, size);
        let (unpacked_offset, unpacked_size) = unpack_u32_pair(packed);
        assert_eq!(offset, unpacked_offset);
        assert_eq!(size, unpacked_size);

        let offset = 1;
        let size = u32::MAX;
        let packed = pack_u32_pair(offset, size);
        let (unpacked_offset, unpacked_size) = unpack_u32_pair(packed);
        assert_eq!(offset, unpacked_offset);
        assert_eq!(size, unpacked_size);
    }

    #[test]
    fn test_convert_parquet_idx_file_name_to_tantivy_file() {
        let test_cases = vec![
            (
                "files/default/logs/quickstart1/2024/02/16/16/7164299619311026293.parquet",
                Some(
                    "files/default/index/quickstart1_logs/2024/02/16/16/7164299619311026293.ttv"
                        .to_string(),
                ),
            ),
            (
                "files/default/metrics/quickstart1/2024/02/16/16/7164299619311026293.parquet",
                Some(
                    "files/default/index/quickstart1_metrics/2024/02/16/16/7164299619311026293.ttv"
                        .to_string(),
                ),
            ),
            (
                "files/default/traces/quickstart1/2024/02/16/16/7164299619311026293.parquet",
                Some(
                    "files/default/index/quickstart1_traces/2024/02/16/16/7164299619311026293.ttv"
                        .to_string(),
                ),
            ),
            (
                "files/default/metadata/quickstart1/2024/02/16/16/7164299619311026293.parquet",
                None,
            ),
            (
                "files/default/index/quickstart1/2024/02/16/16/7164299619311026293.parquet",
                None,
            ),
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                convert_parquet_idx_file_name_to_tantivy_file(input),
                expected
            );
        }
    }
}
