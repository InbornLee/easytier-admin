use serde::Serialize;
use serde_json::Value;

use crate::db::AuditLogRow;
use crate::error::AppResult;
use crate::state::AppState;
use crate::util::now_ms;

use super::Paginated;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogDto {
    pub id: i64,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub detail: Option<Value>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: i64,
}

fn to_dto(row: &AuditLogRow) -> AuditLogDto {
    AuditLogDto {
        id: row.id,
        user_id: row.user_id.clone(),
        username: row.username.clone(),
        action: row.action.clone(),
        resource_type: row.resource_type.clone(),
        resource_id: row.resource_id.clone(),
        detail: row
            .detail
            .as_deref()
            .map(|t| serde_json::from_str(t).unwrap_or_else(|_| Value::String(t.to_string()))),
        ip: row.ip.clone(),
        user_agent: row.user_agent.clone(),
        created_at: row.created_at,
    }
}

pub struct AuditInput {
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub detail: Option<Value>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

pub fn record_audit(state: &AppState, input: AuditInput) {
    let detail = input
        .detail
        .map(|d| serde_json::to_string(&d).unwrap_or_default());
    if let Err(err) = state.db.insert_audit(
        input.user_id.as_deref(),
        input.username.as_deref(),
        &input.action,
        input.resource_type.as_deref(),
        input.resource_id.as_deref(),
        detail.as_deref(),
        input.ip.as_deref(),
        input.user_agent.as_deref(),
        now_ms(),
    ) {
        tracing::error!(error = %err, "写入审计日志失败");
    }
}

pub struct ListAuditParams {
    pub page: i64,
    pub page_size: i64,
    pub action: Option<String>,
    pub username: Option<String>,
    pub search: Option<String>,
    pub user_id: Option<String>,
}

pub fn list_audit_logs(
    state: &AppState,
    params: ListAuditParams,
) -> AppResult<Paginated<AuditLogDto>> {
    let page = params.page.max(1);
    let page_size = params.page_size.clamp(1, 200);
    let (rows, total) = state.db.list_audit_logs(
        params.user_id.as_deref(),
        params.action.as_deref(),
        params.username.as_deref(),
        params.search.as_deref(),
        page,
        page_size,
    )?;
    Ok(Paginated {
        items: rows.iter().map(to_dto).collect(),
        total,
        page,
        page_size,
    })
}
