use serde::Serialize;

use crate::db::NetworkRow;
use crate::util::format::parse_human_size;
use crate::util::now_ms;

use super::network::LiveState;
use super::node::PublicNode;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyNode {
    pub id: String,
    pub label: String,
    pub ipv4: String,
    pub hostname: String,
    pub kind: String,
    pub online: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loss_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nat_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tunnel_proto: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx_bytes: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_bytes: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relayed: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    pub network_id: String,
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
    pub updated_at: i64,
    pub online: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn normalize_ip(ip: Option<&str>) -> String {
    ip.unwrap_or("").split('/').next().unwrap_or("").to_string()
}

pub fn build_topology(
    network: &NetworkRow,
    live: &LiveState,
    client_nodes: &[PublicNode],
) -> Topology {
    let local_id = format!("local:{}", network.id);
    let mut nodes: Vec<TopologyNode> = Vec::new();
    let mut edges: Vec<TopologyEdge> = Vec::new();
    let mut matched_clients: std::collections::HashSet<String> = std::collections::HashSet::new();

    let local_peer = live.peers.iter().find(|p| p.cost == "Local");
    nodes.push(TopologyNode {
        id: local_id.clone(),
        label: network.name.clone(),
        ipv4: normalize_ip(
            local_peer
                .map(|p| p.ipv4.as_str())
                .or(Some(network.ipv4.as_str())),
        ),
        hostname: local_peer
            .map(|p| p.hostname.clone())
            .unwrap_or_else(|| network.hostname.clone()),
        kind: "local".to_string(),
        online: live.online,
        cost: Some("Local".to_string()),
        lat_ms: None,
        loss_rate: None,
        nat_type: Some(
            local_peer
                .map(|p| p.nat_type.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
        ),
        tunnel_proto: None,
        rx_bytes: None,
        tx_bytes: None,
        version: Some(local_peer.map(|p| p.version.clone()).unwrap_or_default()),
        peer_id: local_peer.map(|p| p.id.clone()),
    });

    for peer in &live.peers {
        if peer.cost == "Local" {
            continue;
        }
        let norm_ip = normalize_ip(Some(&peer.ipv4));
        let client = client_nodes.iter().find(|n| {
            (!norm_ip.is_empty() && normalize_ip(n.ipv4.as_deref()) == norm_ip)
                || n.hostname == peer.hostname
        });
        if let Some(client) = client {
            matched_clients.insert(client.id.clone());
        }
        let id = format!("peer:{}", peer.id);
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
        nodes.push(TopologyNode {
            id: id.clone(),
            label: client
                .map(|c| c.name.clone())
                .unwrap_or_else(|| peer.hostname.clone()),
            ipv4: norm_ip,
            hostname: peer.hostname.clone(),
            kind: if client.is_some() { "client" } else { "peer" }.to_string(),
            online: true,
            cost: Some(peer.cost.clone()),
            lat_ms: lat,
            loss_rate: loss,
            nat_type: Some(peer.nat_type.clone()),
            tunnel_proto: Some(peer.tunnel_proto.clone()),
            rx_bytes: Some(parse_human_size(Some(&peer.rx_bytes))),
            tx_bytes: Some(parse_human_size(Some(&peer.tx_bytes))),
            version: Some(peer.version.clone()),
            peer_id: Some(peer.id.clone()),
        });
        let relayed = peer.cost.starts_with("relay");
        let label = if peer.lat_ms != "-" && !peer.lat_ms.is_empty() {
            Some(format!("{} ms", peer.lat_ms))
        } else if relayed {
            Some(format!("{} 跳", peer.cost))
        } else {
            None
        };
        edges.push(TopologyEdge {
            id: format!("{local_id}->{id}"),
            source: local_id.clone(),
            target: id,
            label,
            latency_ms: lat,
            cost: Some(peer.cost.clone()),
            relayed: Some(relayed),
        });
    }

    for client in client_nodes {
        if matched_clients.contains(&client.id) {
            continue;
        }
        nodes.push(TopologyNode {
            id: format!("client:{}", client.id),
            label: client.name.clone(),
            ipv4: normalize_ip(client.ipv4.as_deref()),
            hostname: client.hostname.clone(),
            kind: "client".to_string(),
            online: false,
            cost: None,
            lat_ms: None,
            loss_rate: None,
            nat_type: None,
            tunnel_proto: None,
            rx_bytes: None,
            tx_bytes: None,
            version: Some(String::new()),
            peer_id: None,
        });
    }

    Topology {
        network_id: network.id.clone(),
        nodes,
        edges,
        updated_at: now_ms(),
        online: live.online,
        error: live.error.clone(),
    }
}
