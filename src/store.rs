use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use parking_lot::RwLock;


type Items = HashMap<String, String>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Id {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Item {
    pub short: String,
    pub long_url: String,
}

#[derive(Clone)]
pub struct Store {
  pub grocery_list: Arc<RwLock<Items>>
}

impl Store {
    pub fn new() -> Self {
        Store {
            grocery_list: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
