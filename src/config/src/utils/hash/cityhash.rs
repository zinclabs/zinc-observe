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

pub struct CityHash {}

pub fn new() -> CityHash {
    CityHash {}
}

impl Sum64 for CityHash {
    fn sum64(&mut self, key: &str) -> u64 {
        cityhasher::hash(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cityhash_sum64() {
        let mut h = new();
        for key in &[
            "hello", "world", "foo", "bar", "test", "test1", "test2", "test3",
        ] {
            let sum = h.sum64(key);
            println!("{}: {}", key, sum);
        }
        assert_eq!(h.sum64("hello"), 13009744463427800296);
        assert_eq!(h.sum64("world"), 16436542438370751598);
        assert_eq!(h.sum64("foo"), 6150913649986995171);
        assert_eq!(h.sum64("bar"), 18347965944623823639);
        assert_eq!(h.sum64("test"), 8581389452482819506);
        assert_eq!(h.sum64("test1"), 10258151040114700067);
        assert_eq!(h.sum64("test2"), 13211586869705959535);
        assert_eq!(h.sum64("test3"), 3708989183841402594);
    }
}
