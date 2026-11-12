use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware as axum_mw;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::{AllowOrigin, Any as CorsAny, CorsLayer};

use crate::middleware::rate_limit_global;
use crate::routes::api_router;
use crate::state::SharedState;

pub fn build_router(state: SharedState) -> Router {
    let hsts = std::env::var("COOKIE_SECURE")
        .map(|v| v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let router: Router<SharedState> = Router::new()
        .merge(api_router())
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            rate_limit_global,
        ))
        .layer(axum_mw::from_fn(move |req, next| {
            security_headers(req, next, hsts)
        }))
        .fallback(spa_fallback);

    let router = if state.config.cors_origin.is_empty() {
        router
    } else {
        let origins: Vec<HeaderValue> = state
            .config
            .cors_origin
            .iter()
            .filter_map(|o| o.parse::<HeaderValue>().ok())
            .collect();
        let cors = CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_credentials(true)
            .allow_methods(CorsAny)
            .allow_headers(CorsAny);
        router.layer(cors)
    };

    router.with_state(state)
}

async fn security_headers(req: Request, next: Next, hsts: bool) -> Response {
    let is_api = req.uri().path().starts_with("/api/");
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
             img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; \
             object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'",
        ),
    );
    if hsts {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    if is_api {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}

async fn spa_fallback(State(state): State<SharedState>, req: Request) -> Response {
    let path = req.uri().path().to_string();
    if path.starts_with("/api/") || req.method() != Method::GET {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "code": "NOT_FOUND", "message": "接口不存在" })),
        )
            .into_response();
    }
    serve_static(&state, &path).await
}

async fn serve_static(state: &SharedState, path: &str) -> Response {
    let root = &state.config.frontend_dist;
    if !root.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "code": "NOT_FOUND",
                "message": "前端资源未构建，请先执行 pnpm build",
            })),
        )
            .into_response();
    }

    let rel = path.trim_start_matches('/');
    let candidate = if rel.is_empty() || rel.contains("..") {
        root.join("index.html")
    } else {
        root.join(rel)
    };
    let file = if candidate.is_file() {
        candidate
    } else {
        root.join("index.html")
    };

    match tokio::fs::read(&file).await {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&file).first_or_octet_stream();
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            (headers, bytes).into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "code": "NOT_FOUND", "message": "资源不存在" })),
        )
            .into_response(),
    }
}
