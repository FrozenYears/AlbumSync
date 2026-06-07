// Sync 主页：设备状态卡 + 立即同步 + 实时进度

import { useEffect, useRef, useState } from "react";
import {
  api,
  Channel as TauriChannel,
  formatBytes,
  type DeviceDto,
  type DeviceStatus,
  type SyncEvent,
} from "../lib/api";

interface Props {
  device: DeviceDto;
}

interface ProgressState {
  active: boolean;
  totalFiles: number;
  totalBytes: number;
  doneFiles: number;
  doneBytes: number;
  currentFile: string;
  speedBps: number;
  deletes: number;
  failed: { rel: string; reason: string }[];
  result?: { added: number; updated: number; deleted: number; failed: number; aborted?: boolean };
}

const initial: ProgressState = {
  active: false,
  totalFiles: 0,
  totalBytes: 0,
  doneFiles: 0,
  doneBytes: 0,
  currentFile: "",
  speedBps: 0,
  deletes: 0,
  failed: [],
};

export default function SyncPage({ device }: Props) {
  const [status, setStatus] = useState<DeviceStatus | null>(null);
  const [progress, setProgress] = useState<ProgressState>(initial);
  const [confirmFirst, setConfirmFirst] = useState(false);
  const channelRef = useRef<TauriChannel<SyncEvent> | null>(null);

  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      try {
        const s = await api.deviceStatus(device.id);
        if (!cancelled) setStatus(s);
      } catch {
        if (!cancelled) setStatus({ online: false, latencyMs: null });
      }
    };
    tick();
    const t = setInterval(tick, 5000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, [device.id]);

  async function startSync(force = false) {
    if (!force) {
      // 首次同步给用户一个确认
      setConfirmFirst(true);
      return;
    }
    setConfirmFirst(false);
    const ch = new TauriChannel<SyncEvent>();
    channelRef.current = ch;
    setProgress({ ...initial, active: true });
    ch.onmessage = (msg) => {
      setProgress((p) => reduce(p, msg));
    };
    try {
      await api.syncStart(device.id, ch);
    } catch (e) {
      setProgress((p) => ({ ...p, active: false, failed: [...p.failed, { rel: "sync_start", reason: String(e) }] }));
    }
  }

  async function abort() {
    try {
      await api.syncAbort();
    } catch {
      /* ignore */
    }
  }

  const pct =
    progress.totalBytes > 0 ? Math.min(100, (progress.doneBytes / progress.totalBytes) * 100) : 0;

  return (
    <div className="space-y-6">
      <DeviceCard device={device} status={status} />

      <section className="rounded-lg border border-zinc-200 bg-white p-6">
        <header className="mb-4 flex items-center justify-between">
          <h2 className="text-lg font-semibold">同步</h2>
          {progress.active ? (
            <button
              onClick={abort}
              className="rounded bg-red-100 px-4 py-2 text-sm font-medium text-red-700 hover:bg-red-200"
            >
              中止当前同步
            </button>
          ) : (
            <button
              onClick={() => startSync(false)}
              disabled={!status?.online}
              className="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            >
              立即同步
            </button>
          )}
        </header>

        {!progress.active && !progress.result && (
          <p className="text-sm text-zinc-500">
            {status?.online
              ? "手机已在线，点「立即同步」开始扫描差异并下载新增/变更的文件。"
              : "等待手机上线（请确认 Primitive FTPd 已启动且 IP 端口正确）。"}
          </p>
        )}

        {progress.active && (
          <div className="space-y-3">
            <div className="flex justify-between text-sm">
              <span>
                {progress.doneFiles}/{progress.totalFiles} 个文件
              </span>
              <span>
                {formatBytes(progress.doneBytes)} / {formatBytes(progress.totalBytes)} ·{" "}
                {formatBytes(progress.speedBps)}/s
              </span>
            </div>
            <div className="h-2 overflow-hidden rounded bg-zinc-200">
              <div
                className="h-full bg-blue-500 transition-all"
                style={{ width: `${pct}%` }}
              />
            </div>
            <p className="truncate text-xs text-zinc-500">{progress.currentFile || "扫描远端清单中…"}</p>
            {progress.deletes > 0 && (
              <p className="text-xs text-amber-700">
                本次将把 {progress.deletes} 个手机已删除的文件移入回收站（保留 30 天）
              </p>
            )}
          </div>
        )}

        {!progress.active && progress.result && (
          <div className="rounded bg-green-50 p-4 text-sm text-green-800">
            <div className="font-medium">
              {progress.result.aborted ? "已中止" : "同步完成"}
            </div>
            <div className="mt-1 text-zinc-700">
              新增 {progress.result.added} · 更新 {progress.result.updated} · 删除{" "}
              {progress.result.deleted} · 失败 {progress.result.failed}
            </div>
          </div>
        )}

        {progress.failed.length > 0 && (
          <details className="mt-3">
            <summary className="cursor-pointer text-sm text-red-700">
              {progress.failed.length} 个文件失败
            </summary>
            <ul className="mt-2 max-h-40 overflow-auto text-xs text-zinc-600">
              {progress.failed.slice(0, 20).map((f, i) => (
                <li key={i} className="truncate">
                  <span className="font-mono">{f.rel}</span> — {f.reason}
                </li>
              ))}
              {progress.failed.length > 20 && <li>… 其余 {progress.failed.length - 20} 条见日志</li>}
            </ul>
          </details>
        )}
      </section>

      {confirmFirst && (
        <ConfirmModal
          onCancel={() => setConfirmFirst(false)}
          onConfirm={() => startSync(true)}
        />
      )}
    </div>
  );
}

function DeviceCard({ device, status }: { device: DeviceDto; status: DeviceStatus | null }) {
  return (
    <section className="flex items-center justify-between rounded-lg border border-zinc-200 bg-white p-5">
      <div>
        <h2 className="text-base font-semibold">{device.name}</h2>
        <p className="mt-1 font-mono text-sm text-zinc-500">
          {device.username}@{device.host}:{device.port}
        </p>
        <p className="mt-1 text-xs text-zinc-400">备份到：{device.backupRoot}</p>
      </div>
      <div className="flex flex-col items-end">
        <span className="flex items-center gap-2">
          <span
            className={`h-2 w-2 rounded-full ${
              status?.online ? "bg-green-500" : "bg-zinc-400"
            }`}
          />
          <span className="text-sm">{status?.online ? "在线" : "离线"}</span>
        </span>
        {status?.latencyMs != null && (
          <span className="mt-1 text-xs text-zinc-500">{status.latencyMs} ms</span>
        )}
      </div>
    </section>
  );
}

function ConfirmModal({ onCancel, onConfirm }: { onCancel: () => void; onConfirm: () => void }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <div className="max-w-md rounded-lg bg-white p-6 shadow-xl">
        <h3 className="text-lg font-semibold">开始同步</h3>
        <div className="mt-3 text-sm text-zinc-700">
          <p>本次将：</p>
          <ul className="ml-5 mt-1 list-disc space-y-1">
            <li>扫描手机相册并下载新增 / 变更的文件</li>
            <li>
              <strong>把手机已删除的文件移入回收站</strong>（保留 30 天）—— 这是镜像同步策略
            </li>
          </ul>
          <p className="mt-3 text-xs text-zinc-500">
            首次同步可能需要较长时间，请保持手机在同一 Wi-Fi 上。
          </p>
        </div>
        <div className="mt-5 flex justify-end gap-2">
          <button
            onClick={onCancel}
            className="rounded px-4 py-2 text-sm text-zinc-600 hover:bg-zinc-100"
          >
            取消
          </button>
          <button
            onClick={onConfirm}
            className="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
          >
            开始同步
          </button>
        </div>
      </div>
    </div>
  );
}

function reduce(p: ProgressState, e: SyncEvent): ProgressState {
  switch (e.kind) {
    case "started":
      return {
        ...p,
        active: true,
        totalFiles: e.totalFiles,
        totalBytes: e.totalBytes,
        doneFiles: 0,
        doneBytes: 0,
        deletes: e.deletes,
        failed: [],
        result: undefined,
      };
    case "progress":
      return {
        ...p,
        doneFiles: e.doneFiles,
        doneBytes: e.doneBytes,
        currentFile: e.currentFile,
        speedBps: e.speedBps,
      };
    case "fileDone":
      return p;
    case "failed":
      return { ...p, failed: [...p.failed, { rel: e.relPath, reason: e.reason }] };
    case "finished":
      return {
        ...p,
        active: false,
        result: {
          added: e.added,
          updated: e.updated,
          deleted: e.deleted,
          failed: e.failed,
        },
      };
    case "aborted":
      return {
        ...p,
        active: false,
        result: { added: 0, updated: 0, deleted: 0, failed: 0, aborted: true },
      };
  }
}
