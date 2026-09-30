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
    ]
}

/// 合并默认 flags 与用户覆盖项，得到完整的 flags 对象。
pub fn merge_flags(overrides: &Map<String, Value>) -> Map<String, Value> {
    let mut merged = Map::new();
    for (key, value) in default_flags() {
        merged.insert(key.to_string(), value);
    }
    for (key, value) in overrides {
        merged.insert(key.clone(), value.clone());
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
    pub peers: Vec<String>,
    pub proxy_networks: Vec<String>,
    pub credential_file: Option<String>,
    pub local_private_key: Option<String>,
    pub local_public_key: Option<String>,
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

    if let Some(private_key) = input.local_private_key.as_deref() {
        if !private_key.is_empty() {
            lines.push(String::new());
            lines.push("[secure_mode]".to_string());
            lines.push("enabled = true".to_string());
            lines.push(format!("local_private_key = {}", toml_string(private_key)));
            let public_key = match input.local_public_key.as_deref() {
                Some(k) if !k.is_empty() => k.to_string(),
                _ => derive_public_key(private_key),
            };
            if !public_key.is_empty() {
                lines.push(format!("local_public_key = {}", toml_string(&public_key)));
            }
        }
    }

    for peer in input.peers.iter() {
        if peer.trim().is_empty() {
            continue;
        }
        lines.push(String::new());
        lines.push("[[peer]]".to_string());
        lines.push(format!("uri = {}", toml_string(peer.trim())));
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
    pub credential_secret: &'a str,
    pub peer: &'a str,
    pub ipv4: Option<&'a str>,
    pub hostname: Option<&'a str>,
    pub core_path: &'a str,
    pub listeners: Vec<String>,
}

/// 生成客户端节点加入命令（用于凭据接入）
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
    parts.push(format!(
        "--credential {}",
        shell_quote(params.credential_secret)
    ));
    for listener in listeners {
        parts.push(format!("-l {}", shell_quote(&listener)));
    }
    parts.push(format!("-p {}", shell_quote(params.peer)));
    parts.join(" ")
}
