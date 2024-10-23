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

use rand::distributions::{Alphanumeric, DistString};

pub fn get_rand_element<T>(arr: &[T]) -> &T {
    let mut buf = [0u8; 1];
    getrandom::getrandom(&mut buf).unwrap();
    &arr[buf[0] as usize % arr.len()]
}

pub fn generate_random_string(len: usize) -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), len)
}

/// Generate random number within the given range
pub fn get_rand_num_within(min: u64, max: u64) -> u64 {
    let mut buf = [0u8; 1];
    getrandom::getrandom(&mut buf).unwrap();
    min + buf[0] as u64 % (max - min)
}
