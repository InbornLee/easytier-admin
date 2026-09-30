use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use tokio::sync::broadcast;

use crate::config::{Config, EasyTierSettings};
use crate::db::Db;
use crate::easytier::cli::Cli;
use crate::easytier::{LogLine, ProcessManager};
use crate::services::metrics::MetricsStore;
use crate::util::crypto::Crypto;

/// 简单的固定窗口限流器。
pub struct RateLimiter {
    inner: Mutex<HashMap<String, (u32, Instant)>>,
    max: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max: u32, window: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            max,
            window,
        }
    }

    /// 返回 true 表示允许本次请求。
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut map = self.inner.lock().unwrap();
        if map.len() > 20_000 {
            map.retain(|_, (_, start)| now.duration_since(*start) < self.window);
        }
        let entry = map.entry(key.to_string()).or_insert((0, now));
        if now.duration_since(entry.1) >= self.window {
            *entry = (1, now);
            return true;
        }
        if entry.0 >= self.max {
            return false;
        }
        entry.0 += 1;
        true
    }
}

pub struct AppState {
    pub config: Arc<Config>,
    pub easytier: Arc<RwLock<EasyTierSettings>>,
    pub db: Arc<Db>,
    pub crypto: Crypto,
    pub pm: ProcessManager,
    pub metrics: Arc<Mutex<MetricsStore>>,
    pub log_tx: broadcast::Sender<LogLine>,
    pub jwt_secret: String,
    pub rate_global: RateLimiter,
    pub rate_login: RateLimiter,
    /// 串行化网络端口分配，避免多用户并发创建时分配到相同端口
    pub port_alloc: tokio::sync::Mutex<()>,
    pub started_at: std::time::Instant,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    pub fn easytier_settings(&self) -> EasyTierSettings {
        self.easytier.read().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn cli(&self, rpc_port: u16, timeout: Duration) -> Cli {
        Cli::new(self.easytier_settings().cli_path, rpc_port, timeout)
    }
}
