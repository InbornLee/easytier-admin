use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::error::AppResult;
use crate::state::AppState;
use crate::util::now_ms;

pub const EDITABLE_KEYS: [&str; 6] = [
    "easytier.corePath",
    "easytier.cliPath",
    "easytier.defaultExternalNode",
    "easytier.rpcPortStart",
    "easytier.listenPortStart",
    "log.retentionDays",
];

pub const LOG_RETENTION_KEY: &str = "log.retentionDays";
pub const DEFAULT_LOG_RETENTION_DAYS: i64 = 30;

/// 数据库中的原始覆盖项
pub fn get_settings(state: &AppState) -> AppResult<HashMap<String, String>> {
    let mut map = HashMap::new();
    for (key, value) in state.db.get_settings()? {
        map.insert(key, value);
    }
    Ok(map)
}

/// 当前生效的设置
pub fn get_effective_settings(state: &AppState) -> Map<String, Value> {
    let s = state.easytier_settings();
    let mut map = Map::new();
    map.insert("easytier.corePath".to_string(), json!(s.core_path));
    map.insert("easytier.cliPath".to_string(), json!(s.cli_path));
    map.insert(
        "easytier.defaultExternalNode".to_string(),
        json!(s.default_external_node),
    );
    map.insert("easytier.rpcPortStart".to_string(), json!(s.rpc_port_start));
    map.insert(
        "easytier.listenPortStart".to_string(),
        json!(s.listen_port_start),
    );
    map.insert(
        LOG_RETENTION_KEY.to_string(),
        json!(get_log_retention_days(state)),
    );
    map
}

/// 日志（运行日志与操作审计）保留天数；0 表示永久保留。
pub fn get_log_retention_days(state: &AppState) -> i64 {
    state
        .db
        .get_setting(LOG_RETENTION_KEY)
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|v| *v >= 0)
        .unwrap_or(DEFAULT_LOG_RETENTION_DAYS)
}

pub fn update_settings(state: &AppState, patch: &Map<String, Value>) -> AppResult<()> {
    let now = now_ms();
    for key in EDITABLE_KEYS {
        let Some(value) = patch.get(key) else {
            continue;
        };
        let text = match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        state.db.upsert_setting(key, &text, now)?;
    }
    apply_runtime_overrides(state);
    Ok(())
}

/// 将数据库中的设置应用到运行时
pub fn apply_runtime_overrides(state: &AppState) {
    let Ok(map) = get_settings(state) else {
        return;
    };
    let mut settings = state.easytier_settings();
    if let Some(v) = map.get("easytier.corePath") {
        if !v.is_empty() {
            settings.core_path = v.clone();
        }
    }
    if let Some(v) = map.get("easytier.cliPath") {
        if !v.is_empty() {
            settings.cli_path = v.clone();
        }
    }
    if let Some(v) = map.get("easytier.defaultExternalNode") {
        if !v.is_empty() {
            settings.default_external_node = v.clone();
        }
    }
    if let Some(v) = map
        .get("easytier.rpcPortStart")
        .and_then(|v| v.parse::<u16>().ok())
    {
        settings.rpc_port_start = v;
    }
    if let Some(v) = map
        .get("easytier.listenPortStart")
        .and_then(|v| v.parse::<u16>().ok())
    {
        settings.listen_port_start = v;
    }
    if let Ok(mut guard) = state.easytier.write() {
        *guard = settings;
    }
}
