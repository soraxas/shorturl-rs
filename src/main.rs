mod store;

use std::{collections::HashMap, str::FromStr};
use warp::{ http, Filter};

use futures::future;

use store::{Item, Id, Store};


async fn update_grocery_list(
    item: Item,
    store: Store
    ) -> Result<impl warp::Reply, warp::Rejection> {
        store.grocery_list.write().insert(item.short, item.long_url);

        Ok(warp::reply::with_status(
            "Added items to the grocery list",
            http::StatusCode::CREATED,
        ))
}

async fn delete_grocery_list_item(
    id: Id,
    store: Store
    ) -> Result<impl warp::Reply, warp::Rejection> {
        store.grocery_list.write().remove(&id.name);

        Ok(warp::reply::with_status(
            "Removed item from grocery list",
            http::StatusCode::OK,
        ))
}

async fn get_grocery_list(
    store: Store
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let mut result = HashMap::new();
        let r = store.grocery_list.read();

        for (key,value) in r.iter() {
            result.insert(key, value);
        }

        Ok(warp::reply::json(
            &result
        ))
}

fn delete_json() -> impl Filter<Extract = (Id,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}


fn post_json() -> impl Filter<Extract = (Item,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

#[tokio::main]
async fn main() {
    let store = Store::new();
    let store_filter = warp::any().map(move || store.clone());

    let shorten_url_api = warp::path("v1").and(warp::path("urls")).and(warp::path::end());

    let add_items = warp::post()
        .and(shorten_url_api)
        .and(post_json())
        .and(store_filter.clone())
        .and_then(update_grocery_list);

    let get_items = warp::get()
        .and(shorten_url_api)
        .and(store_filter.clone())
        .and_then(get_grocery_list);

    let delete_item = warp::delete()
        .and(shorten_url_api)
        .and(delete_json())
        .and(store_filter.clone())
        .and_then(delete_grocery_list_item);

    let update_item = warp::put()
        .and(shorten_url_api)
        .and(post_json())
        .and(store_filter.clone())
        .and_then(update_grocery_list);

    let admin_panel_route = warp::path::end().and(warp::fs::dir("www/static"));


    let (_api_addr, api_warp) = warp::serve(admin_panel_route.or(add_items).or(get_items).or(delete_item).or(update_item))
        .bind_ephemeral(([127, 0, 0, 1], 3030));
    println!("Created {} route", "api");

    // let public_route = warp::path::end().and(warp::redirect(Uri::from_static("/v2")));

    // let route = warp::any()
    // .and(warp::path::full())
    // .map(| p: warp::path::FullPath| {
    //     let path = p.as_str();
    //     p.
    //     format!("path: {}\nquery: {:?}", path, "")
    // });

    let route = warp::path!(String).map(|short_code|  {

        // return match store.grocery_list.read().get("a") {
        //     Some(long_url) => http::Response::builder().header(http::header::LOCATION, "https://cs.tinyiu.com").status(http::StatusCode::FOUND).body(""),
        //     None => http::Response::builder().status(http::StatusCode::NOT_FOUND).body("")
        // };

        // if (short_code == "ht") {
        //     return http::Response::builder().header(http::header::LOCATION, "https://cs.tinyiu.com").status(http::StatusCode::FOUND).body("");
        // }

        return http::Response::builder().status(http::StatusCode::NOT_FOUND).body("");
    });





    // let route = warp::any()
    // .map(|| {
    //     warp::redirect(http::Uri::from_static("https://www.google.com"))
    //     // warp::redirect(http::Uri::from_str("/v2"))
    // });


    let (_web_addr, web_warp) = warp::serve(route)
        .bind_ephemeral(([127, 0, 0, 1], 3035));
    println!("Created {} route", "public");


    future::join(api_warp, web_warp).await;
}