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

use std::io::Error;

use actix_web::{
    http::{self, StatusCode},
    HttpResponse,
};
use config::meta::stream::StreamType;

use crate::{
    common::{
        infra::config::STREAM_FUNCTIONS,
        meta::{
            authz::Authz,
            functions::{
                FunctionList, StreamFunctionsList, StreamOrder, StreamTransform, Transform,
            },
            http::HttpResponse as MetaHttpResponse,
        },
        utils::auth::{remove_ownership, set_ownership},
    },
    service::{db, ingestion::compile_vrl_function},
};

const FN_SUCCESS: &str = "Function saved successfully";
const FN_NOT_FOUND: &str = "Function not found";
const FN_ADDED: &str = "Function applied to stream";
const FN_REMOVED: &str = "Function removed from stream";
const FN_DELETED: &str = "Function deleted";
const FN_ALREADY_EXIST: &str = "Function already exist";
const FN_IN_USE: &str =
    "Function is associated with streams, please remove association from streams before deleting:";

pub async fn save_function(org_id: String, mut func: Transform) -> Result<HttpResponse, Error> {
    if let Some(_existing_fn) = check_existing_fn(&org_id, &func.name).await {
        Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            StatusCode::BAD_REQUEST.into(),
            FN_ALREADY_EXIST.to_string(),
        )))
    } else {
        if !func.function.ends_with('.') {
            func.function = format!("{} \n .", func.function);
        }
        if func.trans_type.unwrap() == 0 {
            if let Err(e) = compile_vrl_function(func.function.as_str(), &org_id) {
                return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                    StatusCode::BAD_REQUEST.into(),
                    e.to_string(),
                )));
            }
        }
        extract_num_args(&mut func);
        if let Err(error) = db::functions::set(&org_id, &func.name, &func).await {
            Ok(
                HttpResponse::InternalServerError().json(MetaHttpResponse::message(
                    http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                    error.to_string(),
                )),
            )
        } else {
            set_ownership(&org_id, "functions", Authz::new(&func.name)).await;

            Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                http::StatusCode::OK.into(),
                FN_SUCCESS.to_string(),
            )))
        }
    }
}

#[tracing::instrument(skip(func))]
pub async fn update_function(
    org_id: &str,
    fn_name: &str,
    mut func: Transform,
) -> Result<HttpResponse, Error> {
    let existing_fn = match check_existing_fn(org_id, fn_name).await {
        Some(function) => function,
        None => {
            return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                StatusCode::NOT_FOUND.into(),
                FN_NOT_FOUND.to_string(),
            )));
        }
    };
    if func == existing_fn {
        return Ok(HttpResponse::Ok().json(func));
    }

    // UI mostly like in 1st version won't send streams, so we need to add them back
    // from existing function
    func.streams = existing_fn.streams;

    if !func.function.ends_with('.') {
        func.function = format!("{} \n .", func.function);
    }
    if func.trans_type.unwrap() == 0 {
        if let Err(e) = compile_vrl_function(&func.function, org_id) {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                StatusCode::BAD_REQUEST.into(),
                e.to_string(),
            )));
        }
    }
    extract_num_args(&mut func);
    if let Err(error) = db::functions::set(org_id, &func.name, &func).await {
        return Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::message(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                error.to_string(),
            )),
        );
    }
    Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
        http::StatusCode::OK.into(),
        FN_SUCCESS.to_string(),
    )))
}

pub async fn list_functions(
    org_id: String,
    permitted: Option<Vec<String>>,
) -> Result<HttpResponse, Error> {
    if let Ok(functions) = db::functions::list(&org_id).await {
        let mut result = Vec::new();
        for function in functions {
            if permitted.is_none()
                || permitted
                    .as_ref()
                    .unwrap()
                    .contains(&format!("function:{}", function.name))
                || permitted
                    .as_ref()
                    .unwrap()
                    .contains(&format!("function:_all_{}", org_id))
            {
                result.push(function);
            }
        }

        Ok(HttpResponse::Ok().json(FunctionList { list: result }))
    } else {
        Ok(HttpResponse::Ok().json(FunctionList { list: vec![] }))
    }
}

pub async fn delete_function(org_id: String, fn_name: String) -> Result<HttpResponse, Error> {
    let existing_fn = match check_existing_fn(&org_id, &fn_name).await {
        Some(function) => function,
        None => {
            return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                StatusCode::NOT_FOUND.into(),
                FN_NOT_FOUND.to_string(),
            )));
        }
    };
    if let Some(val) = existing_fn.streams {
        if !val.is_empty() {
            let names = val
                .iter()
                .filter_map(|stream| {
                    if !stream.is_removed {
                        Some(stream.stream.to_owned())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            if !names.is_empty() {
                return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                    StatusCode::BAD_REQUEST.into(),
                    format!("{} {}", FN_IN_USE, names),
                )));
            }
        }
    }
    let result = db::functions::delete(&org_id, &fn_name).await;
    match result {
        Ok(_) => {
            remove_ownership(&org_id, "functions", Authz::new(&fn_name)).await;

            Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                http::StatusCode::OK.into(),
                FN_DELETED.to_string(),
            )))
        }
        Err(_) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            StatusCode::NOT_FOUND.into(),
            FN_NOT_FOUND.to_string(),
        ))),
    }
}

pub async fn list_stream_functions(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
) -> Result<HttpResponse, Error> {
    if let Some(val) = STREAM_FUNCTIONS.get(&format!("{}/{}/{}", org_id, stream_type, stream_name))
    {
        Ok(HttpResponse::Ok().json(val.value()))
    } else {
        Ok(HttpResponse::Ok().json(StreamFunctionsList { list: vec![] }))
    }
}

pub async fn delete_stream_function(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    fn_name: &str,
) -> Result<HttpResponse, Error> {
    let mut existing_fn = match check_existing_fn(org_id, fn_name).await {
        Some(function) => function,
        None => {
            return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                StatusCode::NOT_FOUND.into(),
                FN_NOT_FOUND.to_string(),
            )));
        }
    };

    if let Some(ref mut val) = existing_fn.streams {
        for stream in val.iter_mut() {
            if stream.stream == stream_name && stream.stream_type == stream_type {
                stream.is_removed = true;
                stream.order = 0;
                break;
            }
        }
        if let Err(error) = db::functions::set(org_id, fn_name, &existing_fn).await {
            Ok(
                HttpResponse::InternalServerError().json(MetaHttpResponse::message(
                    http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                    error.to_string(),
                )),
            )
        } else {
            // can't be removed from watcher of function as stream name & type won't be
            // available , hence being removed here
            // let key = format!("{}/{}/{}", org_id, stream_type, stream_name);
            // remove_stream_fn_from_cache(&key, fn_name);
            Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                http::StatusCode::OK.into(),
                FN_REMOVED.to_string(),
            )))
        }
    } else {
        Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            StatusCode::NOT_FOUND.into(),
            FN_NOT_FOUND.to_string(),
        )))
    }
}

pub async fn add_function_to_stream(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    fn_name: &str,
    mut stream_order: StreamOrder,
) -> Result<HttpResponse, Error> {
    let mut existing_fn = match check_existing_fn(org_id, fn_name).await {
        Some(function) => function,
        None => {
            return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                StatusCode::NOT_FOUND.into(),
                FN_NOT_FOUND.to_string(),
            )));
        }
    };

    stream_order.stream = stream_name.to_owned();
    stream_order.stream_type = stream_type;

    if let Some(mut val) = existing_fn.streams {
        if let Some(existing) = val.iter_mut().find(|x| {
            x.stream == stream_order.stream && x.stream_type.eq(&stream_order.stream_type)
        }) {
            existing.is_removed = false;
            existing.order = stream_order.order;
            existing.apply_before_flattening = stream_order.apply_before_flattening;
        } else {
            val.push(stream_order);
        }
        existing_fn.streams = Some(val);
    } else {
        existing_fn.streams = Some(vec![stream_order]);
    }

    if let Err(error) = db::functions::set(org_id, fn_name, &existing_fn).await {
        Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::message(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                error.to_string(),
            )),
        )
    } else {
        Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            FN_ADDED.to_string(),
        )))
    }
}

fn extract_num_args(func: &mut Transform) {
    if func.trans_type.unwrap() == 1 {
        let src: String = func.function.to_owned();
        let start_stream = src.find('(').unwrap();
        let end_stream = src.find(')').unwrap();
        let args = &src[start_stream + 1..end_stream].trim();
        if args.is_empty() {
            func.num_args = 0;
        } else {
            func.num_args = args.split(',').collect::<Vec<&str>>().len() as u8;
        }
    } else {
        let params = func.params.to_owned();
        func.num_args = params.split(',').collect::<Vec<&str>>().len() as u8;
    }
}

async fn check_existing_fn(org_id: &str, fn_name: &str) -> Option<Transform> {
    match db::functions::get(org_id, fn_name).await {
        Ok(function) => Some(function),
        Err(_) => None,
    }
}

fn _remove_stream_fn_from_cache(key: &str, fn_name: &str) {
    if let Some(val) = STREAM_FUNCTIONS.clone().get(key) {
        if val.list.len() > 1 {
            let final_list = val
                .clone()
                .list
                .into_iter()
                .filter(|x| x.transform.name != fn_name)
                .collect::<Vec<StreamTransform>>();
            STREAM_FUNCTIONS.insert(key.to_string(), StreamFunctionsList { list: final_list });
        } else {
            STREAM_FUNCTIONS.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_functions() {
        let mut trans = Transform {
            function: "function(row)  row.square = row[\"Year\"]*row[\"Year\"]  return row end"
                .to_owned(),
            name: "dummyfn".to_owned(),
            params: "row".to_owned(),
            streams: None,
            num_args: 0,
            trans_type: Some(1),
        };

        let mut vrl_trans = Transform {
            name: "vrl_trans".to_owned(),
            function: ". = parse_aws_vpc_flow_log!(row.message) \n .".to_owned(),
            trans_type: Some(1),
            params: "row".to_owned(),
            num_args: 0,
            streams: Some(vec![StreamOrder {
                stream: "test".to_owned(),
                stream_type: StreamType::Logs,
                order: 0,
                is_removed: false,
                apply_before_flattening: false,
            }]),
        };

        extract_num_args(&mut trans);
        extract_num_args(&mut vrl_trans);
        assert_eq!(trans.num_args, 1);
        assert_eq!(vrl_trans.num_args, 1);

        assert_eq!(trans.num_args, 1);

        let res = save_function("nexus".to_owned(), trans).await;
        assert!(res.is_ok());

        let list_resp = list_functions("nexus".to_string(), None).await;
        assert!(list_resp.is_ok());

        assert!(
            delete_function("nexus".to_string(), "dummyfn".to_owned())
                .await
                .is_ok()
        );
    }
}
