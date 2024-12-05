use config::meta::dashboards::{v1, v2, v3, v4, v5};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

/// HTTP request body for the `CreateDashboard` endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDashboardRequestBody(JsonValue);

/// HTTP response body for `CreateDashboard` endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDashboardResponseBody(DashboardDetails);

/// HTTP response body for `GetDashboard` endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct GetDashboardResponseBody(DashboardDetails);

/// HTTP request body for `UpdateDashboard` endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateDashboardRequestBody(JsonValue);

/// HTTP response body for `UpdateDashboard` endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateDashboardResponseBody(DashboardDetails);

/// HTTP URL query component that contains parameters for listing dashboards.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(style = Form, parameter_in = Query)]
pub struct ListDashboardsQuery {
    /// Optional folder ID filter parameter
    ///
    /// If neither `folder` nor any other filter parameter are set then this
    /// will search for all dashboards in the "default" folder.
    ///
    /// If `folder` is not set and another filter parameter, such as `title`, is
    /// set then this will search for dashboards in all folders.
    folder: Option<String>,

    /// The optional case-insensitive title substring with which to filter
    /// dashboards.
    title: Option<String>,
}

/// HTTP response body for `ListDashboards` endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListDashboardsResponseBody {
    pub dashboards: Vec<ListDashboardsResponseBodyItem>,
}

/// An item in the list returned by the `ListDashboards` endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListDashboardsResponseBodyItem {
    #[deprecated(note = "use GetDashboard endpoint to get dashboard details")]
    pub v1: Option<v1::Dashboard>,
    #[deprecated(note = "use GetDashboard endpoint to get dashboard details")]
    pub v2: Option<v2::Dashboard>,
    #[deprecated(note = "use GetDashboard endpoint to get dashboard details")]
    pub v3: Option<v3::Dashboard>,
    #[deprecated(note = "use GetDashboard endpoint to get dashboard details")]
    pub v4: Option<v4::Dashboard>,
    #[deprecated(note = "use GetDashboard endpoint to get dashboard details")]
    pub v5: Option<v5::Dashboard>,
    pub version: i32,
    pub hash: String,
}

/// HTTP request body for `MoveDashboard` endpoint.
#[derive(Debug, Clone, PartialEq, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MoveDashboardRequestBody {
    pub from: String,
    pub to: String,
}

/// Version-specific dashboard details and hash.
#[derive(Debug, Serialize, ToSchema)]
pub struct DashboardDetails {
    pub v1: Option<v1::Dashboard>,
    pub v2: Option<v2::Dashboard>,
    pub v3: Option<v3::Dashboard>,
    pub v4: Option<v4::Dashboard>,
    pub v5: Option<v5::Dashboard>,
    pub version: i32,
    pub hash: String,
}

impl TryFrom<CreateDashboardRequestBody> for config::meta::dashboards::Dashboard {
    type Error = serde_json::Error;

    fn try_from(value: CreateDashboardRequestBody) -> Result<Self, Self::Error> {
        let json = value.0;
        parse_dashboard_request(json)
    }
}

impl From<config::meta::dashboards::Dashboard> for CreateDashboardResponseBody {
    fn from(value: config::meta::dashboards::Dashboard) -> Self {
        Self(value.into())
    }
}

impl From<config::meta::dashboards::Dashboard> for GetDashboardResponseBody {
    fn from(value: config::meta::dashboards::Dashboard) -> Self {
        Self(value.into())
    }
}

impl TryFrom<UpdateDashboardRequestBody> for config::meta::dashboards::Dashboard {
    type Error = serde_json::Error;

    fn try_from(value: UpdateDashboardRequestBody) -> Result<Self, Self::Error> {
        let json = value.0;
        parse_dashboard_request(json)
    }
}

impl From<config::meta::dashboards::Dashboard> for UpdateDashboardResponseBody {
    fn from(value: config::meta::dashboards::Dashboard) -> Self {
        Self(value.into())
    }
}

impl ListDashboardsQuery {
    pub fn into(self, org_id: &str) -> config::meta::dashboards::ListDashboardsParams {
        match self {
            Self {
                folder: Some(f),
                title: Some(t),
            } => config::meta::dashboards::ListDashboardsParams::new(org_id)
                .with_folder_id(&f)
                .where_title_contains(&t),
            Self {
                folder: None,
                title: Some(t),
            } => {
                config::meta::dashboards::ListDashboardsParams::new(org_id).where_title_contains(&t)
            }
            Self {
                folder: Some(f),
                title: None,
            } => config::meta::dashboards::ListDashboardsParams::new(org_id).with_folder_id(&f),
            Self {
                folder: None,
                title: None,
            } => {
                // To preserve backwards-compatability when no filter parameters
                // are given we will list the contents of the default folder.
                config::meta::dashboards::ListDashboardsParams::new(org_id)
                    .with_folder_id(config::meta::folder::DEFAULT_FOLDER)
            }
        }
    }
}

impl From<Vec<config::meta::dashboards::Dashboard>> for ListDashboardsResponseBody {
    fn from(value: Vec<config::meta::dashboards::Dashboard>) -> Self {
        let dashboards = value.into_iter().map(|d| d.into()).collect();
        Self { dashboards }
    }
}

impl From<config::meta::dashboards::Dashboard> for ListDashboardsResponseBodyItem {
    #[allow(deprecated)]
    fn from(value: config::meta::dashboards::Dashboard) -> Self {
        Self {
            hash: value.hash,
            version: value.version,
            // Populate deprecated fields until they are removed from the API.
            v1: value.v1,
            v2: value.v2,
            v3: value.v3,
            v4: value.v4,
            v5: value.v5,
        }
    }
}

impl From<config::meta::dashboards::Dashboard> for DashboardDetails {
    fn from(value: config::meta::dashboards::Dashboard) -> Self {
        Self {
            version: value.version,
            v1: value.v1,
            v2: value.v2,
            v3: value.v3,
            v4: value.v4,
            v5: value.v5,
            hash: value.hash,
        }
    }
}

/// Parses the JSON value from an HTTP request body into a dashboard.
fn parse_dashboard_request(
    value: JsonValue,
) -> Result<config::meta::dashboards::Dashboard, serde_json::Error> {
    // Pull the version field out of the JSON. If it doesn't exist, assume the
    // default version is 1.
    let version = value
        .as_object()
        .and_then(|o| o.get("version"))
        .and_then(|v| v.as_i64())
        .unwrap_or(1);

    let dash = match version {
        1 => {
            let inner: v1::Dashboard = serde_json::from_value(value)?;
            inner.into()
        }
        2 => {
            let inner: v2::Dashboard = serde_json::from_value(value)?;
            inner.into()
        }
        3 => {
            let inner: v3::Dashboard = serde_json::from_value(value)?;
            inner.into()
        }
        4 => {
            let inner: v4::Dashboard = serde_json::from_value(value)?;
            inner.into()
        }
        _ => {
            let inner: v5::Dashboard = serde_json::from_value(value)?;
            inner.into()
        }
    };
    Ok(dash)
}
