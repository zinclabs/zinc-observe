// Copyright 2023 Zinc Labs Inc.
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

use std::{path::Path, sync::Arc, time::Duration};

use ahash::{AHashMap, AHashSet};
use dashmap::{DashMap, DashSet};
use datafusion::arrow::datatypes::Schema;
use dotenv_config::EnvConfig;
use dotenvy::dotenv;
use itertools::chain;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use reqwest::Client;
use sysinfo::{DiskExt, SystemExt};
use tokio::sync::RwLock as TRwLock;
use vector_enrichment::TableRegistry;

use crate::{
    common::{
        meta::{
            alerts,
            functions::{StreamFunctionsList, Transform},
            maxmind::MaxmindClient,
            organization::OrganizationSetting,
            prom::ClusterLeader,
            syslog::SyslogRoute,
            user::User,
        },
        utils::{cgroup, file::get_file_meta},
    },
    service::{enrichment::StreamTable, enrichment_table::geoip::Geoip},
};

pub type FxIndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;
pub type FxIndexSet<K> = indexmap::IndexSet<K, ahash::RandomState>;
pub type RwHashMap<K, V> = DashMap<K, V, ahash::RandomState>;
pub type RwHashSet<K> = DashSet<K, ahash::RandomState>;
pub type RwAHashMap<K, V> = tokio::sync::RwLock<AHashMap<K, V>>;
pub type RwAHashSet<K> = tokio::sync::RwLock<AHashSet<K>>;

pub static VERSION: &str = env!("GIT_VERSION");
pub static COMMIT_HASH: &str = env!("GIT_COMMIT_HASH");
pub static BUILD_DATE: &str = env!("GIT_BUILD_DATE");

pub const MMDB_CITY_FILE_NAME: &str = "GeoLite2-City.mmdb";
pub const MMDB_ASN_FILE_NAME: &str = "GeoLite2-ASN.mmdb";
pub const GEO_IP_CITY_ENRICHMENT_TABLE: &str = "maxmind_city";
pub const GEO_IP_ASN_ENRICHMENT_TABLE: &str = "maxmind_asn";

pub const SIZE_IN_MB: f64 = 1024.0 * 1024.0;
pub const PARQUET_BATCH_SIZE: usize = 8 * 1024;
pub const PARQUET_PAGE_SIZE: usize = 1024 * 1024;
pub const PARQUET_MAX_ROW_GROUP_SIZE: usize = 1024 * 1024;

pub const HAS_FUNCTIONS: bool = true;
pub const FILE_EXT_JSON: &str = ".json";
pub const FILE_EXT_ARROW: &str = ".arrow";
pub const FILE_EXT_PARQUET: &str = ".parquet";

const _DEFAULT_SQL_FULL_TEXT_SEARCH_FIELDS: [&str; 7] =
    ["log", "message", "msg", "content", "data", "events", "json"];
pub static SQL_FULL_TEXT_SEARCH_FIELDS: Lazy<Vec<String>> = Lazy::new(|| {
    chain(
        _DEFAULT_SQL_FULL_TEXT_SEARCH_FIELDS
            .iter()
            .map(|s| s.to_string()),
        CONFIG
            .common
            .feature_fulltext_extra_fields
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            }),
    )
    .collect()
});

const _DEFAULT_DISTINCT_FIELDS: [&str; 2] = ["service_name", "operation_name"];
pub static DISTINCT_FIELDS: Lazy<Vec<String>> = Lazy::new(|| {
    chain(
        _DEFAULT_DISTINCT_FIELDS.iter().map(|s| s.to_string()),
        CONFIG
            .common
            .feature_distinct_extra_fields
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            }),
    )
    .collect()
});

const _DEFAULT_BLOOM_FILTER_FIELDS: [&str; 1] = ["trace_id"];
pub static BLOOM_FILTER_DEFAULT_FIELDS: Lazy<Vec<String>> = Lazy::new(|| {
    chain(
        _DEFAULT_BLOOM_FILTER_FIELDS.iter().map(|s| s.to_string()),
        CONFIG
            .common
            .bloom_filter_default_fields
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            }),
    )
    .collect()
});

pub static CONFIG: Lazy<Config> = Lazy::new(init);
pub static INSTANCE_ID: Lazy<RwHashMap<String, String>> = Lazy::new(Default::default);

pub static TELEMETRY_CLIENT: Lazy<segment::HttpClient> = Lazy::new(|| {
    segment::HttpClient::new(
        Client::builder()
            .connect_timeout(Duration::new(10, 0))
            .build()
            .unwrap(),
        CONFIG.common.telemetry_url.clone(),
    )
});

// global cache variables
pub static KVS: Lazy<RwHashMap<String, bytes::Bytes>> = Lazy::new(Default::default);
pub static STREAM_SCHEMAS: Lazy<RwHashMap<String, Vec<Schema>>> = Lazy::new(Default::default);
pub static STREAM_FUNCTIONS: Lazy<RwHashMap<String, StreamFunctionsList>> =
    Lazy::new(DashMap::default);
pub static QUERY_FUNCTIONS: Lazy<RwHashMap<String, Transform>> = Lazy::new(DashMap::default);
pub static USERS: Lazy<RwHashMap<String, User>> = Lazy::new(DashMap::default);
pub static USERS_RUM_TOKEN: Lazy<Arc<RwHashMap<String, User>>> =
    Lazy::new(|| Arc::new(DashMap::default()));
pub static ROOT_USER: Lazy<RwHashMap<String, User>> = Lazy::new(DashMap::default);
pub static ORGANIZATION_SETTING: Lazy<Arc<RwAHashMap<String, OrganizationSetting>>> =
    Lazy::new(|| Arc::new(tokio::sync::RwLock::new(AHashMap::new())));
pub static PASSWORD_HASH: Lazy<RwHashMap<String, String>> = Lazy::new(DashMap::default);
pub static METRIC_CLUSTER_MAP: Lazy<Arc<RwAHashMap<String, Vec<String>>>> =
    Lazy::new(|| Arc::new(tokio::sync::RwLock::new(AHashMap::new())));
pub static METRIC_CLUSTER_LEADER: Lazy<Arc<RwAHashMap<String, ClusterLeader>>> =
    Lazy::new(|| Arc::new(tokio::sync::RwLock::new(AHashMap::new())));
pub static STREAM_ALERTS: Lazy<RwAHashMap<String, Vec<alerts::Alert>>> =
    Lazy::new(Default::default);
pub static TRIGGERS: Lazy<RwAHashMap<String, alerts::triggers::Trigger>> =
    Lazy::new(Default::default);
pub static ALERTS_TEMPLATES: Lazy<RwHashMap<String, alerts::templates::Template>> =
    Lazy::new(Default::default);
pub static ALERTS_DESTINATIONS: Lazy<RwHashMap<String, alerts::destinations::Destination>> =
    Lazy::new(Default::default);
pub static SYSLOG_ROUTES: Lazy<RwHashMap<String, SyslogRoute>> = Lazy::new(Default::default);
pub static SYSLOG_ENABLED: Lazy<Arc<RwLock<bool>>> = Lazy::new(|| Arc::new(RwLock::new(false)));
pub static ENRICHMENT_TABLES: Lazy<RwHashMap<String, StreamTable>> = Lazy::new(Default::default);
pub static ENRICHMENT_REGISTRY: Lazy<Arc<TableRegistry>> =
    Lazy::new(|| Arc::new(TableRegistry::default()));
pub static LOCAL_SCHEMA_LOCKER: Lazy<Arc<RwAHashMap<String, tokio::sync::RwLock<bool>>>> =
    Lazy::new(|| Arc::new(Default::default)());

pub static MAXMIND_DB_CLIENT: Lazy<Arc<TRwLock<Option<MaxmindClient>>>> =
    Lazy::new(|| Arc::new(TRwLock::new(None)));

pub static GEOIP_CITY_TABLE: Lazy<Arc<RwLock<Option<Geoip>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

pub static GEOIP_ASN_TABLE: Lazy<Arc<RwLock<Option<Geoip>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

#[derive(EnvConfig)]
pub struct Config {
    pub auth: Auth,
    pub http: Http,
    pub grpc: Grpc,
    pub route: Route,
    pub common: Common,
    pub limit: Limit,
    pub compact: Compact,
    pub memory_cache: MemoryCache,
    pub disk_cache: DiskCache,
    pub log: Log,
    pub etcd: Etcd,
    pub sled: Sled,
    pub dynamo: Dynamo,
    pub s3: S3,
    pub tcp: TCP,
    pub prom: Prometheus,
    pub profiling: Pyroscope,
}

/// Render the help string and default value for all the available
/// environment variables in O2.
pub fn render_help() {
    let fields = Config::get_help();
    for (k, v) in fields.iter() {
        if k.is_empty() {
            continue;
        }
        println!(
            "{:<38}: {} [DEFAULT: {}]",
            k,
            v.1.as_deref().unwrap_or(""),
            &v.0,
        );
    }
}

#[derive(EnvConfig)]
pub struct Pyroscope {
    #[env_config(
        name = "ZO_PROF_PYROSCOPE_SERVER_URL",
        default = "http://localhost:4040",
        help = "Default pyroscope server url"
    )]
    pub pyroscope_server_url: String,
    #[env_config(
        name = "ZO_PROF_PYROSCOPE_PROJECT_NAME",
        default = "openobserve",
        help = "A uniquely identifiable pyroscope project name"
    )]
    pub pyroscope_project_name: String,
}

#[derive(EnvConfig)]
pub struct Auth {
    #[env_config(name = "ZO_ROOT_USER_EMAIL", help = "Email of first/super admin user")]
    pub root_user_email: String,
    #[env_config(
        name = "ZO_ROOT_USER_PASSWORD",
        help = "Password for first/super admin user"
    )]
    pub root_user_password: String,
}

#[derive(EnvConfig)]
pub struct Http {
    #[env_config(
        name = "ZO_HTTP_PORT",
        default = 5080,
        help = "openobserve server listen HTTP port"
    )]
    pub port: u16,
    #[env_config(
        name = "ZO_HTTP_ADDR",
        default = "",
        help = "openobserve server listen HTTP ip address"
    )]
    pub addr: String,
    #[env_config(
        name = "ZO_HTTP_IPV6_ENABLED",
        default = false,
        help = "enable ipv6 support for HTTP"
    )]
    pub ipv6_enabled: bool,
}

#[derive(EnvConfig)]
pub struct Grpc {
    #[env_config(
        name = "ZO_GRPC_PORT",
        default = 5081,
        help = "openobserve server listen gRPC port"
    )]
    pub port: u16,
    #[env_config(
        name = "ZO_GRPC_ADDR",
        default = "",
        help = "openobserve server listen gRPC ip address"
    )]
    pub addr: String,
    #[env_config(
        name = "ZO_GRPC_ORG_HEADER_KEY",
        default = "organization",
        help = "header key for sending organization 
    information for traces using OTLP over grpc"
    )]
    pub org_header_key: String,
    #[env_config(name = "ZO_GRPC_STREAM_HEADER_KEY", default = "stream-name")]
    pub stream_header_key: String,
    #[env_config(name = "ZO_INTERNAL_GRPC_TOKEN", default = "")]
    pub internal_grpc_token: String,
}

#[derive(EnvConfig)]
pub struct TCP {
    #[env_config(name = "ZO_TCP_PORT", default = 5514)]
    pub tcp_port: u16,
    #[env_config(name = "ZO_UDP_PORT", default = 5514)]
    pub udp_port: u16,
}

#[derive(EnvConfig)]
pub struct Route {
    #[env_config(
        name = "ZO_ROUTE_TIMEOUT",
        default = 600,
        help = "timeout for router node."
    )]
    pub timeout: u64,
    // zo1-openobserve-ingester.ziox-dev.svc.cluster.local
    #[env_config(name = "ZO_INGESTER_SERVICE_URL", default = "")]
    pub ingester_srv_url: String,
}

#[derive(EnvConfig)]
pub struct Common {
    #[env_config(name = "ZO_APP_NAME", default = "openobserve")]
    pub app_name: String,
    #[env_config(
        name = "ZO_LOCAL_MODE",
        default = true,
        help = "If local mode is set to true ,OpenObserve becomes single node deployment, 
        false indicates cluster mode deployment which supports multiple nodes with different roles. 
        For local mode one needs to configure sled db, for cluster mode one needs to config etcd."
    )]
    pub local_mode: bool,
    // ZO_LOCAL_MODE_STORAGE is ignored when ZO_LOCAL_MODE is set to false
    #[env_config(
        name = "ZO_LOCAL_MODE_STORAGE",
        default = "disk",
        help = "disk or s3, Applicable only for local mode , by default local disk is used as 
        storage, we also support s3 in local mode."
    )]
    pub local_mode_storage: String,
    #[env_config(
        name = "ZO_META_STORE",
        default = "",
        help = "Default is sqlite for local mode, etcd for cluster mode. 
    and supported values are: sqlite, etcd, postgres, dynamodb, the sqlite only support for local mode."
    )]
    pub meta_store: String,
    pub meta_store_external: bool, // external storage no need sync file_list to s3
    #[env_config(
        name = "ZO_META_POSTGRES_DSN",
        default = "",
        help = "If you enable postgres as meta store, you need configure the database source address, 
        like this: postgres://postgres:12345678@localhost:5432/openobserve"
    )]
    pub meta_postgres_dsn: String, // postgres://postgres:12345678@localhost:5432/openobserve
    #[env_config(name = "ZO_META_MYSQL_DSN", default = "")]
    pub meta_mysql_dsn: String, // mysql://root:12345678@localhost:3306/openobserve
    #[env_config(
        name = "ZO_NODE_ROLE",
        default = "all",
        help = "Possible values are : all, ingester, querier, router, compactor, alertmanager, 
        A single node can have multiple roles id desired. Specify roles separated by comma. e.g. compactor,alertmanager"
    )]
    pub node_role: String,
    #[env_config(name = "ZO_CLUSTER_NAME", default = "zo1")]
    pub cluster_name: String,
    #[env_config(
        name = "ZO_INSTANCE_NAME",
        default = "",
        help = "in the cluster mode, each node has a instance name, 
    default is instance hostname."
    )]
    pub instance_name: String,
    #[env_config(name = "ZO_INGESTER_SIDECAR_ENABLED", default = false)]
    pub ingester_sidecar_enabled: bool,
    #[env_config(name = "ZO_INGESTER_SIDECAR_QUERIER", default = false)]
    pub ingester_sidecar_querier: bool,
    #[env_config(
        name = "ZO_DATA_DIR",
        default = "./data/openobserve/",
        help = "On-disk data directory, where openobserve stores everything."
    )]
    pub data_dir: String,
    #[env_config(
        name = "ZO_DATA_WAL_DIR",
        default = "",
        help = "local WAL data directory."
    )] // ./data/openobserve/wal/
    pub data_wal_dir: String,
    #[env_config(
        name = "ZO_DATA_STREAM_DIR",
        default = "",
        help = "streams local data storage directory ,applicable only for local mode."
    )] // ./data/openobserve/stream/
    pub data_stream_dir: String,
    #[env_config(
        name = "ZO_DATA_DB_DIR",
        default = "",
        help = "metadata database local storage directory."
    )] // ./data/openobserve/db/
    pub data_db_dir: String,
    #[env_config(
        name = "ZO_DATA_CACHE_DIR",
        default = "",
        help = "local query cache storage directory, applicable only for cluster mode."
    )] // ./data/openobserve/cache/
    pub data_cache_dir: String,
    #[env_config(
        name = "ZO_BASE_URI",
        default = "",
        help = "if you set OpenObserve with a prefix in k8s nginx ingress, you can set the prefix path."
    )]
    pub base_uri: String,
    #[env_config(
        name = "ZO_WAL_MEMORY_MODE_ENABLED",
        default = false,
        help = "For performance, we can write WAL file into memory instead of write into disk, this will increase 
        ingestion performance, but it has dast lose risk when the system crashed."
    )]
    pub wal_memory_mode_enabled: bool,
    #[env_config(
        name = "ZO_WAL_LINE_MODE_ENABLED",
        default = true,
        help = "Default we write WAL file line by line, it is a bit slow but it safety, you can disable it to increase 
        a bit performance, but it increase WAL file incorrect risk."
    )]
    pub wal_line_mode_enabled: bool,
    #[env_config(
        name = "ZO_COLUMN_TIMESTAMP",
        default = "_timestamp",
        help = "for each log line, if not present with this key , we add a 
    timestamp with this key, used for queries with time range."
    )]
    pub column_timestamp: String,
    #[env_config(
        name = "ZO_WIDENING_SCHEMA_EVOLUTION",
        default = true,
        help = "if set to false user can add new columns to data being ingested 
    but changes to existing data for data type are not supported "
    )]
    pub widening_schema_evolution: bool,
    #[env_config(
        name = "ZO_SKIP_SCHEMA_VALIDATION",
        default = false,
        help = "Default we check ingested every record for schema validation, but if your schema is fixed, you can skip it, 
    this will increase 2x ingestion performance."
    )]
    pub skip_schema_validation: bool,
    #[env_config(
        name = "ZO_FEATURE_PER_THREAD_LOCK",
        default = false,
        help = "Default we check ingested every record for schema validation, but if your schema is fixed, you can skip it, 
    this will increase 2x ingestion performance."
    )]
    pub feature_per_thread_lock: bool,
    #[env_config(
        name = "ZO_FEATURE_FULLTEXT_ON_ALL_FIELDS",
        default = false,
        help = "default full text search uses log, message, msg, content, data, events, json or selected stream fields. 
        Enabling this option will perform full text search on each field, may hamper full text search performance"
    )]
    pub feature_fulltext_on_all_fields: bool,
    #[env_config(
        name = "ZO_FEATURE_FULLTEXT_EXTRA_FIELDS",
        default = "",
        help = "default full text search uses log, message, msg, content, data, events, json as global setting, 
        but you can add more fields as global full text search fields. eg: field1,field2"
    )]
    pub feature_fulltext_extra_fields: String,
    #[env_config(name = "ZO_FEATURE_DISTINCT_EXTRA_FIELDS", default = "")]
    pub feature_distinct_extra_fields: String,
    #[env_config(name = "ZO_FEATURE_FILELIST_DEDUP_ENABLED", default = false)]
    pub feature_filelist_dedup_enabled: bool,
    #[env_config(name = "ZO_FEATURE_QUERY_QUEUE_ENABLED", default = true)]
    pub feature_query_queue_enabled: bool,
    #[env_config(
        name = "ZO_UI_ENABLED",
        default = true,
        help = "default we enable embed UI, one can disable it."
    )]
    pub ui_enabled: bool,
    #[env_config(
        name = "ZO_UI_SQL_BASE64_ENABLED",
        default = false,
        help = "Enable base64 encoding for SQL in UI."
    )]
    pub ui_sql_base64_enabled: bool,
    #[env_config(name = "ZO_METRICS_DEDUP_ENABLED", default = true)]
    pub metrics_dedup_enabled: bool,
    #[env_config(name = "ZO_BLOOM_FILTER_ENABLED", default = true)]
    pub bloom_filter_enabled: bool,
    #[env_config(name = "ZO_BLOOM_FILTER_DEFAULT_FIELDS", default = "")]
    pub bloom_filter_default_fields: String,
    #[env_config(name = "ZO_TRACING_ENABLED", default = false)]
    pub tracing_enabled: bool,
    #[env_config(name = "OTEL_OTLP_HTTP_ENDPOINT", default = "")]
    pub otel_otlp_url: String,
    #[env_config(name = "ZO_TRACING_HEADER_KEY", default = "Authorization")]
    pub tracing_header_key: String,
    #[env_config(
        name = "ZO_TRACING_HEADER_VALUE",
        default = "Basic YWRtaW46Q29tcGxleHBhc3MjMTIz"
    )]
    pub tracing_header_value: String,
    #[env_config(name = "ZO_TELEMETRY", default = true)]
    pub telemetry_enabled: bool,
    #[env_config(name = "ZO_TELEMETRY_URL", default = "https://e1.zinclabs.dev")]
    pub telemetry_url: String,
    #[env_config(name = "ZO_PROMETHEUS_ENABLED", default = true)]
    pub prometheus_enabled: bool,
    #[env_config(name = "ZO_PRINT_KEY_CONFIG", default = false)]
    pub print_key_config: bool,
    #[env_config(name = "ZO_PRINT_KEY_EVENT", default = false)]
    pub print_key_event: bool,
    #[env_config(name = "ZO_PRINT_KEY_SQL", default = false)]
    pub print_key_sql: bool,
    #[env_config(name = "ZO_USAGE_REPORTING_ENABLED", default = false)]
    pub usage_enabled: bool,
    #[env_config(name = "ZO_USAGE_REPORTING_COMPRESSED_SIZE", default = false)]
    pub usage_report_compressed_size: bool,
    #[env_config(name = "ZO_USAGE_ORG", default = "_meta")]
    pub usage_org: String,
    #[env_config(name = "ZO_USAGE_BATCH_SIZE", default = 2000)]
    pub usage_batch_size: usize,
    #[env_config(name = "ZO_MMDB_DATA_DIR")] // ./data/openobserve/mmdb/
    pub mmdb_data_dir: String,
    #[env_config(name = "ZO_MMDB_DISABLE_DOWNLOAD", default = "false")]
    pub mmdb_disable_download: bool,
    #[env_config(name = "ZO_MMDB_UPDATE_DURATION", default = "86400")] // Everyday to test
    pub mmdb_update_duration: u64,

    #[env_config(
        name = "ZO_MMDB_GEOLITE_CITYDB_URL",
        default = "https://geoip.zinclabs.dev/GeoLite2-City.mmdb"
    )]
    pub mmdb_geolite_citydb_url: String,

    #[env_config(
        name = "ZO_MMDB_GEOLITE_ASNDB_URL",
        default = "https://geoip.zinclabs.dev/GeoLite2-ASN.mmdb"
    )]
    pub mmdb_geolite_asndb_url: String,

    #[env_config(
        name = "ZO_MMDB_GEOLITE_CITYDB_SHA256_URL",
        default = "https://geoip.zinclabs.dev/GeoLite2-City.sha256"
    )]
    pub mmdb_geolite_citydb_sha256_url: String,

    #[env_config(
        name = "ZO_MMDB_GEOLITE_CITYDB_SHA256_URL",
        default = "https://geoip.zinclabs.dev/GeoLite2-ASN.sha256"
    )]
    pub mmdb_geolite_asndb_sha256_url: String,

    #[env_config(name = "ZO_DEFAULT_SCRAPE_INTERVAL", default = 15)]
    // Default scrape_interval value 15s
    pub default_scrape_interval: u32,
    #[env_config(name = "ZO_CIRCUIT_BREAKER_ENABLE", default = false)]
    pub memory_circuit_breaker_enable: bool,
    #[env_config(name = "ZO_CIRCUIT_BREAKER_RATIO", default = 100)]
    pub memory_circuit_breaker_ratio: usize,

    #[env_config(
        name = "ZO_RESTRICTED_ROUTES_ON_EMPTY_DATA",
        default = true,
        help = "Control the redirection of a user to ingestion page in case there is no stream found."
    )]
    pub restricted_routes_on_empty_data: bool,
}

#[derive(EnvConfig)]
pub struct Limit {
    // no need set by environment
    pub cpu_num: usize,
    pub mem_total: usize,
    pub disk_total: usize,
    pub disk_free: usize,
    #[env_config(name = "ZO_JSON_LIMIT", default = 209715200)]
    pub req_json_limit: usize,
    #[env_config(name = "ZO_PAYLOAD_LIMIT", default = 209715200)]
    pub req_payload_limit: usize,
    #[env_config(name = "ZO_MAX_FILE_SIZE_ON_DISK", default = 32)] // MB
    pub max_file_size_on_disk: u64,
    #[env_config(name = "ZO_MAX_FILE_RETENTION_TIME", default = 600)] // seconds
    pub max_file_retention_time: u64,
    #[env_config(name = "ZO_FILE_PUSH_INTERVAL", default = 10)] // seconds
    pub file_push_interval: u64,
    #[env_config(name = "ZO_FILE_MOVE_THREAD_NUM", default = 0)]
    pub file_move_thread_num: usize,
    #[env_config(name = "ZO_QUERY_THREAD_NUM", default = 0)]
    pub query_thread_num: usize,
    #[env_config(name = "ZO_QUERY_TIMEOUT", default = 600)]
    pub query_timeout: u64,
    #[env_config(name = "ZO_INGEST_ALLOWED_UPTO", default = 5)] // in hours - in past
    pub ingest_allowed_upto: i64,
    #[env_config(name = "ZO_IGNORE_FILE_RETENTION_BY_STREAM", default = false)]
    pub ignore_file_retention_by_stream: bool,
    #[env_config(name = "ZO_LOGS_FILE_RETENTION", default = "hourly")]
    pub logs_file_retention: String,
    #[env_config(name = "ZO_TRACES_FILE_RETENTION", default = "hourly")]
    pub traces_file_retention: String,
    #[env_config(name = "ZO_METRICS_FILE_RETENTION", default = "daily")]
    pub metrics_file_retention: String,
    #[env_config(name = "ZO_METRICS_LEADER_PUSH_INTERVAL", default = 15)]
    pub metrics_leader_push_interval: u64,
    #[env_config(name = "ZO_METRICS_LEADER_ELECTION_INTERVAL", default = 30)]
    pub metrics_leader_election_interval: i64,
    #[env_config(name = "ZO_HEARTBEAT_INTERVAL", default = 30)] // in minutes
    pub hb_interval: i64,
    #[env_config(name = "ZO_COLS_PER_RECORD_LIMIT", default = 1000)]
    pub req_cols_per_record_limit: usize,
    #[env_config(name = "ZO_HTTP_WORKER_NUM", default = 0)] // equals to cpu_num if 0
    pub http_worker_num: usize,
    #[env_config(name = "ZO_HTTP_WORKER_MAX_BLOCKING", default = 0)] // equals to 1024 if 0
    pub http_worker_max_blocking: usize,
    #[env_config(name = "ZO_CALCULATE_STATS_INTERVAL", default = 600)] // in seconds
    pub calculate_stats_interval: u64,
    #[env_config(name = "ZO_ENRICHMENT_TABLE_LIMIT", default = 10)] // size in mb
    pub enrichment_table_limit: usize,
    #[env_config(name = "ZO_ACTIX_REQ_TIMEOUT", default = 30)] // in second
    pub request_timeout: u64,
    #[env_config(name = "ZO_ACTIX_KEEP_ALIVE", default = 30)] // in second
    pub keep_alive: u64,
}

#[derive(EnvConfig)]
pub struct Compact {
    #[env_config(name = "ZO_COMPACT_ENABLED", default = true)]
    pub enabled: bool,
    #[env_config(name = "ZO_COMPACT_INTERVAL", default = 60)] // seconds
    pub interval: u64,
    #[env_config(name = "ZO_COMPACT_STEP_SECS", default = 3600)] // seconds
    pub step_secs: i64,
    #[env_config(name = "ZO_COMPACT_SYNC_TO_DB_INTERVAL", default = 1800)] // seconds
    pub sync_to_db_interval: u64,
    #[env_config(name = "ZO_COMPACT_MAX_FILE_SIZE", default = 256)] // MB
    pub max_file_size: u64,
    #[env_config(name = "ZO_COMPACT_DATA_RETENTION_DAYS", default = 3650)] // days
    pub data_retention_days: i64,
    #[env_config(name = "ZO_COMPACT_DELETE_FILES_DELAY_HOURS", default = 2)] // hours
    pub delete_files_delay_hours: i64,
    #[env_config(name = "ZO_COMPACT_BLOCKED_ORGS", default = "")] // use comma to split
    pub blocked_orgs: String,
}

#[derive(EnvConfig)]
pub struct MemoryCache {
    #[env_config(name = "ZO_MEMORY_CACHE_ENABLED", default = true)]
    pub enabled: bool,
    #[env_config(name = "ZO_MEMORY_CACHE_CACHE_LATEST_FILES", default = false)]
    pub cache_latest_files: bool,
    // MB, default is 50% of system memory
    #[env_config(name = "ZO_MEMORY_CACHE_MAX_SIZE", default = 0)]
    pub max_size: usize,
    // MB, will skip the cache when a query need cache great than this value, default is 80% of
    // max_size
    #[env_config(name = "ZO_MEMORY_CACHE_SKIP_SIZE", default = 0)]
    pub skip_size: usize,
    // MB, when cache is full will release how many data once time, default is 1% of max_size
    #[env_config(name = "ZO_MEMORY_CACHE_RELEASE_SIZE", default = 0)]
    pub release_size: usize,
    // MB, default is 50% of system memory
    #[env_config(name = "ZO_MEMORY_CACHE_DATAFUSION_MAX_SIZE", default = 0)]
    pub datafusion_max_size: usize,
    #[env_config(name = "ZO_MEMORY_CACHE_DATAFUSION_MEMORY_POOL", default = "")]
    pub datafusion_memory_pool: String,
}

#[derive(EnvConfig)]
pub struct DiskCache {
    #[env_config(name = "ZO_DISK_CACHE_ENABLED", default = true)]
    pub enabled: bool,
    // MB, default is 50% of local volume available space and maximum 100GB
    #[env_config(name = "ZO_DISK_CACHE_MAX_SIZE", default = 0)]
    pub max_size: usize,
    // MB, will skip the cache when a query need cache great than this value, default is 80% of
    // max_size
    #[env_config(name = "ZO_DISK_CACHE_SKIP_SIZE", default = 0)]
    pub skip_size: usize,
    // MB, when cache is full will release how many data once time, default is 1% of max_size
    #[env_config(name = "ZO_DISK_CACHE_RELEASE_SIZE", default = 0)]
    pub release_size: usize,
}

#[derive(EnvConfig)]
pub struct Log {
    #[env_config(name = "RUST_LOG", default = "info")]
    pub level: String,
    #[env_config(name = "ZO_LOG_JSON_FORMAT", default = false)]
    pub json_format: bool,
    #[env_config(name = "ZO_LOG_FILE_DIR", default = "")]
    pub file_dir: String,
    // default is: o2.{hostname}.log
    #[env_config(name = "ZO_LOG_FILE_NAME_PREFIX", default = "")]
    pub file_name_prefix: String,
    // logger timestamp local setup, eg: %Y-%m-%dT%H:%M:%SZ
    #[env_config(name = "ZO_LOG_LOCAL_TIME_FORMAT", default = "")]
    pub local_time_format: String,
    #[env_config(name = "ZO_EVENTS_ENABLED", default = false)]
    pub events_enabled: bool,
    #[env_config(
        name = "ZO_EVENTS_AUTH",
        default = "cm9vdEBleGFtcGxlLmNvbTpUZ0ZzZFpzTUZQdzg2SzRK"
    )]
    pub events_auth: String,
    #[env_config(
        name = "ZO_EVENTS_EP",
        default = "https://api.openobserve.ai/api/debug/events/_json"
    )]
    pub events_url: String,
    #[env_config(name = "ZO_EVENTS_BATCH_SIZE", default = 10)]
    pub events_batch_size: usize,
}

#[derive(Debug, EnvConfig)]
pub struct Etcd {
    #[env_config(name = "ZO_ETCD_ADDR", default = "localhost:2379")]
    pub addr: String,
    #[env_config(name = "ZO_ETCD_PREFIX", default = "/zinc/observe/")]
    pub prefix: String,
    #[env_config(name = "ZO_ETCD_CONNECT_TIMEOUT", default = 5)]
    pub connect_timeout: u64,
    #[env_config(name = "ZO_ETCD_COMMAND_TIMEOUT", default = 5)]
    pub command_timeout: u64,
    #[env_config(name = "ZO_ETCD_LOCK_WAIT_TIMEOUT", default = 3600)]
    pub lock_wait_timeout: u64,
    #[env_config(name = "ZO_ETCD_USER", default = "")]
    pub user: String,
    #[env_config(name = "ZO_ETCD_PASSWORD", default = "")]
    pub password: String,
    #[env_config(name = "ZO_ETCD_CLIENT_CERT_AUTH", default = false)]
    pub cert_auth: bool,
    #[env_config(name = "ZO_ETCD_TRUSTED_CA_FILE", default = "")]
    pub ca_file: String,
    #[env_config(name = "ZO_ETCD_CERT_FILE", default = "")]
    pub cert_file: String,
    #[env_config(name = "ZO_ETCD_KEY_FILE", default = "")]
    pub key_file: String,
    #[env_config(name = "ZO_ETCD_DOMAIN_NAME", default = "")]
    pub domain_name: String,
    #[env_config(name = "ZO_ETCD_LOAD_PAGE_SIZE", default = 1000)]
    pub load_page_size: i64,
    #[env_config(name = "ZO_ETCD_NODE_HEARTBEAT_TTL", default = 10)]
    pub node_heartbeat_ttl: i64,
}

#[derive(EnvConfig)]
pub struct Sled {
    #[env_config(name = "ZO_SLED_DATA_DIR", default = "")] // ./data/openobserve/db/
    pub data_dir: String,
    #[env_config(name = "ZO_SLED_PREFIX", default = "/zinc/observe/")]
    pub prefix: String,
}

#[derive(EnvConfig)]
pub struct Dynamo {
    #[env_config(
        name = "ZO_META_DYNAMO_PREFIX",
        default = "",
        help = "If you enable dynamodb as meta store, you need configure DynamoDB 
    table prefix, default use s3 bucket name."
    )] // default set to s3 bucket name
    pub prefix: String,
    pub file_list_table: String,
    pub file_list_deleted_table: String,
    pub stream_stats_table: String,
    pub org_meta_table: String,
    pub meta_table: String,
    pub schema_table: String,
    pub compact_table: String,
}

#[derive(Debug, EnvConfig)]
pub struct S3 {
    #[env_config(name = "ZO_S3_PROVIDER", default = "")]
    pub provider: String,
    #[env_config(name = "ZO_S3_SERVER_URL", default = "")]
    pub server_url: String,
    #[env_config(name = "ZO_S3_REGION_NAME", default = "")]
    pub region_name: String,
    #[env_config(name = "ZO_S3_ACCESS_KEY", default = "")]
    pub access_key: String,
    #[env_config(name = "ZO_S3_SECRET_KEY", default = "")]
    pub secret_key: String,
    #[env_config(name = "ZO_S3_BUCKET_NAME", default = "")]
    pub bucket_name: String,
    #[env_config(name = "ZO_S3_BUCKET_PREFIX", default = "")]
    pub bucket_prefix: String,
    #[env_config(name = "ZO_S3_CONNECT_TIMEOUT", default = 10)] // seconds
    pub connect_timeout: u64,
    #[env_config(name = "ZO_S3_REQUEST_TIMEOUT", default = 3600)] // seconds
    pub request_timeout: u64,
    // Deprecated, use ZO_S3_FEATURE_FORCE_HOSTED_STYLE instead
    // #[deprecated(since = "0.6.5", note = "use `ZO_S3_FEATURE_FORCE_HOSTED_STYLE` instead")]
    #[env_config(name = "ZO_S3_FEATURE_FORCE_PATH_STYLE", default = false)]
    pub feature_force_path_style: bool,
    #[env_config(name = "ZO_S3_FEATURE_FORCE_HOSTED_STYLE", default = false)]
    pub feature_force_hosted_style: bool,
    #[env_config(name = "ZO_S3_FEATURE_HTTP1_ONLY", default = false)]
    pub feature_http1_only: bool,
    #[env_config(name = "ZO_S3_FEATURE_HTTP2_ONLY", default = false)]
    pub feature_http2_only: bool,
    #[env_config(name = "ZO_S3_ALLOW_INVALID_CERTIFICATES", default = false)]
    pub allow_invalid_certificates: bool,
    #[env_config(name = "ZO_S3_SYNC_TO_CACHE_INTERVAL", default = 600)] // seconds
    pub sync_to_cache_interval: u64,
}

#[derive(Debug, EnvConfig)]
pub struct Prometheus {
    #[env_config(name = "ZO_PROMETHEUS_HA_CLUSTER", default = "cluster")]
    pub ha_cluster_label: String,
    #[env_config(name = "ZO_PROMETHEUS_HA_REPLICA", default = "__replica__")]
    pub ha_replica_label: String,
}

pub fn init() -> Config {
    dotenv().ok();
    let mut cfg = Config::init().unwrap();
    // set cpu num
    let cpu_num = cgroup::get_cpu_limit();
    cfg.limit.cpu_num = cpu_num;
    if cfg.limit.http_worker_num == 0 {
        cfg.limit.http_worker_num = cpu_num;
    }
    if cfg.limit.http_worker_max_blocking == 0 {
        cfg.limit.http_worker_max_blocking = 1024;
    }
    // HACK for thread_num equal to CPU core * 4
    if cfg.limit.query_thread_num == 0 {
        cfg.limit.query_thread_num = cpu_num * 4;
    }
    // HACK for move_file_thread_num equal to CPU core
    if cfg.limit.file_move_thread_num == 0 {
        cfg.limit.file_move_thread_num = cpu_num;
    }

    // check common config
    if let Err(e) = check_common_config(&mut cfg) {
        panic!("common config error: {e}");
    }

    // check data path config
    if let Err(e) = check_path_config(&mut cfg) {
        panic!("data path config error: {e}");
    }

    // check memeory cache
    if let Err(e) = check_memory_cache_config(&mut cfg) {
        panic!("memory cache config error: {e}");
    }

    // check disk cache
    if let Err(e) = check_disk_cache_config(&mut cfg) {
        panic!("disk cache config error: {e}");
    }

    // check etcd config
    if let Err(e) = check_etcd_config(&mut cfg) {
        panic!("etcd config error: {e}");
    }

    // check sled config
    if let Err(e) = check_sled_config(&mut cfg) {
        panic!("sled config error: {e}");
    }

    // check s3 config
    if let Err(e) = check_s3_config(&mut cfg) {
        panic!("s3 config error: {e}");
    }

    // check dynamo config
    if let Err(e) = check_dynamo_config(&mut cfg) {
        panic!("dynamo config error: {e}");
    }

    cfg
}

fn check_common_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if cfg.limit.file_push_interval == 0 {
        cfg.limit.file_push_interval = 60;
    }
    // check max_file_size_on_disk to MB
    cfg.limit.max_file_size_on_disk *= 1024 * 1024;
    if cfg.limit.req_cols_per_record_limit == 0 {
        cfg.limit.req_cols_per_record_limit = 1000;
    }

    // HACK instance_name
    if cfg.common.instance_name.is_empty() {
        cfg.common.instance_name = sysinfo::System::new().host_name().unwrap();
    }

    // HACK for tracing, always disable tracing except ingester and querier
    let local_node_role: Vec<super::cluster::Role> = cfg
        .common
        .node_role
        .clone()
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect();
    if !local_node_role.contains(&super::cluster::Role::All)
        && !local_node_role.contains(&super::cluster::Role::Ingester)
        && !local_node_role.contains(&super::cluster::Role::Querier)
    {
        cfg.common.tracing_enabled = false;
    }

    // format local_mode_storage
    cfg.common.local_mode_storage = cfg.common.local_mode_storage.to_lowercase();

    // format metadata storage
    if cfg.common.meta_store.is_empty() {
        if cfg.common.local_mode {
            cfg.common.meta_store = "sqlite".to_string();
        } else {
            cfg.common.meta_store = "etcd".to_string();
        }
    }
    cfg.common.meta_store = cfg.common.meta_store.to_lowercase();
    if cfg.common.local_mode
        || (cfg.common.meta_store != "sqlite" && cfg.common.meta_store != "etcd")
    {
        cfg.common.meta_store_external = true;
    }
    if cfg.common.meta_store.starts_with("postgres") && cfg.common.meta_postgres_dsn.is_empty() {
        return Err(anyhow::anyhow!(
            "Meta store is PostgreSQL, you must set ZO_META_POSTGRES_DSN"
        ));
    }
    if cfg.common.meta_store.starts_with("mysql") && cfg.common.meta_mysql_dsn.is_empty() {
        return Err(anyhow::anyhow!(
            "Meta store is MySQL, you must set ZO_META_MYSQL_DSN"
        ));
    }

    // check compact_max_file_size to MB
    cfg.compact.max_file_size *= 1024 * 1024;
    if cfg.compact.interval == 0 {
        cfg.compact.interval = 60;
    }
    // check compact_step_secs, min value is 600s
    if cfg.compact.step_secs == 0 {
        cfg.compact.step_secs = 3600;
    } else if cfg.compact.step_secs <= 600 {
        cfg.compact.step_secs = 600;
    }
    if cfg.compact.data_retention_days > 0 && cfg.compact.data_retention_days < 3 {
        return Err(anyhow::anyhow!(
            "Data retention is not allowed to be less than 3 days."
        ));
    }
    if cfg.compact.delete_files_delay_hours < 1 {
        return Err(anyhow::anyhow!(
            "Delete files delay is not allowed to be less than 1 hour."
        ));
    }

    // If the default scrape interval is less than 5s, raise an error
    if cfg.common.default_scrape_interval < 5 {
        return Err(anyhow::anyhow!(
            "Default scrape interval can not be set to lesser than 5s ."
        ));
    }
    Ok(())
}

fn check_path_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if cfg.common.data_dir.is_empty() {
        cfg.common.data_dir = "./data/openobserve/".to_string();
    }
    if !cfg.common.data_dir.ends_with('/') {
        cfg.common.data_dir = format!("{}/", cfg.common.data_dir);
    }
    if cfg.common.data_wal_dir.is_empty() {
        cfg.common.data_wal_dir = format!("{}wal/", cfg.common.data_dir);
    }
    if !cfg.common.data_wal_dir.ends_with('/') {
        cfg.common.data_wal_dir = format!("{}/", cfg.common.data_wal_dir);
    }
    if cfg.common.data_stream_dir.is_empty() {
        cfg.common.data_stream_dir = format!("{}stream/", cfg.common.data_dir);
    }
    if !cfg.common.data_stream_dir.ends_with('/') {
        cfg.common.data_stream_dir = format!("{}/", cfg.common.data_stream_dir);
    }
    if cfg.common.data_db_dir.is_empty() {
        cfg.common.data_db_dir = format!("{}db/", cfg.common.data_dir);
    }
    if !cfg.common.data_db_dir.ends_with('/') {
        cfg.common.data_db_dir = format!("{}/", cfg.common.data_db_dir);
    }
    if cfg.common.data_cache_dir.is_empty() {
        cfg.common.data_cache_dir = format!("{}cache/", cfg.common.data_dir);
    }
    if !cfg.common.data_cache_dir.ends_with('/') {
        cfg.common.data_cache_dir = format!("{}/", cfg.common.data_cache_dir);
    }
    if cfg.common.base_uri.ends_with('/') {
        cfg.common.base_uri = cfg.common.base_uri.trim_end_matches('/').to_string();
    }
    if cfg.sled.data_dir.is_empty() {
        cfg.sled.data_dir = format!("{}db/", cfg.common.data_dir);
    }
    if !cfg.sled.data_dir.ends_with('/') {
        cfg.sled.data_dir = format!("{}/", cfg.sled.data_dir);
    }
    if cfg.common.mmdb_data_dir.is_empty() {
        cfg.common.mmdb_data_dir = format!("{}mmdb/", cfg.common.data_dir);
    }
    if !cfg.common.mmdb_data_dir.ends_with('/') {
        cfg.common.mmdb_data_dir = format!("{}/", cfg.common.mmdb_data_dir);
    }
    Ok(())
}

fn check_etcd_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if !cfg.etcd.prefix.is_empty() && !cfg.etcd.prefix.ends_with('/') {
        cfg.etcd.prefix = format!("{}/", cfg.etcd.prefix);
    }

    if !cfg.etcd.cert_auth {
        return Ok(());
    }
    if let Err(e) = get_file_meta(&cfg.etcd.ca_file) {
        return Err(anyhow::anyhow!("ZO_ETCD_TRUSTED_CA_FILE check err: {}", e));
    }
    if let Err(e) = get_file_meta(&cfg.etcd.cert_file) {
        return Err(anyhow::anyhow!("ZO_ETCD_TRUSTED_CA_FILE check err: {}", e));
    }
    if let Err(e) = get_file_meta(&cfg.etcd.key_file) {
        return Err(anyhow::anyhow!("ZO_ETCD_TRUSTED_CA_FILE check err: {}", e));
    }

    // check domain name
    if cfg.etcd.domain_name.is_empty() {
        let mut name = cfg.etcd.addr.clone();
        if name.contains("//") {
            name = name.split("//").collect::<Vec<&str>>()[1].to_string();
        }
        if name.contains(':') {
            name = name.split(':').collect::<Vec<&str>>()[0].to_string();
        }
        cfg.etcd.domain_name = name;
    }

    Ok(())
}

fn check_sled_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if cfg.sled.data_dir.is_empty() {
        cfg.sled.data_dir = format!("{}db/", cfg.common.data_dir);
    }
    if !cfg.sled.data_dir.ends_with('/') {
        cfg.sled.data_dir = format!("{}/", cfg.sled.data_dir);
    }
    if !cfg.sled.prefix.is_empty() && !cfg.sled.prefix.ends_with('/') {
        cfg.sled.prefix = format!("{}/", cfg.sled.prefix);
    }

    Ok(())
}

fn check_memory_cache_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    let mem_total = cgroup::get_memory_limit();
    cfg.limit.mem_total = mem_total;
    if cfg.memory_cache.max_size == 0 {
        cfg.memory_cache.max_size = mem_total / 2; // 50%
    } else {
        cfg.memory_cache.max_size *= 1024 * 1024;
    }
    if cfg.memory_cache.skip_size == 0 {
        // will skip the cache when a query need cache great than this value, default is
        // 80% of max_size
        cfg.memory_cache.skip_size = cfg.memory_cache.max_size / 10 * 8;
    } else {
        cfg.memory_cache.skip_size *= 1024 * 1024;
    }
    if cfg.memory_cache.release_size == 0 {
        // when cache is full will release how many data once time, default is 1% of
        // max_size
        cfg.memory_cache.release_size = cfg.memory_cache.max_size / 100;
    } else {
        cfg.memory_cache.release_size *= 1024 * 1024;
    }
    if cfg.memory_cache.datafusion_max_size == 0 {
        cfg.memory_cache.datafusion_max_size = mem_total - cfg.memory_cache.max_size;
    } else {
        cfg.memory_cache.datafusion_max_size *= 1024 * 1024;
    }
    Ok(())
}

fn check_disk_cache_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    let mut system = sysinfo::System::new();
    system.refresh_disks_list();
    let mut disks: Vec<(&str, u64, u64)> = system
        .disks()
        .iter()
        .map(|d| {
            (
                d.mount_point().to_str().unwrap(),
                d.total_space(),
                d.available_space(),
            )
        })
        .collect();
    disks.sort_by(|a, b| b.0.cmp(a.0));

    std::fs::create_dir_all(&cfg.common.data_cache_dir).expect("create cache dir success");
    let cache_dir = Path::new(&cfg.common.data_cache_dir)
        .canonicalize()
        .unwrap();
    let cache_dir = cache_dir.to_str().unwrap();
    let disk = disks.iter().find(|d| cache_dir.starts_with(d.0));
    let (disk_total, disk_free) = match disk {
        Some(d) => (d.1, d.2),
        None => (0, 0),
    };
    cfg.limit.disk_total = disk_total as usize;
    cfg.limit.disk_free = disk_free as usize;
    if cfg.disk_cache.max_size == 0 {
        cfg.disk_cache.max_size = cfg.limit.disk_free / 2; // 50%
        if cfg.disk_cache.max_size > 1024 * 1024 * 1024 * 100 {
            cfg.disk_cache.max_size = 1024 * 1024 * 1024 * 100; // 100GB
        }
    } else {
        cfg.disk_cache.max_size *= 1024 * 1024;
    }
    if cfg.disk_cache.skip_size == 0 {
        // will skip the cache when a query need cache great than this value, default is
        // 80% of max_size
        cfg.disk_cache.skip_size = cfg.disk_cache.max_size / 10 * 8;
    } else {
        cfg.disk_cache.skip_size *= 1024 * 1024;
    }
    if cfg.disk_cache.release_size == 0 {
        // when cache is full will release how many data once time, default is 1% of
        // max_size
        cfg.disk_cache.release_size = cfg.disk_cache.max_size / 100;
    } else {
        cfg.disk_cache.release_size *= 1024 * 1024;
    }
    Ok(())
}

fn check_s3_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if !cfg.s3.bucket_prefix.is_empty() && !cfg.s3.bucket_prefix.ends_with('/') {
        cfg.s3.bucket_prefix = format!("{}/", cfg.s3.bucket_prefix);
    }
    if cfg.s3.provider.is_empty() {
        if cfg.s3.server_url.contains(".googleapis.com") {
            cfg.s3.provider = "gcs".to_string();
        } else if cfg.s3.server_url.contains(".aliyuncs.com") {
            cfg.s3.provider = "oss".to_string();
            if !cfg
                .s3
                .server_url
                .contains(&format!("://{}.", cfg.s3.bucket_name))
            {
                cfg.s3.server_url = cfg
                    .s3
                    .server_url
                    .replace("://", &format!("://{}.", cfg.s3.bucket_name));
            }
        } else {
            cfg.s3.provider = "aws".to_string();
        }
    }
    cfg.s3.provider = cfg.s3.provider.to_lowercase();
    if cfg.s3.provider.eq("swift") {
        std::env::set_var("AWS_EC2_METADATA_DISABLED", "true");
    }

    Ok(())
}

fn check_dynamo_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if cfg.common.meta_store.starts_with("dynamo") && cfg.dynamo.prefix.is_empty() {
        cfg.dynamo.prefix = if cfg.s3.bucket_name.is_empty() {
            "default".to_string()
        } else {
            cfg.s3.bucket_name.clone()
        };
    }
    cfg.dynamo.file_list_table = format!("{}-file-list", cfg.dynamo.prefix);
    cfg.dynamo.file_list_deleted_table = format!("{}-file-list-deleted", cfg.dynamo.prefix);
    cfg.dynamo.stream_stats_table = format!("{}-stream-stats", cfg.dynamo.prefix);
    cfg.dynamo.org_meta_table = format!("{}-org-meta", cfg.dynamo.prefix);
    cfg.dynamo.meta_table = format!("{}-meta", cfg.dynamo.prefix);
    cfg.dynamo.schema_table = format!("{}-schema", cfg.dynamo.prefix);
    cfg.dynamo.compact_table = format!("{}-compact", cfg.dynamo.prefix);

    Ok(())
}

#[inline]
pub fn is_local_disk_storage() -> bool {
    CONFIG.common.local_mode && CONFIG.common.local_mode_storage.eq("disk")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config() {
        let mut cfg = Config::init().unwrap();
        cfg.s3.server_url = "https://storage.googleapis.com".to_string();
        cfg.s3.provider = "".to_string();
        check_s3_config(&mut cfg).unwrap();
        assert_eq!(cfg.s3.provider, "gcs");
        cfg.s3.server_url = "https://oss-cn-beijing.aliyuncs.com".to_string();
        cfg.s3.provider = "".to_string();
        check_s3_config(&mut cfg).unwrap();
        assert_eq!(cfg.s3.provider, "oss");
        cfg.s3.server_url = "".to_string();
        cfg.s3.provider = "".to_string();
        check_s3_config(&mut cfg).unwrap();
        assert_eq!(cfg.s3.provider, "aws");

        cfg.memory_cache.max_size = 1024;
        cfg.memory_cache.release_size = 1024;
        check_memory_cache_config(&mut cfg).unwrap();
        assert_eq!(cfg.memory_cache.max_size, 1024 * 1024 * 1024);
        assert_eq!(cfg.memory_cache.release_size, 1024 * 1024 * 1024);

        cfg.limit.file_push_interval = 0;
        cfg.limit.req_cols_per_record_limit = 0;
        cfg.compact.interval = 0;
        cfg.compact.data_retention_days = 10;
        let ret = check_common_config(&mut cfg);
        assert!(ret.is_ok());
        assert_eq!(cfg.compact.data_retention_days, 10);
        assert_eq!(cfg.limit.req_cols_per_record_limit, 1000);

        cfg.compact.data_retention_days = 2;
        let ret = check_common_config(&mut cfg);
        assert!(ret.is_err());

        cfg.common.data_dir = "".to_string();
        let ret = check_path_config(&mut cfg);
        assert!(ret.is_ok());

        cfg.common.data_dir = "/abc".to_string();
        cfg.common.data_wal_dir = "/abc".to_string();
        cfg.common.data_stream_dir = "/abc".to_string();
        cfg.sled.data_dir = "/abc/".to_string();
        cfg.common.base_uri = "/abc/".to_string();
        let ret = check_path_config(&mut cfg);
        assert!(ret.is_ok());
        assert_eq!(cfg.common.data_dir, "/abc/".to_string());
        assert_eq!(cfg.common.data_wal_dir, "/abc/".to_string());
        assert_eq!(cfg.common.data_stream_dir, "/abc/".to_string());
        assert_eq!(cfg.common.data_dir, "/abc/".to_string());
        assert_eq!(cfg.common.base_uri, "/abc".to_string());
    }
}
