// History：列出最近 N 次同步运行

import { useEffect, useState } from "react";
import { api, formatBytes, formatUnix, type DeviceDto, type SyncRunDto } from "../lib/api";

export default function HistoryPage({ device }: { device: DeviceDto }) {
  const [runs, setRuns] = useState<SyncRunDto[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    api
      .listSyncRuns(device.id, 50)
      .then((r) => !cancelled && setRuns(r))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [device.id]);

  if (loading) return <Skeleton />;
  if (runs.length === 0)
    return (
      <Empty title="尚无同步记录" hint="首次同步后这里会显示历次运行的统计。" />
    );

  return (
    <section className="rounded-lg border border-zinc-200 bg-white">
      <header className="border-b border-zinc-200 px-5 py-3 text-sm font-semibold">
        最近 {runs.length} 次同步
      </header>
      <table className="w-full text-sm">
        <thead className="bg-zinc-50 text-xs uppercase text-zinc-500">
          <tr>
            <th className="px-4 py-2 text-left">开始</th>
            <th className="px-4 py-2 text-left">状态</th>
            <th className="px-4 py-2 text-right">新增</th>
            <th className="px-4 py-2 text-right">更新</th>
            <th className="px-4 py-2 text-right">删除</th>
            <th className="px-4 py-2 text-right">失败</th>
            <th className="px-4 py-2 text-right">下载量</th>
            <th className="px-4 py-2 text-right">耗时</th>
          </tr>
        </thead>
        <tbody>
          {runs.map((r) => (
            <tr key={r.id} className="border-t border-zinc-100">
              <td className="px-4 py-2 font-mono">{formatUnix(r.startedAt, true)}</td>
              <td className="px-4 py-2">
                <StatusBadge status={r.status} />
              </td>
              <td className="px-4 py-2 text-right text-green-700">{r.added}</td>
              <td className="px-4 py-2 text-right text-blue-700">{r.updated}</td>
              <td className="px-4 py-2 text-right text-amber-700">{r.deleted}</td>
              <td className="px-4 py-2 text-right text-red-700">{r.failed}</td>
              <td className="px-4 py-2 text-right">{formatBytes(r.bytesDownloaded)}</td>
              <td className="px-4 py-2 text-right">{formatDuration(r.startedAt, r.endedAt)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

function StatusBadge({ status }: { status: SyncRunDto["status"] }) {
  const color =
    status === "succeeded"
      ? "bg-green-100 text-green-700"
      : status === "partial"
        ? "bg-amber-100 text-amber-700"
        : status === "failed"
          ? "bg-red-100 text-red-700"
          : status === "aborted"
            ? "bg-zinc-100 text-zinc-600"
            : "bg-blue-100 text-blue-700";
  return <span className={`rounded px-2 py-0.5 text-xs ${color}`}>{status}</span>;
}

function formatDuration(start: number, end: number | null): string {
  if (!end) return "—";
  const s = end - start;
  if (s < 60) return `${s} 秒`;
  if (s < 3600) return `${Math.floor(s / 60)} 分 ${s % 60} 秒`;
  return `${Math.floor(s / 3600)} 时 ${Math.floor((s % 3600) / 60)} 分`;
}

function Skeleton() {
  return <div className="rounded-lg border border-zinc-200 bg-white p-12 text-center text-zinc-400">加载中…</div>;
}

function Empty({ title, hint }: { title: string; hint: string }) {
  return (
    <div className="rounded-lg border border-zinc-200 bg-white p-12 text-center">
      <p className="text-base font-medium text-zinc-700">{title}</p>
      <p className="mt-2 text-sm text-zinc-500">{hint}</p>
    </div>
  );
}
