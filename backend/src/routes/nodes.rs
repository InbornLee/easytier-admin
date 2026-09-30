use axum::extract::{Json, Path, Query, State};
use axum::response::Json as JsonResponse;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::{bad_request, AppResult};
use crate::middleware::{parse_body, AuthUser};
use crate::services::audit::{record_audit, AuditInput};
use crate::services::network::require_network_access;
use crate::services::node::{
    create_node, delete_node, double_option, enrich_nodes_with_live, get_node_join,
    get_node_row_or_throw, list_nodes_scoped, rotate_node_credential, to_public_node, update_node,
    CreateNodeInput, RotateCredentialInput, UpdateNodeInput,
};
use crate::state::SharedState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodeCreateBody {
    network_id: String,
    name: String,
    hostname: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    ipv4: Option<Option<String>>,
    description: Option<String>,
    listeners: Option<Vec<String>>,
    proxy_networks: Option<Vec<String>>,
    flags: Option<Map<String, Value>>,
    issue_credential: Option<bool>,
    ttl_seconds: Option<i64>,
    groups: Option<Vec<String>>,
    allow_relay: Option<bool>,
    reusable: Option<bool>,
    allowed_proxy_cidrs: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodePatchBody {
    name: Option<String>,
    hostname: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    ipv4: Option<Option<String>>,
    description: Option<String>,
    listeners: Option<Vec<String>>,
    proxy_networks: Option<Vec<String>>,
    flags: Option<Map<String, Value>>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RotateBody {
    ttl_seconds: Option<i64>,
    allow_relay: Option<bool>,
    reusable: Option<bool>,
    groups: Option<Vec<String>>,
    allowed_proxy_cidrs: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodesQuery {
    network_id: Option<String>,
    live: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JoinQuery {
    listen_port: Option<i64>,
    peer: Option<String>,
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/nodes", get(list_handler).post(create_handler))
        .route(
            "/api/nodes/{id}",
            get(get_handler).patch(patch_handler).delete(delete_handler),
        )
        .route("/api/nodes/{id}/join", get(join_handler))
        .route("/api/nodes/{id}/rotate-credential", post(rotate_handler))
}

async fn list_handler(
    State(state): State<SharedState>,
    Query(query): Query<NodesQuery>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let mut nodes = list_nodes_scoped(&state, &user, query.network_id.as_deref())?;
    if query.live.as_deref() != Some("false") {
        nodes = enrich_nodes_with_live(&state, nodes).await?;
    }
    Ok(JsonResponse(json!({ "items": nodes })))
}

async fn create_handler(
    State(state): State<SharedState>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> AppResult<JsonResponse<Value>> {
    let input: NodeCreateBody = parse_body(body)?;
    if input.name.trim().is_empty() {
        return Err(bad_request("节点名称不能为空", None));
    }
    require_network_access(&state, &input.network_id, &user, true)?;
    let network_id = input.network_id.clone();
    let (node, credential) = create_node(
        &state,
        CreateNodeInput {
            network_id: input.network_id,
            name: input.name,
            hostname: input.hostname,
            ipv4: input.ipv4,
            description: input.description,
            listeners: input.listeners,
            proxy_networks: input.proxy_networks,
            flags: input.flags,
            issue_credential: input.issue_credential,
            ttl_seconds: input.ttl_seconds,
            groups: input.groups,
            allow_relay: input.allow_relay,
            reusable: input.reusable,
            allowed_proxy_cidrs: input.allowed_proxy_cidrs,
            created_by: Some(user.id.clone()),
        },
    )
    .await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "node.create".to_string(),
            resource_type: Some("node".to_string()),
            resource_id: Some(node.id.clone()),
            detail: Some(json!({ "name": node.name, "networkId": network_id })),
            ip: None,
            user_agent: None,
        },
    );
    let (credential_value, secret) = match credential {
        Some((cred, secret)) => (Some(cred), Some(secret)),
        None => (None, None),
    };
    Ok(JsonResponse(json!({
        "node": node,
        "credential": credential_value,
        "credentialSecret": secret,
    })))
}

async fn get_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let node = get_node_row_or_throw(&state, &id)?;
    require_network_access(&state, &node.network_id, &user, false)?;
    let enriched = enrich_nodes_with_live(&state, vec![to_public_node(&node)]).await?;
    Ok(JsonResponse(json!({
        "node": enriched.into_iter().next(),
    })))
}

async fn patch_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> AppResult<JsonResponse<Value>> {
    let existing = get_node_row_or_throw(&state, &id)?;
    require_network_access(&state, &existing.network_id, &user, true)?;
    let input: NodePatchBody = parse_body(body)?;
    let node = update_node(
        &state,
        &id,
        UpdateNodeInput {
            name: input.name,
            hostname: input.hostname,
            ipv4: input.ipv4,
            description: input.description,
            listeners: input.listeners,
            proxy_networks: input.proxy_networks,
            flags: input.flags,
        },
    )?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "node.update".to_string(),
            resource_type: Some("node".to_string()),
            resource_id: Some(id),
            detail: None,
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "node": node })))
}

async fn delete_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let node = get_node_row_or_throw(&state, &id)?;
    require_network_access(&state, &node.network_id, &user, true)?;
    delete_node(&state, &id).await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "node.delete".to_string(),
            resource_type: Some("node".to_string()),
            resource_id: Some(id),
            detail: Some(json!({ "name": node.name })),
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "ok": true })))
}

async fn join_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(query): Query<JoinQuery>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let node = get_node_row_or_throw(&state, &id)?;
    require_network_access(&state, &node.network_id, &user, false)?;
    let listen_port = query.listen_port.filter(|p| (1..=65535).contains(p));
    let join = get_node_join(&state, &id, listen_port, query.peer.as_deref()).await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "node.view-join".to_string(),
            resource_type: Some("node".to_string()),
            resource_id: Some(id),
            detail: None,
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!(join)))
}

async fn rotate_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> AppResult<JsonResponse<Value>> {
    let node = get_node_row_or_throw(&state, &id)?;
    require_network_access(&state, &node.network_id, &user, true)?;
    let input: RotateBody = parse_body(body)?;
    let (node, credential, secret) = rotate_node_credential(
        &state,
        &id,
        RotateCredentialInput {
            ttl_seconds: input.ttl_seconds,
            allow_relay: input.allow_relay,
            reusable: input.reusable,
            groups: input.groups,
            allowed_proxy_cidrs: input.allowed_proxy_cidrs,
        },
    )
    .await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "node.rotate-credential".to_string(),
            resource_type: Some("node".to_string()),
            resource_id: Some(id),
            detail: None,
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({
        "node": node,
        "credential": credential,
        "credentialSecret": secret,
    })))
}
