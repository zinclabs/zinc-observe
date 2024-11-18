use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    #[serde(default)]
    pub folder_id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FolderList {
    pub list: Vec<Folder>,
}

pub const DEFAULT_FOLDER: &str = "default";
