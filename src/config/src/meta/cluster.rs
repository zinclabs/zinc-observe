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

use std::{fmt::Debug, str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{get_config, get_instance_id, meta::search::SearchEventType};

pub trait NodeInfo: Debug + Send + Sync {
    fn get_grpc_addr(&self) -> String;
    fn get_auth_token(&self) -> String;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: i32,
    pub uuid: String,
    pub name: String,
    pub http_addr: String,
    pub grpc_addr: String,
    pub role: Vec<Role>,
    #[serde(default)]
    pub role_group: RoleGroup,
    pub cpu_num: u64,
    pub status: NodeStatus,
    #[serde(default)]
    pub scheduled: bool,
    #[serde(default)]
    pub broadcasted: bool,
}

impl Node {
    fn new() -> Self {
        Node {
            id: 0,
            uuid: "".to_string(),
            name: "".to_string(),
            http_addr: "".to_string(),
            grpc_addr: "".to_string(),
            role: vec![],
            role_group: RoleGroup::None,
            cpu_num: 0,
            status: NodeStatus::Prepare,
            scheduled: false,
            broadcasted: false,
        }
    }
    pub fn is_single_node(&self) -> bool {
        self.role.len() == 1 && self.role.contains(&Role::All)
    }
    pub fn is_router(&self) -> bool {
        self.role.contains(&Role::Router)
    }
    pub fn is_ingester(&self) -> bool {
        self.role.contains(&Role::Ingester) || self.role.contains(&Role::All)
    }
    pub fn is_querier(&self) -> bool {
        self.role.contains(&Role::Querier) || self.role.contains(&Role::All)
    }
    pub fn is_interactive_querier(&self) -> bool {
        self.is_querier()
            && (self.role_group == RoleGroup::None || self.role_group == RoleGroup::Interactive)
    }
    pub fn is_background_querier(&self) -> bool {
        self.is_querier()
            && (self.role_group == RoleGroup::None || self.role_group == RoleGroup::Background)
    }
    pub fn is_compactor(&self) -> bool {
        self.role.contains(&Role::Compactor) || self.role.contains(&Role::All)
    }
    pub fn is_flatten_compactor(&self) -> bool {
        self.role.contains(&Role::FlattenCompactor) || self.role.contains(&Role::All)
    }
    pub fn is_alert_manager(&self) -> bool {
        self.role.contains(&Role::AlertManager) || self.role.contains(&Role::All)
    }
}

impl Default for Node {
    fn default() -> Self {
        Node::new()
    }
}

impl NodeInfo for Node {
    fn get_auth_token(&self) -> String {
        get_internal_grpc_token()
    }

    fn get_grpc_addr(&self) -> String {
        self.grpc_addr.clone()
    }
}

pub trait IntoArcVec {
    fn into_arc_vec(self) -> Vec<Arc<dyn NodeInfo>>;
}

impl IntoArcVec for Vec<Node> {
    fn into_arc_vec(self) -> Vec<Arc<dyn NodeInfo>> {
        self.into_iter().map(|n| Arc::new(n) as _).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeStatus {
    Prepare = 1,
    Online = 2,
    Offline = 3,
}

impl From<u32> for NodeStatus {
    fn from(value: u32) -> Self {
        match value {
            1 => NodeStatus::Prepare,
            2 => NodeStatus::Online,
            3 => NodeStatus::Offline,
            _ => NodeStatus::Prepare,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Role {
    All,
    Ingester,
    Querier,
    Compactor,
    Router,
    AlertManager,
    FlattenCompactor,
}

impl FromStr for Role {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "all" => Ok(Role::All),
            "ingester" => Ok(Role::Ingester),
            "querier" => Ok(Role::Querier),
            "compactor" => Ok(Role::Compactor),
            "router" => Ok(Role::Router),
            "alertmanager" | "alert_manager" => Ok(Role::AlertManager),
            "flatten_compactor" => Ok(Role::FlattenCompactor),
            _ => Err(format!("Invalid cluster role: {s}")),
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::All => write!(f, "all"),
            Role::Ingester => write!(f, "ingester"),
            Role::Querier => write!(f, "querier"),
            Role::Compactor => write!(f, "compactor"),
            Role::Router => write!(f, "router"),
            Role::AlertManager => write!(f, "alert_manager"),
            Role::FlattenCompactor => write!(f, "flatten_compactor"),
        }
    }
}

/// Categorizes nodes into different groups.
/// None        -> All tasks
/// Background  -> Low-priority tasks
/// Interactive -> High-priority tasks
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub enum RoleGroup {
    #[default]
    None,
    Interactive,
    Background,
}

impl From<&str> for RoleGroup {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "background" => RoleGroup::Background,
            "interactive" => RoleGroup::Interactive,
            _ => RoleGroup::None,
        }
    }
}

impl From<SearchEventType> for RoleGroup {
    fn from(value: SearchEventType) -> Self {
        match value {
            SearchEventType::Reports
            | SearchEventType::Alerts
            | SearchEventType::DerivedStream
            | SearchEventType::SearchJob => RoleGroup::Background,
            _ => RoleGroup::Interactive,
        }
    }
}

impl std::fmt::Display for RoleGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleGroup::None => write!(f, ""),
            RoleGroup::Interactive => write!(f, "interactive"),
            RoleGroup::Background => write!(f, "background"),
        }
    }
}

#[inline]
pub fn get_internal_grpc_token() -> String {
    let cfg = get_config();
    if cfg.grpc.internal_grpc_token.is_empty() {
        get_instance_id()
    } else {
        cfg.grpc.internal_grpc_token.clone()
    }
}

// CompactionJobType is used to distinguish between current and historical compaction jobs.
pub enum CompactionJobType {
    Current,
    Historical,
}
