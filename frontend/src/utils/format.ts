import dayjs from "dayjs";
import relativeTime from "dayjs/plugin/relativeTime";
import "dayjs/locale/zh-cn";

dayjs.extend(relativeTime);
dayjs.locale("zh-cn");

export function formatTime(ts: number | null | undefined): string {
  if (!ts) return "-";
  return dayjs(ts).format("YYYY-MM-DD HH:mm:ss");
}

export function formatRelative(ts: number | null | undefined): string {
  if (!ts) return "-";
  return dayjs(ts).fromNow();
}

export function formatBytes(bytes: number | null | undefined): string {
  if (!bytes || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let idx = 0;
  let value = bytes;
  while (value >= 1024 && idx < units.length - 1) {
    value /= 1024;
    idx += 1;
  }
  return `${value.toFixed(value >= 100 || idx === 0 ? 0 : 1)} ${units[idx]}`;
}

export function formatDuration(seconds: number): string {
  if (seconds <= 0) return "已过期";
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d} 天 ${h} 小时`;
  if (h > 0) return `${h} 小时 ${m} 分`;
  return `${m} 分钟`;
}

export function secondsToUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const parts: string[] = [];
  if (d) parts.push(`${d}天`);
  if (h) parts.push(`${h}时`);
  parts.push(`${m}分`);
  return parts.join(" ");
}

const ACTION_LABELS: Record<string, string> = {
  "auth.setup": "初始化管理员",
  "auth.login": "登录",
  "auth.login.failed": "登录失败",
  "auth.change-password": "修改密码",
  "user.create": "创建用户",
  "user.update": "更新用户",
  "user.delete": "删除用户",
  "user.reset-password": "重置用户密码",
  "network.create": "创建网络",
  "network.update": "更新网络",
  "network.delete": "删除网络",
  "network.start": "启动网络",
  "network.stop": "停止网络",
  "network.restart": "重启网络",
  "node.create": "创建节点",
  "node.update": "更新节点",
  "node.delete": "删除节点",
  "node.view-join": "查看接入信息",
  "node.rotate-credential": "更换节点凭据",
  "credential.create": "签发临时凭据",
  "credential.revoke": "撤销凭据",
  "credential.delete": "删除凭据",
  "system.update-settings": "更新系统设置",
};

export function actionLabel(action: string): string {
  return ACTION_LABELS[action] ?? action;
}

export function networkStatusLabel(status: string): string {
  switch (status) {
    case "running":
      return "运行中";
    case "error":
      return "异常";
    default:
      return "已停止";
  }
}

export function networkStatusType(status: string): "success" | "error" | "default" {
  switch (status) {
    case "running":
      return "success";
    case "error":
      return "error";
    default:
      return "default";
  }
}

export function credentialStatusLabel(status: string): string {
  switch (status) {
    case "active":
      return "有效";
    case "expired":
      return "已过期";
    case "revoked":
      return "已撤销";
    default:
      return status;
  }
}

export function credentialStatusType(
  status: string,
): "success" | "warning" | "error" | "default" {
  switch (status) {
    case "active":
      return "success";
    case "expired":
      return "warning";
    case "revoked":
      return "error";
    default:
      return "default";
  }
}

export interface CostInfo {
  text: string;
  type: "success" | "warning" | "info" | "default";
}

/** 将 easytier-cli 的 cost 值转为可读标签：Local / p2p / relay(N) */
export function costInfo(cost: string | null | undefined): CostInfo {
  if (!cost) return { text: "-", type: "default" };
  if (cost === "Local") return { text: "本节点", type: "info" };
  if (cost === "p2p") return { text: "直连", type: "success" };
  const relay = cost.match(/^relay\((\d+)\)$/);
  if (relay) return { text: `${relay[1]} 跳中继`, type: "warning" };
  return { text: cost, type: "default" };
}

export function cloneDeep<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
