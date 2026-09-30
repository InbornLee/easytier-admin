use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::rngs::OsRng;

use crate::error::{internal, AppResult};

fn build_argon2() -> Argon2<'static> {
    // OWASP 推荐：内存 64 MiB、迭代 3 次，Argon2id
    let params = Params::new(65536, 3, 1, Some(32)).expect("valid argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

pub async fn hash_password(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        build_argon2()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| internal(format!("密码哈希失败: {e}")))
    })
    .await
    .map_err(|e| internal(format!("密码哈希任务失败: {e}")))?
}

pub async fn verify_password(hash: String, password: String) -> bool {
    tokio::task::spawn_blocking(move || {
        let Ok(parsed) = PasswordHash::new(&hash) else {
            return false;
        };
        build_argon2()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
    .await
    .unwrap_or(false)
}

/// 强密码策略：>= 12 位，包含大小写、数字与特殊字符，且不含用户名
pub fn check_password_policy(password: &str, username: Option<&str>) -> (bool, Vec<String>) {
    let mut errors: Vec<String> = Vec::new();
    let len = password.chars().count();
    if len < 12 {
        errors.push("密码长度至少 12 位".to_string());
    }
    if len > 128 {
        errors.push("密码长度不能超过 128 位".to_string());
    }
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        errors.push("需包含小写字母".to_string());
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        errors.push("需包含大写字母".to_string());
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        errors.push("需包含数字".to_string());
    }
    if !password.chars().any(|c| !c.is_ascii_alphanumeric()) {
        errors.push("需包含特殊字符".to_string());
    }
    if let Some(username) = username {
        if !username.is_empty() && password.to_lowercase().contains(&username.to_lowercase()) {
            errors.push("密码不能包含用户名".to_string());
        }
    }
    (errors.is_empty(), errors)
}
