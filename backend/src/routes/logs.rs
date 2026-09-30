use std::collections::HashSet;
use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Json as JsonResponse;
use axum::routing::get;
use axum::Router;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::error::{forbidden, AppResult};
use crate::middleware::AuthUser;
use crate::services::audit::{list_audit_logs, ListAuditParams};
use crate::services::log::{clear_node_logs, list_node_logs, ListNodeLogsParams};
use crate::services::network::{accessible_network_ids, require_network_access};
use crate::state::SharedState;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AuditQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    action: Option<String>,
    username: Option<String>,
    search: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct NodeLogsQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    network_id: Option<String>,
    node_id: Option<String>,
    level: Option<String>,
    search: Option<String>,
}

#[derive(Deserialize, Default)]
struct RuntimeQuery {
    limit: Option<usize>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ClearQuery {
    network_id: Option<String>,
    before: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct StreamQuery {
    network_id: Option<String>,
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/logs/audit", get(audit_handler))
        .route("/api/logs/nodes", get(nodes_handler).delete(clear_handler))
        .route("/api/logs/runtime/{network_id}", get(runtime_handler))
        .route("/api/logs/stream", get(stream_handler))
}

async fn audit_handler(
    State(state): State<SharedState>,
    Query(query): Query<AuditQuery>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let user_id = if user.role == "admin" {
        None
    } else {
        Some(user.id.clone())
    };
    let result = list_audit_logs(
        &state,
        ListAuditParams {
            page: query.page.unwrap_or(1),
            page_size: query.page_size.unwrap_or(20),
            action: query.action,
            username: query.username,
            search: query.search,
            user_id,
        },
    )?;
    Ok(JsonResponse(json!(result)))
}

async fn nodes_handler(
    State(state): State<SharedState>,
    Query(query): Query<NodeLogsQuery>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let network_ids = match query.network_id.as_deref() {
        Some(nid) => {
            require_network_access(&state, nid, &user, false)?;
            None
        }
        None => accessible_network_ids(&state, &user)?.map(|s| s.into_iter().collect::<Vec<_>>()),
    };
    let result = list_node_logs(
        &state,
        ListNodeLogsParams {
            network_id: query.network_id,
            network_ids,
            node_id: query.node_id,
            level: query.level,
            search: query.search,
            page: query.page.unwrap_or(1),
            page_size: query.page_size.unwrap_or(100),
        },
    )?;
    Ok(JsonResponse(json!(result)))
}

async fn runtime_handler(
    State(state): State<SharedState>,
    Path(network_id): Path<String>,
    Query(query): Query<RuntimeQuery>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    require_network_access(&state, &network_id, &user, false)?;
    let limit = query.limit.unwrap_or(300);
    Ok(JsonResponse(json!({
        "items": state.pm.recent_logs(&network_id, limit),
    })))
}

async fn clear_handler(
    State(state): State<SharedState>,
    Query(query): Query<ClearQuery>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    if let Some(nid) = query.network_id.as_deref() {
        require_network_access(&state, nid, &user, true)?;
    } else if user.role != "admin" {
        return Err(forbidden("仅管理员可清理全部日志"));
    }
    let deleted = clear_node_logs(&state, query.network_id.as_deref(), query.before)?;
    Ok(JsonResponse(json!({ "deleted": deleted })))
}

async fn stream_handler(
    State(state): State<SharedState>,
    Query(query): Query<StreamQuery>,
    user: AuthUser,
) -> AppResult<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>> {
    let network_filter: Option<String> = match query.network_id.as_deref() {
        Some(nid) => {
            require_network_access(&state, nid, &user, false)?;
            Some(nid.to_string())
        }
        None => None,
    };
    let allowed: Option<HashSet<String>> = accessible_network_ids(&state, &user)?;

    let rx = state.pm.subscribe();
    let ready = futures_util::stream::once(async {
        Ok(Event::default().event("ready").data("{\"ok\":true}"))
    });
    let logs = futures_util::stream::unfold(
        (rx, network_filter, allowed),
        |(mut rx, network_filter, allowed)| async move {
            loop {
                match rx.recv().await {
                    Ok(line) => {
                        if let Some(ref nid) = network_filter {
                            if &line.network_id != nid {
                                continue;
                            }
                        }
                        if let Some(ref set) = allowed {
                            if !set.contains(&line.network_id) {
                                continue;
                            }
                        }
                        let data = serde_json::to_string(&line).unwrap_or_default();
                        return Some((
                            Ok(Event::default().event("log").data(data)),
                            (rx, network_filter, allowed),
                        ));
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        },
    );
    Ok(Sse::new(ready.chain(logs)).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}
