use axum::extract::{Query, State};
use axum::response::Json as JsonResponse;
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::AppResult;
use crate::middleware::AuthUser;
use crate::services::dashboard::{get_dashboard_summary, get_dashboard_traffic};
use crate::state::SharedState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrafficQuery {
    network_id: Option<String>,
    hours: Option<i64>,
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/dashboard/summary", get(summary_handler))
        .route("/api/dashboard/traffic", get(traffic_handler))
}

async fn summary_handler(
    State(state): State<SharedState>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    Ok(JsonResponse(json!(get_dashboard_summary(&state, &user)?)))
}

async fn traffic_handler(
    State(state): State<SharedState>,
    Query(query): Query<TrafficQuery>,
    user: AuthUser,
) -> AppResult<JsonResponse<Value>> {
    let hours = query.hours.unwrap_or(1);
    Ok(JsonResponse(json!({
        "series": get_dashboard_traffic(&state, &user, query.network_id.as_deref(), hours)?,
    })))
}
