pub mod audit;
pub mod auth;
pub mod credential;
pub mod dashboard;
pub mod log;
pub mod metrics;
pub mod network;
pub mod node;
pub mod settings;
pub mod topology;

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// 安全解析 JSON 字符串数组
pub(crate) fn safe_string_array(text: &str) -> Vec<String> {
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Array(items)) => items
            .into_iter()
            .map(|v| match v {
                Value::String(s) => s,
                other => other.to_string(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// 安全解析 JSON 对象
pub(crate) fn safe_object(text: &str) -> Map<String, Value> {
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}
