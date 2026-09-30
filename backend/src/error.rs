use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

/// 统一的 API 错误类型。所有 service / route 都返回 `AppResult<T>`。
#[derive(Debug, Clone)]
pub struct AppError {
    pub status: u16,
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}

impl AppError {
    pub fn new(status: u16, message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut body = json!({
            "code": self.code,
            "message": self.message,
        });
        if let Some(details) = self.details {
            body["details"] = details;
        }
        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn bad_request(msg: impl Into<String>, details: Option<Value>) -> AppError {
    let mut e = AppError::new(400, msg, "BAD_REQUEST");
    e.details = details;
    e
}

pub fn unauthorized(msg: impl Into<String>) -> AppError {
    AppError::new(401, msg, "UNAUTHORIZED")
}

pub fn forbidden(msg: impl Into<String>) -> AppError {
    AppError::new(403, msg, "FORBIDDEN")
}

pub fn not_found(msg: impl Into<String>) -> AppError {
    AppError::new(404, msg, "NOT_FOUND")
}

pub fn conflict(msg: impl Into<String>) -> AppError {
    AppError::new(409, msg, "CONFLICT")
}

pub fn easytier_error(msg: impl Into<String>, details: Option<Value>) -> AppError {
    let mut e = AppError::new(502, msg, "EASYTIER_ERROR");
    e.details = details;
    e
}

pub fn internal(msg: impl Into<String>) -> AppError {
    AppError::new(500, msg, "INTERNAL_ERROR")
}

pub fn validation_error(issues: Vec<(String, String)>) -> AppError {
    let list: Vec<Value> = issues
        .into_iter()
        .map(|(path, message)| json!({ "path": path, "message": message }))
        .collect();
    AppError::new(400, "参数校验失败", "VALIDATION_ERROR").with_details(json!({ "issues": list }))
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        tracing::error!(error = %err, "数据库操作失败");
        internal("数据库操作失败")
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        tracing::error!(error = %err, "IO 操作失败");
        internal("文件操作失败")
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        tracing::error!(error = %err, "JSON 解析失败");
        internal("数据解析失败")
    }
}
