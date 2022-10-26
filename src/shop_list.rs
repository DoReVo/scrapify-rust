use core::panic;
use std::path::Path;

use serde::{Deserialize, Serialize};

// Path to config file containing the list of shops to scrape
const CONFIG_PATH: &str = "shop_list.yaml";

#[derive(Serialize, Deserialize)]
pub struct ShopConfig {
    pub shop_id: String,
    pub shop_name: String,
    pub search_query: String,
    pub product_match: Vec<String>,
}

pub async fn read_config() -> Vec<ShopConfig> {
    // Read shop list from disk
    let raw_bytes = match tokio::fs::read(Path::new(&CONFIG_PATH)).await {
        Ok(raw_bytes) => raw_bytes,
        Err(err) => panic!("Error reading config file {:?}", err),
    };
    // Parse shop list
    let shop_list: Vec<ShopConfig> = match serde_yaml::from_slice(&raw_bytes) {
        Ok(shop_list) => shop_list,
        Err(err) => panic!("Error parsing YAML config into struct {:?}", err),
    };

    return shop_list;
}
