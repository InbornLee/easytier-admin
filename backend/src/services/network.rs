use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use futures_util::future::join_all;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::db::NetworkRow;
use crate::easytier::toml::{build_network_toml, merge_flags, NetworkTomlInput};
use crate::easytier::types::{NodeInfo, PeerListItem, RouteListItem};
use crate::error::{bad_request, conflict, forbidden, not_found, AppResult};
use crate::middleware::AuthUser;
use crate::state::AppState;
use crate::util::crypto::random_token;
use crate::util::format::parse_human_size;
use crate::util::now_ms;
use crate::util::x25519::{derive_key_pair, derive_public_key};

use super::safe_object;
use super::safe_string_array;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicNetwork {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub network_name: String,
    pub ipv4: String,
    pub dhcp: bool,
    pub hostname: String,
    pub instance_name: String,
    pub listeners: Vec<String>,
    pub mapped_listeners: Vec<String>,
    pub peers: Vec<String>,
    pub external_node: Option<String>,
    pub listen_port: i64,
    pub rpc_port: i64,
    pub secure_mode: bool,
    pub shared_node_public_key: String,
    pub flags: Map<String, Value>,
    pub auto_start: bool,
    pub status: String,
    pub last_error: Option<String>,
    pub owner_id: Option<String>,
    pub access: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub fn to_public_network(state: &AppState, row: &NetworkRow, access: AccessLevel) -> PublicNetwork {
    let private_key = state
        .crypto
        .try_decrypt(row.local_private_key_enc.as_deref());
    let flags = merge_flags(&safe_object(&row.flags));
    PublicNetwork {
        id: row.id.clone(),
        name: row.name.clone(),
        description: row.description.clone(),
        network_name: row.network_name.clone(),
        ipv4: row.ipv4.clone(),
        dhcp: row.dhcp,
        hostname: row.hostname.clone(),
        instance_name: row.instance_name.clone(),
        listeners: safe_string_array(&row.listeners),
        mapped_listeners: safe_string_array(&row.mapped_listeners),
        peers: safe_string_array(&row.peers),
        external_node: row.external_node.clone(),
        listen_port: row.listen_port,
        rpc_port: row.rpc_port,
        secure_mode: row.secure_mode,
        shared_node_public_key: if private_key.is_empty() {
            String::new()
        } else {
            derive_public_key(&private_key)
        },
        flags,
        auto_start: row.auto_start,
        status: row.status.clone(),
        last_error: row.last_error.clone(),
        owner_id: row.owner_id.clone(),
        access: access.as_str().to_string(),
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

#[derive(Default)]
pub struct NetworkFields {
    pub name: Option<String>,
    pub description: Option<String>,
    pub network_name: Option<String>,
    pub network_secret: Option<String>,
    pub ipv4: Option<String>,
    pub dhcp: Option<bool>,
    pub hostname: Option<String>,
    pub instance_name: Option<String>,
    pub listeners: Option<Vec<String>>,
    pub mapped_listeners: Option<Vec<String>>,
    pub peers: Option<Vec<String>>,
    pub external_node: Option<String>,
    pub listen_port: Option<i64>,
    pub rpc_port: Option<i64>,
    pub secure_mode: Option<bool>,
    pub auto_start: Option<bool>,
    pub start_now: Option<bool>,
    pub flags: Option<Map<String, Value>>,
}

pub fn get_network_row(state: &AppState, id: &str) -> AppResult<Option<NetworkRow>> {
    state.db.get_network(id)
}

pub fn get_network_row_or_throw(state: &AppState, id: &str) -> AppResult<NetworkRow> {
    get_network_row(state, id)?.ok_or_else(|| not_found("网络不存在"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
    View,
    Manage,
    Owner,
}

impl AccessLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            AccessLevel::View => "view",
            AccessLevel::Manage => "manage",
            AccessLevel::Owner => "owner",
        }
    }

    pub fn can_manage(self) -> bool {
        matches!(self, AccessLevel::Manage | AccessLevel::Owner)
    }
}

/// 计算用户对某个网络的访问级别；None 表示无权限。
pub fn access_level(
    state: &AppState,
    row: &NetworkRow,
    user: &AuthUser,
) -> AppResult<Option<AccessLevel>> {
    if user.role == "admin" {
        return Ok(Some(AccessLevel::Manage));
    }
    if row.owner_id.as_deref() == Some(user.id.as_str()) {
        return Ok(Some(AccessLevel::Owner));
    }
    match state.db.get_share_permission(&row.id, &user.id)? {
        Some(p) if p == "manage" => Ok(Some(AccessLevel::Manage)),
        Some(_) => Ok(Some(AccessLevel::View)),
        None => Ok(None),
    }
}

/// 校验用户对该网络的访问权限；无权限按“网络不存在”处理，避免泄露。
pub fn require_network_access(
    state: &AppState,
    id: &str,
    user: &AuthUser,
    need_manage: bool,
) -> AppResult<NetworkRow> {
    let row = get_network_row_or_throw(state, id)?;
    match access_level(state, &row, user)? {
        Some(level) if !need_manage || level.can_manage() => Ok(row),
        Some(_) => Err(forbidden("没有权限管理该网络")),
        None => Err(not_found("网络不存在")),
    }
}

/// 用户可见的网络 ID 集合；None 表示全部（管理员）。
pub fn accessible_network_ids(
    state: &AppState,
    user: &AuthUser,
) -> AppResult<Option<HashSet<String>>> {
    if user.role == "admin" {
        return Ok(None);
    }
    let rows = state.db.list_networks_for_user(&user.id)?;
    Ok(Some(rows.into_iter().map(|r| r.id).collect()))
}

pub fn list_networks_for_user(state: &AppState, user: &AuthUser) -> AppResult<Vec<PublicNetwork>> {
    if user.role == "admin" {
        return Ok(state
            .db
            .list_networks()?
            .iter()
            .map(|row| to_public_network(state, row, AccessLevel::Manage))
            .collect());
    }
    let rows = state.db.list_networks_for_user(&user.id)?;
    let permissions: HashMap<String, String> = state
        .db
        .list_share_permissions_for_user(&user.id)?
        .into_iter()
        .collect();
    Ok(rows
        .iter()
        .map(|row| {
            let level = if row.owner_id.as_deref() == Some(user.id.as_str()) {
                AccessLevel::Owner
            } else if permissions
                .get(&row.id)
                .map(|p| p == "manage")
                .unwrap_or(false)
            {
                AccessLevel::Manage
            } else {
                AccessLevel::View
            };
            to_public_network(state, row, level)
        })
        .collect())
}

/// 管理员视角的网络列表（供内部/管理用途）
pub fn list_networks(state: &AppState) -> AppResult<Vec<PublicNetwork>> {
    Ok(state
        .db
        .list_networks()?
        .iter()
        .map(|row| to_public_network(state, row, AccessLevel::Manage))
        .collect())
}

// ------------------------------------------------------------------ 端口分配

async fn is_port_free(port: u16) -> bool {
    match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
        Ok(listener) => {
            drop(listener);
            true
        }
        Err(_) => false,
    }
}

async fn find_free_port(start: u16, used: &HashSet<u16>) -> AppResult<u16> {
    for offset in 0..2000u32 {
        let port = start as u32 + offset;
        if port > 65535 {
            break;
        }
        let port = port as u16;
        if used.contains(&port) {
            continue;
        }
        if is_port_free(port).await {
            return Ok(port);
        }
    }
    Err(bad_request("无法分配可用端口，请检查端口范围设置", None))
}

async fn find_free_port_pair(start: u16, used: &mut HashSet<u16>) -> AppResult<u16> {
    for offset in 0..2000u32 {
        let port = start as u32 + offset;
        if port + 1 > 65535 {
            break;
        }
        let port = port as u16;
        if used.contains(&port) || used.contains(&(port + 1)) {
            continue;
        }
        if is_port_free(port).await && is_port_free(port + 1).await {
            used.insert(port);
            used.insert(port + 1);
            return Ok(port);
        }
    }
    Err(bad_request("无法分配可用端口，请检查端口范围设置", None))
}

fn parse_listener_ports(listeners: &[String]) -> Vec<u16> {
    let mut ports = Vec::new();
    for listener in listeners {
        // 形如 tcp://0.0.0.0:11011 / udp://[::]:11011 / ws://0.0.0.0:11012/
        if let Some(idx) = listener.rfind(':') {
            let digits: String = listener[idx + 1..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(port) = digits.parse::<u16>() {
                if port > 0 {
                    ports.push(port);
                }
            }
        }
    }
    ports
}

fn insert_row_ports(used: &mut HashSet<u16>, row: &NetworkRow) {
    if (1..=65535).contains(&row.listen_port) {
        let port = row.listen_port as u16;
        used.insert(port);
        if port < 65535 {
            // 默认使用 port+1 作为 ws 监听端口
            used.insert(port + 1);
        }
    }
    if (1..=65535).contains(&row.rpc_port) {
        used.insert(row.rpc_port as u16);
    }
    for port in parse_listener_ports(&safe_string_array(&row.listeners)) {
        used.insert(port);
    }
}

fn remove_row_ports(used: &mut HashSet<u16>, row: &NetworkRow) {
    if (1..=65535).contains(&row.listen_port) {
        let port = row.listen_port as u16;
        used.remove(&port);
        if port < 65535 {
            used.remove(&(port + 1));
        }
    }
    if (1..=65535).contains(&row.rpc_port) {
        used.remove(&(row.rpc_port as u16));
    }
    for port in parse_listener_ports(&safe_string_array(&row.listeners)) {
        used.remove(&port);
    }
}

/// 汇总所有已有网络已占用的本地端口：
/// 监听端口及其 ws 端口（port+1）、RPC（共享节点）端口，以及 listeners 中声明的端口。
pub fn collect_used_ports(state: &AppState) -> AppResult<HashSet<u16>> {
    let mut used = HashSet::new();
    for row in state.db.list_networks()? {
        insert_row_ports(&mut used, &row);
    }
    Ok(used)
}

async fn find_free_rpc(state: &AppState, used: &mut HashSet<u16>) -> AppResult<u16> {
    let settings = state.easytier_settings();
    let port = find_free_port(settings.rpc_port_start, used).await?;
    used.insert(port);
    Ok(port)
}

/// 校验用户指定的监听端口（含 ws 端口 port+1）是否可用。
async fn validate_listen_port(port: i64, used: &mut HashSet<u16>) -> AppResult<u16> {
    if !(1..=65535).contains(&port) {
        return Err(bad_request("监听端口超出范围", None));
    }
    let port = port as u16;
    let ws = port.saturating_add(1);
    if used.contains(&port) || used.contains(&ws) {
        return Err(conflict(format!(
            "监听端口 {port}（ws 端口 {ws}）已被其他网络占用"
        )));
    }
    if !is_port_free(port).await || (port < 65535 && !is_port_free(ws).await) {
        return Err(conflict(format!(
            "监听端口 {port}（ws 端口 {ws}）已被系统占用"
        )));
    }
    used.insert(port);
    used.insert(ws);
    Ok(port)
}

/// 校验用户指定的 RPC（共享节点）端口是否可用。
async fn validate_rpc_port(port: i64, used: &mut HashSet<u16>) -> AppResult<u16> {
    if !(1..=65535).contains(&port) {
        return Err(bad_request("RPC 端口超出范围", None));
    }
    let port = port as u16;
    if used.contains(&port) {
        return Err(conflict(format!("RPC 端口 {port} 已被其他网络占用")));
    }
    if !is_port_free(port).await {
        return Err(conflict(format!("RPC 端口 {port} 已被系统占用")));
    }
    used.insert(port);
    Ok(port)
}

fn default_listeners(port: i64) -> Vec<String> {
    vec![
        format!("tcp://0.0.0.0:{port}"),
        format!("udp://0.0.0.0:{port}"),
        format!("ws://0.0.0.0:{}/", port + 1),
    ]
}

/// 未显式提供监听器时，按实际分配的监听端口生成默认监听器。
fn resolve_listeners(listeners: Vec<String>, listen_port: i64) -> Vec<String> {
    if listeners.is_empty() {
        default_listeners(listen_port)
    } else {
        listeners
    }
}

// ------------------------------------------------------------------ 配置文件

pub fn write_network_config(state: &AppState, row: &NetworkRow) -> AppResult<PathBuf> {
    let network_secret = state.crypto.try_decrypt(Some(&row.network_secret_enc));
    let local_private_key = state
        .crypto
        .try_decrypt(row.local_private_key_enc.as_deref());

    let mut peers = safe_string_array(&row.peers);
    if let Some(external) = row.external_node.as_deref() {
        if !external.is_empty() {
            peers.insert(0, external.to_string());
        }
    }
    let listeners = safe_string_array(&row.listeners);

    let credential_file = state
        .config
        .credential_file_path_for(&row.id)
        .to_string_lossy()
        .to_string();

    let input = NetworkTomlInput {
        instance_name: row.instance_name.clone(),
        hostname: row.hostname.clone(),
        network_name: row.network_name.clone(),
        network_secret: if network_secret.is_empty() {
            None
        } else {
            Some(network_secret)
        },
        ipv4: if row.dhcp {
            None
        } else {
            Some(row.ipv4.clone())
        },
        dhcp: row.dhcp,
        listeners,
        mapped_listeners: safe_string_array(&row.mapped_listeners),
        exit_nodes: Vec::new(),
        peers,
        proxy_networks: Vec::new(),
        credential_file: Some(credential_file),
        local_private_key: if local_private_key.is_empty() {
            None
        } else {
            Some(local_private_key)
        },
        local_public_key: None,
        flags: safe_object(&row.flags),
    };

    let toml = build_network_toml(&input);
    let file_path = state.config.config_path_for(&row.id);
    fs::write(&file_path, toml)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600));
    }
    Ok(file_path)
}

// ------------------------------------------------------------------ CRUD

pub async fn create_network(
    state: &AppState,
    input: NetworkFields,
    owner_id: Option<&str>,
) -> AppResult<PublicNetwork> {
    let name = input.name.unwrap_or_default().trim().to_string();
    if name.is_empty() {
        return Err(bad_request("网络名称不能为空", None));
    }
    let network_name = input
        .network_name
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| name.clone());
    if network_name.is_empty() {
        return Err(bad_request("EasyTier 网络名称不能为空", None));
    }

    // 端口分配 + 落库加锁：避免多用户并发创建时分配到相同端口
    let guard = state.port_alloc.lock().await;
    if state.db.list_networks()?.iter().any(|n| n.name == name) {
        return Err(conflict("已存在同名网络"));
    }

    let mut used = collect_used_ports(state)?;
    let listen_port = match input.listen_port {
        Some(port) => validate_listen_port(port, &mut used).await? as i64,
        None => {
            let settings = state.easytier_settings();
            find_free_port_pair(settings.listen_port_start, &mut used).await? as i64
        }
    };
    let rpc_port = match input.rpc_port {
        Some(port) => validate_rpc_port(port, &mut used).await? as i64,
        None => find_free_rpc(state, &mut used).await? as i64,
    };

    // 校验用户自定义监听器占用的端口（同一网络内 tcp/udp 复用同端口属正常，需去重）
    let provided_listeners: Vec<String> = input
        .listeners
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect();
    if !provided_listeners.is_empty() {
        let mut local_ports: HashSet<u16> = HashSet::new();
        for port in parse_listener_ports(&provided_listeners) {
            if !local_ports.insert(port) {
                continue; // 本网络内重复（如 tcp/udp 同端口）
            }
            if used.contains(&port) {
                return Err(conflict(format!("监听器端口 {port} 已被其他网络占用")));
            }
            if !is_port_free(port).await {
                return Err(conflict(format!("监听器端口 {port} 已被系统占用")));
            }
            used.insert(port);
        }
    }

    let secure_mode = input.secure_mode.unwrap_or(true);
    let keys = if secure_mode {
        Some(derive_key_pair())
    } else {
        None
    };
    let network_secret = input
        .network_secret
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| random_token(24));
    let id = random_token(9);
    let ts = now_ms();
    let settings = state.easytier_settings();

    let row = NetworkRow {
        id: id.clone(),
        name,
        description: input.description,
        network_name,
        network_secret_enc: state.crypto.encrypt(&network_secret),
        ipv4: input.ipv4.unwrap_or_else(|| "10.126.126.1/24".to_string()),
        dhcp: input.dhcp.unwrap_or(false),
        hostname: input
            .hostname
            .unwrap_or_else(|| format!("et-shared-{}", &id[..id.len().min(6)])),
        instance_name: input
            .instance_name
            .unwrap_or_else(|| format!("net-{}", &id[..id.len().min(6)])),
        listeners: serde_json::to_string(&resolve_listeners(provided_listeners, listen_port))?,
        mapped_listeners: serde_json::to_string(&input.mapped_listeners.unwrap_or_default())?,
        peers: serde_json::to_string(&input.peers.unwrap_or_default())?,
        external_node: match input.external_node {
            // 未显式提供时使用全局默认公共节点；显式留空则不设置初始 Peer
            None => Some(settings.default_external_node.clone()),
            Some(v) => {
                let v = v.trim();
                if v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                }
            }
        },
        listen_port,
        rpc_port,
        secure_mode,
        local_private_key_enc: keys.map(|k| state.crypto.encrypt(&k.private_key)),
        credential_file: Some(
            state
                .config
                .credential_file_path_for(&id)
                .to_string_lossy()
                .to_string(),
        ),
        flags: serde_json::to_string(&merge_flags(&input.flags.unwrap_or_default()))?,
        auto_start: input.auto_start.unwrap_or(false),
        status: "stopped".to_string(),
        last_error: None,
        owner_id: owner_id.map(|s| s.to_string()),
        created_at: ts,
        updated_at: ts,
    };

    state.db.insert_network(&row)?;
    write_network_config(state, &row)?;
    // 端口已落库，释放分配锁
    drop(guard);
    tracing::info!(network_id = %id, name = %row.name, "已创建网络");

    if input.start_now.unwrap_or(false) || row.auto_start {
        start_network(state, &id).await?;
    }
    Ok(to_public_network(
        state,
        &get_network_row_or_throw(state, &id)?,
        AccessLevel::Owner,
    ))
}

pub async fn update_network(
    state: &AppState,
    id: &str,
    input: NetworkFields,
) -> AppResult<PublicNetwork> {
    let mut row = get_network_row_or_throw(state, id)?;
    let was_running = state.pm.is_running(id);

    if let Some(v) = input.name {
        row.name = v.trim().to_string();
    }
    if let Some(v) = input.description {
        row.description = Some(v);
    }
    if let Some(v) = input.network_name {
        row.network_name = v.trim().to_string();
    }
    if let Some(v) = input.network_secret {
        if !v.is_empty() {
            row.network_secret_enc = state.crypto.encrypt(&v);
        }
    }
    if let Some(v) = input.ipv4 {
        row.ipv4 = v;
    }
    if let Some(v) = input.dhcp {
        row.dhcp = v;
    }
    if let Some(v) = input.hostname {
        row.hostname = v;
    }
    if let Some(v) = input.instance_name {
        row.instance_name = v;
    }
    if let Some(v) = input.listeners.as_ref() {
        row.listeners = serde_json::to_string(v)?;
    }
    if let Some(v) = input.mapped_listeners {
        row.mapped_listeners = serde_json::to_string(&v)?;
    }
    if let Some(v) = input.peers {
        row.peers = serde_json::to_string(&v)?;
    }
    if let Some(v) = input.external_node {
        let trimmed = v.trim().to_string();
        row.external_node = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        };
    }
    if let Some(v) = input.auto_start {
        row.auto_start = v;
    }
    if let Some(v) = input.secure_mode {
        row.secure_mode = v;
        if v && state
            .crypto
            .try_decrypt(row.local_private_key_enc.as_deref())
            .is_empty()
        {
            row.local_private_key_enc = Some(state.crypto.encrypt(&derive_key_pair().private_key));
        }
    }
    if let Some(v) = input.flags {
        row.flags = serde_json::to_string(&merge_flags(&v))?;
    }

    let next_listen = input.listen_port.unwrap_or(row.listen_port);
    let next_rpc = input.rpc_port.unwrap_or(row.rpc_port);
    if next_listen != row.listen_port || next_rpc != row.rpc_port {
        if was_running {
            return Err(bad_request("请先停止网络后再修改监听端口 / RPC 端口", None));
        }
        {
            // 端口变更同样需要排除其他网络已占用的端口（排除自身）
            let _guard = state.port_alloc.lock().await;
            let mut used = collect_used_ports(state)?;
            remove_row_ports(&mut used, &row);
            if next_listen != row.listen_port {
                validate_listen_port(next_listen, &mut used).await?;
            }
            if next_rpc != row.rpc_port {
                validate_rpc_port(next_rpc, &mut used).await?;
            }
        }
        row.listen_port = next_listen;
        row.rpc_port = next_rpc;
        if input.listeners.is_none() {
            row.listeners = serde_json::to_string(&default_listeners(next_listen))?;
        }
    }

    row.updated_at = now_ms();
    state.db.update_network(&row)?;
    let updated = get_network_row_or_throw(state, id)?;
    write_network_config(state, &updated)?;

    if was_running {
        restart_network(state, id).await?;
    }
    Ok(to_public_network(
        state,
        &get_network_row_or_throw(state, id)?,
        AccessLevel::Manage,
    ))
}

pub async fn delete_network(state: &AppState, id: &str) -> AppResult<()> {
    let row = get_network_row_or_throw(state, id)?;
    if state.pm.is_running(id) {
        state.pm.stop(id, 8000).await;
    }
    for file in [
        state.config.config_path_for(id),
        state.config.credential_file_path_for(id),
    ] {
        if file.exists() {
            if let Err(err) = fs::remove_file(&file) {
                tracing::warn!(error = %err, path = %file.display(), "删除网络文件失败");
            }
        }
    }
    state.db.delete_credentials_by_network(id)?;
    state.db.delete_nodes_by_network(id)?;
    state.db.delete_node_logs_by_network(id)?;
    state.db.delete_shares_by_network(id)?;
    state.db.delete_network(id)?;
    tracing::info!(network_id = id, name = %row.name, "已删除网络");
    Ok(())
}

pub async fn start_network(state: &AppState, id: &str) -> AppResult<()> {
    let row = get_network_row_or_throw(state, id)?;
    let config_path = write_network_config(state, &row)?;
    state
        .pm
        .start(id, &config_path.to_string_lossy(), row.rpc_port);
    Ok(())
}

pub async fn stop_network(state: &AppState, id: &str) -> AppResult<()> {
    get_network_row_or_throw(state, id)?;
    state.pm.stop(id, 8000).await;
    Ok(())
}

pub async fn restart_network(state: &AppState, id: &str) -> AppResult<()> {
    let row = get_network_row_or_throw(state, id)?;
    let config_path = write_network_config(state, &row)?;
    state
        .pm
        .restart(id, &config_path.to_string_lossy(), row.rpc_port)
        .await;
    Ok(())
}

/// 转移网络归属。原所有者会保留「可管理」分享权限，避免突然失去访问。
pub fn transfer_network_owner(
    state: &AppState,
    row: &NetworkRow,
    target_user_id: &str,
) -> AppResult<NetworkRow> {
    let mut updated = row.clone();
    let previous = updated.owner_id.clone();
    if previous.as_deref() == Some(target_user_id) {
        return Err(bad_request("该用户已是网络所有者", None));
    }
    if let Some(prev) = previous.as_deref() {
        if prev != target_user_id {
            state.db.upsert_share(&row.id, prev, "manage", now_ms())?;
        }
    }
    state.db.delete_share(&row.id, target_user_id)?;
    updated.owner_id = Some(target_user_id.to_string());
    updated.updated_at = now_ms();
    state.db.update_network(&updated)?;
    Ok(updated)
}

pub fn get_config_text(state: &AppState, id: &str) -> AppResult<String> {
    let row = get_network_row_or_throw(state, id)?;
    let path = state.config.config_path_for(id);
    let text = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        write_network_config(state, &row)?;
        fs::read_to_string(&path)?
    };
    Ok(mask_secrets(&text))
}

fn mask_secrets(text: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("network_secret") || trimmed.starts_with("local_private_key") {
                match line.find('=') {
                    Some(idx) => format!("{} \"********\"", &line[..=idx]),
                    None => line.to_string(),
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ------------------------------------------------------------------ 实时状态

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveState {
    pub online: bool,
    pub running: bool,
    pub node_info: Option<NodeInfo>,
    pub peers: Vec<PeerListItem>,
    pub routes: Vec<RouteListItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn get_live_state(state: &AppState, id: &str) -> AppResult<LiveState> {
    let row = get_network_row_or_throw(state, id)?;
    let running = state.pm.is_running(id);
    let cli = state.cli(row.rpc_port as u16, Duration::from_millis(5000));
    let (peers, routes, node_info) =
        tokio::join!(cli.peer_list(), cli.route_list(), cli.node_info());
    match (peers, routes, node_info) {
        (Ok(peers), Ok(routes), Ok(node_info)) => Ok(LiveState {
            online: true,
            running,
            node_info: Some(node_info),
            peers,
            routes,
            error: None,
        }),
        (p, r, n) => {
            let error = p
                .err()
                .or_else(|| r.err())
                .or_else(|| n.err())
                .map(|e| e.message)
                .unwrap_or_else(|| "未知错误".to_string());
            Ok(LiveState {
                online: false,
                running,
                node_info: None,
                peers: Vec::new(),
                routes: Vec::new(),
                error: Some(error),
            })
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficSummary {
    pub rx_bytes: f64,
    pub tx_bytes: f64,
    pub online_networks: i64,
    pub peer_count: i64,
}

pub async fn collect_traffic(state: &AppState) -> AppResult<TrafficSummary> {
    let rows = state.db.list_networks()?;
    let mut futures = Vec::new();
    for row in rows {
        if !state.pm.is_running(&row.id) {
            continue;
        }
        let cli = state.cli(row.rpc_port as u16, Duration::from_millis(4000));
        futures.push(async move { cli.peer_list().await });
    }
    let results = join_all(futures).await;

    let mut rx_bytes = 0.0;
    let mut tx_bytes = 0.0;
    let mut online_networks = 0i64;
    let mut peer_count = 0i64;
    for result in results {
        if let Ok(peers) = result {
            online_networks += 1;
            peer_count += (peers.len() as i64 - 1).max(0);
            for p in peers {
                rx_bytes += parse_human_size(Some(&p.rx_bytes));
                tx_bytes += parse_human_size(Some(&p.tx_bytes));
            }
        }
    }
    Ok(TrafficSummary {
        rx_bytes,
        tx_bytes,
        online_networks,
        peer_count,
    })
}
