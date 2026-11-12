use std::sync::Arc;

use crate::db::Db;
use crate::error::AppResult;
use crate::util::now_ms;

/// 同一账号在统计窗口内允许的连续失败次数。
const USER_MAX_FAILURES: i64 = 5;
/// 账号失败统计窗口（毫秒）。窗口内累计失败达到阈值即锁定。
const USER_WINDOW_MS: i64 = 15 * 60 * 1000;
/// 账号首次触发锁定的时长（秒），之后按失败次数指数增长。
const USER_BASE_LOCK_SECS: i64 = 60;
/// 账号单次锁定上限（秒）。
const USER_MAX_LOCK_SECS: i64 = 30 * 60;

/// 同一来源 IP 在统计窗口内允许的失败总次数（覆盖撞库 / 密码喷洒）。
const IP_MAX_FAILURES: i64 = 20;
const IP_WINDOW_MS: i64 = 10 * 60 * 1000;
const IP_LOCK_SECS: i64 = 10 * 60;

/// 超过该时间未再失败时，失败计数归零。
const COUNTER_RESET_MS: i64 = 60 * 60 * 1000;

/// 记录清理阈值（毫秒）。
const RETENTION_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy)]
pub struct Lockout {
    /// 建议客户端重试前等待的秒数。
    pub retry_after: i64,
}

/// 基于 SQLite 的登录失败锁定器。
///
/// 同时按「账号」与「来源 IP」两个维度累计失败次数：
/// - 账号维度用于阻止针对单个账号的暴力破解，连续失败会指数级延长锁定；
/// - IP 维度用于阻止撞库 / 密码喷洒（大量不同用户名、相同来源）。
///
/// 失败计数落库，服务重启不会丢失锁定状态；成功登录只清除账号维度的计数，
/// 避免攻击者用自己掌握的账号成功登录后重置 IP 维度的爆破计数。
pub struct LoginGuard {
    db: Arc<Db>,
}

impl LoginGuard {
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    fn user_scope(username: &str) -> String {
        format!("user:{}", normalize_username(username))
    }

    fn ip_scope(ip: &str) -> String {
        format!("ip:{ip}")
    }

    /// 登录前检查是否处于锁定状态，返回需要等待的秒数。
    pub fn check(&self, ip: &str, username: &str) -> AppResult<Option<Lockout>> {
        let now = now_ms();
        let mut retry_after = 0;
        for scope in [Self::user_scope(username), Self::ip_scope(ip)] {
            if let Some(row) = self.db.get_login_attempt(&scope)? {
                if row.locked_until > now {
                    retry_after = retry_after.max(secs_ceil(row.locked_until - now));
                }
            }
        }
        Ok((retry_after > 0).then_some(Lockout { retry_after }))
    }

    /// 记录一次登录失败；若本次失败触发（或延长）锁定则返回剩余等待秒数。
    pub fn record_failure(&self, ip: &str, username: &str) -> AppResult<Option<Lockout>> {
        let now = now_ms();
        let mut retry_after = 0;

        if let Some(secs) = self.bump(
            &Self::user_scope(username),
            now,
            USER_WINDOW_MS,
            USER_MAX_FAILURES,
            Some((USER_BASE_LOCK_SECS, USER_MAX_LOCK_SECS)),
        )? {
            retry_after = retry_after.max(secs);
        }
        if let Some(secs) = self.bump(
            &Self::ip_scope(ip),
            now,
            IP_WINDOW_MS,
            IP_MAX_FAILURES,
            Some((IP_LOCK_SECS, IP_LOCK_SECS)),
        )? {
            retry_after = retry_after.max(secs);
        }

        Ok((retry_after > 0).then_some(Lockout { retry_after }))
    }

    /// 登录成功：清除账号维度的失败计数。
    pub fn record_success(&self, _ip: &str, username: &str) -> AppResult<()> {
        self.db.delete_login_attempt(&Self::user_scope(username))?;
        Ok(())
    }

    /// 累加指定维度的失败次数，返回本次触发的锁定剩余秒数。
    fn bump(
        &self,
        scope: &str,
        now: i64,
        window: i64,
        threshold: i64,
        lock: Option<(i64, i64)>,
    ) -> AppResult<Option<i64>> {
        let existing = self.db.get_login_attempt(scope)?;
        let expired = match &existing {
            None => true,
            Some(row) => {
                row.locked_until <= now
                    && (now - row.first_failure_at > window
                        || now - row.last_failure_at > COUNTER_RESET_MS)
            }
        };
        let (mut failures, first_failure_at) = if expired {
            (0, now)
        } else {
            let row = existing.expect("existing attempt row");
            (row.failures, row.first_failure_at)
        };

        failures += 1;
        let mut locked_until = 0;
        if failures >= threshold {
            if let Some((base, cap)) = lock {
                let steps = (failures - threshold).clamp(0, 6) as u32;
                let secs = base.saturating_mul(1 << steps).min(cap);
                locked_until = now + secs * 1000;
            }
        }

        self.db.upsert_login_attempt(
            scope,
            failures,
            first_failure_at,
            now,
            locked_until,
            now - RETENTION_MS,
        )?;

        Ok((locked_until > now).then(|| secs_ceil(locked_until - now)))
    }
}

fn normalize_username(username: &str) -> String {
    username.trim().to_lowercase()
}

fn secs_ceil(ms: i64) -> i64 {
    (ms.max(0) + 999) / 1000
}
