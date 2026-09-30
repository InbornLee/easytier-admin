use axum::extract::{Json, State};
use axum::response::Json as JsonResponse;
use axum::routing::get;
use axum::Router;
use serde_json::{json, Map, Value};

use crate::easytier::cli::check_binaries;
use crate::error::{bad_request, AppResult};
use crate::middleware::{AdminUser, AuthUser};
use crate::services::audit::{record_audit, AuditInput};
use crate::services::settings::{get_effective_settings, get_settings, update_settings};
use crate::state::SharedState;
use crate::util::now_ms;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/system/health", get(health_handler))
        .route("/api/system/info", get(info_handler))
        .route("/api/system/binaries", get(binaries_handler))
        .route(
            "/api/system/settings",
            get(settings_get_handler).put(settings_put_handler),
        )
}

async fn health_handler(State(state): State<SharedState>) -> JsonResponse<Value> {
    JsonResponse(json!({
        "status": "ok",
        "time": now_ms(),
        "version": "1.0.0",
        "env": state.config.env,
    }))
}

async fn info_handler(
    State(state): State<SharedState>,
    _user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    Ok(JsonResponse(json!({
        "version": "1.0.0",
        "node": option_env!("RUSTC_VERSION").unwrap_or("Rust"),
        "platform": format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
        "uptime": state.started_at.elapsed().as_secs(),
        "dataDir": state.config.data_dir_string(),
        "easytier": get_effective_settings(&state),
    })))
}

async fn binaries_handler(
    State(state): State<SharedState>,
    _user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let settings = state.easytier_settings();
    let status = check_binaries(&settings.cli_path, &settings.core_path).await;
    Ok(JsonResponse(json!(status)))
}

async fn settings_get_handler(
    State(state): State<SharedState>,
    _admin: AdminUser,
) -> AppResult<JsonResponse<Value>> {
    Ok(JsonResponse(json!({
        "effective": get_effective_settings(&state),
        "overrides": get_settings(&state)?,
    })))
}

fn validate_port(key: &str, value: &Value) -> AppResult<()> {
    let valid = value
        .as_i64()
        .map(|n| (1..=65535).contains(&n))
        .unwrap_or(false);
    if !valid {
        return Err(bad_request(format!("{key} 需为 1-65535 的整数"), None));
    }
    Ok(())
}

fn validate_non_empty(key: &str, value: &Value) -> AppResult<()> {
    let valid = value.as_str().map(|s| !s.is_empty()).unwrap_or(false);
    if !valid {
        return Err(bad_request(format!("{key} 不能为空"), None));
    }
    Ok(())
}

async fn settings_put_handler(
    State(state): State<SharedState>,
    admin: AdminUser,
    Json(body): Json<Value>,
) -> AppResult<JsonResponse<Value>> {
    let Some(object) = body.as_object() else {
        return Err(bad_request("参数校验失败", None));
    };

    let string_keys = [
        "easytier.corePath",
        "easytier.cliPath",
        "easytier.defaultExternalNode",
    ];
    let port_keys = ["easytier.rpcPortStart", "easytier.listenPortStart"];

    let mut patch: Map<String, Value> = Map::new();
    for key in string_keys {
        if let Some(value) = object.get(key) {
            validate_non_empty(key, value)?;
            patch.insert(key.to_string(), value.clone());
        }
    }
    for key in port_keys {
        if let Some(value) = object.get(key) {
            validate_port(key, value)?;
            patch.insert(key.to_string(), value.clone());
        }
    }
    if let Some(value) = object.get("log.retentionDays") {
        let valid = value
            .as_i64()
            .map(|n| (0..=3650).contains(&n))
            .unwrap_or(false);
        if !valid {
            return Err(bad_request(
                "log.retentionDays 需为 0-3650 的整数（0 表示永久保留）",
                None,
            ));
        }
        patch.insert("log.retentionDays".to_string(), value.clone());
    }

    update_settings(&state, &patch)?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(admin.0.id),
            username: Some(admin.0.username),
            action: "system.update-settings".to_string(),
            resource_type: Some("settings".to_string()),
            resource_id: None,
            detail: Some(Value::Object(patch)),
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({
        "effective": get_effective_settings(&state),
    })))
}
