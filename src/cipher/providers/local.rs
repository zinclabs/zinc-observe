use serde::{Deserialize, Serialize};

#[derive(Hash, Clone, Debug, Serialize, Deserialize)]
pub enum Algorithm {}

pub struct Local {
    pub algorithm: Algorithm,
    pub key: String,
}
