export type Role = "admin" | "operator" | "viewer";

export interface User {
  id: string;
  username: string;
  displayName: string | null;
  email: string | null;
  role: Role;
  disabled: boolean;
  createdAt: number;
  updatedAt: number;
  lastLoginAt: number | null;
}

export interface EasyTierFlags {
  default_protocol: string;
  dev_name: string;
  enable_encryption: boolean;
  enable_ipv6: boolean;
  mtu: number;
  latency_first: boolean;
  enable_exit_node: boolean;
  no_tun: boolean;
  use_smoltcp: boolean;
  relay_network_whitelist: string;
  disable_p2p: boolean;
  relay_all_peer_rpc: boolean;
  disable_udp_hole_punching: boolean;
  disable_tcp_hole_punching: boolean;
  private_mode: boolean;
  [key: string]: string | number | boolean;
}

/**
 * 客户端节点高级设置（`[flags]`）。键名与默认值对照官方配置助手
 * 的 `common.FlagsInConfig`（localName），后端 `default_flags()` 中登记了同样的默认值，
 * 因此只有非默认项会被写进节点 config.toml。
 */
export interface NodeAdvancedFlags {
  [key: string]: string | number | boolean;
  default_protocol: string;
  dev_name: string;
  mtu: number;
  bind_device: boolean;
  enable_encryption: boolean;
  encryption_algorithm: string;
  disable_p2p: boolean;
  p2p_only: boolean;
  lazy_p2p: boolean;
  need_p2p: boolean;
  disable_tcp_hole_punching: boolean;
  disable_udp_hole_punching: boolean;
  disable_sym_hole_punching: boolean;
  disable_upnp: boolean;
  multi_thread: boolean;
  multi_thread_count: number;
  latency_first: boolean;
  enable_kcp_proxy: boolean;
  disable_kcp_input: boolean;
  disable_relay_kcp: boolean;
  enable_quic_proxy: boolean;
  disable_quic_input: boolean;
  disable_relay_quic: boolean;
  quic_listen_port: number;
  use_smoltcp: boolean;
  relay_network_whitelist: string;
  relay_all_peer_rpc: boolean;
  enable_exit_node: boolean;
  proxy_forward_by_system: boolean;
  private_mode: boolean;
  disable_relay_data: boolean;
  foreign_relay_bps_limit: number;
  instance_recv_bps_limit: number;
  no_tun: boolean;
  enable_ipv6: boolean;
  accept_dns: boolean;
  tld_dns_zone: string;
  enable_udp_broadcast_relay: boolean;
}

export const NODE_DEFAULT_FLAGS: NodeAdvancedFlags = {
  default_protocol: "tcp",
  dev_name: "",
  mtu: 1380,
  bind_device: false,
  enable_encryption: true,
  encryption_algorithm: "",
  disable_p2p: false,
  p2p_only: false,
  lazy_p2p: false,
  need_p2p: true,
  disable_tcp_hole_punching: false,
  disable_udp_hole_punching: false,
  disable_sym_hole_punching: false,
  disable_upnp: false,
  multi_thread: false,
  multi_thread_count: 0,
  latency_first: false,
  enable_kcp_proxy: false,
  disable_kcp_input: false,
  disable_relay_kcp: false,
  enable_quic_proxy: false,
  disable_quic_input: false,
  disable_relay_quic: false,
  quic_listen_port: 0,
  use_smoltcp: false,
  relay_network_whitelist: "*",
  relay_all_peer_rpc: false,
  enable_exit_node: false,
  proxy_forward_by_system: false,
  private_mode: false,
  disable_relay_data: false,
  foreign_relay_bps_limit: 0,
  instance_recv_bps_limit: 0,
  no_tun: false,
  enable_ipv6: true,
  accept_dns: false,
  tld_dns_zone: "",
  enable_udp_broadcast_relay: false,
};

export type NetworkStatus = "stopped" | "running" | "error";

export interface Network {
  id: string;
  name: string;
  description: string | null;
  networkName: string;
  ipv4: string;
  dhcp: boolean;
  hostname: string;
  instanceName: string;
  listeners: string[];
  mappedListeners: string[];
  peers: string[];
  externalNode: string | null;
  listenPort: number;
  rpcPort: number;
  secureMode: boolean;
  sharedNodePublicKey: string;
  flags: EasyTierFlags;
  autoStart: boolean;
  status: NetworkStatus;
  lastError: string | null;
  ownerId: string | null;
  access: "owner" | "manage" | "view";
  createdAt: number;
  updatedAt: number;
}

export interface SelectableUser {
  id: string;
  username: string;
  displayName: string | null;
  role: Role;
  disabled: boolean;
}

export interface NetworkShare {
  userId: string;
  username: string;
  displayName: string | null;
  role: Role;
  permission: "view" | "manage";
  createdAt: number;
}

export interface NetworkShares {
  owner: { id: string; username: string; displayName: string | null } | null;
  items: NetworkShare[];
}

export interface NodeLiveInfo {
  online: boolean;
  cost: string;
  latMs: number | null;
  lossRate: number | null;
  rxBytes: number;
  txBytes: number;
  tunnelProto: string;
  natType: string;
  peerId: string;
  version: string;
}

export interface NodeItem {
  id: string;
  networkId: string;
  name: string;
  hostname: string;
  ipv4: string | null;
  type: string;
  description: string | null;
  listeners: string[];
  proxyNetworks: string[];
  flags: Record<string, string | number | boolean>;
  credentialId: string | null;
  peerId: number | null;
  status: string;
  lastSeenAt: number | null;
  createdAt: number;
  updatedAt: number;
  live?: NodeLiveInfo;
}

export type CredentialStatus = "active" | "expired" | "revoked";

export interface Credential {
  id: string;
  networkId: string;
  nodeId: string | null;
  credentialId: string;
  groups: string[];
  allowRelay: boolean;
  reusable: boolean;
  allowedProxyCidrs: string[];
  ttlSeconds: number;
  expiresAt: number;
  revoked: boolean;
  createdBy: string | null;
  createdAt: number;
  updatedAt: number;
  status: CredentialStatus;
  remainingSeconds: number;
}

export interface PeerListItem {
  cidr: string;
  ipv4: string;
  hostname: string;
  cost: string;
  lat_ms: string;
  loss_rate: string;
  rx_bytes: string;
  tx_bytes: string;
  tunnel_proto: string;
  nat_type: string;
  id: string;
  version: string;
}

export interface RouteListItem {
  ipv4: string;
  hostname: string;
  proxy_cidrs: string;
  next_hop_ipv4: string;
  next_hop_hostname: string;
  next_hop_lat: number;
  path_len: number;
  path_latency: number;
  version: string;
}

export interface LiveState {
  online: boolean;
  running: boolean;
  nodeInfo: Record<string, unknown> | null;
  peers: PeerListItem[];
  routes: RouteListItem[];
  error?: string;
}

export interface TopologyNode {
  id: string;
  label: string;
  ipv4: string;
  hostname: string;
  kind: "local" | "peer" | "client";
  online: boolean;
  cost?: string;
  latMs?: number | null;
  lossRate?: number | null;
  natType?: string;
  tunnelProto?: string;
  rxBytes?: number;
  txBytes?: number;
  version?: string;
  peerId?: string;
}

export interface TopologyEdge {
  id: string;
  source: string;
  target: string;
  label?: string;
  latencyMs?: number | null;
  cost?: string;
  relayed?: boolean;
}

export interface Topology {
  networkId: string;
  nodes: TopologyNode[];
  edges: TopologyEdge[];
  updatedAt: number;
  online: boolean;
  error?: string;
}

export interface DashboardSummary {
  initialized: boolean;
  users: { total: number; admins: number };
  networks: { total: number; running: number; error: number };
  nodes: { total: number; online: number; offline: number; unknown: number };
  credentials: {
    total: number;
    active: number;
    expiringSoon: number;
    expired: number;
    revoked: number;
  };
  recentAudit: Array<{
    id: number;
    username: string | null;
    action: string;
    resourceType: string | null;
    resourceId: string | null;
    createdAt: number;
  }>;
  recentErrors: Array<{
    id: number;
    networkId: string | null;
    message: string;
    createdAt: number;
  }>;
}

export interface TrafficPoint {
  t: number;
  rx: number;
  tx: number;
}

export interface AuditLog {
  id: number;
  userId: string | null;
  username: string | null;
  action: string;
  resourceType: string | null;
  resourceId: string | null;
  detail: unknown;
  ip: string | null;
  userAgent: string | null;
  createdAt: number;
}

export interface NodeLog {
  id: number;
  networkId: string | null;
  nodeId: string | null;
  source: string;
  level: string;
  message: string;
  createdAt: number;
}

export interface Paginated<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
}

export interface BinaryStatus {
  available: boolean;
  version?: string;
  error?: string;
}

export interface SystemBinaries {
  cli: BinaryStatus;
  core: BinaryStatus;
}

export interface SystemInfo {
  version: string;
  node: string;
  platform: string;
  uptime: number;
  dataDir: string;
  easytier: Record<string, string | number>;
}

export interface JoinInfo {
  node: NodeItem;
  network: Network;
  nodeType: "fixed" | "temporary";
  peer: string;
  listenPort: number;
  listeners: string[];
  command: string;
  configToml: string;
  sharedNodePublicKey: string;
  peerPublicKey: string;
  /** 固定节点接入用 network_secret（KDF 派生值）；临时节点为空字符串。 */
  networkSecret: string;
  /** 固定节点的本地静态公钥（由 local_private_key 派生）；临时节点为空字符串。 */
  localPublicKey: string;
  /** 临时节点的凭据信息；固定节点为 null。 */
  credential: {
    id: string;
    credentialId: string;
    secret: string;
    expiresAt: number;
    status: string;
  } | null;
}
