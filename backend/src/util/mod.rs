pub mod crypto;
pub mod format;
pub mod password;
pub mod x25519;

use std::time::{SystemTime, UNIX_EPOCH};

/// 当前 Unix 毫秒时间戳
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
