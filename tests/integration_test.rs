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

#[cfg(test)]
mod tests {
    use core::time;
    use std::{env, fs, str, sync::Once, thread};

    use actix_web::{http::header::ContentType, test, web, App};
    use bytes::{Bytes, BytesMut};
    use chrono::Utc;
    use config::{get_config, utils::json};
    use openobserve::{
        common::meta::dashboards::{v1, Dashboard, Dashboards},
        handler::http::router::*,
    };
    use prost::Message;
    use proto::prometheus_rpc;

    static START: Once = Once::new();

    fn setup() -> (&'static str, &'static str) {
        START.call_once(|| {
            env::set_var("ZO_ROOT_USER_EMAIL", "root@example.com");
            env::set_var("ZO_ROOT_USER_PASSWORD", "Complexpass#123");
            env::set_var("ZO_LOCAL_MODE", "true");
            env::set_var("ZO_MAX_FILE_SIZE_ON_DISK", "1");
            env::set_var("ZO_FILE_PUSH_INTERVAL", "1");
            env::set_var("ZO_PAYLOAD_LIMIT", "209715200");
            env::set_var("ZO_JSON_LIMIT", "209715200");
            env::set_var("ZO_TIME_STAMP_COL", "_timestamp");

            env_logger::init_from_env(
                env_logger::Env::new().default_filter_or(&get_config().log.level),
            );

            log::info!("setup Invoked");
        });
        (
            "Authorization",
            "Basic cm9vdEBleGFtcGxlLmNvbTpDb21wbGV4cGFzcyMxMjM=",
        )
    }

    async fn e2e_100_tear_down() {
        log::info!("Tear Down Invoked");
        fs::remove_dir_all("./data").expect("Delete local dir failed");
    }

    #[test]
    async fn e2e_test() {
        // make sure data dir is deleted before we run integ tests
        fs::remove_dir_all("./data")
            .unwrap_or_else(|e| log::info!("Error deleting local dir: {}", e));

        setup();

        // register node
        openobserve::common::infra::cluster::register_and_keepalive()
            .await
            .unwrap();
        // init config
        config::init().await.unwrap();
        // init infra
        infra::init().await.unwrap();
        openobserve::common::infra::init().await.unwrap();
        // ingester init
        ingester::init().await.unwrap();
        // init job
        openobserve::job::init().await.unwrap();

        for _i in 0..3 {
            e2e_1_post_bulk().await;
        }

        // ingest
        e2e_post_json().await;
        e2e_post_multi().await;
        e2e_post_trace().await;
        e2e_post_metrics().await;
        // e2e_post_kinesis_data().await;

        // streams
        e2e_get_stream().await;
        e2e_get_stream_schema().await;
        e2e_get_org_summary().await;
        e2e_post_stream_settings().await;
        e2e_get_org().await;

        // functions
        e2e_post_function().await;
        e2e_add_stream_function().await;
        e2e_list_functions().await;
        e2e_list_stream_functions().await;
        e2e_remove_stream_function().await;
        e2e_delete_function().await;

        // FIXME: Revise and restore the e2e tests for search API calls.
        // They have been broken by https://github.com/openobserve/openobserve/pull/570
        //
        // // search
        // e2e_search().await;
        // e2e_search_around().await;

        // users
        e2e_post_user().await;
        e2e_update_user().await;
        e2e_update_user_with_empty().await;
        e2e_add_user_to_org().await;
        e2e_list_users().await;
        e2e_delete_user().await;
        e2e_get_organizations().await;
        e2e_get_user_passcode().await;
        e2e_update_user_passcode().await;
        e2e_user_authentication().await;
        e2e_user_authentication_with_error().await;

        // dashboards
        {
            let board = e2e_create_dashboard().await;
            let list = e2e_list_dashboards().await;
            assert_eq!(list.dashboards[0], board.clone());

            let board = e2e_update_dashboard(v1::Dashboard {
                title: "e2e test".to_owned(),
                description: "Logs flow downstream".to_owned(),
                ..board.v1.unwrap()
            })
            .await;
            assert_eq!(
                e2e_get_dashboard(&board.clone().v1.unwrap().dashboard_id).await,
                board
            );
            e2e_delete_dashboard(&board.v1.unwrap().dashboard_id).await;
            assert!(e2e_list_dashboards().await.dashboards.is_empty());
        }

        // alert
        e2e_post_alert_template().await;
        e2e_get_alert_template().await;
        e2e_list_alert_template().await;
        e2e_post_alert_destination().await;
        e2e_get_alert_destination().await;
        e2e_list_alert_destinations().await;
        e2e_post_alert().await;
        e2e_get_alert().await;
        e2e_list_alerts().await;
        e2e_list_real_time_alerts().await;
        e2e_delete_alert().await;
        e2e_delete_alert_destination().await;
        e2e_delete_alert_template().await;

        // syslog
        e2e_post_syslog_route().await;
        e2e_list_syslog_routes().await;

        // others
        e2e_health_check().await;
        e2e_config().await;
        e2e_100_tear_down().await;

        // clear
        e2e_delete_stream().await;
    }

    async fn e2e_1_post_bulk() {
        let auth = setup();
        let path = "./tests/input.json";
        let body_str = fs::read_to_string(path).expect("Read file failed");
        let thread_id: usize = 0;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .app_data(web::Data::new(thread_id))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/_bulk", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_post_json() {
        let auth = setup();
        let body_str = "[{\"Year\": 1896, \"City\": \"Athens\", \"Sport\": \"Aquatics\", \"Discipline\": \"Swimming\", \"Athlete\": \"HERSCHMANN, Otto\", \"Country\": \"AUT\", \"Gender\": \"Men\", \"Event\": \"100M Freestyle\", \"Medal\": \"Silver\", \"Season\": \"summer\",\"_timestamp\":1665136888163792}]";
        let thread_id: usize = 0;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .app_data(web::Data::new(thread_id))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/{}/_json", "e2e", "olympics_schema"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_post_multi() {
        let auth = setup();
        let body_str = "{\"Year\": 1896, \"City\": \"Athens\", \"Sport\": \"Aquatics\", \"Discipline\": \"Swimming\", \"Athlete\": \"HERSCHMANN, Otto\", \"Country\": \"AUT\", \"Gender\": \"Men\", \"Event\": \"100M Freestyle\", \"Medal\": \"Silver\", \"Season\": \"summer\",\"_timestamp\":1665136888163792}";
        let thread_id: usize = 0;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .app_data(web::Data::new(thread_id))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/{}/_multi", "e2e", "olympics_schema"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_get_stream() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/streams", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_get_stream_schema() {
        let auth = setup();
        let one_sec = time::Duration::from_millis(15000);
        thread::sleep(one_sec);
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/{}/streams/{}/schema",
                "e2e", "olympics_schema"
            ))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_post_stream_settings() {
        let auth = setup();
        let body_str = r#"{"partition_keys":{"add":[{"field":"test_key"}],"remove":[]}, "full_text_search_keys":{"add":["log"],"remove":[]}}"#;
        // app
        let thread_id: usize = 0;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .app_data(web::Data::new(thread_id))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::put()
            .uri(&format!(
                "/api/{}/streams/{}/settings",
                "e2e", "olympics_schema"
            ))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_delete_stream() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::delete()
            .uri(&format!("/api/{}/streams/{}", "e2e", "olympics_schema"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_get_org() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_post_function() {
        let auth = setup();
        let body_str = r#"{
            "name": "e2etestfn",
            "function":".sqNew,err = .Year*.Year \n .",
            "params":"row"
        }"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/functions", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_add_stream_function() {
        let auth = setup();
        let body_str = r#"{
                                "order":1
                            }"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::put()
            .uri(&format!(
                "/api/{}/streams/{}/functions/{}",
                "e2e", "olympics_schema", "e2etestfn"
            ))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_list_functions() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/functions", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_list_stream_functions() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/{}/streams/{}/functions",
                "e2e", "olympics_schema"
            ))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_delete_function() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::delete()
            .uri(&format!("/api/{}/functions/{}", "e2e", "e2etestfn"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_remove_stream_function() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::delete()
            .uri(&format!(
                "/api/{}/streams/{}/functions/{}",
                "e2e", "olympics_schema", "e2etestfn"
            ))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[allow(dead_code)] // TODO: enable this test
    async fn e2e_search() {
        let auth = setup();
        let body_str = r#"{"query":{"sql":"select * from olympics_schema",
                                "from": 0,
                                "size": 100
                                        }
                            }"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/_search", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[allow(dead_code)] // TODO: enable this test
    async fn e2e_search_around() {
        let auth = setup();

        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let ts = chrono::Utc::now().timestamp_micros();

        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/{}/{}/_around?key={}&size=10",
                "e2e", "olympics_schema", ts
            ))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_list_users() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/users", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_get_organizations() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/api/organizations")
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_get_user_passcode() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/passcode", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_update_user_passcode() {
        let auth = setup();
        let body_str = "";
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::put()
            .uri(&format!("/api/{}/passcode", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_user_authentication() {
        let _auth = setup();
        let body_str = r#"{
                                "name": "root@example.com",
                                "password": "Complexpass#123"
                            }"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/auth/login")
            .insert_header(ContentType::json())
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_user_authentication_with_error() {
        let _auth = setup();
        let body_str = r#"{
                                "name": "root2@example.com",
                                "password": "Complexpass#123"
                            }"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/auth/login")
            .insert_header(ContentType::json())
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(!resp.status().is_success());
    }

    async fn e2e_post_user() {
        let auth = setup();
        let body_str = r#"{
                                "email": "nonadmin@example.com",
                                "password": "Abcd12345",
                                "role": "member"
                            }"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/users", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_update_user() {
        let auth = setup();
        let body_str = r#"{
                                "email": "nonadmin@example.com",
                                "password": "Abcd12345",
                                "role": "member"
                            }"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::put()
            .uri(&format!("/api/{}/users/{}", "e2e", "nonadmin@example.com"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_update_user_with_empty() {
        let auth = setup();
        let body_str = r#"{}"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::put()
            .uri(&format!("/api/{}/users/{}", "e2e", "nonadmin@example.com"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(!resp.status().is_success());
    }

    async fn e2e_add_user_to_org() {
        let auth = setup();
        let body_str = r#"{
            "role":"member"
        }"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/users/{}", "e2e", "root@example.com"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_delete_user() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::delete()
            .uri(&format!("/api/{}/users/{}", "e2e", "nonadmin@example.com"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_create_dashboard() -> Dashboard {
        let auth = setup();
        let body_str = r##"{"title":"b2","dashboardId":"","description":"desc2","role":"","owner":"root@example.com","created":"2023-03-30T07:49:41.744+00:00","panels":[{"id":"Panel_ID7857010","type":"bar","fields":{"stream":"default","stream_type":"logs","x":[{"label":"Timestamp","alias":"x_axis_1","column":"_timestamp","color":null,"aggregationFunction":"histogram"}],"y":[{"label":"Kubernetes Host","alias":"y_axis_1","column":"kubernetes_host","color":"#5960b2","aggregationFunction":"count"}],"filter":[{"type":"condition","values":[],"column":"method","operator":"Is Not Null","value":null}]},"config":{"title":"p5","description":"sample config blah blah blah","show_legends":true},"query":"SELECT histogram(_timestamp) as \"x_axis_1\", count(kubernetes_host) as \"y_axis_1\"  FROM \"default\" WHERE method IS NOT NULL GROUP BY \"x_axis_1\" ORDER BY \"x_axis_1\"","customQuery":false}],"layouts":[{"x":0,"y":0,"w":12,"h":13,"i":1,"panelId":"Panel_ID7857010","static":false}]}"##;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/dashboards", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();

        let body = test::call_and_read_body(&app, req).await;
        json::from_slice(&body).unwrap()
    }

    async fn e2e_list_dashboards() -> Dashboards {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/dashboards", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();

        let body = test::call_and_read_body(&app, req).await;
        json::from_slice(&body).unwrap()
    }

    async fn e2e_update_dashboard(dashboard: v1::Dashboard) -> Dashboard {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::put()
            .uri(&format!(
                "/api/{}/dashboards/{}",
                "e2e", dashboard.dashboard_id
            ))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(json::to_string(&dashboard).unwrap())
            .to_request();

        let body = test::call_and_read_body(&app, req).await;
        json::from_slice(&body).unwrap()
    }

    async fn e2e_get_dashboard(dashboard_id: &str) -> Dashboard {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/dashboards/{dashboard_id}", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();

        let body = test::call_and_read_body(&app, req).await;
        json::from_slice(&body).unwrap()
    }

    async fn e2e_delete_dashboard(dashboard_id: &str) {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::delete()
            .uri(&format!("/api/{}/dashboards/{dashboard_id}", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_post_trace() {
        let auth = setup();
        let path = "./tests/trace_input.json";
        let body_str = fs::read_to_string(path).expect("Read file failed");

        // app
        let thread_id: usize = 0;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .app_data(web::Data::new(thread_id))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/traces", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_post_metrics() {
        let auth = setup();

        let loc_lable: Vec<prometheus_rpc::Label> = vec![
            prometheus_rpc::Label {
                name: "__name__".to_string(),
                value: "grafana_api_dashboard_save_milliseconds_count".to_string(),
            },
            prometheus_rpc::Label {
                name: "cluster".to_string(),
                value: "prom-k8s".to_string(),
            },
            prometheus_rpc::Label {
                name: "__replica__".to_string(),
                value: "prom-k8s-0".to_string(),
            },
        ];

        let mut loc_samples: Vec<prometheus_rpc::Sample> = vec![];

        for i in 1..2 {
            loc_samples.push(prometheus_rpc::Sample {
                value: i as f64,
                timestamp: Utc::now().timestamp_micros(),
            });
        }
        loc_samples.push(prometheus_rpc::Sample {
            value: f64::NEG_INFINITY,
            timestamp: Utc::now().timestamp_micros(),
        });
        loc_samples.push(prometheus_rpc::Sample {
            value: f64::INFINITY,
            timestamp: Utc::now().timestamp_micros(),
        });

        loc_samples.push(prometheus_rpc::Sample {
            value: f64::NAN,
            timestamp: Utc::now().timestamp_micros(),
        });
        let loc_exemp: Vec<prometheus_rpc::Exemplar> = vec![];
        let loc_hist: Vec<prometheus_rpc::Histogram> = vec![];

        let ts = prometheus_rpc::TimeSeries {
            labels: loc_lable,
            samples: loc_samples,
            exemplars: loc_exemp,
            histograms: loc_hist,
        };

        let metadata: Vec<prometheus_rpc::MetricMetadata> = vec![];
        let wr_req: prometheus_rpc::WriteRequest = prometheus_rpc::WriteRequest {
            timeseries: vec![ts],
            metadata,
        };
        let mut out = BytesMut::with_capacity(wr_req.encoded_len());
        wr_req.encode(&mut out).expect("Out of memory");
        let data: Bytes = out.into();
        let body = snap::raw::Encoder::new()
            .compress_vec(&data)
            .expect("Out of memory");

        // app
        let thread_id: usize = 0;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .app_data(web::Data::new(thread_id))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/prometheus/api/v1/write", "e2e"))
            .insert_header(("X-Prometheus-Remote-Write-Version", "0.1.0"))
            .insert_header(("Content-Encoding", "snappy"))
            .insert_header(("Content-Type", "application/x-protobuf"))
            .append_header(auth)
            .set_payload(body)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_get_org_summary() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/summary", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        log::info!("{:?}", resp.status());
        assert!(resp.status().is_success());
    }

    async fn e2e_post_alert_template() {
        let auth = setup();
        let body_str = r#"{"name":"slackTemplate","body":"{\"text\":\"For stream {stream_name} of organization {org_name} alert {alert_name} of type {alert_type} is active app_name {app_name}\"}"}"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/alerts/templates", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_get_alert_template() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/{}/alerts/templates/{}",
                "e2e", "slackTemplate"
            ))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_delete_alert_template() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::delete()
            .uri(&format!(
                "/api/{}/alerts/templates/{}",
                "e2e", "slackTemplate"
            ))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_list_alert_template() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/alerts/templates", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_post_alert_destination() {
        let auth = setup();
        let body_str = r#"{
                "name": "slack",
                "url": "https://dummy/alert",
                "method": "post",
                "template": "slackTemplate",
                "headers":{
                    "x_org_id":"Test_header"
                }
            }"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/alerts/destinations", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_get_alert_destination() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/alerts/destinations/{}", "e2e", "slack"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_delete_alert_destination() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::delete()
            .uri(&format!("/api/{}/alerts/destinations/{}", "e2e", "slack"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        log::info!("{:?}", resp.status());
        assert!(resp.status().is_success());
    }

    async fn e2e_list_alert_destinations() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/alerts/destinations", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_post_alert() {
        let auth = setup();
        let body_str = r#"{
                                "name": "alertChk",
                                "stream_type": "logs",
                                "stream_name": "olympics_schema",
                                "is_real_time": false,
                                "query_condition": {
                                    "conditions": [{
                                        "column": "country",
                                        "operator": "=",
                                        "value": "USA"
                                    }]
                                },
                                "trigger_condition": {
                                    "period": 5,
                                    "threshold": 1,
                                    "silence": 10
                                },
                                "destinations": ["slack"],
                                "context_attributes":{
                                    "app_name":"App1"
                                }
                            }"#;
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/{}/alerts", "e2e", "olympics_schema"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_get_alert() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/{}/{}/alerts/{}",
                "e2e", "olympics_schema", "alertChk"
            ))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        log::info!("{:?}", resp.status());
        assert!(resp.status().is_success());
    }

    async fn e2e_delete_alert() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::delete()
            .uri(&format!(
                "/api/{}/{}/alerts/{}",
                "e2e", "olympics_schema", "alertChk"
            ))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        log::info!("{:?}", resp.status());
        assert!(resp.status().is_success());
    }

    async fn e2e_list_alerts() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/alerts", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_list_real_time_alerts() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/{}/alerts", "e2e", "olympics_schema"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_health_check() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/healthz")
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_config() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_config_routes)
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/config")
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_post_syslog_route() {
        let auth = setup();
        let body_str = r#"{
                                "orgId": "acceptLogs",
                                "streamName":"syslog",
                                "subnets": [
                                            "192.168.0.0/16",
                                            "127.0.0.0/8",
                                            "172.16.0.0/12"
                                        ]
                            }"#;

        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::post()
            .uri(&format!("/api/{}/syslog-routes", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .set_payload(body_str)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    async fn e2e_list_syslog_routes() {
        let auth = setup();
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(get_config().limit.req_json_limit))
                .app_data(web::PayloadConfig::new(
                    get_config().limit.req_payload_limit,
                ))
                .configure(get_service_routes)
                .configure(get_basic_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri(&format!("/api/{}/syslog-routes", "e2e"))
            .insert_header(ContentType::json())
            .append_header(auth)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
