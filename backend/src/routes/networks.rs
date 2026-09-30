use axum::extract::{Json, Path, Query, State};
use axum::response::Json as JsonResponse;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::{bad_request, forbidden, not_found, AppResult};
use crate::middleware::{parse_body, AuthUser};
use crate::services::audit::{record_audit, AuditInput};
use crate::services::credential::list_credentials;
use crate::services::metrics::remove_network_series;
use crate::services::network::{
    access_level, create_network, delete_network, get_config_text, get_live_state,
    get_network_row_or_throw, list_networks_for_user, require_network_access, restart_network,
    start_network, stop_network, to_public_network, transfer_network_owner, update_network,
    AccessLevel, NetworkFields,
};
use crate::services::node::{enrich_nodes_with_live, list_nodes};
use crate::services::topology::build_topology;
use crate::state::SharedState;
use crate::util::now_ms;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct NetworkInput {
    name: Option<String>,
    description: Option<String>,
    network_name: Option<String>,
    network_secret: Option<String>,
    ipv4: Option<String>,
    dhcp: Option<bool>,
    hostname: Option<String>,
    instance_name: Option<String>,
    listeners: Option<Vec<String>>,
    mapped_listeners: Option<Vec<String>>,
    peers: Option<Vec<String>>,
    external_node: Option<String>,
    listen_port: Option<i64>,
    rpc_port: Option<i64>,
    secure_mode: Option<bool>,
    auto_start: Option<bool>,
    start_now: Option<bool>,
    flags: Option<Map<String, Value>>,
}

impl From<NetworkInput> for NetworkFields {
    fn from(input: NetworkInput) -> Self {
        NetworkFields {
            name: input.name,
            description: input.description,
            network_name: input.network_name,
            network_secret: input.network_secret,
            ipv4: input.ipv4,
            dhcp: input.dhcp,
            hostname: input.hostname,
            instance_name: input.instance_name,
            listeners: input.listeners,
            mapped_listeners: input.mapped_listeners,
            peers: input.peers,
            external_node: input.external_node,
            listen_port: input.listen_port,
            rpc_port: input.rpc_port,
            secure_mode: input.secure_mode,
            auto_start: input.auto_start,
            start_now: input.start_now,
            flags: input.flags,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShareBody {
    user_id: String,
    permission: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferBody {
    user_id: String,
}

fn validate(input: &NetworkInput, require_name: bool) -> AppResult<()> {
    if require_name {
        match input.name.as_deref() {
            Some(n) if !n.trim().is_empty() && n.chars().count() <= 64 => {}
            _ => return Err(bad_request("网络名称不能为空且不超过 64 字符", None)),
        }
    }
    if let Some(n) = input.network_name.as_deref() {
        if n.chars().count() > 128 {
            return Err(bad_request("EasyTier 网络名称不能超过 128 字符", None));
        }
    }
    if let Some(s) = input.network_secret.as_deref() {
        let len = s.chars().count();
        if !(6..=128).contains(&len) {
            return Err(bad_request("网络密钥长度需为 6-128 位", None));
        }
    }
    for (label, port) in [
        ("监听端口", input.listen_port),
        ("RPC 端口", input.rpc_port),
    ] {
        if let Some(port) = port {
            if !(1..=65535).contains(&port) {
                return Err(bad_request(format!("{label}超出范围"), None));
            }
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct LogsQuery {
    limit: Option<usize>,
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/networks", get(list_handler).post(create_handler))
        .route(
            "/api/networks/{id}",
            get(get_handler)
                .patch(update_handler)
                .delete(delete_handler),
        )
        .route("/api/networks/{id}/start", post(start_handler))
        .route("/api/networks/{id}/stop", post(stop_handler))
        .route("/api/networks/{id}/restart", post(restart_handler))
        .route("/api/networks/{id}/config", get(config_handler))
        .route("/api/networks/{id}/live", get(live_handler))
        .route("/api/networks/{id}/topology", get(topology_handler))
        .route("/api/networks/{id}/peers", get(peers_handler))
        .route("/api/networks/{id}/routes", get(routes_handler))
        .route("/api/networks/{id}/logs", get(runtime_logs_handler))
        .route("/api/networks/{id}/credentials", get(credentials_handler))
        .route("/api/networks/{id}/nodes", get(nodes_handler))
        .route(
            "/api/networks/{id}/shares",
            get(list_shares_handler).post(create_share_handler),
        )
        .route(
            "/api/networks/{id}/shares/{user_id}",
            axum::routing::delete(delete_share_handler),
        )
        .route("/api/networks/{id}/transfer", post(transfer_handler))
}

async fn list_handler(
    State(state): State<SharedState>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    Ok(JsonResponse(json!({
        "items": list_networks_for_user(&state, &user)?,
    })))
}

async fn create_handler(
    State(state): State<SharedState>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> AppResult<JsonResponse<Value>> {
    let input: NetworkInput = parse_body(body)?;
    validate(&input, true)?;
    let network = create_network(&state, input.into(), Some(&user.id)).await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "network.create".to_string(),
            resource_type: Some("network".to_string()),
            resource_id: Some(network.id.clone()),
            detail: Some(json!({ "name": network.name, "networkName": network.network_name })),
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "network": network })))
}

async fn get_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let row = require_network_access(&state, &id, &user, false)?;
    let level = access_level(&state, &row, &user)?.unwrap_or(AccessLevel::View);
    Ok(JsonResponse(json!({
        "network": to_public_network(&state, &row, level),
    })))
}

async fn update_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, true)?;
    let input: NetworkInput = parse_body(body)?;
    validate(&input, false)?;
    let network = update_network(&state, &id, input.into()).await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "network.update".to_string(),
            resource_type: Some("network".to_string()),
            resource_id: Some(id),
            detail: None,
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "network": network })))
}

async fn delete_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let row = require_network_access(&state, &id, &user, true)?;
    delete_network(&state, &id).await?;
    remove_network_series(&state, &id);
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "network.delete".to_string(),
            resource_type: Some("network".to_string()),
            resource_id: Some(id),
            detail: Some(json!({ "name": row.name })),
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "ok": true })))
}

async fn start_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, true)?;
    start_network(&state, &id).await?;
    audit_action(&state, &user, "network.start", &id);
    Ok(JsonResponse(json!({ "ok": true })))
}

async fn stop_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, true)?;
    stop_network(&state, &id).await?;
    audit_action(&state, &user, "network.stop", &id);
    Ok(JsonResponse(json!({ "ok": true })))
}

async fn restart_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, true)?;
    restart_network(&state, &id).await?;
    audit_action(&state, &user, "network.restart", &id);
    Ok(JsonResponse(json!({ "ok": true })))
}

async fn config_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, false)?;
    Ok(JsonResponse(
        json!({ "config": get_config_text(&state, &id)? }),
    ))
}

async fn live_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, false)?;
    let live = get_live_state(&state, &id).await?;
    Ok(JsonResponse(json!(live)))
}

async fn topology_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let row = require_network_access(&state, &id, &user, false)?;
    let live = get_live_state(&state, &id).await?;
    let nodes = list_nodes(&state, Some(&id))?;
    let topology = build_topology(&row, &live, &nodes);
    Ok(JsonResponse(json!(topology)))
}

async fn peers_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, false)?;
    let live = get_live_state(&state, &id).await?;
    Ok(JsonResponse(json!({
        "online": live.online,
        "peers": live.peers,
        "error": live.error,
    })))
}

async fn routes_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, false)?;
    let live = get_live_state(&state, &id).await?;
    Ok(JsonResponse(json!({
        "online": live.online,
        "routes": live.routes,
        "error": live.error,
    })))
}

async fn runtime_logs_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(query): Query<LogsQuery>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, false)?;
    let limit = query.limit.unwrap_or(300);
    Ok(JsonResponse(json!({
        "items": state.pm.recent_logs(&id, limit),
    })))
}

async fn credentials_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, false)?;
    Ok(JsonResponse(
        json!({ "items": list_credentials(&state, Some(&id))? }),
    ))
}

async fn nodes_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, false)?;
    let nodes = enrich_nodes_with_live(&state, list_nodes(&state, Some(&id))?).await?;
    Ok(JsonResponse(json!({ "items": nodes })))
}

// ------------------------------------------------------------------ 分享

async fn list_shares_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let row = require_network_access(&state, &id, &user, true)?;
    let shares = state.db.list_shares(&id)?;
    let items: Vec<Value> = shares
        .iter()
        .map(|s| {
            let target = state.db.get_user_by_id(&s.user_id).ok().flatten();
            json!({
                "userId": s.user_id,
                "username": target.as_ref().map(|u| u.username.clone()).unwrap_or_default(),
                "displayName": target.as_ref().and_then(|u| u.display_name.clone()),
                "role": target.as_ref().map(|u| u.role.clone()).unwrap_or_default(),
                "permission": s.permission,
                "createdAt": s.created_at,
            })
        })
        .collect();
    let owner = row
        .owner_id
        .as_deref()
        .and_then(|oid| state.db.get_user_by_id(oid).ok().flatten())
        .map(|o| json!({ "id": o.id, "username": o.username, "displayName": o.display_name }));
    Ok(JsonResponse(json!({ "owner": owner, "items": items })))
}

async fn create_share_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> AppResult<JsonResponse<Value>> {
    let row = require_network_access(&state, &id, &user, true)?;
    let input: ShareBody = parse_body(body)?;
    if input.user_id == user.id {
        return Err(bad_request("不能分享给自己", None));
    }
    if row.owner_id.as_deref() == Some(input.user_id.as_str()) {
        return Err(bad_request("不能分享给网络所有者", None));
    }
    let target = state
        .db
        .get_user_by_id(&input.user_id)?
        .ok_or_else(|| not_found("目标用户不存在"))?;
    if target.disabled {
        return Err(bad_request("目标用户已被禁用", None));
    }
    let permission = input.permission.unwrap_or_else(|| "view".to_string());
    if permission != "view" && permission != "manage" {
        return Err(bad_request("权限只能是 view 或 manage", None));
    }
    state
        .db
        .upsert_share(&id, &input.user_id, &permission, now_ms())?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "network.share".to_string(),
            resource_type: Some("network".to_string()),
            resource_id: Some(id),
            detail: Some(json!({ "targetUser": input.user_id, "permission": permission })),
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "ok": true })))
}

async fn delete_share_handler(
    State(state): State<SharedState>,
    Path((id, target_user_id)): Path<(String, String)>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &id, &user, true)?;
    state.db.delete_share(&id, &target_user_id)?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "network.unshare".to_string(),
            resource_type: Some("network".to_string()),
            resource_id: Some(id),
            detail: Some(json!({ "targetUser": target_user_id })),
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "ok": true })))
}

async fn transfer_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> AppResult<JsonResponse<Value>> {
    let row = get_network_row_or_throw(&state, &id)?;
    let is_admin = user.role == "admin";
    let is_owner = row.owner_id.as_deref() == Some(user.id.as_str());
    if !is_admin && !is_owner {
        return Err(forbidden("只有网络所有者或管理员可以转移归属"));
    }
    let input: TransferBody = parse_body(body)?;
    if input.user_id == user.id {
        return Err(bad_request("不能转移给自己", None));
    }
    let target = state
        .db
        .get_user_by_id(&input.user_id)?
        .ok_or_else(|| not_found("目标用户不存在"))?;
    if target.disabled {
        return Err(bad_request("目标用户已被禁用", None));
    }
    let previous = row.owner_id.clone();
    let updated = transfer_network_owner(&state, &row, &input.user_id)?;
    let level = access_level(&state, &updated, &user)?.unwrap_or(AccessLevel::Manage);
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "network.transfer".to_string(),
            resource_type: Some("network".to_string()),
            resource_id: Some(id),
            detail: Some(json!({
                "from": previous,
                "to": input.user_id,
                "toUsername": target.username,
            })),
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(
        json!({ "network": to_public_network(&state, &updated, level) }),
    ))
}

fn audit_action(state: &SharedState, user: &AuthUser, action: &str, id: &str) {
    record_audit(
        state,
        AuditInput {
            user_id: Some(user.id.clone()),
            username: Some(user.username.clone()),
            action: action.to_string(),
            resource_type: Some("network".to_string()),
            resource_id: Some(id.to_string()),
            detail: None,
            ip: None,
            user_agent: None,
        },
    );
}
