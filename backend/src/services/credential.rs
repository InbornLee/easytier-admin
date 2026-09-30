use std::time::Duration;

use serde::Serialize;
use uuid::Uuid;

use crate::db::CredentialRow;
use crate::error::{bad_request, not_found, AppResult};
use crate::state::AppState;
use crate::util::now_ms;

use super::network::{accessible_network_ids, get_network_row_or_throw, require_network_access};
use super::safe_string_array;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicCredential {
    pub id: String,
    pub network_id: String,
    pub node_id: Option<String>,
    pub credential_id: String,
    pub groups: Vec<String>,
    pub allow_relay: bool,
    pub reusable: bool,
    pub allowed_proxy_cidrs: Vec<String>,
    pub ttl_seconds: i64,
    pub expires_at: i64,
    pub revoked: bool,
    pub created_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: String,
    pub remaining_seconds: i64,
}

pub fn to_public_credential(row: &CredentialRow) -> PublicCredential {
    let now = now_ms();
    let status = if row.revoked {
        "revoked"
    } else if row.expires_at <= now {
        "expired"
    } else {
        "active"
    };
    PublicCredential {
        id: row.id.clone(),
        network_id: row.network_id.clone(),
        node_id: row.node_id.clone(),
        credential_id: row.credential_id.clone(),
        groups: safe_string_array(&row.groups),
        allow_relay: row.allow_relay,
        reusable: row.reusable,
        allowed_proxy_cidrs: safe_string_array(&row.allowed_proxy_cidrs),
        ttl_seconds: row.ttl_seconds,
        expires_at: row.expires_at,
        revoked: row.revoked,
        created_by: row.created_by.clone(),
        created_at: row.created_at,
        updated_at: row.updated_at,
        status: status.to_string(),
        remaining_seconds: ((row.expires_at - now) / 1000).max(0),
    }
}

pub fn list_credentials(
    state: &AppState,
    network_id: Option<&str>,
) -> AppResult<Vec<PublicCredential>> {
    Ok(state
        .db
        .list_credentials(network_id)?
        .iter()
        .map(to_public_credential)
        .collect())
}

/// 按用户可见范围列出凭据：管理员全部，其他用户仅自己可见网络下的凭据。
pub fn list_credentials_scoped(
    state: &AppState,
    user: &crate::middleware::AuthUser,
    network_id: Option<&str>,
) -> AppResult<Vec<PublicCredential>> {
    if let Some(nid) = network_id {
        require_network_access(state, nid, user, false)?;
        return list_credentials(state, Some(nid));
    }
    let allowed = accessible_network_ids(state, user)?;
    let all = list_credentials(state, None)?;
    Ok(match allowed {
        None => all,
        Some(set) => all
            .into_iter()
            .filter(|c| set.contains(&c.network_id))
            .collect(),
    })
}

pub fn get_credential_row(state: &AppState, id: &str) -> AppResult<Option<CredentialRow>> {
    state.db.get_credential(id)
}

pub fn get_credential_row_or_throw(state: &AppState, id: &str) -> AppResult<CredentialRow> {
    get_credential_row(state, id)?.ok_or_else(|| not_found("凭据不存在"))
}

#[derive(Default)]
pub struct CreateCredentialInput {
    pub network_id: String,
    pub node_id: Option<String>,
    pub ttl_seconds: Option<i64>,
    pub groups: Option<Vec<String>>,
    pub allow_relay: Option<bool>,
    pub reusable: Option<bool>,
    pub allowed_proxy_cidrs: Option<Vec<String>>,
    pub credential_id: Option<String>,
    pub created_by: Option<String>,
}

pub async fn create_credential(
    state: &AppState,
    input: CreateCredentialInput,
) -> AppResult<(PublicCredential, String)> {
    let network = get_network_row_or_throw(state, &input.network_id)?;
    if !state.pm.is_running(&network.id) {
        return Err(bad_request(
            "网络实例未运行，无法签发凭据。请先启动该网络。",
            None,
        ));
    }
    let ttl = input.ttl_seconds.unwrap_or(7 * 24 * 3600).max(60);

    let cli = state.cli(network.rpc_port as u16, Duration::from_millis(10000));
    let response = cli
        .credential_generate(
            ttl,
            input.credential_id.as_deref(),
            input.groups.as_deref(),
            input.allow_relay.unwrap_or(false),
            input.allowed_proxy_cidrs.as_deref(),
            input.reusable.unwrap_or(true),
        )
        .await?;

    if response.credential_id.is_empty() || response.credential_secret.is_empty() {
        return Err(bad_request(
            "凭据签发失败：easytier-cli 返回内容不完整",
            None,
        ));
    }

    let expires_at = if response.expiry_unix > 0 {
        response.expiry_unix * 1000
    } else {
        now_ms() + ttl * 1000
    };
    let ts = now_ms();
    let reusable = input.reusable.unwrap_or(true);

    let row = CredentialRow {
        id: Uuid::new_v4().to_string(),
        network_id: network.id.clone(),
        node_id: input.node_id.clone(),
        credential_id: response.credential_id.clone(),
        secret_enc: state.crypto.encrypt(&response.credential_secret),
        groups: serde_json::to_string(&input.groups.clone().unwrap_or_default())?,
        allow_relay: input.allow_relay.unwrap_or(false),
        reusable,
        allowed_proxy_cidrs: serde_json::to_string(
            &input.allowed_proxy_cidrs.clone().unwrap_or_default(),
        )?,
        ttl_seconds: ttl,
        expires_at,
        revoked: false,
        created_by: input.created_by.clone(),
        created_at: ts,
        updated_at: ts,
    };
    state.db.insert_credential(&row)?;
    Ok((to_public_credential(&row), response.credential_secret))
}

pub async fn revoke_credential(state: &AppState, id: &str) -> AppResult<PublicCredential> {
    let row = get_credential_row_or_throw(state, id)?;
    let network = get_network_row_or_throw(state, &row.network_id)?;
    if state.pm.is_running(&network.id) {
        let cli = state.cli(network.rpc_port as u16, Duration::from_millis(8000));
        // 即使 RPC 撤销失败也标记为已撤销；重启后可再次同步
        let _ = cli.credential_revoke(&row.credential_id).await;
    }
    state.db.set_credential_revoked(id, true, now_ms())?;
    Ok(to_public_credential(&get_credential_row_or_throw(
        state, id,
    )?))
}

pub async fn delete_credential(state: &AppState, id: &str) -> AppResult<()> {
    let row = get_credential_row_or_throw(state, id)?;
    if !row.revoked {
        let _ = revoke_credential(state, id).await;
    }
    state.db.delete_credential(id)?;
    state.db.clear_node_credential_refs(id, now_ms())?;
    Ok(())
}

pub fn get_credential_secret(state: &AppState, id: &str) -> AppResult<String> {
    let row = get_credential_row_or_throw(state, id)?;
    Ok(state.crypto.try_decrypt(Some(&row.secret_enc)))
}
