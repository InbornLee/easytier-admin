import dayjs from "dayjs";
import relativeTime from "dayjs/plugin/relativeTime";
import "dayjs/locale/zh-cn";
import "dayjs/locale/en";
import { getLocale, translate } from "@/i18n";

dayjs.extend(relativeTime);

function syncDayjsLocale(): void {
  dayjs.locale(getLocale() === "zh-CN" ? "zh-cn" : "en");
}
syncDayjsLocale();
if (typeof window !== "undefined") {
  window.addEventListener("et:locale", syncDayjsLocale);
}

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
  if (seconds <= 0) return translate("duration.expired");
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return translate("duration.daysHours", { d, h });
  if (h > 0) return translate("duration.hoursMinutes", { h, m });
  return translate("duration.minutes", { m });
}

export function secondsToUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const parts: string[] = [];
  if (d) parts.push(translate("duration.daysShort", { d }));
  if (h) parts.push(translate("duration.hoursShort", { h }));
  parts.push(translate("duration.minutesShort", { m }));
  return parts.join(" ");
}

const ACTION_KEYS: Record<string, string> = {
  "auth.setup": "action.authSetup",
  "auth.login": "action.authLogin",
  "auth.login.failed": "action.authLoginFailed",
  "auth.change-password": "action.authChangePassword",
  "user.create": "action.userCreate",
  "user.update": "action.userUpdate",
  "user.delete": "action.userDelete",
  "user.reset-password": "action.userResetPassword",
  "network.create": "action.networkCreate",
  "network.update": "action.networkUpdate",
  "network.delete": "action.networkDelete",
  "network.start": "action.networkStart",
  "network.stop": "action.networkStop",
  "network.restart": "action.networkRestart",
  "network.share": "action.networkShare",
  "network.unshare": "action.networkUnshare",
  "network.transfer": "action.networkTransfer",
  "node.create": "action.nodeCreate",
  "node.update": "action.nodeUpdate",
  "node.delete": "action.nodeDelete",
  "node.view-join": "action.nodeViewJoin",
  "node.rotate-credential": "action.nodeRotateCredential",
  "credential.create": "action.credentialCreate",
  "credential.revoke": "action.credentialRevoke",
  "credential.delete": "action.credentialDelete",
  "system.update-settings": "action.systemUpdateSettings",
};

export function actionLabel(action: string): string {
  const key = ACTION_KEYS[action];
  return key ? translate(key) : action;
}

export function networkStatusLabel(status: string): string {
  if (status === "running" || status === "stopped" || status === "error") {
    return translate(`status.network.${status}`);
  }
  return status;
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

export function nodeStatusLabel(status: string): string {
  if (status === "online" || status === "offline" || status === "unknown") {
    return translate(`status.node.${status}`);
  }
  return status;
}

export function credentialStatusLabel(status: string): string {
  if (status === "active" || status === "expired" || status === "revoked") {
    return translate(`status.credential.${status}`);
  }
  return status;
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

/** Convert an easytier-cli cost value into a readable label: Local / p2p / relay(N) */
export function costInfo(cost: string | null | undefined): CostInfo {
  if (!cost) return { text: "-", type: "default" };
  if (cost === "Local") return { text: translate("cost.local"), type: "info" };
  if (cost === "p2p") return { text: translate("cost.p2p"), type: "success" };
  const relay = cost.match(/^relay\((\d+)\)$/);
  if (relay) return { text: translate("cost.relay", { n: relay[1] }), type: "warning" };
  return { text: cost, type: "default" };
}

export function secondsToDuration(seconds: number): string {
  return formatDuration(seconds);
}

export function cloneDeep<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
