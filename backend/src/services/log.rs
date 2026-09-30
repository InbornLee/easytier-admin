use std::time::Duration;

use serde::Serialize;

use crate::db::NodeLogRow;
use crate::error::AppResult;
use crate::state::{AppState, SharedState};
use crate::util::now_ms;

use super::settings::get_log_retention_days;
use super::Paginated;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeLogDto {
    pub id: i64,
    pub network_id: Option<String>,
    pub node_id: Option<String>,
    pub source: String,
    pub level: String,
    pub message: String,
    pub created_at: i64,
}

fn to_dto(row: &NodeLogRow) -> NodeLogDto {
    NodeLogDto {
        id: row.id,
        network_id: row.network_id.clone(),
        node_id: row.node_id.clone(),
        source: row.source.clone(),
        level: row.level.clone(),
        message: row.message.clone(),
        created_at: row.created_at,
    }
}

pub struct ListNodeLogsParams {
    pub network_id: Option<String>,
    pub network_ids: Option<Vec<String>>,
    pub node_id: Option<String>,
    pub level: Option<String>,
    pub search: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

pub fn list_node_logs(
    state: &AppState,
    params: ListNodeLogsParams,
) -> AppResult<Paginated<NodeLogDto>> {
    let page = params.page.max(1);
    let page_size = params.page_size.clamp(1, 500);
    let (rows, total) = state.db.list_node_logs(
        params.network_id.as_deref(),
        params.network_ids.as_deref(),
        params.node_id.as_deref(),
        params.level.as_deref(),
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

pub fn clear_node_logs(
    state: &AppState,
    network_id: Option<&str>,
    before: Option<i64>,
) -> AppResult<i64> {
    state.db.clear_node_logs(network_id, before)
}

/// 根据「日志储存期限」清理过期的运行日志与操作审计；retention_days <= 0 表示永久保留。
pub fn prune_logs(state: &AppState) -> AppResult<(i64, i64)> {
    let days = get_log_retention_days(state);
    if days <= 0 {
        return Ok((0, 0));
    }
    let cutoff = now_ms() - days * 24 * 3600 * 1000;
    let node_logs = state.db.delete_node_logs_before(cutoff)?;
    let audit_logs = state.db.delete_audit_logs_before(cutoff)?;
    Ok((node_logs, audit_logs))
}

/// 启动后台日志清理任务：启动时立即执行一次，之后每 6 小时执行一次。
pub fn start_log_cleanup(state: &SharedState) {
    let state = state.clone();
    tokio::spawn(async move {
        run_prune(&state);
        let mut interval = tokio::time::interval(Duration::from_secs(6 * 3600));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            run_prune(&state);
        }
    });
}

fn run_prune(state: &AppState) {
    match prune_logs(state) {
        Ok((node_logs, audit_logs)) if node_logs + audit_logs > 0 => {
            tracing::info!(node_logs, audit_logs, "已按储存期限清理历史日志");
        }
        Ok(_) => {}
        Err(err) => tracing::warn!(error = %err, "清理历史日志失败"),
    }
}
