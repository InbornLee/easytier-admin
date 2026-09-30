use axum::extract::{Json, Path, State};
use axum::response::Json as JsonResponse;
use axum::routing::{get, patch, post};
use axum::Router;
use serde::Deserialize;
use serde_json::json;

use crate::error::{bad_request, AppResult};
use crate::middleware::{parse_body, AdminUser, AuthUser};
use crate::services::audit::{record_audit, AuditInput};
use crate::services::auth::{
    create_user, delete_user, get_user_by_id, list_selectable_users, list_users, reset_password,
    update_user, CreateUserInput, UpdateUserInput,
};
use crate::state::SharedState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateUserBody {
    username: String,
    password: String,
    display_name: Option<String>,
    email: Option<String>,
    #[serde(default = "default_role")]
    role: String,
}

fn default_role() -> String {
    "operator".to_string()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchUserBody {
    display_name: Option<String>,
    email: Option<String>,
    role: Option<String>,
    disabled: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetPasswordBody {
    new_password: String,
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route(
            "/api/users",
            get(list_users_handler).post(create_user_handler),
        )
        .route("/api/users/selectable", get(selectable_handler))
        .route(
            "/api/users/{id}",
            patch(update_user_handler).delete(delete_user_handler),
        )
        .route(
            "/api/users/{id}/reset-password",
            post(reset_password_handler),
        )
}

async fn selectable_handler(
    State(state): State<SharedState>,
    _user: AuthUser,
) -> AppResult<JsonResponse<serde_json::Value>> {
    Ok(JsonResponse(
        json!({ "items": list_selectable_users(&state)? }),
    ))
}

async fn list_users_handler(
    State(state): State<SharedState>,
    _admin: AdminUser,
) -> AppResult<JsonResponse<serde_json::Value>> {
    Ok(JsonResponse(json!({ "items": list_users(&state)? })))
}

async fn create_user_handler(
    State(state): State<SharedState>,
    admin: AdminUser,
    Json(body): Json<serde_json::Value>,
) -> AppResult<JsonResponse<serde_json::Value>> {
    let input: CreateUserBody = parse_body(body)?;
    let user = create_user(
        &state,
        CreateUserInput {
            username: input.username,
            password: input.password,
            display_name: input.display_name,
            email: input.email.filter(|e| !e.is_empty()),
            role: Some(input.role),
            skip_policy_check: false,
        },
    )
    .await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(admin.0.id),
            username: Some(admin.0.username),
            action: "user.create".to_string(),
            resource_type: Some("user".to_string()),
            resource_id: Some(user.id.clone()),
            detail: Some(json!({ "username": user.username, "role": user.role })),
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "user": user })))
}

async fn update_user_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    admin: AdminUser,
    Json(body): Json<serde_json::Value>,
) -> AppResult<JsonResponse<serde_json::Value>> {
    let input: PatchUserBody = parse_body(body)?;
    if id == admin.0.id && input.disabled == Some(true) {
        return Err(bad_request("不能禁用当前登录账号", None));
    }
    let user = update_user(
        &state,
        &id,
        UpdateUserInput {
            display_name: input.display_name,
            email: input.email.filter(|e| !e.is_empty()),
            role: input.role,
            disabled: input.disabled,
        },
    )?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(admin.0.id),
            username: Some(admin.0.username),
            action: "user.update".to_string(),
            resource_type: Some("user".to_string()),
            resource_id: Some(id),
            detail: Some(json!({
                "displayName": user.display_name,
                "email": user.email,
                "role": user.role,
                "disabled": user.disabled,
            })),
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "user": user })))
}

async fn reset_password_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    admin: AdminUser,
    Json(body): Json<serde_json::Value>,
) -> AppResult<JsonResponse<serde_json::Value>> {
    let input: ResetPasswordBody = parse_body(body)?;
    if get_user_by_id(&state, &id)?.is_none() {
        return Err(bad_request("用户不存在", None));
    }
    reset_password(&state, &id, &input.new_password).await?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(admin.0.id),
            username: Some(admin.0.username),
            action: "user.reset-password".to_string(),
            resource_type: Some("user".to_string()),
            resource_id: Some(id),
            detail: None,
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "ok": true })))
}

async fn delete_user_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    admin: AdminUser,
) -> AppResult<JsonResponse<serde_json::Value>> {
    if id == admin.0.id {
        return Err(bad_request("不能删除当前登录账号", None));
    }
    delete_user(&state, &id)?;
    record_audit(
        &state,
        AuditInput {
            user_id: Some(admin.0.id),
            username: Some(admin.0.username),
            action: "user.delete".to_string(),
            resource_type: Some("user".to_string()),
            resource_id: Some(id),
            detail: None,
            ip: None,
            user_agent: None,
        },
    );
    Ok(JsonResponse(json!({ "ok": true })))
}
