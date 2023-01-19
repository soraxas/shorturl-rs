use serde::{Deserialize, Serialize};

pub type ShortCode = String;
pub type Url = String;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShortUrlMapping {
    pub id: ShortCode,
    pub url: Url,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Meta {
    pub address: Option<String>,
    pub header: Option<String>,
}