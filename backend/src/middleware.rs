use std::net::SocketAddr;

use axum::extract::{ConnectInfo, FromRequestParts, Request, State};
use axum::http::header;
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header as JwtHeader, Validation,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{forbidden, internal, unauthorized, AppError, AppResult};
use crate::services::auth::{get_user_by_id, PublicUser};
use crate::state::SharedState;
use crate::util::now_ms;

pub const TOKEN_COOKIE: &str = "et_admin_token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub role: String,
    pub exp: usize,
}

pub fn sign_token(state: &SharedState, user: &PublicUser) -> AppResult<String> {
    let exp = (now_ms() / 1000) as usize + state.config.jwt_ttl as usize;
    let claims = Claims {
        sub: user.id.clone(),
        username: user.username.clone(),
        role: user.role.clone(),
        exp,
    };
    encode(
        &JwtHeader::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| internal(format!("生成令牌失败: {e}")))
}

fn decode_token(state: &SharedState, token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .ok()
    .map(|data| data.claims)
}

fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(cookie) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        for part in cookie.split(';') {
            let part = part.trim();
            if let Some(value) = part.strip_prefix(&format!("{TOKEN_COOKIE}=")) {
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    if let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = value.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub role: String,
}

impl FromRequestParts<SharedState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {
        let token =
            token_from_headers(&parts.headers).ok_or_else(|| unauthorized("未登录或登录已过期"))?;
        let claims =
            decode_token(state, &token).ok_or_else(|| unauthorized("未登录或登录已过期"))?;
        let user = get_user_by_id(state, &claims.sub)
            .map_err(|_| unauthorized("未登录或登录已过期"))?
            .ok_or_else(|| unauthorized("账号不存在或已被禁用"))?;
        if user.disabled {
            return Err(unauthorized("账号不存在或已被禁用"));
        }
        Ok(AuthUser {
            id: user.id,
            username: user.username,
            role: user.role,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthUser);

impl FromRequestParts<SharedState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role != "admin" {
            return Err(forbidden("需要管理员权限"));
        }
        Ok(AdminUser(user))
    }
}

pub fn client_ip(headers: &HeaderMap, fallback: Option<SocketAddr>) -> String {
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = xff.split(',').next() {
            let first = first.trim();
            if !first.is_empty() {
                return first.to_string();
            }
        }
    }
    fallback
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn cookie_secure() -> bool {
    std::env::var("COOKIE_SECURE")
        .map(|v| v == "true")
        .unwrap_or(false)
}

pub fn auth_cookie_value(token: &str, ttl: u64) -> String {
    format!(
        "{TOKEN_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={ttl}{}",
        if cookie_secure() { "; Secure" } else { "" }
    )
}

pub fn clear_cookie_value() -> String {
    format!("{TOKEN_COOKIE}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax")
}

/// 构建带可选 Set-Cookie 的 JSON 响应
pub fn json_response<T: Serialize>(body: T, cookie: Option<String>) -> Response {
    let mut response = Json(body).into_response();
    if let Some(cookie) = cookie {
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }
    response
}

/// 全局限流中间件（600 次 / 分钟）
pub async fn rate_limit_global(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let ip = client_ip(request.headers(), Some(addr));
    if !state.rate_global.check(&ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "code": "RATE_LIMITED",
                "message": "请求过于频繁，请稍后重试",
            })),
        )
            .into_response();
    }
    next.run(request).await
}

/// 校验 JSON 请求体
pub fn parse_body<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> AppResult<T> {
    serde_json::from_value(value).map_err(|e| {
        AppError::new(400, format!("参数校验失败: {e}"), "VALIDATION_ERROR").with_details(json!({
            "issues": [{ "path": "", "message": e.to_string() }]
        }))
    })
}
