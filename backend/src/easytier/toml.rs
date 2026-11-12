//! EasyTier 配置文件（TOML）生成与加入命令构造。

use serde_json::{json, Map, Value};

use crate::util::x25519::derive_public_key;

/// 客户端节点默认监听器
pub const DEFAULT_NODE_LISTENERS: [&str; 2] = ["tcp://0.0.0.0:11010", "udp://0.0.0.0:11010"];

/// EasyTier 默认 flags（保持固定顺序）
pub fn default_flags() -> Vec<(&'static str, Value)> {
    vec![
        ("default_protocol", json!("tcp")),
        ("dev_name", json!("")),
        ("enable_encryption", json!(true)),
        ("enable_ipv6", json!(true)),
        ("mtu", json!(1380)),
        ("latency_first", json!(false)),
        ("enable_exit_node", json!(false)),
        ("no_tun", json!(false)),
        ("use_smoltcp", json!(false)),
        ("relay_network_whitelist", json!("*")),
        ("disable_p2p", json!(false)),
        ("relay_all_peer_rpc", json!(false)),
        ("disable_udp_hole_punching", json!(false)),
        ("disable_tcp_hole_punching", json!(false)),
        ("private_mode", json!(false)),
        // 客户端节点高级设置（对照官方配置助手 FlagsInConfig 的 localName/默认值）。
        ("bind_device", json!(false)),
        ("encryption_algorithm", json!("")),
        ("multi_thread", json!(false)),
        ("multi_thread_count", json!(0)),
        ("p2p_only", json!(false)),
        ("lazy_p2p", json!(false)),
        ("need_p2p", json!(true)),
        ("disable_sym_hole_punching", json!(false)),
        ("disable_upnp", json!(false)),
        ("enable_kcp_proxy", json!(false)),
        ("disable_kcp_input", json!(false)),
        ("disable_relay_kcp", json!(false)),
        ("enable_quic_proxy", json!(false)),
        ("disable_quic_input", json!(false)),
        ("disable_relay_quic", json!(false)),
        ("quic_listen_port", json!(0)),
        ("proxy_forward_by_system", json!(false)),
        ("accept_dns", json!(false)),
        ("tld_dns_zone", json!("")),
        ("enable_udp_broadcast_relay", json!(false)),
        ("disable_relay_data", json!(false)),
        ("foreign_relay_bps_limit", json!(0)),
        ("instance_recv_bps_limit", json!(0)),
    ]
}

/// flags 键名白名单：仅允许 EasyTier 配置使用的标识符，
/// 防止通过换行 / `[section]` 等字符注入额外 TOML 配置。
pub fn is_valid_flag_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// 校验用户提交的 flags；返回可读的错误信息。
pub fn validate_flags(flags: &Map<String, Value>) -> Result<(), String> {
    for (key, value) in flags {
        if !is_valid_flag_key(key) {
            return Err(format!("无效的 flags 键名: {key}"));
        }
        match value {
            Value::Bool(_) | Value::Number(_) => {}
            Value::String(s) if s.chars().count() <= 1024 => {}
            Value::String(_) => return Err(format!("flags.{key} 字符串过长")),
            _ => return Err(format!("flags.{key} 仅支持字符串、数字或布尔值")),
        }
    }
    Ok(())
}

/// 合并默认 flags 与用户覆盖项，得到完整的 flags 对象。
/// 非法键名会被丢弃（API 层另有校验），避免污染 TOML。
pub fn merge_flags(overrides: &Map<String, Value>) -> Map<String, Value> {
    let mut merged = Map::new();
    for (key, value) in default_flags() {
        merged.insert(key.to_string(), value);
    }
    for (key, value) in overrides {
        if is_valid_flag_key(key) {
            merged.insert(key.clone(), value.clone());
        }
    }
    merged
}

fn toml_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn toml_array(values: &[String]) -> String {
    let items: Vec<String> = values.iter().map(|v| toml_string(v)).collect();
    format!("[{}]", items.join(", "))
}

fn toml_value(value: &Value) -> String {
    match value {
        Value::String(s) => toml_string(s),
        Value::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Null => toml_string(""),
        other => toml_string(&other.to_string()),
    }
}

pub fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_./:@%+=,-".contains(c))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub struct NetworkTomlInput {
    pub instance_name: String,
    pub hostname: String,
    pub network_name: String,
    pub network_secret: Option<String>,
    pub ipv4: Option<String>,
    pub dhcp: bool,
    pub listeners: Vec<String>,
    pub mapped_listeners: Vec<String>,
    pub exit_nodes: Vec<String>,
    /// 初始 Peer 列表；第一个 Peer 可附带 `peer_public_key` 公钥锁定。
    pub peers: Vec<String>,
    pub proxy_networks: Vec<String>,
    pub credential_file: Option<String>,
    /// 安全模式开关。开启后输出 `[secure_mode] enabled = true`。
    pub secure_mode: bool,
    /// 本地固定私钥（base64）。共享节点应显式指定，避免重启后公钥变化。
    pub local_private_key: Option<String>,
    /// 本地固定公钥（base64）。通常留空由私钥自动派生并写入（兼容 EasyTier 2.6.x）。
    pub local_public_key: Option<String>,
    /// 锁定第一个 Peer 的公钥（base64），对应官方 `[[peer]] peer_public_key`。
    pub peer_public_key: Option<String>,
    pub flags: Map<String, Value>,
}

/// 生成 EasyTier 配置文件内容（最小化：省略空值与默认值）
pub fn build_network_toml(input: &NetworkTomlInput) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push(format!(
        "instance_name = {}",
        toml_string(&input.instance_name)
    ));
    lines.push(format!("hostname = {}", toml_string(&input.hostname)));
    if input.dhcp {
        lines.push("dhcp = true".to_string());
    } else if let Some(ipv4) = input.ipv4.as_deref() {
        if !ipv4.is_empty() {
            lines.push(format!("ipv4 = {}", toml_string(ipv4)));
        }
    }

    let listeners: Vec<&String> = input.listeners.iter().filter(|s| !s.is_empty()).collect();
    if !listeners.is_empty() {
        lines.push(format!(
            "listeners = {}",
            toml_array(&listeners.iter().map(|s| (*s).clone()).collect::<Vec<_>>())
        ));
    }
    let mapped: Vec<&String> = input
        .mapped_listeners
        .iter()
        .filter(|s| !s.is_empty())
        .collect();
    if !mapped.is_empty() {
        lines.push(format!(
            "mapped_listeners = {}",
            toml_array(&mapped.iter().map(|s| (*s).clone()).collect::<Vec<_>>())
        ));
    }
    let exit_nodes: Vec<&String> = input.exit_nodes.iter().filter(|s| !s.is_empty()).collect();
    if !exit_nodes.is_empty() {
        lines.push(format!(
            "exit_nodes = {}",
            toml_array(&exit_nodes.iter().map(|s| (*s).clone()).collect::<Vec<_>>())
        ));
    }
    if let Some(credential_file) = input.credential_file.as_deref() {
        if !credential_file.is_empty() {
            lines.push(format!(
                "credential_file = {}",
                toml_string(credential_file)
            ));
        }
    }

    for cidr in input.proxy_networks.iter() {
        if cidr.trim().is_empty() {
            continue;
        }
        let (real, mapped_cidr) = match cidr.split_once("->") {
            Some((a, b)) => (a.trim().to_string(), Some(b.trim().to_string())),
            None => (cidr.trim().to_string(), None),
        };
        lines.push(String::new());
        lines.push("[[proxy_network]]".to_string());
        lines.push(format!("cidr = {}", toml_string(&real)));
        if let Some(mapped_cidr) = mapped_cidr {
            lines.push(format!("mapped_cidr = {}", toml_string(&mapped_cidr)));
        }
    }

    lines.push(String::new());
    lines.push("[network_identity]".to_string());
    lines.push(format!(
        "network_name = {}",
        toml_string(&input.network_name)
    ));
    if let Some(secret) = input.network_secret.as_deref() {
        if !secret.is_empty() {
            lines.push(format!("network_secret = {}", toml_string(secret)));
        }
    }

    // 安全模式：`enabled = true` 与密钥对互相独立。
    // 共享节点 / 固定节点需要固定 `local_private_key`（否则重启后公钥变化导致客户端锁定失效）；
    // 临时凭据节点只需 `enabled = true`，凭据本身通过命令行 `--credential` 传入。
    //
    // 重要：必须同时写出 `local_public_key`（由私钥派生）。EasyTier 2.6.x 的
    // `SecureModeConfig::public_key()` 在 `local_public_key` 缺失时会报
    // "local public key is not set"（由私钥自动派生的 `normalize_secure_mode_config`
    // 是后续版本才加入的）。显式写出对新旧版本都兼容。
    if input.secure_mode {
        lines.push(String::new());
        lines.push("[secure_mode]".to_string());
        lines.push("enabled = true".to_string());
        if let Some(private_key) = input.local_private_key.as_deref() {
            if !private_key.is_empty() {
                lines.push(format!("local_private_key = {}", toml_string(private_key)));
                let public_key = input
                    .local_public_key
                    .as_deref()
                    .filter(|k| !k.is_empty())
                    .map(|k| k.to_string())
                    .unwrap_or_else(|| derive_public_key(private_key));
                if !public_key.is_empty() {
                    lines.push(format!("local_public_key = {}", toml_string(&public_key)));
                }
            }
        }
    }

    let peers: Vec<String> = input
        .peers
        .iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    for (index, peer) in peers.iter().enumerate() {
        lines.push(String::new());
        lines.push("[[peer]]".to_string());
        lines.push(format!("uri = {}", toml_string(peer)));
        if index == 0 {
            if let Some(public_key) = input.peer_public_key.as_deref() {
                if !public_key.is_empty() {
                    lines.push(format!("peer_public_key = {}", toml_string(public_key)));
                }
            }
        }
    }

    // flags：仅输出与默认值不同的项
    let defaults = default_flags();
    let mut overrides: Vec<(String, &Value)> = Vec::new();
    for (key, default_value) in defaults.iter() {
        if let Some(value) = input.flags.get(*key) {
            if value != default_value {
                overrides.push(((*key).to_string(), value));
            }
        }
    }
    for (key, value) in input.flags.iter() {
        if !defaults.iter().any(|(k, _)| k == key) {
            overrides.push((key.clone(), value));
        }
    }
    if !overrides.is_empty() {
        lines.push(String::new());
        lines.push("[flags]".to_string());
        for (key, value) in overrides {
            lines.push(format!("{key} = {}", toml_value(value)));
        }
    }

    lines.push(String::new());
    lines.join("\n")
}

pub struct JoinCommandParams<'a> {
    pub network_name: &'a str,
    /// 临时节点：凭据密钥（`--credential`）。
    pub credential_secret: Option<&'a str>,
    /// 固定节点：网络密钥（`--network-secret`）。
    pub network_secret: Option<&'a str>,
    /// 固定节点：本节点持久化静态私钥（`--local-private-key`）。
    pub local_private_key: Option<&'a str>,
    pub peer: &'a str,
    pub ipv4: Option<&'a str>,
    pub hostname: Option<&'a str>,
    pub core_path: &'a str,
    pub listeners: Vec<String>,
}

/// 生成客户端节点加入命令。
///
/// - 固定节点：`--network-secret <派生密钥> --secure-mode`（无 `--credential`）
/// - 临时节点：`--credential <凭据密钥> --secure-mode`（无 `network_secret`）
pub fn build_join_command(params: &JoinCommandParams<'_>) -> String {
    let listeners: Vec<String> = if !params.listeners.is_empty() {
        params
            .listeners
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect()
    } else {
        DEFAULT_NODE_LISTENERS
            .iter()
            .map(|s| s.to_string())
            .collect()
    };

    let mut parts: Vec<String> = vec![params.core_path.to_string()];
    if let Some(hostname) = params.hostname {
        parts.push(format!("--hostname {}", shell_quote(hostname)));
    }
    match params.ipv4 {
        Some(ipv4) if !ipv4.is_empty() => parts.push(format!("--ipv4 {}", shell_quote(ipv4))),
        _ => parts.push("-d".to_string()),
    }
    parts.push(format!(
        "--network-name {}",
        shell_quote(params.network_name)
    ));
    parts.push("--secure-mode".to_string());
    if let Some(secret) = params.credential_secret {
        parts.push(format!("--credential {}", shell_quote(secret)));
    }
    if let Some(secret) = params.network_secret {
        parts.push(format!("--network-secret {}", shell_quote(secret)));
    }
    if let Some(key) = params.local_private_key {
        parts.push(format!("--local-private-key {}", shell_quote(key)));
    }
    for listener in listeners {
        parts.push(format!("-l {}", shell_quote(&listener)));
    }
    parts.push(format!("-p {}", shell_quote(params.peer)));
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> NetworkTomlInput {
        NetworkTomlInput {
            instance_name: "net-abc".to_string(),
            hostname: "host".to_string(),
            network_name: "office".to_string(),
            network_secret: Some("secret".to_string()),
            ipv4: Some("10.126.126.1/24".to_string()),
            dhcp: false,
            listeners: vec!["tcp://0.0.0.0:11010".to_string()],
            mapped_listeners: vec![],
            exit_nodes: vec![],
            peers: vec![],
            proxy_networks: vec![],
            credential_file: Some("/tmp/credentials.json".to_string()),
            secure_mode: true,
            local_private_key: None,
            local_public_key: None,
            peer_public_key: None,
            flags: Map::new(),
        }
    }

    /// 场景二（共享节点/固定节点）：写入私钥，并**同时**写入由私钥派生的公钥。
    /// EasyTier 2.6.x 在 local_public_key 缺失时会报 "local public key is not set"。
    #[test]
    fn secure_mode_writes_private_key_and_derived_public_key() {
        let mut input = base_input();
        input.local_private_key = Some("4n+5ZbbbEvFyWIahq2y7/ZNot0gTUCHVyyVZPQ0ASh4=".to_string());
        let toml = build_network_toml(&input);
        assert!(toml.contains("[secure_mode]"));
        assert!(toml.contains("enabled = true"));
        assert!(
            toml.contains("local_private_key = \"4n+5ZbbbEvFyWIahq2y7/ZNot0gTUCHVyyVZPQ0ASh4=\"")
        );
        // 公钥必须存在且与私钥匹配。
        assert!(
            toml.contains("local_public_key = \"UfdSzl893uZIEgWHpN+pOiVEVmYoPnF76UNVeHQIvmA=\"")
        );
    }

    /// 场景三（客户端侧）：凭据节点只有 enabled = true，无 network_secret / 私钥。
    #[test]
    fn credential_client_enables_secure_mode_without_secret() {
        let mut input = base_input();
        input.network_secret = None;
        input.peers = vec!["tcp://relay.example.com:11010".to_string()];
        input.peer_public_key = Some("PUB".to_string());
        let toml = build_network_toml(&input);
        assert!(toml.contains("[secure_mode]"));
        assert!(toml.contains("enabled = true"));
        assert!(!toml.contains("local_private_key"));
        assert!(!toml.contains("network_secret"));
        assert!(toml.contains("[[peer]]"));
        assert!(toml.contains("peer_public_key = \"PUB\""));
    }

    #[test]
    fn peer_public_key_only_applies_to_first_peer() {
        let mut input = base_input();
        input.peers = vec!["tcp://a:1".to_string(), "tcp://b:2".to_string()];
        input.peer_public_key = Some("PUB".to_string());
        let toml = build_network_toml(&input);
        assert_eq!(toml.matches("peer_public_key").count(), 1);
    }

    fn join_params<'a>(cred: Option<&'a str>, secret: Option<&'a str>) -> JoinCommandParams<'a> {
        JoinCommandParams {
            network_name: "office",
            credential_secret: cred,
            network_secret: secret,
            local_private_key: None,
            peer: "tcp://relay.example.com:11010",
            ipv4: Some("10.126.126.2"),
            hostname: Some("node-1"),
            core_path: "easytier-core",
            listeners: vec!["tcp://0.0.0.0:11010".to_string()],
        }
    }

    /// 固定节点：使用 --network-secret，且不带 --credential。
    #[test]
    fn fixed_node_command_uses_network_secret() {
        let cmd = build_join_command(&join_params(None, Some("DERIVED")));
        assert!(cmd.contains("--network-secret DERIVED"));
        assert!(cmd.contains("--secure-mode"));
        assert!(!cmd.contains("--credential"));
    }

    /// 固定节点：一键命令携带本节点持久化私钥。
    #[test]
    fn fixed_node_command_includes_local_private_key() {
        let mut params = join_params(None, Some("DERIVED"));
        params.local_private_key = Some("NODEKEY");
        let cmd = build_join_command(&params);
        assert!(cmd.contains("--local-private-key NODEKEY"));
    }

    /// 临时节点：使用 --credential，且不带 --network-secret。
    #[test]
    fn temporary_node_command_uses_credential() {
        let cmd = build_join_command(&join_params(Some("CRED"), None));
        assert!(cmd.contains("--credential CRED"));
        assert!(cmd.contains("--secure-mode"));
        assert!(!cmd.contains("--network-secret"));
    }

    /// 固定节点配置：network_secret 写入 [network_identity]。
    #[test]
    fn fixed_node_config_contains_network_secret() {
        let mut input = base_input();
        input.network_secret = Some("DERIVED".to_string());
        let toml = build_network_toml(&input);
        assert!(toml.contains("[network_identity]"));
        assert!(toml.contains("network_secret = \"DERIVED\""));
        assert!(toml.contains("[secure_mode]"));
    }

    /// 客户端节点高级 flags：与默认值相同的项不输出，覆盖项写入 [flags]。
    #[test]
    fn client_node_advanced_flags_emit_only_overrides() {
        let mut input = base_input();
        let mut flags = Map::new();
        flags.insert("multi_thread".to_string(), json!(true));
        flags.insert("accept_dns".to_string(), json!(true));
        flags.insert("p2p_only".to_string(), json!(false)); // 默认值 -> 不输出
        flags.insert("need_p2p".to_string(), json!(false)); // 默认 true，显式关闭 -> 输出
        flags.insert("relay_network_whitelist".to_string(), json!("office"));
        flags.insert("foreign_relay_bps_limit".to_string(), json!(0)); // 默认值 -> 不输出
        input.flags = flags;
        let toml = build_network_toml(&input);
        assert!(toml.contains("[flags]"));
        assert!(toml.contains("multi_thread = true"));
        assert!(toml.contains("accept_dns = true"));
        assert!(toml.contains("relay_network_whitelist = \"office\""));
        assert!(toml.contains("need_p2p = false"));
        assert!(!toml.contains("p2p_only"));
        assert!(!toml.contains("foreign_relay_bps_limit"));
    }

    #[test]
    fn secure_mode_off_omits_block_and_key() {
        let mut input = base_input();
        input.secure_mode = false;
        input.local_private_key = Some("PRIV".to_string());
        let toml = build_network_toml(&input);
        assert!(!toml.contains("[secure_mode]"));
        assert!(!toml.contains("local_private_key"));
    }
}
