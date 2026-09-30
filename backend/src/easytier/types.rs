//! easytier-cli -o json 输出类型定义。
//! 为兼容 easytier 各版本字段类型差异（数字/字符串/null），统一使用宽松反序列化。

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

pub fn de_string_flex<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<Value>::deserialize(d)?;
    Ok(v.as_ref().map(value_to_string).unwrap_or_default())
}

pub fn de_i64_flex<'de, D>(d: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<Value>::deserialize(d)?;
    Ok(match v {
        None | Some(Value::Null) => 0,
        Some(Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_f64().map(|f| f as i64))
            .unwrap_or(0),
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0),
        Some(Value::Bool(b)) => i64::from(b),
        _ => 0,
    })
}

pub fn de_f64_flex<'de, D>(d: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<Value>::deserialize(d)?;
    Ok(match v {
        None | Some(Value::Null) => 0.0,
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0.0),
        Some(Value::Bool(b)) => {
            if b {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    })
}

/// `easytier-cli peer list -o json` 数组元素
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeerListItem {
    #[serde(default, deserialize_with = "de_string_flex")]
    pub cidr: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub ipv4: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub hostname: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub cost: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub lat_ms: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub loss_rate: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub rx_bytes: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub tx_bytes: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub tunnel_proto: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub nat_type: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub version: String,
}

/// `easytier-cli route list -o json` 数组元素
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteListItem {
    #[serde(default, deserialize_with = "de_string_flex")]
    pub ipv4: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub hostname: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub proxy_cidrs: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub next_hop_ipv4: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub next_hop_hostname: String,
    #[serde(default, deserialize_with = "de_f64_flex")]
    pub next_hop_lat: f64,
    #[serde(default, deserialize_with = "de_i64_flex")]
    pub path_len: i64,
    #[serde(default, deserialize_with = "de_f64_flex")]
    pub path_latency: f64,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub next_hop_ipv4_lat_first: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub next_hop_hostname_lat_first: String,
    #[serde(default, deserialize_with = "de_i64_flex")]
    pub path_len_lat_first: i64,
    #[serde(default, deserialize_with = "de_f64_flex")]
    pub path_latency_lat_first: f64,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeInfo {
    #[serde(default, deserialize_with = "de_i64_flex")]
    pub peer_id: i64,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub ipv4_addr: String,
    #[serde(default)]
    pub proxy_cidrs: Vec<String>,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub hostname: String,
    #[serde(default)]
    pub stun_info: Option<Value>,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub inst_id: String,
    #[serde(default)]
    pub listeners: Vec<String>,
    #[serde(default)]
    pub config: Value,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub version: String,
    #[serde(default)]
    pub ip_list: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CredentialInfo {
    #[serde(default, deserialize_with = "de_string_flex")]
    pub credential_id: String,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub allow_relay: bool,
    #[serde(default, deserialize_with = "de_i64_flex")]
    pub expiry_unix: i64,
    #[serde(default)]
    pub allowed_proxy_cidrs: Vec<String>,
    #[serde(default)]
    pub reusable: bool,
    #[serde(default)]
    pub public_key_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerateCredentialResponse {
    #[serde(default, deserialize_with = "de_string_flex")]
    pub credential_id: String,
    #[serde(default, deserialize_with = "de_string_flex")]
    pub credential_secret: String,
    #[serde(default, deserialize_with = "de_i64_flex")]
    pub expiry_unix: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RevokeCredentialResponse {
    #[serde(default)]
    pub success: bool,
}
