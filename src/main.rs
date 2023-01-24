mod config;
mod db_store;
mod types;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use warp::{http, Filter, Rejection};

use db_store::Store;
use futures::future;
use serde_json;
use types::{Meta, ShortUrlMapping, Url};
use warp::reject::MethodNotAllowed;

type Empty = ();

fn convert_header_to_json(
    headers: &http::HeaderMap<http::HeaderValue>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut json_map = serde_json::Map::new();

    for (k, v) in headers {
        let v_str = String::from_utf8_lossy(v.as_bytes()).into_owned();
        json_map.insert(k.as_str().to_owned(), serde_json::json!(v_str));
    }

    json_map
}

fn convert_json_to_string(json: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    match serde_json::to_string(&json) {
        Ok(val) => Some(val),
        Err(e) => None,
    }
}

fn convert_header_to_string(headers: &http::HeaderMap<http::HeaderValue>) -> Option<String> {
    convert_json_to_string(&convert_header_to_json(&headers))
}

async fn add_shorturl(
    short_code: String,
    item: ShortUrlMapping,
    store: Arc<Mutex<Store>>,
    addr: Option<SocketAddr>,
    header: http::HeaderMap,
) -> Result<impl warp::Reply, warp::Rejection> {
    let addr = match addr {
        Some(val) => Some(val.to_string()),
        None => None,
    };

    match store.lock().unwrap().insert(
        &short_code,
        &item.url,
        &Meta {
            address: addr,
            header: convert_header_to_string(&header),
        },
    ) {
        Ok(_) => Ok(warp::reply::with_status(
            "Added.".to_string(),
            http::StatusCode::CREATED,
        )),
        Err(e) => Ok(warp::reply::with_status(
            format!("Failed. {}", e),
            http::StatusCode::CONFLICT,
        )),
    }
}

async fn delete_shorturl(
    short_code: String,
    // _: Empty,
    store: Arc<Mutex<Store>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // store.grocery_list.write().remove(&id.name);
    match store.lock().unwrap().remove(&short_code) {
        Ok(val) => {
            if val > 0 {
                Ok(warp::reply::with_status(
                    "Removed.".to_string(),
                    http::StatusCode::OK,
                ))
            } else {
                Ok(warp::reply::with_status(
                    "Item does not exists.".to_string(),
                    http::StatusCode::BAD_REQUEST,
                ))
            }
        }
        Err(e) => Ok(warp::reply::with_status(
            format!("Failed to remove. {}", e),
            http::StatusCode::BAD_REQUEST,
        )),
    }
}

#[derive(Debug)]
struct Unauthorized;

#[derive(Debug)]
struct InvalidParameter;

impl warp::reject::Reject for Unauthorized {}

impl warp::reject::Reject for InvalidParameter {}

const API_TOKEN_HEADER: &str = "x-api-key";

async fn authorize_token(token: String, store: Arc<Mutex<Store>>) -> Result<(), Rejection> {
    let uid = 0;
    match store.lock().unwrap().check_api_key(uid, &token) {
        true => Ok(()),
        _ => Err(warp::reject::custom(Unauthorized)),
    }
}

pub fn api_token_filter(
    store: Arc<Mutex<Store>>,
) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::header::header(API_TOKEN_HEADER)
        .and(warp::any().map(move || store.clone()))
        .and_then(authorize_token)
        .and(warp::any())
        .untuple_one()
        .and(warp::any())
}

async fn get_urls_access_log(
    store: Arc<Mutex<Store>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(warp::reply::json(
        &store.lock().unwrap().get_summarised_access_logs().unwrap(),
    ))

    // match store.lock().unwrap().get_summarised_access_logs() {
    //     Ok(val) =>Ok(warp::reply::json(&val)),
    //     Err(e) => Ok((warp::reply::json(()))),
    // }

    // for (key, value) in r.iter() {
    //     result.insert(key, value);
    // }

    // Ok(warp::reply::json(()))
}

// Custom rejection handler that maps rejections into responses.
async fn handle_rejection(err: Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    if err.is_not_found() {
        Ok(warp::reply::with_status(
            "NOT_FOUND",
            http::StatusCode::NOT_FOUND,
        ))
    } else if let Some(_) = err.find::<Unauthorized>() {
        Ok(warp::reply::with_status(
            "UNAUTHORIZED",
            http::StatusCode::UNAUTHORIZED,
        ))
    } else if let Some(_) = err.find::<InvalidParameter>() {
        Ok(warp::reply::with_status(
            "BAD_REQUEST",
            http::StatusCode::BAD_REQUEST,
        ))
    } else if let Some(_) = err.find::<MethodNotAllowed>() {
        Ok(warp::reply::with_status(
            "METHOD_NOT_ALLOWED",
            http::StatusCode::METHOD_NOT_ALLOWED,
        ))
    } else {
        eprintln!("unhandled rejection: {:?}", err);
        Ok(warp::reply::with_status(
            "INTERNAL_SERVER_ERROR",
            http::StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}

fn delete_json() -> impl Filter<Extract = (Empty,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

fn post_json() -> impl Filter<Extract = (ShortUrlMapping,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

#[tokio::main]
async fn main() {
    let get_store = move || Arc::new(Mutex::new(Store::new().unwrap())).clone();

    let protected = || warp::any().and(api_token_filter(get_store())).clone();

    let store_filter = warp::any().map(get_store);
    let add_meta_filter = warp::any()
        .and(warp::addr::remote())
        .and(warp::header::headers_cloned());

    let add_items = protected()
        .and(warp::post())
        .and(warp::path("v1"))
        .and(warp::path("url"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(post_json())
        .and(store_filter.clone())
        .and(add_meta_filter)
        .and_then(add_shorturl);

    let delete_item = protected()
        .and(warp::delete())
        .and(warp::path("v1"))
        .and(warp::path("url"))
        .and(warp::path::param())
        .and(warp::path::end())
        // .and(delete_json())
        .and(store_filter.clone())
        .and_then(delete_shorturl);

    let get_access_logs = protected()
        .and(warp::get())
        .and(warp::path("v1"))
        .and(warp::path("logs"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and_then(get_urls_access_log);

    // let list_api_key = warp::get()
    //     .and(warp::path("v1"))
    //     .and(warp::path("api_keys"))
    //     .and(warp::path::end())
    //     .and(store_filter.clone())
    //     .map(
    //         |store: Arc<Mutex<Store>>, | {
    //             warp::reply::json(
    //                 &store.lock().unwrap().list_api_key().unwrap(),
    //             )
    //         },
    //     );

    // let update_item = warp::put()
    //     .and(warp::path("v1"))
    //     .and(warp::path("url"))
    //     .and(warp::path::end())
    //     .and(post_json())
    //     .and(store_filter.clone())
    //     .and(add_meta_filter)
    //     .and_then(add_shorturl);

    let admin_panel_route = warp::any() //protected()
        .and(warp::path::end())
        .and(warp::fs::dir("www/static"));

    let (_api_addr, api_warp) = warp::serve(
        admin_panel_route
            //     get_access_logs
            //     delete_item
            .or(get_access_logs)
            .or(add_items)
            .or(delete_item)
            .recover(handle_rejection), // .or(update_item))
    )
    .bind_ephemeral((config::LOCALHOST, config::PORT_API));
    // println!("Created {} route", "api");

    let shorturl_service_route = warp::path!(String)
        .and(store_filter.clone())
        .and(add_meta_filter)
        .map(
            |short_code: String,
             store: Arc<Mutex<Store>>,
             addr: Option<SocketAddr>,
             header: http::HeaderMap| {
                let addr = match addr {
                    Some(val) => Some(val.to_string()),
                    None => None,
                };

                match store.lock().unwrap().get(
                    short_code.as_str(),
                    &Meta {
                        address: addr,
                        header: convert_header_to_string(&header),
                    },
                ) {
                    // fonud a match
                    Some(long_url) => http::Response::builder()
                        .status(config::CONFIG.redirect_http_type)
                        .header(http::header::LOCATION, long_url)
                        .body(""),
                    None => match &config::CONFIG.address_to_rederect_if_not_found {
                        // a fallback url is set
                        Some(fallback_url) => http::Response::builder()
                            .status(config::CONFIG.redirect_http_type)
                            .header(http::header::LOCATION, fallback_url)
                            .body(""),
                        // return 404
                        None => http::Response::builder()
                            .status(http::StatusCode::NOT_FOUND)
                            .body(""),
                    },
                }
            },
        );

    let (_web_addr, web_warp) = warp::serve(shorturl_service_route)
        .bind_ephemeral((config::LOCALHOST, config::PORT_SERVICE));

    {
        let store = get_store.clone()();
        let mut locked_store = store.lock().unwrap();
        let uid = 0;
        if !locked_store.has_api_key(uid) {
            locked_store.create_api_key(uid).unwrap();
        }

        let api_keys = locked_store.list_api_key(uid).unwrap();

        for api_key in api_keys {
            println!("> api key: {}", api_key);
        }
    }

    future::join(api_warp, web_warp).await;
}
