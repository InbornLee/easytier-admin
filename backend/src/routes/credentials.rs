use axum::extract::{Json, Path, Query, State};
use axum::response::Json as JsonResponse;
use axum::routing::{delete, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::AppResult;
use crate::middleware::{parse_body, AuthUser};
use crate::services::audit::{record_audit, AuditInput};
use crate::services::credential::{
    create_credential, delete_credential, get_credential_row_or_throw, list_credentials_scoped,
    revoke_credential, CreateCredentialInput,
};
use crate::services::network::require_network_access;
use crate::state::SharedState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CredentialQuery {
    network_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCredentialBody {
    network_id: String,
    node_id: Option<String>,
    #[serde(default = "default_ttl")]
    ttl_seconds: i64,
    groups: Option<Vec<String>>,
    allow_relay: Option<bool>,
    reusable: Option<bool>,
    allowed_proxy_cidrs: Option<Vec<String>>,
}

fn default_ttl() -> i64 {
    7 * 24 * 3600
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route(
            "/api/credentials",
            axum::routing::get(list_handler).post(create_handler),
        )
        .route("/api/credentials/{id}/revoke", post(revoke_handler))
        .route("/api/credentials/{id}", delete(delete_handler))
}

async fn list_handler(
    State(state): State<SharedState>,
    Query(query): Query<CredentialQuery>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    Ok(JsonResponse(json!({
        "items": list_credentials_scoped(&state, &user, query.network_id.as_deref())?,
    })))
}

async fn create_handler(
    State(state): State<SharedState>,
    user: AuthUser,
    Json(body): Json<Value>,
) -> AppResult<JsonResponse<Value>> {
    let input: CreateCredentialBody = parse_body(body)?;
    require_network_access(&state, &input.network_id, &user, true)?;
    let network_id = input.network_id.clone();
    let ttl = input.ttl_seconds;
    let (credential, secret) = create_credential(
        &state,
        CreateCredentialInput {
            network_id: input.network_id,
            node_id: input.node_id,
            ttl_seconds: Some(input.ttl_seconds),
            groups: input.groups,
            allow_relay: input.allow_relay,
            reusable: input.reusable,
            allowed_proxy_cidrs: input.allowed_proxy_cidrs,
            credential_id: None,
            created_by: Some(user.id.clone()),
        },
    )
    .await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "credential.create".to_string(),
            resource_type: Some("credential".to_string()),
            resource_id: Some(credential.id.clone()),
            detail: Some(json!({
                "networkId": network_id,
                "credentialId": credential.credential_id,
                "ttlSeconds": ttl,
            })),
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({
        "credential": credential,
        "secret": secret,
    })))
}

async fn revoke_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let row = get_credential_row_or_throw(&state, &id)?;
    require_network_access(&state, &row.network_id, &user, true)?;
    let credential = revoke_credential(&state, &id).await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "credential.revoke".to_string(),
            resource_type: Some("credential".to_string()),
            resource_id: Some(id),
            detail: None,
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "credential": credential })))
}

async fn delete_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let row = get_credential_row_or_throw(&state, &id)?;
    require_network_access(&state, &row.network_id, &user, true)?;
    delete_credential(&state, &id).await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id),
            username: Some(user.username),
            action: "credential.delete".to_string(),
            resource_type: Some("credential".to_string()),
            resource_id: Some(id),
            detail: None,
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "ok": true })))
}
