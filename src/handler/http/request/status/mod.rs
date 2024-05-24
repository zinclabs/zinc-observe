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

use std::io::Error;

use actix_web::{
    cookie,
    cookie::{Cookie, SameSite},
    get,
    http::header,
    put, web, HttpRequest, HttpResponse,
};
use arrow_schema::Schema;
use config::{
    cluster::{is_ingester, LOCAL_NODE_ROLE, LOCAL_NODE_UUID},
    meta::cluster::NodeStatus,
    utils::{json, schema_ext::SchemaExt},
    CONFIG, INSTANCE_ID, QUICK_MODEL_FIELDS, SQL_FULL_TEXT_SEARCH_FIELDS,
};
use hashbrown::HashMap;
use infra::{
    cache, file_list,
    schema::{
        STREAM_SCHEMAS, STREAM_SCHEMAS_COMPRESSED, STREAM_SCHEMAS_FIELDS, STREAM_SCHEMAS_LATEST,
    },
};
use serde::Serialize;
use utoipa::ToSchema;
#[cfg(feature = "enterprise")]
use {
    crate::common::utils::jwt::verify_decode_token,
    crate::handler::http::auth::{
        jwt::process_token,
        validator::{ID_TOKEN_HEADER, PKCE_STATE_ORG},
    },
    config::{ider, utils::base64},
    o2_enterprise::enterprise::{
        common::{
            infra::config::O2_CONFIG,
            settings::{get_logo, get_logo_text},
        },
        dex::service::auth::{exchange_code, get_dex_login, get_jwks, refresh_token},
    },
    std::io::ErrorKind,
};

use crate::{
    common::{
        infra::{cluster, config::*},
        meta::{functions::ZoFunction, http::HttpResponse as MetaHttpResponse, user::AuthTokens},
    },
    service::{db, search::datafusion::DEFAULT_FUNCTIONS},
};

#[derive(Serialize, ToSchema)]
pub struct HealthzResponse {
    status: String,
}

#[derive(Serialize)]
struct ConfigResponse<'a> {
    version: String,
    instance: String,
    commit_hash: String,
    build_date: String,
    build_type: String,
    default_fts_keys: Vec<String>,
    default_quick_mode_fields: Vec<String>,
    telemetry_enabled: bool,
    default_functions: Vec<ZoFunction<'a>>,
    sql_base64_enabled: bool,
    timestamp_column: String,
    syslog_enabled: bool,
    data_retention_days: i64,
    restricted_routes_on_empty_data: bool,
    sso_enabled: bool,
    native_login_enabled: bool,
    rbac_enabled: bool,
    super_cluster_enabled: bool,
    query_on_stream_selection: bool,
    show_stream_stats_doc_num: bool,
    custom_logo_text: String,
    custom_slack_url: String,
    custom_docs_url: String,
    rum: Rum,
    custom_logo_img: Option<String>,
    custom_hide_menus: String,
    meta_org: String,
    quick_mode_enabled: bool,
    user_defined_schemas_enabled: bool,
    all_fields_name: String,
}

#[derive(Serialize)]
struct Rum {
    pub enabled: bool,
    pub client_token: String,
    pub application_id: String,
    pub site: String,
    pub service: String,
    pub env: String,
    pub version: String,
    pub organization_identifier: String,
    pub api_version: String,
    pub insecure_http: bool,
}

/// Healthz
#[utoipa::path(
    path = "/healthz",
    tag = "Meta",
    responses(
        (status = 200, description="Staus OK", content_type = "application/json", body = HealthzResponse, example = json!({"status": "ok"}))
    )
)]
#[get("/healthz")]
pub async fn healthz() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(HealthzResponse {
        status: "ok".to_string(),
    }))
}

/// Healthz of the node for scheduled status
#[utoipa::path(
    path = "/schedulez",
    tag = "Meta",
    responses(
        (status = 200, description="Staus OK", content_type = "application/json", body = HealthzResponse, example = json!({"status": "ok"})),
        (status = 404, description="Staus Not OK", content_type = "application/json", body = HealthzResponse, example = json!({"status": "not ok"})),
    )
)]
#[get("/schedulez")]
pub async fn schedulez() -> Result<HttpResponse, Error> {
    let node_id = LOCAL_NODE_UUID.clone();
    let Some(node) = cluster::get_node_by_uuid(&node_id).await else {
        return Ok(HttpResponse::NotFound().json(HealthzResponse {
            status: "not ok".to_string(),
        }));
    };
    Ok(if node.scheduled && node.status == NodeStatus::Online {
        HttpResponse::Ok().json(HealthzResponse {
            status: "ok".to_string(),
        })
    } else {
        HttpResponse::NotFound().json(HealthzResponse {
            status: "not ok".to_string(),
        })
    })
}

#[get("")]
pub async fn zo_config() -> Result<HttpResponse, Error> {
    #[cfg(feature = "enterprise")]
    let sso_enabled = O2_CONFIG.dex.dex_enabled;
    #[cfg(not(feature = "enterprise"))]
    let sso_enabled = false;
    #[cfg(feature = "enterprise")]
    let native_login_enabled = O2_CONFIG.dex.native_login_enabled;
    #[cfg(not(feature = "enterprise"))]
    let native_login_enabled = true;

    #[cfg(feature = "enterprise")]
    let rbac_enabled = O2_CONFIG.openfga.enabled;
    #[cfg(not(feature = "enterprise"))]
    let rbac_enabled = false;

    #[cfg(feature = "enterprise")]
    let super_cluster_enabled = O2_CONFIG.super_cluster.enabled;
    #[cfg(not(feature = "enterprise"))]
    let super_cluster_enabled = false;

    #[cfg(feature = "enterprise")]
    let custom_logo_text = match get_logo_text().await {
        Some(data) => data,
        None => O2_CONFIG.common.custom_logo_text.clone(),
    };
    #[cfg(not(feature = "enterprise"))]
    let custom_logo_text = "".to_string();
    #[cfg(feature = "enterprise")]
    let custom_slack_url = &O2_CONFIG.common.custom_slack_url;
    #[cfg(not(feature = "enterprise"))]
    let custom_slack_url = "";
    #[cfg(feature = "enterprise")]
    let custom_docs_url = &O2_CONFIG.common.custom_docs_url;
    #[cfg(not(feature = "enterprise"))]
    let custom_docs_url = "";

    #[cfg(feature = "enterprise")]
    let logo = get_logo().await;

    #[cfg(not(feature = "enterprise"))]
    let logo = None;

    #[cfg(feature = "enterprise")]
    let custom_hide_menus = &O2_CONFIG.common.custom_hide_menus;
    #[cfg(not(feature = "enterprise"))]
    let custom_hide_menus = "";

    #[cfg(feature = "enterprise")]
    let build_type = "enterprise";
    #[cfg(not(feature = "enterprise"))]
    let build_type = "opensource";

    Ok(HttpResponse::Ok().json(ConfigResponse {
        version: VERSION.to_string(),
        instance: INSTANCE_ID.get("instance_id").unwrap().to_string(),
        commit_hash: COMMIT_HASH.to_string(),
        build_date: BUILD_DATE.to_string(),
        build_type: build_type.to_string(),
        telemetry_enabled: CONFIG.common.telemetry_enabled,
        default_fts_keys: SQL_FULL_TEXT_SEARCH_FIELDS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        default_quick_mode_fields: QUICK_MODEL_FIELDS.to_vec(),
        default_functions: DEFAULT_FUNCTIONS.to_vec(),
        sql_base64_enabled: CONFIG.common.ui_sql_base64_enabled,
        timestamp_column: CONFIG.common.column_timestamp.clone(),
        syslog_enabled: *SYSLOG_ENABLED.read(),
        data_retention_days: CONFIG.compact.data_retention_days,
        restricted_routes_on_empty_data: CONFIG.common.restricted_routes_on_empty_data,
        sso_enabled,
        native_login_enabled,
        rbac_enabled,
        super_cluster_enabled,
        query_on_stream_selection: CONFIG.common.query_on_stream_selection,
        show_stream_stats_doc_num: CONFIG.common.show_stream_dates_doc_num,
        custom_logo_text,
        custom_slack_url: custom_slack_url.to_string(),
        custom_docs_url: custom_docs_url.to_string(),
        custom_logo_img: logo,
        custom_hide_menus: custom_hide_menus.to_string(),
        rum: Rum {
            enabled: CONFIG.rum.enabled,
            client_token: CONFIG.rum.client_token.to_string(),
            application_id: CONFIG.rum.application_id.to_string(),
            site: CONFIG.rum.site.to_string(),
            service: CONFIG.rum.service.to_string(),
            env: CONFIG.rum.env.to_string(),
            version: CONFIG.rum.version.to_string(),
            organization_identifier: CONFIG.rum.organization_identifier.to_string(),
            api_version: CONFIG.rum.api_version.to_string(),
            insecure_http: CONFIG.rum.insecure_http,
        },
        meta_org: CONFIG.common.usage_org.to_string(),
        quick_mode_enabled: CONFIG.limit.quick_mode_enabled,
        user_defined_schemas_enabled: CONFIG.common.allow_user_defined_schemas,
        all_fields_name: CONFIG.common.all_fields_name.to_string(),
    }))
}

#[get("/status")]
pub async fn cache_status() -> Result<HttpResponse, Error> {
    let mut stats: HashMap<&str, json::Value> = HashMap::default();
    stats.insert("LOCAL_NODE_UUID", json::json!(LOCAL_NODE_UUID.clone()));
    stats.insert("LOCAL_NODE_NAME", json::json!(&CONFIG.common.instance_name));
    stats.insert("LOCAL_NODE_ROLE", json::json!(&CONFIG.common.node_role));
    let nodes = cluster::get_cached_online_nodes().await;
    stats.insert("NODE_LIST", json::json!(nodes));

    let (stream_num, stream_schema_num, mem_size) = get_stream_schema_status().await;
    stats.insert("STREAM_SCHEMA", json::json!({"stream_num": stream_num,"stream_schema_num": stream_schema_num, "mem_size": mem_size}));

    let stream_num = cache::stats::get_stream_stats_len();
    let mem_size = cache::stats::get_stream_stats_in_memory_size();
    stats.insert(
        "STREAM_STATS",
        json::json!({"stream_num": stream_num, "mem_size": mem_size}),
    );

    let mem_file_num = cache::file_data::memory::len().await;
    let (mem_max_size, mem_cur_size) = cache::file_data::memory::stats().await;
    let disk_file_num = cache::file_data::disk::len().await;
    let (disk_max_size, disk_cur_size) = cache::file_data::disk::stats().await;
    stats.insert(
        "FILE_DATA",
        json::json!({
            "memory":{"cache_files":mem_file_num, "cache_limit":mem_max_size,"cache_bytes": mem_cur_size},
            "disk":{"cache_files":disk_file_num, "cache_limit":disk_max_size,"cache_bytes": disk_cur_size}
        }),
    );

    let file_list_num = file_list::len().await;
    let file_list_max_id = file_list::get_max_pk_value().await.unwrap_or_default();
    stats.insert(
        "FILE_LIST",
        json::json!({"num":file_list_num,"max_id": file_list_max_id}),
    );

    let tmpfs_mem_size = cache::tmpfs::stats().unwrap();
    stats.insert("TMPFS", json::json!({ "mem_size": tmpfs_mem_size }));

    let last_file_list_offset = db::compact::file_list::get_offset().await.unwrap();
    stats.insert(
        "COMPACT",
        json::json!({"file_list_offset": last_file_list_offset}),
    );

    Ok(HttpResponse::Ok().json(stats))
}

async fn get_stream_schema_status() -> (usize, usize, usize) {
    let mut stream_num = 0;
    let mut stream_schema_num = 0;
    let mut mem_size = 0;
    let r = STREAM_SCHEMAS.read().await;
    for (key, val) in r.iter() {
        stream_num += 1;
        mem_size += std::mem::size_of::<Vec<Schema>>();
        mem_size += key.len();
        for schema in val.iter() {
            stream_schema_num += 1;
            mem_size += schema.1.size();
        }
    }
    drop(r);
    let r = STREAM_SCHEMAS_COMPRESSED.read().await;
    for (key, val) in r.iter() {
        stream_num += 1;
        mem_size += key.len();
        mem_size += val.len();
    }
    drop(r);
    let r = STREAM_SCHEMAS_LATEST.read().await;
    for (key, schema) in r.iter() {
        stream_num += 1;
        stream_schema_num += 1;
        mem_size += key.len();
        mem_size += schema.schema().size();
    }
    drop(r);
    (stream_num, stream_schema_num, mem_size)
}

#[cfg(feature = "enterprise")]
#[get("/redirect")]
pub async fn redirect(req: HttpRequest) -> Result<HttpResponse, Error> {
    use crate::common::meta::user::AuthTokens;

    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let code = match query.get("code") {
        Some(code) => code,
        None => {
            return Err(Error::new(ErrorKind::Other, "no code in request"));
        }
    };

    match query.get("state") {
        Some(code) => match crate::service::kv::get(PKCE_STATE_ORG, code).await {
            Ok(_) => {
                let _ = crate::service::kv::delete(PKCE_STATE_ORG, code).await;
            }
            Err(_) => {
                return Err(Error::new(ErrorKind::Other, "invalid state in request"));
            }
        },

        None => {
            return Err(Error::new(ErrorKind::Other, "no state in request"));
        }
    };

    log::info!("entering exchange_code: {}", code);

    match exchange_code(code).await {
        Ok(login_data) => {
            let login_url;
            let access_token = login_data.access_token;
            let keys = get_jwks().await;
            let token_ver =
                verify_decode_token(&access_token, &keys, &O2_CONFIG.dex.client_id, true).await;
            let id_token;
            match token_ver {
                Ok(res) => {
                    id_token = json::to_string(&json::json!({
                        "email": res.0.user_email,
                        "name": res.0.user_name,
                        "family_name": res.0.family_name,
                        "given_name": res.0.given_name,
                        "is_valid": res.0.is_valid,
                    }))
                    .unwrap();
                    login_url = format!(
                        "{}#id_token={}.{}",
                        login_data.url,
                        ID_TOKEN_HEADER,
                        base64::encode(&id_token)
                    );
                    process_token(res).await
                }
                Err(e) => return Ok(HttpResponse::Unauthorized().json(e.to_string())),
            }

            // generate new UUID for access token & store token in DB
            let session_id = ider::uuid();

            // store session_id in cluster co-ordinator
            let _ = crate::service::session::set_session(&session_id, &access_token).await;

            let access_token = format!("session {}", session_id);

            let tokens = json::to_string(&AuthTokens {
                access_token,
                refresh_token: login_data.refresh_token,
            })
            .unwrap();

            let mut auth_cookie = Cookie::new("auth_tokens", tokens);
            auth_cookie.set_expires(
                cookie::time::OffsetDateTime::now_utc()
                    + cookie::time::Duration::seconds(CONFIG.auth.cookie_max_age),
            );
            auth_cookie.set_http_only(true);
            auth_cookie.set_secure(CONFIG.auth.cookie_secure_only);
            auth_cookie.set_path("/");
            if CONFIG.auth.cookie_same_site_lax {
                auth_cookie.set_same_site(SameSite::Lax);
            } else {
                auth_cookie.set_same_site(SameSite::None);
            }
            log::info!("Redirecting user after processing token");
            Ok(HttpResponse::Found()
                .append_header((header::LOCATION, login_url))
                .cookie(auth_cookie)
                .finish())
        }
        Err(e) => Ok(HttpResponse::Unauthorized().json(e.to_string())),
    }
}

#[cfg(feature = "enterprise")]
#[get("/dex_login")]
pub async fn dex_login() -> Result<HttpResponse, Error> {
    use o2_enterprise::enterprise::dex::meta::auth::PreLoginData;

    let login_data: PreLoginData = get_dex_login();
    let state = login_data.state;
    let _ = crate::service::kv::set(PKCE_STATE_ORG, &state, state.to_owned().into()).await;

    Ok(HttpResponse::Ok().json(login_data.url))
}

#[cfg(feature = "enterprise")]
#[get("/dex_refresh")]
async fn refresh_token_with_dex(req: actix_web::HttpRequest) -> HttpResponse {
    let token = if let Some(cookie) = req.cookie("auth_tokens") {
        let auth_tokens: AuthTokens = json::from_str(cookie.value()).unwrap_or_default();

        // remove old session id from cluster co-ordinator

        let access_token = auth_tokens.access_token;
        if access_token.starts_with("session") {
            crate::service::session::remove_session(access_token.strip_prefix("session ").unwrap())
                .await;
        }

        auth_tokens.refresh_token
    } else {
        return HttpResponse::Unauthorized().finish();
    };

    // Exchange the refresh token for a new access token
    match refresh_token(&token).await {
        Ok((access_token, refresh_token)) => {
            // generate new UUID for access token & store token in DB
            let session_id = ider::uuid();

            // store session_id in cluster co-ordinator
            let _ = crate::service::session::set_session(&session_id, &access_token).await;

            let access_token = format!("session {}", session_id);

            let tokens = json::to_string(&AuthTokens {
                access_token,
                refresh_token,
            })
            .unwrap();

            let mut auth_cookie = Cookie::new("auth_tokens", tokens);
            auth_cookie.set_expires(
                cookie::time::OffsetDateTime::now_utc()
                    + cookie::time::Duration::seconds(CONFIG.auth.cookie_max_age),
            );
            auth_cookie.set_http_only(true);
            auth_cookie.set_secure(CONFIG.auth.cookie_secure_only);
            auth_cookie.set_path("/");
            if CONFIG.auth.cookie_same_site_lax {
                auth_cookie.set_same_site(SameSite::Lax);
            } else {
                auth_cookie.set_same_site(SameSite::None);
            }

            HttpResponse::Ok().cookie(auth_cookie).finish()
        }
        Err(_) => {
            let tokens = json::to_string(&AuthTokens::default()).unwrap();
            let mut auth_cookie = Cookie::new("auth_tokens", tokens);
            auth_cookie.set_expires(
                cookie::time::OffsetDateTime::now_utc()
                    + cookie::time::Duration::seconds(CONFIG.auth.cookie_max_age),
            );
            auth_cookie.set_http_only(true);
            auth_cookie.set_secure(CONFIG.auth.cookie_secure_only);
            auth_cookie.set_path("/");
            if CONFIG.auth.cookie_same_site_lax {
                auth_cookie.set_same_site(SameSite::Lax);
            } else {
                auth_cookie.set_same_site(SameSite::None);
            }

            HttpResponse::Unauthorized()
                .append_header((header::LOCATION, "/"))
                .cookie(auth_cookie)
                .finish()
        }
    }
}

#[get("/logout")]
async fn logout(req: actix_web::HttpRequest) -> HttpResponse {
    // remove the session

    if let Some(cookie) = req.cookie("auth_tokens") {
        let auth_tokens: AuthTokens = json::from_str(cookie.value()).unwrap_or_default();
        let access_token = auth_tokens.access_token;
        if access_token.starts_with("session") {
            crate::service::session::remove_session(access_token.strip_prefix("session ").unwrap())
                .await;
        }
    };

    let tokens = json::to_string(&AuthTokens::default()).unwrap();
    let mut auth_cookie = Cookie::new("auth_tokens", tokens);
    auth_cookie.set_expires(
        cookie::time::OffsetDateTime::now_utc()
            + cookie::time::Duration::seconds(CONFIG.auth.cookie_max_age),
    );
    auth_cookie.set_http_only(true);
    auth_cookie.set_secure(CONFIG.auth.cookie_secure_only);
    auth_cookie.set_path("/");
    if CONFIG.auth.cookie_same_site_lax {
        auth_cookie.set_same_site(SameSite::Lax);
    } else {
        auth_cookie.set_same_site(SameSite::None);
    }

    HttpResponse::Ok()
        .append_header((header::LOCATION, "/"))
        .cookie(auth_cookie)
        .finish()
}

#[put("/enable")]
async fn enable_node(req: HttpRequest) -> Result<HttpResponse, Error> {
    let node_id = LOCAL_NODE_UUID.clone();
    let Some(mut node) = cluster::get_node_by_uuid(&node_id).await else {
        return Ok(MetaHttpResponse::not_found("node not found"));
    };

    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let enable = match query.get("value") {
        Some(v) => v.parse::<bool>().unwrap_or_default(),
        None => false,
    };
    node.scheduled = enable;
    match cluster::update_local_node(&node).await {
        Ok(_) => Ok(MetaHttpResponse::json(true)),
        Err(e) => Ok(MetaHttpResponse::internal_error(e)),
    }
}

#[put("/flush")]
async fn flush_node() -> Result<HttpResponse, Error> {
    if !is_ingester(&LOCAL_NODE_ROLE) {
        return Ok(MetaHttpResponse::not_found("local node is not an ingester"));
    };

    match ingester::flush_all().await {
        Ok(_) => Ok(MetaHttpResponse::json(true)),
        Err(e) => Ok(MetaHttpResponse::internal_error(e)),
    }
}

#[get("/stream_fields/{org_id}/{stream_type}/{stream_name}")]
async fn stream_fields(path: web::Path<(String, String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, stream_type, stream_name) = path.into_inner();
    let key = format!("{}/{}/{}", org_id, stream_type, stream_name);
    let r = STREAM_SCHEMAS_FIELDS.read().await;
    Ok(MetaHttpResponse::json(match r.get(&key) {
        Some((updated, fields)) => json::json!({"updated_at": *updated, "fields": fields}),
        None => json::json!({"updated_at": 0, "fields": []}),
    }))
}
