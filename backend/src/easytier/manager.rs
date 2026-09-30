//! EasyTier 核心进程管理器。

use std::collections::{HashMap, VecDeque};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::{broadcast, mpsc};

use crate::config::{Config, EasyTierSettings};
use crate::db::{Db, NewNodeLog};
use crate::util::now_ms;

const MAX_BUFFER: usize = 2000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub network_id: String,
    pub level: String,
    pub message: String,
    pub created_at: i64,
}

struct ManagedProcess {
    network_id: String,
    pid: Option<u32>,
    started_at: i64,
    stopping: AtomicBool,
    buffer: StdMutex<VecDeque<LogLine>>,
    kill_tx: mpsc::UnboundedSender<()>,
}

struct Inner {
    procs: StdMutex<HashMap<String, Arc<ManagedProcess>>>,
    pending: StdMutex<Vec<LogLine>>,
    log_tx: broadcast::Sender<LogLine>,
    db: Arc<Db>,
    config: Arc<Config>,
    easytier: Arc<RwLock<EasyTierSettings>>,
}

#[derive(Clone)]
pub struct ProcessManager {
    inner: Arc<Inner>,
}

fn detect_level(line: &str) -> &'static str {
    for token in line.split(|c: char| !c.is_ascii_alphabetic()) {
        match token {
            "TRACE" => return "trace",
            "DEBUG" => return "debug",
            "WARN" | "WARNING" => return "warn",
            "ERROR" => return "error",
            "INFO" => return "info",
            _ => {}
        }
    }
    "info"
}

impl ProcessManager {
    pub fn new(
        db: Arc<Db>,
        config: Arc<Config>,
        easytier: Arc<RwLock<EasyTierSettings>>,
        log_tx: broadcast::Sender<LogLine>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                procs: StdMutex::new(HashMap::new()),
                pending: StdMutex::new(Vec::new()),
                log_tx,
                db,
                config,
                easytier,
            }),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogLine> {
        self.inner.log_tx.subscribe()
    }

    pub fn is_running(&self, network_id: &str) -> bool {
        self.inner
            .procs
            .lock()
            .unwrap()
            .get(network_id)
            .map(|p| !p.stopping.load(Ordering::SeqCst))
            .unwrap_or(false)
    }

    pub fn recent_logs(&self, network_id: &str, limit: usize) -> Vec<LogLine> {
        let procs = self.inner.procs.lock().unwrap();
        match procs.get(network_id) {
            Some(p) => {
                let buf = p.buffer.lock().unwrap();
                let start = buf.len().saturating_sub(limit);
                buf.iter().skip(start).cloned().collect()
            }
            None => Vec::new(),
        }
    }

    pub fn start(&self, network_id: &str, config_path: &str, rpc_port: i64) {
        if self.is_running(network_id) {
            return;
        }
        self.inner.procs.lock().unwrap().remove(network_id);

        let log_dir = self.inner.config.logs_dir.join(network_id);
        let _ = std::fs::create_dir_all(&log_dir);

        let core_path = self
            .inner
            .easytier
            .read()
            .map(|s| s.core_path.clone())
            .unwrap_or_else(|_| "easytier-core".to_string());

        let mut cmd = Command::new(&core_path);
        cmd.arg("-c")
            .arg(config_path)
            .arg("--console-log-level")
            .arg("info")
            .arg("--file-log-level")
            .arg("info")
            .arg("--file-log-dir")
            .arg(&log_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if rpc_port > 0 {
            // RPC 门户仅供本机控制台（easytier-cli -p 127.0.0.1:<port>）使用，
            // 绑定 127.0.0.1 避免在宿主机网络上暴露控制通道
            cmd.arg("--rpc-portal").arg(format!("127.0.0.1:{rpc_port}"));
        }

        match cmd.spawn() {
            Ok(mut child) => {
                let pid = child.id();
                let (kill_tx, kill_rx) = mpsc::unbounded_channel::<()>();
                let managed = Arc::new(ManagedProcess {
                    network_id: network_id.to_string(),
                    pid,
                    started_at: now_ms(),
                    stopping: AtomicBool::new(false),
                    buffer: StdMutex::new(VecDeque::new()),
                    kill_tx,
                });
                self.inner
                    .procs
                    .lock()
                    .unwrap()
                    .insert(network_id.to_string(), managed.clone());

                if let Some(stdout) = child.stdout.take() {
                    self.spawn_reader(managed.clone(), stdout);
                }
                if let Some(stderr) = child.stderr.take() {
                    self.spawn_reader(managed.clone(), stderr);
                }

                let pm = self.clone();
                let nid = network_id.to_string();
                tokio::spawn(async move {
                    // 独立的终止任务：收到停止请求后先 SIGTERM，超时再 SIGKILL
                    let (done_tx, done_rx) = tokio::sync::oneshot::channel::<()>();
                    tokio::spawn(async move {
                        let mut kill_rx = kill_rx;
                        // 未收到停止请求时，等待 kill_tx 被释放（进程退出/记录移除）
                        if kill_rx.recv().await.is_none() {
                            return;
                        }
                        #[cfg(unix)]
                        if let Some(pid) = pid {
                            unsafe {
                                libc::kill(pid as i32, libc::SIGTERM);
                            }
                        }
                        // 等待进程真正退出，超时则强制 SIGKILL，避免 PID 复用误杀
                        if tokio::time::timeout(Duration::from_secs(8), done_rx)
                            .await
                            .is_err()
                        {
                            #[cfg(unix)]
                            if let Some(pid) = pid {
                                unsafe {
                                    libc::kill(pid as i32, libc::SIGKILL);
                                }
                            }
                        }
                    });

                    let status = child.wait().await;
                    let _ = done_tx.send(());
                    pm.on_exit(&nid, status);
                });

                self.set_network_status(network_id, "running", None);
                self.push_system_log(network_id, "info", "easytier-core 已启动");
                tracing::info!(network_id, pid = ?pid, "easytier-core 已启动");
            }
            Err(err) => {
                tracing::error!(error = %err, network_id, "easytier-core 进程启动失败");
                self.set_network_status(network_id, "error", Some(&err.to_string()));
                self.push_system_log(network_id, "error", &format!("进程启动失败: {err}"));
            }
        }
    }

    pub async fn stop(&self, network_id: &str, timeout_ms: u64) {
        let proc = self.inner.procs.lock().unwrap().get(network_id).cloned();
        let Some(proc) = proc else {
            return;
        };
        proc.stopping.store(true, Ordering::SeqCst);
        self.push_system_log(network_id, "info", "正在停止 easytier-core …");
        let _ = proc.kill_tx.send(());

        let deadline = Instant::now() + Duration::from_millis(timeout_ms + 3000);
        while Instant::now() < deadline {
            if !self.inner.procs.lock().unwrap().contains_key(network_id) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        if self
            .inner
            .procs
            .lock()
            .unwrap()
            .remove(network_id)
            .is_some()
        {
            self.set_network_status(network_id, "stopped", None);
        }
    }

    pub async fn restart(&self, network_id: &str, config_path: &str, rpc_port: i64) {
        self.stop(network_id, 8000).await;
        self.start(network_id, config_path, rpc_port);
    }

    pub async fn shutdown(&self) {
        let ids: Vec<String> = self.inner.procs.lock().unwrap().keys().cloned().collect();
        for id in ids {
            self.stop(&id, 3000).await;
        }
        self.flush_pending();
    }

    pub async fn resume_auto_start(&self, entries: Vec<(String, String, i64)>) {
        for (id, config_path, rpc_port) in entries {
            self.start(&id, &config_path, rpc_port);
        }
    }

    pub fn start_flush_loop(&self) {
        let pm = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                pm.flush_pending();
            }
        });
    }

    // ------------------------------------------------------------ 内部

    fn spawn_reader<R>(&self, proc: Arc<ManagedProcess>, reader: R)
    where
        R: AsyncRead + Unpin + Send + 'static,
    {
        let pm = self.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(reader);
            let mut buf: Vec<u8> = Vec::new();
            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let text = String::from_utf8_lossy(&buf)
                            .trim_end_matches(['\n', '\r'])
                            .to_string();
                        pm.handle_line(&proc, text);
                    }
                    Err(_) => break,
                }
            }
        });
    }

    fn handle_line(&self, proc: &Arc<ManagedProcess>, text: String) {
        if text.trim().is_empty() {
            return;
        }
        let line = LogLine {
            network_id: proc.network_id.clone(),
            level: detect_level(&text).to_string(),
            message: text,
            created_at: now_ms(),
        };
        {
            let mut buf = proc.buffer.lock().unwrap();
            if buf.len() >= MAX_BUFFER {
                buf.pop_front();
            }
            buf.push_back(line.clone());
        }
        self.inner.pending.lock().unwrap().push(line.clone());
        let _ = self.inner.log_tx.send(line);
    }

    fn push_system_log(&self, network_id: &str, level: &str, message: &str) {
        let line = LogLine {
            network_id: network_id.to_string(),
            level: level.to_string(),
            message: message.to_string(),
            created_at: now_ms(),
        };
        self.inner.pending.lock().unwrap().push(line.clone());
        let _ = self.inner.log_tx.send(line);
    }

    fn flush_pending(&self) {
        let batch: Vec<LogLine> = {
            let mut pending = self.inner.pending.lock().unwrap();
            if pending.is_empty() {
                return;
            }
            std::mem::take(&mut *pending)
        };
        let logs: Vec<NewNodeLog> = batch
            .into_iter()
            .map(|l| NewNodeLog {
                network_id: Some(l.network_id),
                node_id: None,
                source: "core".to_string(),
                level: l.level,
                message: l.message,
                created_at: l.created_at,
            })
            .collect();
        if let Err(err) = self.inner.db.insert_node_logs(&logs) {
            tracing::error!(error = %err, "写入节点日志失败");
        }
    }

    fn on_exit(&self, network_id: &str, status: std::io::Result<std::process::ExitStatus>) {
        let managed = self.inner.procs.lock().unwrap().remove(network_id);
        let was_stopping = managed
            .as_ref()
            .map(|m| m.stopping.load(Ordering::SeqCst))
            .unwrap_or(false);

        let code = status.as_ref().ok().and_then(|s| s.code());
        #[cfg(unix)]
        let signal = status
            .as_ref()
            .ok()
            .and_then(|s| std::os::unix::process::ExitStatusExt::signal(s));
        #[cfg(not(unix))]
        let signal: Option<i32> = None;

        let code_text = code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "null".to_string());
        let signal_text = signal
            .map(|s| s.to_string())
            .unwrap_or_else(|| "null".to_string());

        if was_stopping || code == Some(0) {
            self.push_system_log(
                network_id,
                "info",
                &format!("easytier-core 已退出 (code={code_text}, signal={signal_text})"),
            );
            self.set_network_status(network_id, "stopped", None);
        } else {
            self.push_system_log(
                network_id,
                "error",
                &format!("easytier-core 已退出 (code={code_text}, signal={signal_text})"),
            );
            self.set_network_status(
                network_id,
                "error",
                Some(&format!(
                    "进程异常退出 (code={code_text}, signal={signal_text})"
                )),
            );
        }
    }

    fn set_network_status(&self, network_id: &str, status: &str, last_error: Option<&str>) {
        if let Err(err) = self
            .inner
            .db
            .set_network_status(network_id, status, last_error, now_ms())
        {
            tracing::error!(error = %err, network_id, "更新网络状态失败");
        }
    }
}
