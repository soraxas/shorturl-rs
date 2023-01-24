use serde::{Deserialize, Serialize};

pub type Url = String;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShortUrlMapping {
    // pub id: ShortCode,
    pub url: Url,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Meta {
    pub address: Option<String>,
    pub header: Option<String>,
}

#[derive(Debug, Copy, Clone)]
pub enum MetaType {
    Create = 1,
    Access = 2,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AccessLog {
    pub code: String,
    pub url: Option<Url>,
    pub last_access: Option<String>,
    pub access_count: u16,
}
