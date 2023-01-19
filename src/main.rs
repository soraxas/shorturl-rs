mod config;
mod db_store;
mod url_mapping;

use std::{collections::HashMap, net::SocketAddr, sync::{Arc, Mutex}};
use warp::{http, Filter};

use futures::future;
use serde_json;
use db_store::{Store};
use url_mapping::{ShortCode, Url, ShortUrlMapping, Meta};

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
        Err(e) => None
    }
}

fn convert_header_to_string(headers: &http::HeaderMap<http::HeaderValue>) -> Option<String> {
    convert_json_to_string(&convert_header_to_json(&headers))
}

async fn add_shorturl(
    item: ShortUrlMapping,
    store: Arc<Mutex<Store>>,
    addr: Option<SocketAddr>,
    header: http::HeaderMap,
) -> Result<impl warp::Reply, warp::Rejection> {
    let addr = match addr {
        Some(val) => Some(val.to_string()),
        None => None,
    };

    match store.lock().unwrap().put(
        item.id,
        item.url,
        &Meta { address: addr, header: convert_header_to_string(&header) },
    ) {
        Ok(_) => Ok(warp::reply::with_status(
            "Added.".to_string(),
            http::StatusCode::CREATED,
        )),
        Err(e) => {
            println!("{}", e);
            Ok(warp::reply::with_status(
                format!("Failed. {}", e),
                http::StatusCode::CONFLICT,
            ))
        }
    }
}

async fn delete_shorturl(
    id: ShortCode,
    store: Arc<Mutex<Store>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // store.grocery_list.write().remove(&id.name);
    store.lock().unwrap().remove(id);

    Ok(warp::reply::with_status(
        "Removed item from grocery list",
        http::StatusCode::OK,
    ))
}

async fn get_urls_all(store: Arc<Mutex<Store>>) -> Result<impl warp::Reply, warp::Rejection> {
    // let mut result = HashMap::new();
    let mut result: HashMap<String, String> = HashMap::new();
    // let r = store.lock().unwrap().get_all();

    // for (key, value) in r.iter() {
    //     result.insert(key, value);
    // }

    Ok(warp::reply::json(&result))
    // Ok(warp::reply::json(()))
}

fn delete_json() -> impl Filter<Extract=(ShortCode, ), Error=warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

fn post_json() -> impl Filter<Extract=(ShortUrlMapping, ), Error=warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

#[tokio::main]
async fn main() {
    let store = Arc::new(Mutex::new(Store::new().unwrap()));

    let store_filter = warp::any().map(move || store.clone());
    let add_meta_filter = warp::any()
        .and(warp::addr::remote())
        .and(warp::header::headers_cloned());

    let shorten_url_api = warp::path("v1")
        .and(warp::path("urls"))
        .and(warp::path::end());

    let add_items = warp::post()
        .and(shorten_url_api)
        .and(post_json())
        .and(store_filter.clone())
        .and(add_meta_filter)
        .and_then(add_shorturl);

    let get_items = warp::get()
        .and(shorten_url_api)
        .and(store_filter.clone())
        .and_then(get_urls_all);

    let delete_item = warp::delete()
        .and(shorten_url_api)
        .and(delete_json())
        .and(store_filter.clone())
        .and_then(delete_shorturl);

    let update_item = warp::put()
        .and(shorten_url_api)
        .and(post_json())
        .and(store_filter.clone())
        .and(add_meta_filter)
        .and_then(add_shorturl);

    let admin_panel_route = warp::path::end().and(warp::fs::dir("www/static"));

    let (_api_addr, api_warp) = warp::serve(
        admin_panel_route
            .or(add_items)
            .or(get_items)
            .or(delete_item)
            .or(update_item),
    )
        .bind_ephemeral((config::LOCALHOST, config::PORT_API));
    // println!("Created {} route", "api");

    let shorturl_service_route = warp::path!(String)
        .and(store_filter.clone())
        .and(add_meta_filter)
        .map(
            |short_code: String, store: Arc<Mutex<Store>>, addr: Option<SocketAddr>, header:
            http::HeaderMap| {
                let addr = match addr {
                    Some(val) => Some(val.to_string()),
                    None => None,
                };

                match store.lock().unwrap().get(
                    short_code.as_str(),
                    &Meta { address: addr, header: convert_header_to_string(&header) },
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
    // println!("Created {} route", "public");

    future::join(api_warp, web_warp).await;
}
