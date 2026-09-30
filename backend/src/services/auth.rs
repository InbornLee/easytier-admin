use serde::Serialize;
use uuid::Uuid;

use crate::db::UserRow;
use crate::error::{bad_request, conflict, not_found, AppResult};
use crate::state::AppState;
use crate::util::now_ms;
use crate::util::password::{check_password_policy, hash_password, verify_password};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicUser {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: String,
    pub disabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_login_at: Option<i64>,
}

pub fn is_valid_role(role: &str) -> bool {
    matches!(role, "admin" | "operator" | "viewer")
}

fn to_public(row: &UserRow) -> PublicUser {
    PublicUser {
        id: row.id.clone(),
        username: row.username.clone(),
        display_name: row.display_name.clone(),
        email: row.email.clone(),
        role: row.role.clone(),
        disabled: row.disabled,
        created_at: row.created_at,
        updated_at: row.updated_at,
        last_login_at: row.last_login_at,
    }
}

fn is_valid_username(username: &str) -> bool {
    let len = username.chars().count();
    if !(3..=32).contains(&len) {
        return false;
    }
    username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

pub struct CreateUserInput {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub skip_policy_check: bool,
}

pub async fn create_user(state: &AppState, input: CreateUserInput) -> AppResult<PublicUser> {
    let username = input.username.trim().to_string();
    if !is_valid_username(&username) {
        return Err(bad_request(
            "用户名需为 3-32 位字母、数字、下划线、点或短横线",
            None,
        ));
    }
    if state.db.get_user_by_username(&username)?.is_some() {
        return Err(conflict("用户名已存在"));
    }

    if !input.skip_policy_check {
        let (ok, errors) = check_password_policy(&input.password, Some(&username));
        if !ok {
            return Err(bad_request(
                format!("密码不符合要求: {}", errors.join("；")),
                None,
            ));
        }
    }

    let role = input.role.unwrap_or_else(|| "admin".to_string());
    if !is_valid_role(&role) {
        return Err(bad_request("无效的角色", None));
    }

    let now = now_ms();
    let row = UserRow {
        id: Uuid::new_v4().to_string(),
        username,
        password_hash: hash_password(input.password).await?,
        display_name: input.display_name,
        email: input.email,
        role,
        disabled: false,
        created_at: now,
        updated_at: now,
        last_login_at: None,
    };
    state.db.insert_user(&row)?;
    Ok(to_public(&row))
}

pub async fn verify_login(
    state: &AppState,
    username: &str,
    password: &str,
) -> AppResult<Option<PublicUser>> {
    let Some(row) = state.db.get_user_by_username(username.trim())? else {
        return Ok(None);
    };
    if row.disabled {
        return Ok(None);
    }
    if !verify_password(row.password_hash.clone(), password.to_string()).await {
        return Ok(None);
    }
    let now = now_ms();
    state.db.update_user_last_login(&row.id, now)?;
    let mut public = to_public(&row);
    public.last_login_at = Some(now);
    Ok(Some(public))
}

pub async fn change_password(
    state: &AppState,
    user_id: &str,
    old_password: &str,
    new_password: &str,
) -> AppResult<()> {
    let Some(row) = state.db.get_user_by_id(user_id)? else {
        return Err(not_found("用户不存在"));
    };
    if !verify_password(row.password_hash.clone(), old_password.to_string()).await {
        return Err(bad_request("原密码错误", None));
    }
    let (ok, errors) = check_password_policy(new_password, Some(&row.username));
    if !ok {
        return Err(bad_request(
            format!("新密码不符合要求: {}", errors.join("；")),
            None,
        ));
    }
    if verify_password(row.password_hash.clone(), new_password.to_string()).await {
        return Err(bad_request("新密码不能与原密码相同", None));
    }
    let hash = hash_password(new_password.to_string()).await?;
    state.db.update_user_password(user_id, &hash, now_ms())?;
    Ok(())
}

pub async fn reset_password(state: &AppState, user_id: &str, new_password: &str) -> AppResult<()> {
    let Some(row) = state.db.get_user_by_id(user_id)? else {
        return Err(not_found("用户不存在"));
    };
    let (ok, errors) = check_password_policy(new_password, Some(&row.username));
    if !ok {
        return Err(bad_request(
            format!("密码不符合要求: {}", errors.join("；")),
            None,
        ));
    }
    let hash = hash_password(new_password.to_string()).await?;
    state.db.update_user_password(user_id, &hash, now_ms())?;
    Ok(())
}

pub struct UpdateUserInput {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub disabled: Option<bool>,
}

pub fn update_user(
    state: &AppState,
    user_id: &str,
    patch: UpdateUserInput,
) -> AppResult<PublicUser> {
    let Some(row) = state.db.get_user_by_id(user_id)? else {
        return Err(not_found("用户不存在"));
    };
    let role = patch.role.unwrap_or(row.role.clone());
    if !is_valid_role(&role) {
        return Err(bad_request("无效的角色", None));
    }
    let display_name = patch.display_name.or(row.display_name.clone());
    let email = patch.email.or(row.email.clone());
    let disabled = patch.disabled.unwrap_or(row.disabled);
    state.db.update_user_profile(
        user_id,
        display_name.as_deref(),
        email.as_deref(),
        &role,
        disabled,
        now_ms(),
    )?;
    Ok(PublicUser {
        id: row.id,
        username: row.username,
        display_name,
        email,
        role,
        disabled,
        created_at: row.created_at,
        updated_at: now_ms(),
        last_login_at: row.last_login_at,
    })
}

pub fn get_user_by_id(state: &AppState, id: &str) -> AppResult<Option<PublicUser>> {
    Ok(state.db.get_user_by_id(id)?.as_ref().map(to_public))
}

pub fn list_users(state: &AppState) -> AppResult<Vec<PublicUser>> {
    Ok(state.db.list_users()?.iter().map(to_public).collect())
}

pub fn delete_user(state: &AppState, user_id: &str) -> AppResult<()> {
    let Some(row) = state.db.get_user_by_id(user_id)? else {
        return Err(not_found("用户不存在"));
    };
    if row.role == "admin" && !row.disabled && state.db.count_active_admins()? <= 1 {
        return Err(bad_request("不能删除最后一个管理员账号", None));
    }
    state.db.delete_user(user_id)?;
    state.db.delete_shares_by_user(user_id)?;
    Ok(())
}

/// 共享选择器使用的精简用户信息（不包含邮箱等敏感字段）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectableUser {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub role: String,
    pub disabled: bool,
}

pub fn list_selectable_users(state: &AppState) -> AppResult<Vec<SelectableUser>> {
    Ok(state
        .db
        .list_users()?
        .iter()
        .map(|row| SelectableUser {
            id: row.id.clone(),
            username: row.username.clone(),
            display_name: row.display_name.clone(),
            role: row.role.clone(),
            disabled: row.disabled,
        })
        .collect())
}
