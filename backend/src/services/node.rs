use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::db::NodeRow;
use crate::easytier::toml::{
    build_join_command, build_network_toml, JoinCommandParams, NetworkTomlInput,
    DEFAULT_NODE_LISTENERS,
};
use crate::error::{bad_request, not_found, AppResult};
use crate::state::AppState;
use crate::util::format::{parse_human_size, parse_int_safe};
use crate::util::now_ms;
use crate::util::x25519::{derive_key_pair, derive_public_key};

use super::credential::{
    create_credential, get_credential_row, get_credential_secret, revoke_credential,
    to_public_credential, CreateCredentialInput, PublicCredential,
};
use super::network::{
    accessible_network_ids, get_network_row, get_network_row_or_throw, require_network_access,
    to_public_network, AccessLevel, PublicNetwork,
};
use super::{safe_object, safe_string_array};

/// 节点类型：固定节点（使用 network_secret 接入）与临时节点（使用凭据接入）。
pub const NODE_TYPE_FIXED: &str = "fixed";
pub const NODE_TYPE_TEMPORARY: &str = "temporary";

/// 归一化节点类型：仅 `fixed` 视为固定节点，其余（`temporary` / 历史 `client` / 空）均视为临时节点。
pub fn normalize_node_type(value: Option<&str>) -> &'static str {
    match value.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
        Some("fixed") | Some("permanent") | Some("static") => NODE_TYPE_FIXED,
        _ => NODE_TYPE_TEMPORARY,
    }
}

/// 是否为固定节点（历史 `client` 记录视作临时节点）。
pub fn is_fixed_node(node: &NodeRow) -> bool {
    node.r#type == NODE_TYPE_FIXED
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeLiveInfo {
    pub online: bool,
    pub cost: String,
    pub lat_ms: Option<f64>,
    pub loss_rate: Option<f64>,
    pub rx_bytes: f64,
    pub tx_bytes: f64,
    pub tunnel_proto: String,
    pub nat_type: String,
    pub peer_id: String,
    pub version: String,
}

impl NodeLiveInfo {
    fn offline() -> Self {
        Self {
            online: false,
            cost: String::new(),
            lat_ms: None,
            loss_rate: None,
            rx_bytes: 0.0,
            tx_bytes: 0.0,
            tunnel_proto: String::new(),
            nat_type: String::new(),
            peer_id: String::new(),
            version: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicNode {
    pub id: String,
    pub network_id: String,
    pub name: String,
    pub hostname: String,
    pub ipv4: Option<String>,
    pub r#type: String,
    pub description: Option<String>,
    pub listeners: Vec<String>,
    pub proxy_networks: Vec<String>,
    pub flags: Map<String, Value>,
    pub credential_id: Option<String>,
    pub peer_id: Option<i64>,
    pub status: String,
    pub last_seen_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live: Option<NodeLiveInfo>,
}

pub fn to_public_node(row: &NodeRow) -> PublicNode {
    PublicNode {
        id: row.id.clone(),
        network_id: row.network_id.clone(),
        name: row.name.clone(),
        hostname: row.hostname.clone(),
        ipv4: row.ipv4.clone(),
        r#type: row.r#type.clone(),
        description: row.description.clone(),
        listeners: safe_string_array(&row.listeners),
        proxy_networks: safe_string_array(&row.proxy_networks),
        flags: safe_object(&row.flags),
        credential_id: row.credential_id.clone(),
        peer_id: row.peer_id,
        status: row.status.clone(),
        last_seen_at: row.last_seen_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
        live: None,
    }
}

pub fn get_node_row(state: &AppState, id: &str) -> AppResult<Option<NodeRow>> {
    state.db.get_node(id)
}

pub fn get_node_row_or_throw(state: &AppState, id: &str) -> AppResult<NodeRow> {
    get_node_row(state, id)?.ok_or_else(|| not_found("节点不存在"))
}

pub fn list_nodes(state: &AppState, network_id: Option<&str>) -> AppResult<Vec<PublicNode>> {
    Ok(state
        .db
        .list_nodes(network_id)?
        .iter()
        .map(to_public_node)
        .collect())
}

/// 按用户可见范围列出节点：管理员全部，其他用户仅自己可见网络下的节点。
pub fn list_nodes_scoped(
    state: &AppState,
    user: &crate::middleware::AuthUser,
    network_id: Option<&str>,
) -> AppResult<Vec<PublicNode>> {
    if let Some(nid) = network_id {
        require_network_access(state, nid, user, false)?;
        return list_nodes(state, Some(nid));
    }
    let allowed = accessible_network_ids(state, user)?;
    let all = list_nodes(state, None)?;
    Ok(match allowed {
        None => all,
        Some(set) => all
            .into_iter()
            .filter(|n| set.contains(&n.network_id))
            .collect(),
    })
}

fn suggest_ipv4(state: &AppState, network: &crate::db::NetworkRow) -> AppResult<Option<String>> {
    let base = network.ipv4.split('/').next().unwrap_or("");
    let parts: Vec<&str> = base.split('.').collect();
    if parts.len() != 4 {
        return Ok(None);
    }
    let prefix = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
    let mut used: std::collections::HashSet<i64> = std::collections::HashSet::new();
    used.insert(1);
    for ip in state.db.list_node_ipv4s(&network.id)? {
        if let Some(ip) = ip {
            let last = ip
                .split('/')
                .next()
                .unwrap_or("")
                .split('.')
                .nth(3)
                .and_then(|s| s.parse::<i64>().ok());
            if let Some(last) = last {
                used.insert(last);
            }
        }
    }
    for n in 2..255i64 {
        if !used.contains(&n) {
            return Ok(Some(format!("{prefix}.{n}")));
        }
    }
    Ok(None)
}

fn slug(input: &str) -> String {
    let ascii: String = input
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let ascii = ascii.trim_matches('-');
    let ascii: String = ascii.chars().take(32).collect();
    if ascii.len() >= 2 && ascii.chars().any(|c| c.is_ascii_lowercase()) {
        ascii
    } else {
        format!("node-{}", &Uuid::new_v4().to_string()[..6])
    }
}

/// 区分“字段缺失”与“显式 null”的双层 Option 反序列化。
pub fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(d).map(Some)
}

#[derive(Default)]
pub struct CreateNodeInput {
    pub network_id: String,
    pub name: String,
    pub hostname: Option<String>,
    pub ipv4: Option<Option<String>>,
    pub description: Option<String>,
    pub listeners: Option<Vec<String>>,
    pub proxy_networks: Option<Vec<String>>,
    pub flags: Option<Map<String, Value>>,
    /// `fixed`（固定节点，用 network_secret）或 `temporary`（临时节点，签发凭据）。
    pub node_type: Option<String>,
    pub issue_credential: Option<bool>,
    pub ttl_seconds: Option<i64>,
    pub groups: Option<Vec<String>>,
    pub allow_relay: Option<bool>,
    pub reusable: Option<bool>,
    pub allowed_proxy_cidrs: Option<Vec<String>>,
    pub created_by: Option<String>,
}

pub async fn create_node(
    state: &AppState,
    input: CreateNodeInput,
) -> AppResult<(PublicNode, Option<(PublicCredential, String)>)> {
    let network = get_network_row_or_throw(state, &input.network_id)?;
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(bad_request("节点名称不能为空", None));
    }

    let hostname = input
        .hostname
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| slug(&name));
    let ipv4 = match input.ipv4 {
        Some(None) => None,
        Some(Some(v)) => {
            let v = v.trim().to_string();
            if v.is_empty() {
                suggest_ipv4(state, &network)?
            } else {
                Some(v)
            }
        }
        None => suggest_ipv4(state, &network)?,
    };

    let now = now_ms();
    let node_id = Uuid::new_v4().to_string();
    let node_type = normalize_node_type(input.node_type.as_deref());
    // 固定节点使用持久化的静态私钥（secure mode 身份固定，重启后公钥不变）。
    // 临时节点用凭据作为身份，无需 local_private_key。
    let local_private_key_enc = if node_type == NODE_TYPE_FIXED {
        Some(state.crypto.encrypt(&derive_key_pair().private_key))
    } else {
        None
    };
    let listeners: Vec<String> = match input.listeners {
        Some(list) if !list.is_empty() => list.into_iter().filter(|s| !s.is_empty()).collect(),
        _ => DEFAULT_NODE_LISTENERS
            .iter()
            .map(|s| s.to_string())
            .collect(),
    };

    let row = NodeRow {
        id: node_id.clone(),
        network_id: network.id.clone(),
        name,
        hostname,
        ipv4,
        r#type: node_type.to_string(),
        description: input.description,
        listeners: serde_json::to_string(&listeners)?,
        proxy_networks: serde_json::to_string(&input.proxy_networks.unwrap_or_default())?,
        flags: serde_json::to_string(&input.flags.unwrap_or_default())?,
        credential_id: None,
        local_private_key_enc,
        peer_id: None,
        status: "unknown".to_string(),
        last_seen_at: None,
        created_at: now,
        updated_at: now,
    };
    state.db.insert_node(&row)?;

    // 仅临时节点签发凭据；固定节点直接用网络的 network_secret 接入。
    let mut credential = None;
    if node_type == NODE_TYPE_TEMPORARY && input.issue_credential != Some(false) {
        let result = create_credential(
            state,
            CreateCredentialInput {
                network_id: network.id.clone(),
                node_id: Some(node_id.clone()),
                ttl_seconds: input.ttl_seconds,
                groups: input.groups,
                allow_relay: input.allow_relay,
                reusable: input.reusable,
                allowed_proxy_cidrs: input.allowed_proxy_cidrs,
                credential_id: None,
                created_by: input.created_by,
            },
        )
        .await;
        match result {
            Ok((cred, secret)) => {
                state
                    .db
                    .update_node_credential(&node_id, Some(&cred.id), now_ms())?;
                credential = Some((cred, secret));
            }
            Err(err) => {
                // 凭据签发失败时回滚节点创建
                state.db.delete_node(&node_id)?;
                return Err(err);
            }
        }
    }

    Ok((
        to_public_node(&get_node_row_or_throw(state, &node_id)?),
        credential,
    ))
}

pub struct UpdateNodeInput {
    pub name: Option<String>,
    pub hostname: Option<String>,
    pub ipv4: Option<Option<String>>,
    pub description: Option<String>,
    pub listeners: Option<Vec<String>>,
    pub proxy_networks: Option<Vec<String>>,
    pub flags: Option<Map<String, Value>>,
}

pub fn update_node(state: &AppState, id: &str, patch: UpdateNodeInput) -> AppResult<PublicNode> {
    let mut row = get_node_row_or_throw(state, id)?;
    if let Some(v) = patch.name {
        row.name = v;
    }
    if let Some(v) = patch.hostname {
        row.hostname = v;
    }
    if let Some(v) = patch.ipv4 {
        row.ipv4 = v;
    }
    if let Some(v) = patch.description {
        row.description = Some(v);
    }
    if let Some(v) = patch.listeners {
        row.listeners =
            serde_json::to_string(&v.into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>())?;
    }
    if let Some(v) = patch.proxy_networks {
        row.proxy_networks = serde_json::to_string(&v)?;
    }
    if let Some(v) = patch.flags {
        row.flags = serde_json::to_string(&v)?;
    }
    row.updated_at = now_ms();
    state.db.update_node(&row)?;
    Ok(to_public_node(&get_node_row_or_throw(state, id)?))
}

pub async fn delete_node(state: &AppState, id: &str) -> AppResult<()> {
    let row = get_node_row_or_throw(state, id)?;
    if let Some(credential_id) = row.credential_id.as_deref() {
        let _ = revoke_credential(state, credential_id).await;
        state.db.delete_credential(credential_id)?;
    }
    state.db.delete_node(id)?;
    Ok(())
}

/// 提取 `scheme://host:port` 中的 (host, port)，用于判断 peer 是否指向本共享节点。
fn normalize_endpoint(uri: &str) -> Option<(String, String)> {
    let rest = uri.trim();
    let rest = match rest.split_once("://") {
        Some((_, r)) => r,
        None => rest,
    };
    let rest = rest.trim_end_matches('/');
    if let Some(rest) = rest.strip_prefix('[') {
        let (host, port) = rest.split_once("]:")?;
        return Some((host.to_ascii_lowercase(), port.to_string()));
    }
    let (host, port) = rest.rsplit_once(':')?;
    Some((host.to_ascii_lowercase(), port.to_string()))
}

/// peer 是否指向本共享节点自身。
///
/// 只有直连本共享节点时才应写入 `peer_public_key` 做公钥锁定（官方场景二）；
/// 公共中继 / 未知地址无法锁定，返回 false（UI 会提示未锁定）。
fn peer_points_to_shared_node(network: &crate::db::NetworkRow, peer: &str) -> bool {
    let peer = peer.trim();
    if let Some(target) = normalize_endpoint(peer) {
        let mut candidates = safe_string_array(&network.mapped_listeners);
        candidates.extend(safe_string_array(&network.listeners));
        for candidate in candidates {
            let Some(candidate) = normalize_endpoint(&candidate) else {
                continue;
            };
            // 通配监听地址（0.0.0.0/::）不是可达地址，不能作为锁定目标。
            if matches!(candidate.0.as_str(), "0.0.0.0" | "::" | "[::]") {
                continue;
            }
            if candidate == target {
                return true;
            }
        }
    }
    false
}

/// 解析客户端接入 peer，并返回该 peer 是否指向本共享节点自身。
fn resolve_join_peer(
    state: &AppState,
    network: &crate::db::NetworkRow,
    override_peer: Option<&str>,
) -> (String, bool) {
    if let Some(peer) = override_peer {
        let peer = peer.trim();
        if !peer.is_empty() {
            let is_self = peer_points_to_shared_node(network, peer);
            return (peer.to_string(), is_self);
        }
    }
    let mapped: Vec<String> = safe_string_array(&network.mapped_listeners)
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    if let Some(first) = mapped.first() {
        // 映射监听器即共享节点对外可达地址，可锁定共享节点公钥。
        return (first.clone(), true);
    }
    if let Some(external) = network.external_node.as_deref() {
        if !external.trim().is_empty() {
            let external = external.trim().to_string();
            let is_self = peer_points_to_shared_node(network, &external);
            return (external, is_self);
        }
    }
    let peers: Vec<String> = safe_string_array(&network.peers)
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    if let Some(first) = peers.first() {
        let is_self = peer_points_to_shared_node(network, first);
        return (first.clone(), is_self);
    }
    (state.easytier_settings().default_external_node, false)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinCredentialInfo {
    pub id: String,
    pub credential_id: String,
    pub secret: String,
    pub expires_at: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeJoinInfo {
    pub node: PublicNode,
    pub network: PublicNetwork,
    /// `fixed` 或 `temporary`。
    pub node_type: String,
    pub peer: String,
    pub listen_port: i64,
    pub listeners: Vec<String>,
    pub command: String,
    pub config_toml: String,
    pub shared_node_public_key: String,
    /// 已写入客户端配置的 `[[peer]] peer_public_key`；空字符串表示未锁定。
    pub peer_public_key: String,
    /// 固定节点接入用的 network_secret（KDF 派生值）；临时节点为空字符串。
    pub network_secret: String,
    /// 固定节点的本地静态公钥（由 local_private_key 派生）；临时节点为空字符串。
    pub local_public_key: String,
    /// 临时节点的凭据信息；固定节点为 null。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<JoinCredentialInfo>,
}

pub async fn get_node_join(
    state: &AppState,
    id: &str,
    listen_port: Option<i64>,
    peer_override: Option<&str>,
) -> AppResult<NodeJoinInfo> {
    let mut node = get_node_row_or_throw(state, id)?;
    let network = get_network_row_or_throw(state, &node.network_id)?;
    let fixed = is_fixed_node(&node);
    let node_type = if fixed {
        NODE_TYPE_FIXED
    } else {
        NODE_TYPE_TEMPORARY
    };

    // 固定节点：取出（或在缺失时生成并持久化）本节点的静态私钥，供 [secure_mode] 使用。
    // 这样节点重启后公钥不变，符合“固定节点”语义。
    let local_private_key = if fixed {
        let existing = state
            .crypto
            .try_decrypt(node.local_private_key_enc.as_deref());
        if existing.is_empty() {
            let key = derive_key_pair().private_key;
            node.local_private_key_enc = Some(state.crypto.encrypt(&key));
            node.updated_at = now_ms();
            state.db.update_node(&node)?;
            key
        } else {
            existing
        }
    } else {
        String::new()
    };

    // 临时节点：校验凭据有效性；固定节点：取网络派生密钥（用户明文从未落库）。
    let (credential_info, join_secret) = if fixed {
        let derived = state.crypto.try_decrypt(Some(&network.network_secret_enc));
        if derived.is_empty() {
            return Err(bad_request(
                "该网络尚未设置网络密钥，请先在网络设置中配置 network_secret",
                None,
            ));
        }
        (None, derived)
    } else {
        let Some(credential_id) = node.credential_id.as_deref() else {
            return Err(bad_request("该节点尚未签发凭据，请先重新生成凭据", None));
        };
        let Some(cred_row) = get_credential_row(state, credential_id)? else {
            return Err(bad_request("凭据记录不存在，请重新生成凭据", None));
        };
        let cred = to_public_credential(&cred_row);
        if cred.status != "active" {
            let reason = if cred.status == "revoked" {
                "撤销"
            } else {
                "过期"
            };
            return Err(bad_request(format!("凭据已{reason}，请重新生成"), None));
        }
        let secret = get_credential_secret(state, &cred_row.id)?;
        (
            Some(JoinCredentialInfo {
                id: cred.id,
                credential_id: cred.credential_id,
                secret: secret.clone(),
                expires_at: cred.expires_at,
                status: cred.status,
            }),
            secret,
        )
    };

    let (peer, peer_is_shared_node) = resolve_join_peer(state, &network, peer_override);
    let public_network = to_public_network(state, &network, AccessLevel::View);

    // 官方场景二：经共享节点接入时锁定其公钥，防止中间人劫持。
    // 仅当 peer 直连本共享节点且网络开启安全模式时才写入 peer_public_key。
    let shared_node_public_key = public_network.shared_node_public_key.clone();
    let peer_public_key =
        if peer_is_shared_node && network.secure_mode && !shared_node_public_key.is_empty() {
            shared_node_public_key.clone()
        } else {
            String::new()
        };

    let stored_listeners = safe_string_array(&node.listeners);
    let listeners = match listen_port {
        Some(port) => vec![
            format!("tcp://0.0.0.0:{port}"),
            format!("udp://0.0.0.0:{port}"),
        ],
        None => {
            if stored_listeners.is_empty() {
                DEFAULT_NODE_LISTENERS
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            } else {
                stored_listeners
            }
        }
    };
    let effective_listen_port = listen_port.unwrap_or(11010);

    let config_toml = build_network_toml(&NetworkTomlInput {
        instance_name: format!("node-{}", node.hostname).chars().take(40).collect(),
        hostname: node.hostname.clone(),
        network_name: network.network_name.clone(),
        // 固定节点在配置中写入网络密钥；临时节点只用命令行 --credential。
        network_secret: if fixed {
            Some(join_secret.clone())
        } else {
            None
        },
        ipv4: node.ipv4.clone(),
        dhcp: node.ipv4.is_none(),
        listeners: listeners.clone(),
        mapped_listeners: Vec::new(),
        exit_nodes: Vec::new(),
        peers: vec![peer.clone()],
        proxy_networks: safe_string_array(&node.proxy_networks),
        credential_file: None,
        // 官方场景三：临时凭据节点也处于安全模式（凭据经命令行 --credential 传入）。
        secure_mode: network.secure_mode,
        // 固定节点写入持久化的专属私钥；临时节点没有。
        local_private_key: if fixed && !local_private_key.is_empty() {
            Some(local_private_key.clone())
        } else {
            None
        },
        // 由 build_network_toml 根据私钥自动派生并写入 local_public_key（兼容 EasyTier 2.6.x）。
        local_public_key: None,
        peer_public_key: if peer_public_key.is_empty() {
            None
        } else {
            Some(peer_public_key.clone())
        },
        flags: safe_object(&node.flags),
    });

    let command = build_join_command(&JoinCommandParams {
        network_name: &network.network_name,
        credential_secret: if fixed {
            None
        } else {
            Some(join_secret.as_str())
        },
        network_secret: if fixed {
            Some(join_secret.as_str())
        } else {
            None
        },
        local_private_key: if fixed && !local_private_key.is_empty() {
            Some(local_private_key.as_str())
        } else {
            None
        },
        peer: &peer,
        ipv4: node.ipv4.as_deref(),
        hostname: Some(&node.hostname),
        // 展示给用户的一键命令用相对路径，便于在目标设备上直接执行；
        // 控制台自身启动 easytier-core 仍使用设置里的真实路径。
        core_path: "./easytier-core",
        listeners: listeners.clone(),
    });

    Ok(NodeJoinInfo {
        node: to_public_node(&node),
        shared_node_public_key: public_network.shared_node_public_key.clone(),
        peer_public_key,
        network: public_network,
        node_type: node_type.to_string(),
        peer,
        listen_port: effective_listen_port,
        listeners,
        command,
        config_toml,
        network_secret: if fixed {
            join_secret.clone()
        } else {
            String::new()
        },
        local_public_key: if fixed && !local_private_key.is_empty() {
            derive_public_key(&local_private_key)
        } else {
            String::new()
        },
        credential: credential_info,
    })
}

#[derive(Default)]
pub struct RotateCredentialInput {
    pub ttl_seconds: Option<i64>,
    pub allow_relay: Option<bool>,
    pub reusable: Option<bool>,
    pub groups: Option<Vec<String>>,
    pub allowed_proxy_cidrs: Option<Vec<String>>,
}

pub async fn rotate_node_credential(
    state: &AppState,
    id: &str,
    input: RotateCredentialInput,
) -> AppResult<(PublicNode, PublicCredential, String)> {
    let node = get_node_row_or_throw(state, id)?;
    if is_fixed_node(&node) {
        return Err(bad_request("固定节点使用网络密钥接入，无需轮换凭据", None));
    }
    if let Some(credential_id) = node.credential_id.as_deref() {
        let _ = revoke_credential(state, credential_id).await;
        state.db.delete_credential(credential_id)?;
    }
    let (credential, secret) = create_credential(
        state,
        CreateCredentialInput {
            network_id: node.network_id.clone(),
            node_id: Some(node.id.clone()),
            ttl_seconds: input.ttl_seconds,
            groups: input.groups,
            allow_relay: input.allow_relay,
            reusable: input.reusable,
            allowed_proxy_cidrs: input.allowed_proxy_cidrs,
            credential_id: None,
            created_by: None,
        },
    )
    .await?;
    state
        .db
        .update_node_credential(id, Some(&credential.id), now_ms())?;
    Ok((
        to_public_node(&get_node_row_or_throw(state, id)?),
        credential,
        secret,
    ))
}

pub async fn enrich_nodes_with_live(
    state: &AppState,
    nodes: Vec<PublicNode>,
) -> AppResult<Vec<PublicNode>> {
    let mut by_network: HashMap<String, Vec<PublicNode>> = HashMap::new();
    for node in nodes {
        by_network
            .entry(node.network_id.clone())
            .or_default()
            .push(node);
    }

    let mut results: Vec<PublicNode> = Vec::new();

    for (network_id, group) in by_network.into_iter() {
        let network = get_network_row(state, &network_id)?;
        let running = state.pm.is_running(&network_id);
        let Some(network) = network else {
            results.extend(group.into_iter().map(|mut n| {
                n.live = Some(NodeLiveInfo::offline());
                n
            }));
            continue;
        };
        if !running {
            results.extend(group.into_iter().map(|mut n| {
                n.live = Some(NodeLiveInfo::offline());
                n
            }));
            continue;
        }

        let cli = state.cli(network.rpc_port as u16, Duration::from_millis(4000));
        match cli.peer_list().await {
            Ok(peers) => {
                for mut node in group {
                    let norm_ip = node
                        .ipv4
                        .as_deref()
                        .map(|s| s.split('/').next().unwrap_or("").to_string())
                        .unwrap_or_default();
                    let matched = peers.iter().find(|p| {
                        p.cost != "Local"
                            && ((!norm_ip.is_empty() && p.ipv4 == norm_ip)
                                || p.hostname == node.hostname)
                    });
                    if let Some(peer) = matched {
                        let now = now_ms();
                        node.status = "online".to_string();
                        node.last_seen_at = Some(now);
                        let lat = if peer.lat_ms == "-" {
                            None
                        } else {
                            peer.lat_ms.parse::<f64>().ok().filter(|v| v.is_finite())
                        };
                        let loss = if peer.loss_rate == "-" {
                            None
                        } else {
                            peer.loss_rate
                                .parse::<f64>()
                                .ok()
                                .filter(|v| v.is_finite())
                                .map(|v| v / 100.0)
                        };
                        node.live = Some(NodeLiveInfo {
                            online: true,
                            cost: peer.cost.clone(),
                            lat_ms: lat,
                            loss_rate: loss,
                            rx_bytes: parse_human_size(Some(&peer.rx_bytes)),
                            tx_bytes: parse_human_size(Some(&peer.tx_bytes)),
                            tunnel_proto: peer.tunnel_proto.clone(),
                            nat_type: peer.nat_type.clone(),
                            peer_id: peer.id.clone(),
                            version: peer.version.clone(),
                        });
                        let peer_id = parse_int_safe(Some(&peer.id));
                        state.db.set_node_runtime(
                            &node.id,
                            "online",
                            Some(now),
                            if peer_id != 0 { Some(peer_id) } else { None },
                        )?;
                    } else {
                        node.status = "offline".to_string();
                        node.live = Some(NodeLiveInfo::offline());
                        state.db.set_node_runtime(&node.id, "offline", None, None)?;
                    }
                    results.push(node);
                }
            }
            Err(_) => {
                results.extend(group.into_iter().map(|mut n| {
                    n.live = Some(NodeLiveInfo::offline());
                    n
                }));
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NetworkRow;

    fn network(listeners: &str, mapped: &str) -> NetworkRow {
        NetworkRow {
            id: "n1".to_string(),
            name: "n".to_string(),
            description: None,
            network_name: "office".to_string(),
            network_secret_enc: String::new(),
            ipv4: "10.0.0.1/24".to_string(),
            dhcp: false,
            hostname: "h".to_string(),
            instance_name: "i".to_string(),
            listeners: listeners.to_string(),
            mapped_listeners: mapped.to_string(),
            peers: "[]".to_string(),
            external_node: None,
            listen_port: 11010,
            rpc_port: 15888,
            secure_mode: true,
            local_private_key_enc: None,
            credential_file: None,
            flags: "{}".to_string(),
            auto_start: false,
            status: "stopped".to_string(),
            last_error: None,
            owner_id: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn normalizes_endpoints() {
        assert_eq!(
            normalize_endpoint("tcp://Host.EXAMPLE.com:11010/"),
            Some(("host.example.com".to_string(), "11010".to_string()))
        );
        assert_eq!(
            normalize_endpoint("tcp://[::1]:11010"),
            Some(("::1".to_string(), "11010".to_string()))
        );
        assert_eq!(normalize_endpoint("11010"), None);
    }

    /// 场景二：只有直连本共享节点（映射监听器 / 具体监听地址）才允许公钥锁定。
    #[test]
    fn detects_direct_shared_node_peer() {
        let n = network(
            "[\"tcp://0.0.0.0:11010\"]",
            "[\"tcp://relay.example.com:11011\"]",
        );
        assert!(peer_points_to_shared_node(
            &n,
            "tcp://relay.example.com:11011"
        ));
        assert!(!peer_points_to_shared_node(
            &n,
            "tcp://public.easytier.cn:11010"
        ));
        // 通配监听地址不是可达地址，不能作为锁定目标。
        assert!(!peer_points_to_shared_node(&n, "tcp://0.0.0.0:11010"));
    }

    #[test]
    fn normalizes_node_type() {
        assert_eq!(normalize_node_type(Some("fixed")), NODE_TYPE_FIXED);
        assert_eq!(normalize_node_type(Some(" FIXED ")), NODE_TYPE_FIXED);
        assert_eq!(normalize_node_type(Some("temporary")), NODE_TYPE_TEMPORARY);
        // 历史 "client" 记录视作临时节点。
        assert_eq!(normalize_node_type(Some("client")), NODE_TYPE_TEMPORARY);
        assert_eq!(normalize_node_type(None), NODE_TYPE_TEMPORARY);
    }

    #[test]
    fn detects_fixed_node() {
        let mut n = NodeRow {
            id: "node-1".to_string(),
            network_id: "n1".to_string(),
            name: "n".to_string(),
            hostname: "h".to_string(),
            ipv4: None,
            r#type: "client".to_string(),
            description: None,
            listeners: "[]".to_string(),
            proxy_networks: "[]".to_string(),
            flags: "{}".to_string(),
            credential_id: None,
            local_private_key_enc: None,
            peer_id: None,
            status: "unknown".to_string(),
            last_seen_at: None,
            created_at: 0,
            updated_at: 0,
        };
        assert!(!is_fixed_node(&n));
        n.r#type = "fixed".to_string();
        assert!(is_fixed_node(&n));
    }
}
