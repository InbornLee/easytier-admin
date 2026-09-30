//! easytier-cli 子进程调用封装。

use std::process::Stdio;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::error::{easytier_error, AppResult};

use super::types::{
    CredentialInfo, GenerateCredentialResponse, NodeInfo, PeerListItem, RevokeCredentialResponse,
    RouteListItem,
};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(8000);

pub struct CliRunResult {
    pub stdout: String,
    pub stderr: String,
    pub code: Option<i32>,
}

async fn read_all<R>(reader: Option<R>) -> Vec<u8>
where
    R: AsyncReadExt + Unpin,
{
    let mut buf = Vec::new();
    if let Some(mut reader) = reader {
        let _ = reader.read_to_end(&mut buf).await;
    }
    buf
}

pub async fn run_process(
    bin: &str,
    args: &[String],
    timeout: Duration,
) -> std::io::Result<CliRunResult> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn()?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_task = tokio::spawn(read_all(stdout));
    let err_task = tokio::spawn(read_all(stderr));

    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(status) => {
            let out = out_task.await.unwrap_or_default();
            let err = err_task.await.unwrap_or_default();
            let code = status.ok().and_then(|s| s.code());
            Ok(CliRunResult {
                stdout: String::from_utf8_lossy(&out).into_owned(),
                stderr: String::from_utf8_lossy(&err).into_owned(),
                code,
            })
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("命令执行超时（{}ms）", timeout.as_millis()),
            ))
        }
    }
}

fn unwrap_multi_instance(parsed: Value) -> Value {
    if let Value::Array(items) = &parsed {
        let multi = !items.is_empty()
            && items
                .iter()
                .all(|item| item.get("instance_id").is_some() && item.get("result").is_some());
        if multi {
            return Value::Array(
                items
                    .iter()
                    .map(|i| i.get("result").cloned().unwrap_or(Value::Null))
                    .collect(),
            );
        }
    }
    parsed
}

async fn run_cli_value(
    cli_path: &str,
    rpc_port: u16,
    args: &[String],
    timeout: Duration,
    verbose: bool,
) -> AppResult<Value> {
    let mut cli_args: Vec<String> = vec![
        "-p".to_string(),
        format!("127.0.0.1:{rpc_port}"),
        "-o".to_string(),
        "json".to_string(),
    ];
    if verbose {
        cli_args.push("-v".to_string());
    }
    cli_args.extend(args.iter().cloned());

    let result = run_process(cli_path, &cli_args, timeout)
        .await
        .map_err(|e| {
            easytier_error(
                format!("调用 easytier-cli 失败: {e}"),
                Some(json!({ "args": cli_args })),
            )
        })?;

    if result.code != Some(0) {
        let detail = if !result.stderr.trim().is_empty() {
            result.stderr.trim().to_string()
        } else {
            result.stdout.trim().to_string()
        };
        let detail = if detail.is_empty() {
            "无输出".to_string()
        } else {
            detail
        };
        return Err(easytier_error(
            format!(
                "easytier-cli 执行失败（退出码 {}）: {detail}",
                result
                    .code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "null".to_string())
            ),
            Some(json!({ "args": cli_args, "code": result.code })),
        ));
    }

    let text = result.stdout.trim();
    if text.is_empty() {
        return Ok(Value::Array(vec![]));
    }
    let parsed: Value = serde_json::from_str(text).map_err(|_| {
        easytier_error(
            "easytier-cli 返回了无法解析的 JSON",
            Some(json!({ "output": text.chars().take(500).collect::<String>() })),
        )
    })?;
    Ok(unwrap_multi_instance(parsed))
}

async fn run_cli_json<T: DeserializeOwned>(
    cli_path: &str,
    rpc_port: u16,
    args: &[String],
    timeout: Duration,
    verbose: bool,
) -> AppResult<T> {
    let value = run_cli_value(cli_path, rpc_port, args, timeout, verbose).await?;
    serde_json::from_value(value)
        .map_err(|e| easytier_error(format!("easytier-cli JSON 结构不匹配: {e}"), None))
}

async fn run_cli_raw(
    cli_path: &str,
    rpc_port: u16,
    args: &[String],
    timeout: Duration,
) -> AppResult<String> {
    let mut cli_args: Vec<String> = vec!["-p".to_string(), format!("127.0.0.1:{rpc_port}")];
    cli_args.extend(args.iter().cloned());
    let result = run_process(cli_path, &cli_args, timeout)
        .await
        .map_err(|e| {
            easytier_error(
                format!("调用 easytier-cli 失败: {e}"),
                Some(json!({ "args": cli_args })),
            )
        })?;
    if result.code != Some(0) {
        let detail = if !result.stderr.trim().is_empty() {
            result.stderr.trim().to_string()
        } else {
            result.stdout.trim().to_string()
        };
        return Err(easytier_error(
            format!("easytier-cli 执行失败: {detail}"),
            Some(json!({ "args": cli_args })),
        ));
    }
    Ok(result.stdout)
}

/// 绑定某个网络实例 RPC 端口的 CLI 客户端
#[derive(Clone)]
pub struct Cli {
    pub path: String,
    pub rpc_port: u16,
    pub timeout: Duration,
    pub verbose: bool,
}

impl Cli {
    pub fn new(path: impl Into<String>, rpc_port: u16, timeout: Duration) -> Self {
        Self {
            path: path.into(),
            rpc_port,
            timeout,
            verbose: false,
        }
    }

    pub async fn json<T: DeserializeOwned>(&self, args: &[&str]) -> AppResult<T> {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        run_cli_json(
            &self.path,
            self.rpc_port,
            &owned,
            self.timeout,
            self.verbose,
        )
        .await
    }

    pub async fn raw(&self, args: &[&str]) -> AppResult<String> {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        run_cli_raw(&self.path, self.rpc_port, &owned, self.timeout).await
    }

    pub async fn peer_list(&self) -> AppResult<Vec<PeerListItem>> {
        self.json(&["peer", "list"]).await
    }

    pub async fn route_list(&self) -> AppResult<Vec<RouteListItem>> {
        self.json(&["route", "list"]).await
    }

    pub async fn node_info(&self) -> AppResult<NodeInfo> {
        self.json(&["node", "info"]).await
    }

    pub async fn credential_list(&self) -> AppResult<Vec<CredentialInfo>> {
        let value = run_cli_value(
            &self.path,
            self.rpc_port,
            &["credential".to_string(), "list".to_string()],
            self.timeout,
            self.verbose,
        )
        .await?;
        if let Value::Array(_) = value {
            return serde_json::from_value(value)
                .map_err(|e| easytier_error(format!("凭据列表解析失败: {e}"), None));
        }
        if let Some(list) = value.get("credentials") {
            return serde_json::from_value(list.clone())
                .map_err(|e| easytier_error(format!("凭据列表解析失败: {e}"), None));
        }
        Ok(Vec::new())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn credential_generate(
        &self,
        ttl: i64,
        credential_id: Option<&str>,
        groups: Option<&[String]>,
        allow_relay: bool,
        allowed_proxy_cidrs: Option<&[String]>,
        reusable: bool,
    ) -> AppResult<GenerateCredentialResponse> {
        let mut args: Vec<String> = vec![
            "credential".to_string(),
            "generate".to_string(),
            "--ttl".to_string(),
            ttl.to_string(),
        ];
        if let Some(id) = credential_id {
            args.push("--credential-id".to_string());
            args.push(id.to_string());
        }
        if let Some(groups) = groups {
            if !groups.is_empty() {
                args.push("--groups".to_string());
                args.push(groups.join(","));
            }
        }
        if allow_relay {
            args.push("--allow-relay".to_string());
            args.push("true".to_string());
        }
        if let Some(cidrs) = allowed_proxy_cidrs {
            if !cidrs.is_empty() {
                args.push("--allowed-proxy-cidrs".to_string());
                args.push(cidrs.join(","));
            }
        }
        args.push("--reusable".to_string());
        args.push(reusable.to_string());
        run_cli_json(&self.path, self.rpc_port, &args, self.timeout, self.verbose).await
    }

    pub async fn credential_revoke(
        &self,
        credential_id: &str,
    ) -> AppResult<RevokeCredentialResponse> {
        self.json(&["credential", "revoke", credential_id]).await
    }

    pub async fn logger_set(&self, level: &str) -> AppResult<String> {
        self.raw(&["logger", "set", level]).await
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BinaryStatus {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BinariesStatus {
    pub cli: BinaryStatus,
    pub core: BinaryStatus,
}

async fn check_binary(bin: &str) -> BinaryStatus {
    match run_process(bin, &["--version".to_string()], Duration::from_secs(5)).await {
        Ok(result) if result.code == Some(0) => {
            let text = if result.stdout.trim().is_empty() {
                result.stderr
            } else {
                result.stdout
            };
            BinaryStatus {
                available: true,
                version: text.trim().lines().next().map(|s| s.to_string()),
                error: None,
            }
        }
        Ok(result) => {
            let detail = if !result.stderr.trim().is_empty() {
                result.stderr.trim().to_string()
            } else {
                result.stdout.trim().to_string()
            };
            BinaryStatus {
                available: false,
                version: None,
                error: Some(detail),
            }
        }
        Err(err) => BinaryStatus {
            available: false,
            version: None,
            error: Some(err.to_string()),
        },
    }
}

/// 检查 easytier-cli / easytier-core 是否可用
pub async fn check_binaries(cli_path: &str, core_path: &str) -> BinariesStatus {
    let (cli, core) = tokio::join!(check_binary(cli_path), check_binary(core_path));
    BinariesStatus { cli, core }
}
