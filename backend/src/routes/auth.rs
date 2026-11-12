use axum::extract::{ConnectInfo, Json, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;

use crate::error::{bad_request, AppError, AppResult};
use crate::middleware::{
    auth_cookie_value, clear_cookie_value, client_ip, json_response, parse_body, sign_token,
    AuthUser, TOKEN_COOKIE,
};
use crate::services::audit::{record_audit, AuditInput};
use crate::services::auth::{
    change_password, create_user, verify_login, CreateUserInput, PublicUser,
};
use crate::state::SharedState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupInput {
    username: String,
    password: String,
    display_name: Option<String>,
    email: Option<String>,
    setup_token: Option<String>,
}

#[derive(Deserialize)]
struct LoginInput {
    username: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangePasswordInput {
    old_password: String,
    new_password: String,
}

#[derive(serde::Serialize)]
struct AuthResponse {
    user: PublicUser,
    token: String,
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/auth/status", get(status))
        .route("/api/auth/setup", post(setup))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
        .route("/api/auth/change-password", post(change_password_handler))
        .route("/api/auth/token", get(token_status))
}

async fn status(State(state): State<SharedState>) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(json!({
        "initialized": state.db.count_users()? > 0,
        // 仅告知是否需要初始化令牌，不泄露令牌内容
        "setupTokenRequired": state.config.setup_token.is_some(),
    })))
}

/// 常量时间比较，避免通过响应时间推测初始化令牌。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

async fn setup(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Response> {
    if state.db.count_users()? > 0 {
        return Err(bad_request("系统已完成初始化，无法重复创建管理员", None));
    }
    let input: SetupInput = parse_body(body)?;
    // 可选初始化令牌：防止未初始化的实例在公网被抢注管理员。
    if let Some(expected) = state.config.setup_token.as_deref() {
        let header_token = headers
            .get("x-setup-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let provided = if header_token.is_empty() {
            input.setup_token.as_deref().unwrap_or("")
        } else {
            header_token
        };
        if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
            return Err(AppError::new(403, "初始化令牌无效", "SETUP_TOKEN_INVALID"));
        }
    }
    let user = create_user(
        &state,
        CreateUserInput {
            username: input.username,
            password: input.password,
            display_name: input.display_name,
            email: input.email.filter(|e| !e.is_empty()),
            role: Some("admin".to_string()),
            skip_policy_check: false,
        },
    )
    .await?;
    let token = sign_token(&state, &user)?;
    let ip = client_ip(&headers, Some(addr), state.config.trust_proxy);
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id.clone()),
            username: Some(user.username.clone()),
            action: "auth.setup".to_string(),
            resource_type: Some("user".to_string()),
            resource_id: Some(user.id.clone()),
            detail: None,
            ip: Some(ip),
            user_agent: headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
        },
    );
    Ok(json_response(
        AuthResponse {
            user,
            token: token.clone(),
        },
        Some(auth_cookie_value(
            &token,
            state.config.jwt_ttl,
            state.config.cookie_persist,
        )),
    ))
}

fn login_locked(retry_after: i64) -> AppError {
    AppError::new(
        429,
        format!("登录失败次数过多，请在 {retry_after} 秒后重试"),
        "LOGIN_LOCKED",
    )
    .with_details(json!({ "retryAfter": retry_after }))
}

async fn login(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Response> {
    let input: LoginInput = parse_body(body)?;
    let ip = client_ip(&headers, Some(addr), state.config.trust_proxy);
    // 先检查失败锁定（不消耗固定窗口配额），锁定期间直接返回剩余等待时间
    if let Some(lockout) = state.login_guard.check(&ip, &input.username)? {
        return Err(login_locked(lockout.retry_after));
    }
    if !state.rate_login.check(&ip) {
        return Err(AppError::new(
            429,
            "登录尝试过于频繁，请稍后重试",
            "RATE_LIMITED",
        ));
    }
    let user = verify_login(&state, &input.username, &input.password).await?;
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    match user {
        Some(user) => {
            state.login_guard.record_success(&ip, &input.username)?;
            let token = sign_token(&state, &user)?;
            record_audit(
                &state,
                AuditInput {
                    user_id: Some(user.id.clone()),
                    username: Some(user.username.clone()),
                    action: "auth.login".to_string(),
                    resource_type: None,
                    resource_id: None,
                    detail: None,
                    ip: Some(ip),
                    user_agent,
                },
            );
            Ok(json_response(
                AuthResponse {
                    user,
                    token: token.clone(),
                },
                Some(auth_cookie_value(
                    &token,
                    state.config.jwt_ttl,
                    state.config.cookie_persist,
                )),
            ))
        }
        None => {
            let lockout = state.login_guard.record_failure(&ip, &input.username)?;
            record_audit(
                &state,
                AuditInput {
                    user_id: None,
                    username: Some(input.username),
                    action: "auth.login.failed".to_string(),
                    resource_type: None,
                    resource_id: None,
                    detail: None,
                    ip: Some(ip),
                    user_agent,
                },
            );
            // 账号不存在与密码错误返回完全一致的提示，避免用户名枚举；
            // 达到失败阈值后改为返回锁定与剩余秒数，前端可据此倒计时。
            if let Some(lockout) = lockout {
                return Err(login_locked(lockout.retry_after));
            }
            Err(bad_request("用户名或密码错误", None))
        }
    }
}

async fn logout() -> Response {
    json_response(json!({ "ok": true }), Some(clear_cookie_value()))
}

async fn me(user: AuthUser) -> Json<serde_json::Value> {
    Json(json!({
        "user": { "id": user.id, "username": user.username, "role": user.role }
    }))
}

async fn change_password_handler(
    State(state): State<SharedState>,
    user: AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let input: ChangePasswordInput = parse_body(body)?;
    change_password(&state, &user.id, &input.old_password, &input.new_password).await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(user.id.clone()),
            username: Some(user.username.clone()),
            action: "auth.change-password".to_string(),
            resource_type: None,
            resource_id: None,
            detail: None,
            ip: Some(client_ip(&headers, Some(addr), state.config.trust_proxy)),
            user_agent: headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
        },
    );
    Ok(Json(json!({ "ok": true })))
}

async fn token_status(headers: HeaderMap) -> Json<serde_json::Value> {
    let has_cookie = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|c| c.contains(&format!("{TOKEN_COOKIE}=")))
        .unwrap_or(false);
    Json(json!({ "hasCookie": has_cookie }))
}
