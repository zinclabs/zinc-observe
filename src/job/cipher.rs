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

use config::get_config;

use crate::cipher::registry::REGISTRY;

pub async fn run() -> Result<(), anyhow::Error> {
    // TODO impl
    let config = get_config();
    let key = crate::cipher::cipher::Key::new("".to_string());
    {
        let mut reg = REGISTRY.write();
        reg.add_key("test_key".to_string(), Box::new(key));
    }
    Ok(())
}
