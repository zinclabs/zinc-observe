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

use config::{get_config, meta::stream::StreamType};
use utoipa::{openapi::security::SecurityScheme, Modify, OpenApi};

use crate::{common::meta, handler::http::request};

#[derive(OpenApi)]
#[openapi(
    paths(
        request::status::healthz,
        request::users::list,
        request::users::save,
        request::users::update,
        request::users::delete,
        request::users::add_user_to_org,
        request::organization::org::organizations,
        request::organization::org::org_summary,
        request::organization::org::get_user_passcode,
        request::organization::org::update_user_passcode,
        request::organization::org::get_user_rumtoken,
        request::organization::org::update_user_rumtoken,
        request::organization::org::create_user_rumtoken,
        request::organization::settings::get,
        request::organization::settings::create,
        request::stream::list,
        request::stream::schema,
        request::stream::settings,
        request::stream::update_settings,
        request::stream::delete_fields,
        request::stream::delete,
        request::logs::ingest::bulk,
        request::logs::ingest::multi,
        request::logs::ingest::json,
        request::traces::traces_write,
        request::traces::get_latest_traces,
        request::metrics::ingest::json,
        request::promql::remote_write,
        request::promql::query_get,
        request::promql::query_range_get,
        request::promql::metadata,
        request::promql::series_get,
        request::promql::labels_get,
        request::promql::label_values,
        request::promql::format_query_get,
        request::enrichment_table::save_enrichment_table,
        request::rum::ingest::log,
        request::rum::ingest::data,
        request::rum::ingest::sessionreplay,
        request::search::search,
        request::search::search_partition,
        request::search::around,
        request::search::values,
        request::search::search_history,
        request::search::saved_view::create_view,
        request::search::saved_view::delete_view,
        request::search::saved_view::get_view,
        request::search::saved_view::get_views,
        request::search::saved_view::update_view,
        request::folders::delete_folder,
        request::folders::create_folder,
        request::folders::list_folders,
        request::folders::get_folder,
        request::folders::update_folder,
        request::folders::deprecated::delete_folder,
        request::folders::deprecated::create_folder,
        request::folders::deprecated::list_folders,
        request::folders::deprecated::get_folder,
        request::folders::deprecated::update_folder,
        request::functions::list_functions,
        request::functions::update_function,
        request::functions::save_function,
        request::functions::delete_function,
        request::functions::list_pipeline_dependencies,
        request::functions::test_function,
        request::dashboards::create_dashboard,
        request::dashboards::update_dashboard,
        request::dashboards::list_dashboards,
        request::dashboards::get_dashboard,
        request::dashboards::delete_dashboard,
        request::dashboards::move_dashboard,
        request::alerts::deprecated::save_alert,
        request::alerts::deprecated::update_alert,
        request::alerts::deprecated::list_stream_alerts,
        request::alerts::deprecated::list_alerts,
        request::alerts::deprecated::get_alert,
        request::alerts::deprecated::delete_alert,
        request::alerts::deprecated::enable_alert,
        request::alerts::deprecated::trigger_alert,
        request::alerts::create_alert,
        request::alerts::get_alert,
        request::alerts::update_alert,
        request::alerts::delete_alert,
        request::alerts::list_alerts,
        request::alerts::enable_alert,
        request::alerts::trigger_alert,
        request::alerts::move_alerts,
        request::alerts::templates::list_templates,
        request::alerts::templates::get_template,
        request::alerts::templates::save_template,
        request::alerts::templates::update_template,
        request::alerts::templates::delete_template,
        request::alerts::destinations::list_destinations,
        request::alerts::destinations::get_destination,
        request::alerts::destinations::save_destination,
        request::alerts::destinations::update_destination,
        request::alerts::destinations::delete_destination,
        request::kv::get,
        request::kv::set,
        request::kv::delete,
        request::kv::list,
        request::syslog::create_route,
        request::syslog::update_route,
        request::syslog::list_routes,
        request::syslog::delete_route,
        request::clusters::list_clusters,
        request::short_url::shorten,
        request::short_url::retrieve,
    ),
    components(
        schemas(
            meta::http::HttpResponse,
            StreamType,
            meta::stream::Stream,
            meta::stream::StreamProperty,
            meta::stream::StreamDeleteFields,
            meta::stream::ListStream,
            config::meta::stream::StreamSettings,
            config::meta::stream::StreamPartition,
            config::meta::stream::StreamPartitionType,
            config::meta::stream::StreamStats,
            config::meta::stream::PartitionTimeLevel,
            config::meta::stream::UpdateStreamSettings,
            config::meta::dashboards::Dashboard,
            config::meta::dashboards::v1::AxisItem,
            config::meta::dashboards::v1::Dashboard,
            config::meta::dashboards::v1::AggregationFunc,
            config::meta::dashboards::v1::Layout,
            config::meta::dashboards::v1::Panel,
            config::meta::dashboards::v1::PanelConfig,
            config::meta::dashboards::v1::PanelFields,
            config::meta::dashboards::v1::PanelFilter,
            config::meta::dashboards::v1::Variables,
            config::meta::dashboards::v1::QueryData,
            config::meta::dashboards::v1::CustomFieldsOption,
            config::meta::dashboards::v1::VariableList,
            // Dashboards
            crate::handler::http::models::dashboards::CreateDashboardRequestBody,
            crate::handler::http::models::dashboards::CreateDashboardResponseBody,
            crate::handler::http::models::dashboards::GetDashboardResponseBody,
            crate::handler::http::models::dashboards::UpdateDashboardRequestBody,
            crate::handler::http::models::dashboards::UpdateDashboardResponseBody,
            crate::handler::http::models::dashboards::ListDashboardsResponseBody,
            crate::handler::http::models::dashboards::ListDashboardsResponseBodyItem,
            crate::handler::http::models::dashboards::MoveDashboardRequestBody,
            config::meta::alerts::alert::Alert,
            config::meta::alerts::Aggregation,
            config::meta::alerts::AggFunction,
            config::meta::alerts::Condition,
            config::meta::alerts::CompareHistoricData,
            config::meta::alerts::destinations::Destination,
            config::meta::alerts::destinations::DestinationWithTemplate,
            config::meta::alerts::destinations::HTTPType,
            config::meta::alerts::destinations::DestinationType,
            config::meta::alerts::FrequencyType,
            config::meta::alerts::Operator,
            config::meta::alerts::QueryType,
            config::meta::alerts::QueryCondition,
            config::meta::alerts::TriggerCondition,
            config::meta::alerts::templates::Template,
            // Alerts
            crate::handler::http::models::alerts::requests::CreateAlertRequestBody,
            crate::handler::http::models::alerts::requests::UpdateAlertRequestBody,
            crate::handler::http::models::alerts::responses::GetAlertResponseBody,
            crate::handler::http::models::alerts::responses::ListAlertsResponseBody,
            crate::handler::http::models::alerts::responses::ListAlertsResponseBodyItem,
            crate::handler::http::models::alerts::responses::EnableAlertResponseBody,
            crate::handler::http::models::alerts::Alert,
            crate::handler::http::models::alerts::TriggerCondition,
            crate::handler::http::models::alerts::CompareHistoricData,
            crate::handler::http::models::alerts::FrequencyType,
            crate::handler::http::models::alerts::QueryCondition,
            crate::handler::http::models::alerts::Aggregation,
            crate::handler::http::models::alerts::AggFunction,
            crate::handler::http::models::alerts::QueryType,
            crate::handler::http::models::alerts::Condition,
            crate::handler::http::models::alerts::Operator,
            // Folders
            crate::handler::http::models::folders::CreateFolderRequestBody,
            crate::handler::http::models::folders::CreateFolderResponseBody,
            crate::handler::http::models::folders::GetFolderResponseBody,
            crate::handler::http::models::folders::ListFoldersResponseBody,
            crate::handler::http::models::folders::UpdateFolderRequestBody,
            crate::handler::http::models::folders::FolderType,
            config::meta::function::Transform,
            config::meta::function::FunctionList,
            config::meta::function::StreamOrder,
            config::meta::function::TestVRLRequest,
            config::meta::sql::OrderBy,
            config::meta::search::Query,
            config::meta::search::Request,
            config::meta::search::RequestEncoding,
            config::meta::search::Response,
            config::meta::search::ResponseTook,
            config::meta::search::ResponseNodeTook,
            config::meta::search::SearchEventType,
            config::meta::search::SearchEventContext,
            config::meta::search::SearchPartitionRequest,
            config::meta::search::SearchPartitionResponse,
            config::meta::search::SearchHistoryRequest,
            config::meta::search::CancelQueryResponse,
            config::meta::search::QueryStatusResponse,
            config::meta::search::QueryStatus,
            config::meta::search::QueryInfo,
            config::meta::search::ScanStats,
            config::meta::short_url::ShortenUrlRequest,
            config::meta::short_url::ShortenUrlResponse,
            meta::ingestion::RecordStatus,
            meta::ingestion::StreamStatus,
            meta::ingestion::IngestionResponse,
            meta::saved_view::View,
            meta::saved_view::ViewWithoutData,
            meta::saved_view::ViewsWithoutData,
            meta::saved_view::CreateViewRequest,
            meta::saved_view::DeleteViewResponse,
            meta::saved_view::CreateViewResponse,
            meta::saved_view::UpdateViewRequest,
            meta::user::UpdateUser,
            meta::user::UserRequest,
            meta::user::UserRole,
            meta::user::UserOrgRole,
            meta::user::UserList,
            meta::user::UserResponse,
            meta::user::SignInResponse,
            meta::organization::OrgSummary,
            meta::organization::StreamSummary,
            meta::organization::PipelineSummary,
            meta::organization::AlertSummary,
            meta::organization::OrganizationResponse,
            meta::organization::OrgDetails,
            meta::organization::OrgUser,
            meta::organization::IngestionPasscode,
            meta::organization::PasscodeResponse,
            meta::organization::OrganizationSetting,
            meta::organization::OrganizationSettingResponse,
            meta::organization::RumIngestionResponse,
            meta::organization::RumIngestionToken,
            request::status::HealthzResponse,
            meta::ingestion::BulkResponse,
            meta::ingestion::BulkResponseItem,
            meta::ingestion::ShardResponse,
            meta::ingestion::BulkResponseError,
            meta::syslog::SyslogRoute,
            meta::syslog::SyslogRoutes,
            config::meta::promql::Metadata,
            config::meta::promql::MetricType,
            // Functions

         ),
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Meta", description = "Meta details about the OpenObserve state itself. e.g. healthz"),
        (name = "Auth", description = "User login authentication"),
        (name = "Logs", description = "Logs data ingestion operations"),
        (name = "Dashboards", description = "Dashboard operations"),
        (name = "Search", description = "Search/Query operations"),
        (name = "Saved Views", description = "Collection of saved search views for easy retrieval"),
        (name = "Alerts", description = "Alerts retrieval & management operations"),
        (name = "Functions", description = "Functions retrieval & management operations"),
        (name = "Organizations", description = "Organizations retrieval & management operations"),
        (name = "Streams", description = "Stream retrieval & management operations"),
        (name = "Users", description = "Users retrieval & management operations"),
        (name = "KV", description = "Key Value retrieval & management operations"),
        (name = "Metrics", description = "Metrics data ingestion operations"),
        (name = "Traces", description = "Traces data ingestion operations"),
        (name = "Syslog Routes", description = "Syslog Routes retrieval & management operations"),
        (name = "Clusters", description = "Super cluster operations"),
        (name = "Short Url", description = "Short Url Service"),
    ),
    info(
        description = "OpenObserve API documents [https://openobserve.ai/docs/](https://openobserve.ai/docs/)",
        contact(name = "OpenObserve", email = "hello@zinclabs.io", url = "https://openobserve.ai/"),
    ),
)]
pub struct ApiDoc;

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let cfg = get_config();
        if !cfg.common.base_uri.is_empty() {
            openapi.servers = Some(vec![utoipa::openapi::Server::new(&cfg.common.base_uri)]);
        }
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "Authorization",
            SecurityScheme::ApiKey(utoipa::openapi::security::ApiKey::Header(
                utoipa::openapi::security::ApiKeyValue::new("Authorization"),
            )),
        );
    }
}
