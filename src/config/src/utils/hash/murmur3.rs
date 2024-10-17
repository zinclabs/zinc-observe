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

use std::io::Cursor;

use murmur3::murmur3_x64_128;

use super::Sum64;

pub struct Mr3 {}

pub fn new() -> Mr3 {
    Mr3 {}
}

impl Sum64 for Mr3 {
    fn sum64(&mut self, key: &str) -> u64 {
        let mut r = Cursor::new(key);
        let ret = murmur3_x64_128(&mut r, 0).unwrap();
        ret as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_murmur3_sum64() {
        let mut h = new();
        for key in &[
            "hello", "world", "foo", "bar", "test", "test1", "test2", "test3",
        ] {
            let sum = h.sum64(key);
            println!("{}: {}", key, sum);
        }
        assert_eq!(h.sum64("hello"), 14688674573012802306);
        assert_eq!(h.sum64("world"), 8198091784597505258);
        assert_eq!(h.sum64("foo"), 16316970633193145697);
        assert_eq!(h.sum64("bar"), 10535706080149431812);
        assert_eq!(h.sum64("test"), 12429135405209477533);
        assert_eq!(h.sum64("test1"), 271003567416724429);
        assert_eq!(h.sum64("test2"), 17808217402673344406);
        assert_eq!(h.sum64("test3"), 11977005607778562912);
    }
}
