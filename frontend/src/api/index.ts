import client from "./client";
import type {
  AuditLog,
  BinaryStatus,
  Credential,
  DashboardSummary,
  JoinInfo,
  LiveState,
  Network,
  NetworkShares,
  NodeItem,
  NodeLog,
  Paginated,
  RouteListItem,
  SelectableUser,
  SystemBinaries,
  SystemInfo,
  Topology,
  TrafficPoint,
  User,
} from "@/types";

interface ApiResponse<T> {
  // 大多数接口直接返回 T
  data: T;
}

// ---------------------------------------------------------------- Auth

export const authApi = {
  status: () => client.get<{ initialized: boolean }>("/api/auth/status").then((r) => r.data),
  setup: (payload: {
    username: string;
    password: string;
    displayName?: string;
    email?: string;
  }) => client.post<{ user: User }>("/api/auth/setup", payload).then((r) => r.data),
  login: (payload: { username: string; password: string }) =>
    client.post<{ user: User }>("/api/auth/login", payload).then((r) => r.data),
  logout: () => client.post("/api/auth/logout").then((r) => r.data),
  me: () => client.get<{ user: { id: string; username: string; role: string } }>("/api/auth/me").then((r) => r.data),
  changePassword: (payload: { oldPassword: string; newPassword: string }) =>
    client.post("/api/auth/change-password", payload).then((r) => r.data),
};

// ---------------------------------------------------------------- Users

export const userApi = {
  list: () => client.get<{ items: User[] }>("/api/users").then((r) => r.data.items),
  create: (payload: {
    username: string;
    password: string;
    displayName?: string;
    email?: string;
    role: string;
  }) => client.post<{ user: User }>("/api/users", payload).then((r) => r.data.user),
  update: (
    id: string,
    payload: { displayName?: string; email?: string; role?: string; disabled?: boolean },
  ) => client.patch<{ user: User }>(`/api/users/${id}`, payload).then((r) => r.data.user),
  resetPassword: (id: string, newPassword: string) =>
    client.post(`/api/users/${id}/reset-password`, { newPassword }).then((r) => r.data),
  remove: (id: string) => client.delete(`/api/users/${id}`).then((r) => r.data),
  selectable: () =>
    client.get<{ items: SelectableUser[] }>("/api/users/selectable").then((r) => r.data.items),
};

// ---------------------------------------------------------------- Networks

export interface NetworkPayload {
  name: string;
  description?: string;
  networkName?: string;
  networkSecret?: string;
  ipv4?: string;
  dhcp?: boolean;
  hostname?: string;
  instanceName?: string;
  listeners?: string[];
  mappedListeners?: string[];
  peers?: string[];
  externalNode?: string;
  listenPort?: number;
  rpcPort?: number;
  secureMode?: boolean;
  autoStart?: boolean;
  startNow?: boolean;
  flags?: Record<string, string | number | boolean>;
}

export const networkApi = {
  list: () => client.get<{ items: Network[] }>("/api/networks").then((r) => r.data.items),
  get: (id: string) =>
    client.get<{ network: Network }>(`/api/networks/${id}`).then((r) => r.data.network),
  create: (payload: NetworkPayload) =>
    client.post<{ network: Network }>("/api/networks", payload).then((r) => r.data.network),
  update: (id: string, payload: Partial<NetworkPayload>) =>
    client.patch<{ network: Network }>(`/api/networks/${id}`, payload).then((r) => r.data.network),
  remove: (id: string) => client.delete(`/api/networks/${id}`).then((r) => r.data),
  start: (id: string) => client.post(`/api/networks/${id}/start`).then((r) => r.data),
  stop: (id: string) => client.post(`/api/networks/${id}/stop`).then((r) => r.data),
  restart: (id: string) => client.post(`/api/networks/${id}/restart`).then((r) => r.data),
  config: (id: string) =>
    client.get<{ config: string }>(`/api/networks/${id}/config`).then((r) => r.data.config),
  live: (id: string) => client.get<LiveState>(`/api/networks/${id}/live`).then((r) => r.data),
  topology: (id: string) =>
    client.get<Topology>(`/api/networks/${id}/topology`).then((r) => r.data),
  peers: (id: string) =>
    client
      .get<{ online: boolean; peers: unknown[]; error?: string }>(`/api/networks/${id}/peers`)
      .then((r) => r.data),
  routes: (id: string) =>
    client
      .get<{ online: boolean; routes: RouteListItem[]; error?: string }>(
        `/api/networks/${id}/routes`,
      )
      .then((r) => r.data),
  logs: (id: string, limit = 300) =>
    client
      .get<{ items: NodeLog[] }>(`/api/networks/${id}/logs`, { params: { limit } })
      .then((r) => r.data.items),
  nodes: (id: string) =>
    client.get<{ items: NodeItem[] }>(`/api/networks/${id}/nodes`).then((r) => r.data.items),
  credentials: (id: string) =>
    client
      .get<{ items: Credential[] }>(`/api/networks/${id}/credentials`)
      .then((r) => r.data.items),
  shares: (id: string) =>
    client.get<NetworkShares>(`/api/networks/${id}/shares`).then((r) => r.data),
  share: (id: string, payload: { userId: string; permission?: "view" | "manage" }) =>
    client.post(`/api/networks/${id}/shares`, payload).then((r) => r.data),
  unshare: (id: string, userId: string) =>
    client.delete(`/api/networks/${id}/shares/${userId}`).then((r) => r.data),
  transfer: (id: string, userId: string) =>
    client
      .post<{ network: Network }>(`/api/networks/${id}/transfer`, { userId })
      .then((r) => r.data.network),
};

// ---------------------------------------------------------------- Nodes

export const nodeApi = {
  list: (networkId?: string, live = true) =>
    client
      .get<{ items: NodeItem[] }>("/api/nodes", {
        params: { networkId, live: live ? undefined : "false" },
      })
      .then((r) => r.data.items),
  get: (id: string) => client.get<{ node: NodeItem }>(`/api/nodes/${id}`).then((r) => r.data.node),
  create: (payload: {
    networkId: string;
    name: string;
    hostname?: string;
    ipv4?: string | null;
    description?: string;
    listeners?: string[];
    proxyNetworks?: string[];
    flags?: Record<string, string | number | boolean>;
    issueCredential?: boolean;
    ttlSeconds?: number;
    groups?: string[];
    allowRelay?: boolean;
    reusable?: boolean;
    allowedProxyCidrs?: string[];
  }) =>
    client
      .post<{ node: NodeItem; credential?: Credential; credentialSecret?: string }>(
        "/api/nodes",
        payload,
      )
      .then((r) => r.data),
  update: (
    id: string,
    payload: Partial<{
      name: string;
      hostname: string;
      ipv4: string | null;
      description: string;
      listeners: string[];
      proxyNetworks: string[];
      flags: Record<string, string | number | boolean>;
    }>,
  ) => client.patch<{ node: NodeItem }>(`/api/nodes/${id}`, payload).then((r) => r.data.node),
  remove: (id: string) => client.delete(`/api/nodes/${id}`).then((r) => r.data),
  join: (id: string, listenPort?: number, peer?: string) =>
    client
      .get<JoinInfo>(`/api/nodes/${id}/join`, { params: { listenPort, peer } })
      .then((r) => r.data),
  rotateCredential: (
    id: string,
    payload?: {
      ttlSeconds?: number;
      allowRelay?: boolean;
      reusable?: boolean;
      groups?: string[];
      allowedProxyCidrs?: string[];
    },
  ) =>
    client
      .post<{ node: NodeItem; credential: Credential; credentialSecret: string }>(
        `/api/nodes/${id}/rotate-credential`,
        payload ?? {},
      )
      .then((r) => r.data),
};

// ---------------------------------------------------------------- Credentials

export const credentialApi = {
  list: (networkId?: string) =>
    client
      .get<{ items: Credential[] }>("/api/credentials", { params: { networkId } })
      .then((r) => r.data.items),
  create: (payload: {
    networkId: string;
    nodeId?: string;
    ttlSeconds: number;
    groups?: string[];
    allowRelay?: boolean;
    reusable?: boolean;
    allowedProxyCidrs?: string[];
  }) =>
    client
      .post<{ credential: Credential; secret: string }>("/api/credentials", payload)
      .then((r) => r.data),
  revoke: (id: string) =>
    client.post<{ credential: Credential }>(`/api/credentials/${id}/revoke`).then((r) => r.data),
  remove: (id: string) => client.delete(`/api/credentials/${id}`).then((r) => r.data),
};

// ---------------------------------------------------------------- Logs

export const logApi = {
  audit: (params: {
    page?: number;
    pageSize?: number;
    action?: string;
    username?: string;
    search?: string;
  }) => client.get<Paginated<AuditLog>>("/api/logs/audit", { params }).then((r) => r.data),
  nodes: (params: {
    page?: number;
    pageSize?: number;
    networkId?: string;
    level?: string;
    search?: string;
  }) => client.get<Paginated<NodeLog>>("/api/logs/nodes", { params }).then((r) => r.data),
  clearNodes: (networkId?: string) =>
    client.delete("/api/logs/nodes", { params: { networkId } }).then((r) => r.data),
  streamUrl: (networkId?: string) =>
    `/api/logs/stream${networkId ? `?networkId=${encodeURIComponent(networkId)}` : ""}`,
};

// ---------------------------------------------------------------- Dashboard

export const dashboardApi = {
  summary: () =>
    client.get<DashboardSummary>("/api/dashboard/summary").then((r) => r.data),
  traffic: (networkId?: string, hours = 1) =>
    client
      .get<{ series: TrafficPoint[] }>("/api/dashboard/traffic", {
        params: { networkId, hours },
      })
      .then((r) => r.data.series),
};

// ---------------------------------------------------------------- System

export const systemApi = {
  info: () => client.get<SystemInfo>("/api/system/info").then((r) => r.data),
  binaries: () => client.get<SystemBinaries>("/api/system/binaries").then((r) => r.data),
  settings: () =>
    client
      .get<{ effective: Record<string, string | number>; overrides: Record<string, string> }>(
        "/api/system/settings",
      )
      .then((r) => r.data),
  updateSettings: (payload: Record<string, string | number>) =>
    client
      .put<{ effective: Record<string, string | number> }>("/api/system/settings", payload)
      .then((r) => r.data),
};

export type { BinaryStatus, ApiResponse };
