use std::net::{IpAddr, SocketAddr};

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
use crate::services::auth::PublicUser;
use crate::state::SharedState;
use crate::util::crypto::sha256_hex;
use crate::util::now_ms;

pub const TOKEN_COOKIE: &str = "et_admin_token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub role: String,
    /// 密码版本：由服务端密钥与当前密码哈希派生。修改/重置密码后旧令牌立即失效。
    #[serde(default)]
    pub pv: String,
    pub exp: usize,
}

fn password_version(state: &SharedState, password_hash: &str) -> String {
    sha256_hex(&format!("{}:pv:{}", state.jwt_secret, password_hash))
}

pub fn sign_token(state: &SharedState, user: &PublicUser) -> AppResult<String> {
    let row = state
        .db
        .get_user_by_id(&user.id)?
        .ok_or_else(|| internal("用户不存在"))?;
    let exp = (now_ms() / 1000) as usize + state.config.jwt_ttl as usize;
    let claims = Claims {
        sub: user.id.clone(),
        username: user.username.clone(),
        role: user.role.clone(),
        pv: password_version(state, &row.password_hash),
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
        let row = state
            .db
            .get_user_by_id(&claims.sub)
            .map_err(|_| unauthorized("未登录或登录已过期"))?
            .ok_or_else(|| unauthorized("账号不存在或已被禁用"))?;
        if row.disabled {
            return Err(unauthorized("账号不存在或已被禁用"));
        }
        // 令牌与当前密码绑定：修改/重置密码后旧令牌立即失效（不等 JWT 过期）。
        if claims.pv.is_empty() || claims.pv != password_version(state, &row.password_hash) {
            return Err(unauthorized("登录状态已失效，请重新登录"));
        }
        Ok(AuthUser {
            id: row.id,
            username: row.username,
            role: row.role,
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

pub fn client_ip(headers: &HeaderMap, fallback: Option<SocketAddr>, trust_proxy: bool) -> String {
    if trust_proxy {
        // X-Real-IP 通常由反向代理以 $remote_addr 覆盖写入，优先采信。
        if let Some(real) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
            if let Some(ip) = parse_ip(real) {
                return ip.to_string();
            }
        }
        // X-Forwarded-For 形如 client, proxy1, proxy2。
        // 从右向左取第一个合法地址：右侧由最近的反向代理写入，客户端无法伪造；
        // 直接取第一个值会被客户端自带的伪造头绕过登录限流。
        if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            for part in xff.rsplit(',') {
                if let Some(ip) = parse_ip(part) {
                    return ip.to_string();
                }
            }
        }
    }
    fallback
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn parse_ip(raw: &str) -> Option<IpAddr> {
    raw.trim().parse::<IpAddr>().ok()
}

fn cookie_secure() -> bool {
    std::env::var("COOKIE_SECURE")
        .map(|v| v == "true")
        .unwrap_or(false)
}

pub fn auth_cookie_value(token: &str, ttl: u64, persist: bool) -> String {
    // 默认签发会话 Cookie（不写 Max-Age），关闭浏览器即登出；
    // 仅当显式配置 COOKIE_PERSIST=true 时才按 JWT_TTL 持久化。
    let max_age = if persist && ttl > 0 {
        format!("; Max-Age={ttl}")
    } else {
        String::new()
    };
    format!(
        "{TOKEN_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax{max_age}{}",
        if cookie_secure() { "; Secure" } else { "" }
    )
}

pub fn clear_cookie_value() -> String {
    format!("{TOKEN_COOKIE}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax")
}

/// 构建带可选 Set-Cookie 的 JSON 响应
pub fn json_response<T: Serialize>(body: T, cookie: Option<String>) -> Response {
    let mut response = Json(body).into_response();
    // 认证响应（含令牌）不应被浏览器或中间代理缓存
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
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
    let ip = client_ip(request.headers(), Some(addr), state.config.trust_proxy);
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
