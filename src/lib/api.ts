// Tauri 后端 API 封装：所有 invoke 命令都在这里聚合
// 类型必须与 src-tauri/src/db/models.rs 中的 DTO 保持同步

import { invoke, Channel } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ============== 类型 ==============

export interface DeviceDto {
  id: number;
  name: string;
  host: string;
  port: number;
  username: string;
  backupRoot: string;
  isActive: boolean;
}

export interface DeviceForm {
  name: string;
  host: string;
  port: number;
  username: string;
  password: string;
  backupRoot: string;
}

export interface ConnectionResult {
  ok: boolean;
  serverBanner: string | null;
  error: string | null;
}

export interface DeviceStatus {
  online: boolean;
  latencyMs: number | null;
}

export interface SyncRunDto {
  id: number;
  deviceId: number;
  startedAt: number;
  endedAt: number | null;
  status: "running" | "succeeded" | "partial" | "failed" | "aborted";
  added: number;
  updated: number;
  deleted: number;
  failed: number;
  bytesDownloaded: number;
  errorSummary: string | null;
}

export interface TrashItemDto {
  id: number;
  deviceId: number;
  originalRel: string;
  size: number;
  deletedAt: number;
  expireAt: number;
}

export interface SettingsDto {
  retentionDays: number;
  autoStart: boolean;
  concurrency: number;
  includeGlobs: string[];
  excludeGlobs: string[];
}

export interface SettingsForm extends SettingsDto {}

// 同步进度事件（与 events.rs SyncEvent 对齐）
export type FileAction = "added" | "updated" | "deleted" | "skipped";

export type SyncEvent =
  | { kind: "started"; runId: number; totalFiles: number; totalBytes: number; deletes: number }
  | { kind: "progress"; doneFiles: number; doneBytes: number; currentFile: string; speedBps: number }
  | { kind: "fileDone"; relPath: string; action: FileAction }
  | { kind: "failed"; relPath: string; reason: string }
  | { kind: "finished"; runId: number; added: number; updated: number; deleted: number; failed: number }
  | { kind: "aborted"; runId: number };

export interface AppError {
  kind: string;
  message: string;
}

// ============== 命令封装 ==============

export const api = {
  ping: () => invoke<string>("ping"),

  // device
  getActiveDevice: () => invoke<DeviceDto | null>("get_active_device"),
  saveDevice: (form: DeviceForm) => invoke<DeviceDto>("save_device", { form }),
  deleteDevice: (id: number) => invoke<void>("delete_device", { id }),
  testConnection: (form: DeviceForm) => invoke<ConnectionResult>("test_connection", { form }),
  deviceStatus: (id: number) => invoke<DeviceStatus>("device_status", { id }),

  // sync
  syncStart: (deviceId: number, onEvent: Channel<SyncEvent>) =>
    invoke<void>("sync_start", { deviceId, onEvent }),
  syncAbort: () => invoke<void>("sync_abort"),

  // history
  listSyncRuns: (deviceId: number, limit?: number) =>
    invoke<SyncRunDto[]>("list_sync_runs", { deviceId, limit }),

  // trash
  listTrash: (deviceId: number, search?: string) =>
    invoke<TrashItemDto[]>("list_trash", { deviceId, search }),
  restoreTrash: (ids: number[]) => invoke<void>("restore_trash", { ids }),
  purgeTrash: (ids: number[]) => invoke<void>("purge_trash", { ids }),

  // settings
  getSettings: () => invoke<SettingsDto>("get_settings"),
  updateSettings: (form: SettingsForm) => invoke<void>("update_settings", { form }),
};

// 暴露 Channel 让 UI 创建
export { Channel };

// 全局广播事件订阅
export function listenAppReady(cb: () => void): Promise<UnlistenFn> {
  return listen("app-ready", cb);
}

export function listenDeviceStatus(cb: (p: { deviceId: number; online: boolean }) => void) {
  return listen<{ deviceId: number; online: boolean }>("device-status-changed", (e) => cb(e.payload));
}

export function listenTraySync(cb: () => void): Promise<UnlistenFn> {
  return listen("tray-sync-clicked", cb);
}

// ============== 工具 ==============

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const k = 1024;
  const u = ["KB", "MB", "GB", "TB"];
  let v = n / k;
  let i = 0;
  while (v >= k && i < u.length - 1) {
    v /= k;
    i++;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${u[i]}`;
}

export function formatUnix(unix: number, withSeconds = false): string {
  if (!unix) return "—";
  const d = new Date(unix * 1000);
  const fmt = new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: withSeconds ? "2-digit" : undefined,
    hour12: false,
  });
  return fmt.format(d);
}

export function daysFromNow(unix: number): number {
  const now = Math.floor(Date.now() / 1000);
  return Math.max(0, Math.ceil((unix - now) / 86400));
}
