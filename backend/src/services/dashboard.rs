use serde::Serialize;

use crate::error::AppResult;
use crate::middleware::AuthUser;
use crate::state::AppState;
use crate::util::now_ms;

use super::metrics::{get_traffic_series, TrafficPoint};
use super::network::{accessible_network_ids, require_network_access};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsersSummary {
    pub total: i64,
    pub admins: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworksSummary {
    pub total: i64,
    pub running: i64,
    pub error: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodesSummary {
    pub total: i64,
    pub online: i64,
    pub offline: i64,
    pub unknown: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsSummary {
    pub total: i64,
    pub active: i64,
    pub expiring_soon: i64,
    pub expired: i64,
    pub revoked: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentAudit {
    pub id: i64,
    pub username: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub created_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentError {
    pub id: i64,
    pub network_id: Option<String>,
    pub message: String,
    pub created_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub initialized: bool,
    pub users: UsersSummary,
    pub networks: NetworksSummary,
    pub nodes: NodesSummary,
    pub credentials: CredentialsSummary,
    pub recent_audit: Vec<RecentAudit>,
    pub recent_errors: Vec<RecentError>,
}

pub fn get_dashboard_summary(state: &AppState, user: &AuthUser) -> AppResult<DashboardSummary> {
    let allowed = accessible_network_ids(state, user)?;
    let network_rows = match &allowed {
        None => state.db.list_networks()?,
        Some(_) => state.db.list_networks_for_user(&user.id)?,
    };
    let node_rows: Vec<_> = state
        .db
        .list_nodes(None)?
        .into_iter()
        .filter(|n| {
            allowed
                .as_ref()
                .map(|s| s.contains(&n.network_id))
                .unwrap_or(true)
        })
        .collect();
    let cred_rows: Vec<_> = state
        .db
        .list_credentials(None)?
        .into_iter()
        .filter(|c| {
            allowed
                .as_ref()
                .map(|s| s.contains(&c.network_id))
                .unwrap_or(true)
        })
        .collect();
    let now = now_ms();
    let soon = now + 24 * 3600 * 1000;

    let is_admin = user.role == "admin";
    let recent_audit = state
        .db
        .list_recent_audit(if is_admin { 10 } else { 100 })?
        .iter()
        .filter(|row| is_admin || row.user_id.as_deref() == Some(user.id.as_str()))
        .take(10)
        .map(|row| RecentAudit {
            id: row.id,
            username: row.username.clone(),
            action: row.action.clone(),
            resource_type: row.resource_type.clone(),
            resource_id: row.resource_id.clone(),
            created_at: row.created_at,
        })
        .collect();

    let recent_errors = state
        .db
        .list_recent_errors(if allowed.is_none() { 10 } else { 200 })?
        .iter()
        .filter(|row| {
            allowed
                .as_ref()
                .map(|s| {
                    row.network_id
                        .as_deref()
                        .map(|n| s.contains(n))
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .take(10)
        .map(|row| RecentError {
            id: row.id,
            network_id: row.network_id.clone(),
            message: row.message.clone(),
            created_at: row.created_at,
        })
        .collect();

    Ok(DashboardSummary {
        initialized: state.db.count_users()? > 0,
        users: UsersSummary {
            total: state.db.count_users()?,
            admins: state.db.count_active_admins()?,
        },
        networks: NetworksSummary {
            total: network_rows.len() as i64,
            running: network_rows
                .iter()
                .filter(|n| n.status == "running")
                .count() as i64,
            error: network_rows.iter().filter(|n| n.status == "error").count() as i64,
        },
        nodes: NodesSummary {
            total: node_rows.len() as i64,
            online: node_rows.iter().filter(|n| n.status == "online").count() as i64,
            offline: node_rows.iter().filter(|n| n.status == "offline").count() as i64,
            unknown: node_rows.iter().filter(|n| n.status == "unknown").count() as i64,
        },
        credentials: CredentialsSummary {
            total: cred_rows.len() as i64,
            active: cred_rows
                .iter()
                .filter(|c| !c.revoked && c.expires_at > now)
                .count() as i64,
            expiring_soon: cred_rows
                .iter()
                .filter(|c| !c.revoked && c.expires_at > now && c.expires_at <= soon)
                .count() as i64,
            expired: cred_rows
                .iter()
                .filter(|c| !c.revoked && c.expires_at <= now)
                .count() as i64,
            revoked: cred_rows.iter().filter(|c| c.revoked).count() as i64,
        },
        recent_audit,
        recent_errors,
    })
}

pub fn get_dashboard_traffic(
    state: &AppState,
    user: &AuthUser,
    network_id: Option<&str>,
    hours: i64,
) -> AppResult<Vec<TrafficPoint>> {
    let since = now_ms() - hours * 3600 * 1000;
    if let Some(nid) = network_id {
        require_network_access(state, nid, user, false)?;
        return Ok(get_traffic_series(state, Some(nid), Some(since)));
    }
    match accessible_network_ids(state, user)? {
        None => Ok(get_traffic_series(state, None, Some(since))),
        Some(set) => Ok(state.metrics.lock().unwrap().sum_series(&set, Some(since))),
    }
}
