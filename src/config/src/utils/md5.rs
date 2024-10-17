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

// Get the md5 hash of a string
fn hash(input: &str) -> String {
    let digest = md5::compute(input.as_bytes());
    format!("{:x}", digest)
}

// Get a short hash of a string
pub fn short_hash(input: &str) -> String {
    // md5 hash will always consist of 32 characters
    let hash = hash(input);

    // get the middle index of the hash string
    // and extract 16 characters from the middle
    let mid_index = hash.len() / 2;
    let short_id = &hash[mid_index - 8..mid_index + 8];
    short_id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash() {
        let input = "hello world";
        let expected = "5eb63bbbe01eeed093cb22bb8f5acdc3";
        let res = hash(input);
        assert_eq!(res, expected);
        assert_eq!(res.len(), 32);
    }

    #[test]
    fn test_short_hash() {
        let input = "hello world";
        let expected = "e01eeed093cb22bb";
        assert_eq!(short_hash(input), expected);
    }
}
