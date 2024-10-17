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

pub fn zero_or<T>(v: T, def: T) -> T
where
    T: PartialEq + Default,
{
    if v == Default::default() { def } else { v }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_or() {
        assert_eq!(zero_or(0, 1), 1);
        assert_eq!(zero_or(2, 1), 2);
        assert_eq!(zero_or(0, 0), 0);
        assert_eq!(zero_or(0.0, 1.1), 1.1);
        assert_eq!(zero_or(2.1, 1.1), 2.1);
        assert_eq!(zero_or("", "v"), "v");
        assert_eq!(zero_or("vv", "v"), "vv");
    }
}
