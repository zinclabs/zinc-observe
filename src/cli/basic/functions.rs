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

use std::process::exit;

use config::Config;

/// Render the help string and default value for all the available
/// environment variables in O2.
pub(crate) fn render_help(enable_check: bool) {
    let fields = Config::get_help();

    let max_len = fields
        .iter()
        .map(|(key, _)| key.len())
        .max()
        .unwrap_or_default();

    let mut empty_env_variables = vec!["Following environment variables are not set:"];
    for (k, v) in fields.iter() {
        if k.is_empty() {
            continue;
        }
        if enable_check {
            if v.1.as_deref().unwrap_or("").is_empty() {
                empty_env_variables.push(k);
            }
        } else {
            println!(
                "{:<width$}: <default: {}> {} ",
                k,
                &v.0,
                v.1.as_deref().unwrap_or(""),
                width = max_len + 1,
            );
        }
    }

    // gt 1 because the first element is the message
    if empty_env_variables.len() > 1 && enable_check {
        println!("{}", empty_env_variables.join("\n"));
        exit(1);
    }
}
