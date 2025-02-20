use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(style = Form, parameter_in = Query)]
#[serde(rename_all = "snake_case")]
#[into_params(rename_all = "snake_case")]
pub struct UploadEnrichmentTableQuery {
    pub append: bool,
}
