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

use super::Sum64;

// offset64 FNVa offset basis. See https://en.wikipedia.org/wiki/Fowler–Noll–Vo_hash_function#FNV-1a_hash
const OFFSET64: u64 = 14695981039346656037;
// prime64 FNVa prime value. See https://en.wikipedia.org/wiki/Fowler–Noll–Vo_hash_function#FNV-1a_hash
const PRIME64: u64 = 1099511628211;

/// refer: https://github.com/allegro/bigcache/blob/main/fnv.go
#[derive(Default)]
pub struct Fnv64a {}

pub fn new() -> Fnv64a {
    Fnv64a::new()
}

impl Fnv64a {
    pub fn new() -> Fnv64a {
        Fnv64a {}
    }
}

impl Sum64 for Fnv64a {
    fn sum64(&mut self, key: &str) -> u64 {
        let mut hash = OFFSET64;
        for byte in key.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(PRIME64);
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv64a() {
        let mut h = new();
        for key in &[
            "hello", "world", "foo", "bar", "test", "test1", "test2", "test3",
        ] {
            let sum = h.sum64(key);
            println!("{}: {}", key, sum);
        }
        assert_eq!(h.sum64("hello"), 11831194018420276491);
        assert_eq!(h.sum64("world"), 5717881983045765875);
        assert_eq!(h.sum64("foo"), 15902901984413996407);
        assert_eq!(h.sum64("bar"), 16101355973854746);
        assert_eq!(h.sum64("test"), 18007334074686647077);
        assert_eq!(h.sum64("test1"), 2271358237066212092);
        assert_eq!(h.sum64("test2"), 2271361535601096725);
        assert_eq!(h.sum64("test3"), 2271360436089468514);
    }
}
