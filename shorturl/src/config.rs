use once_cell::sync::Lazy;

use std::env;
use warp::{http, hyper::StatusCode};

pub const LOCALHOST: [u8; 4] = [0, 0, 0, 0];
pub const PORT_SERVICE: u16 = 8080;
pub const PORT_API: u16 = 8081;

pub fn ip_to_string(ip: [u8; 4]) -> String {
    format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}

pub struct Config {
    pub redirect_http_type: StatusCode,
    pub address_to_rederect_if_not_found: Option<String>,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let mut config = Config {
        redirect_http_type: http::StatusCode::MOVED_PERMANENTLY,
        address_to_rederect_if_not_found: None,
    };

    match env::var("SHORTURL_USE_302") {
        Ok(_) => config.redirect_http_type = http::StatusCode::FOUND,
        Err(_) => (),
    }

    match env::var("SHORTURL_ADDRESS_TO_REDIRECT_IF_NOT_FOUND") {
        Ok(val) => config.address_to_rederect_if_not_found = Some(val),
        Err(_) => (),
    }

    return config;
});
