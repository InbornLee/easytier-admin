// 本 crate 是 TypeScript 后端的等价移植，保留了一部分与前端约定或未来扩展相关的
// 辅助函数（如 collect_traffic、logger_set、凭据列举等），因此允许未被引用的项。
#![allow(dead_code)]

mod app;
mod config;
mod db;
mod easytier;
mod error;
mod middleware;
mod routes;
mod services;
mod state;
mod util;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::db::Db;
use crate::easytier::cli::check_binaries;
use crate::easytier::{LogLine, ProcessManager};
use crate::services::metrics::{start_metrics_collector, MetricsStore};
use crate::services::{network, settings};
use crate::state::{AppState, RateLimiter, SharedState};
use crate::util::crypto::{sha256_hex, Crypto};

fn init_tracing(config: &Config) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.log_level.clone()));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

#[tokio::main]
async fn main() {
    let config = Config::load();
    init_tracing(&config);
    if let Err(err) = run(config).await {
        tracing::error!(error = %err, "控制台启动失败");
        std::process::exit(1);
    }
}

async fn run(config: Config) -> anyhow::Result<()> {
    let config = Arc::new(config);

    let db = Arc::new(Db::open(&config.db_path)?);
    db.ensure_schema()?;

    let easytier = Arc::new(RwLock::new(config.easytier.clone()));
    let (log_tx, _) = broadcast::channel::<LogLine>(1024);
    let pm = ProcessManager::new(db.clone(), config.clone(), easytier.clone(), log_tx.clone());

    let crypto = Crypto::new(&config.app_secret);
    let jwt_secret = sha256_hex(&format!("{}:jwt-v1", config.app_secret));

    let state: SharedState = Arc::new(AppState {
        config: config.clone(),
        easytier,
        db: db.clone(),
        crypto,
        pm: pm.clone(),
        metrics: Arc::new(Mutex::new(MetricsStore::default())),
        log_tx,
        jwt_secret,
        rate_global: RateLimiter::new(600, Duration::from_secs(60)),
        rate_login: RateLimiter::new(10, Duration::from_secs(60)),
        port_alloc: tokio::sync::Mutex::new(()),
        started_at: Instant::now(),
    });

    // 加载运行时设置覆盖
    settings::apply_runtime_overrides(&state);
    start_metrics_collector(&state);
    crate::services::log::start_log_cleanup(&state);
    pm.start_flush_loop();

    // 恢复自动启动的网络
    let auto_start = db.list_auto_start_networks()?;
    if !auto_start.is_empty() {
        let mut entries = Vec::new();
        for row in &auto_start {
            match network::write_network_config(&state, row) {
                Ok(path) => entries.push((
                    row.id.clone(),
                    path.to_string_lossy().to_string(),
                    row.rpc_port,
                )),
                Err(err) => {
                    tracing::error!(error = %err, network_id = %row.id, "写入网络配置失败")
                }
            }
        }
        let count = entries.len();
        pm.resume_auto_start(entries).await;
        tracing::info!(count, "已恢复自动启动的网络实例");
    }

    // 异步检查二进制可用性（不阻塞启动）
    {
        let state = state.clone();
        tokio::spawn(async move {
            let settings = state.easytier_settings();
            let res = check_binaries(&settings.cli_path, &settings.core_path).await;
            if !res.core.available || !res.cli.available {
                tracing::warn!(
                    cli = ?res.cli,
                    core = ?res.core,
                    "easytier-core / easytier-cli 不可用，请检查 EASYTIER_CORE_PATH / EASYTIER_CLI_PATH"
                );
            } else {
                tracing::info!(
                    core = ?res.core.version,
                    cli = ?res.cli.version,
                    "EasyTier 二进制检测通过"
                );
            }
        });
    }

    let router = app::build_router(state.clone());
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(
        url = format!("http://{}:{}", config.host, config.port),
        env = %config.env,
        "EasyTier 控制台已启动"
    );

    let shutdown_pm = pm.clone();
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(shutdown_pm))
    .await?;

    Ok(())
}

async fn shutdown_signal(pm: ProcessManager) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    tracing::info!("正在关闭控制台…");
    pm.shutdown().await;
}
