// Trash：回收站（30 天软删除项）

import { useEffect, useState } from "react";
import {
  api,
  daysFromNow,
  formatBytes,
  formatUnix,
  type DeviceDto,
  type TrashItemDto,
} from "../lib/api";

export default function TrashPage({ device }: { device: DeviceDto }) {
  const [items, setItems] = useState<TrashItemDto[]>([]);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  async function refresh() {
    setLoading(true);
    try {
      const r = await api.listTrash(device.id, search || undefined);
      setItems(r);
      setSelected(new Set());
    } finally {
      setLoading(false);
    }
  }
  useEffect(() => {
    refresh();
  }, [device.id]);

  function toggle(id: number) {
    const ns = new Set(selected);
    if (ns.has(id)) ns.delete(id);
    else ns.add(id);
    setSelected(ns);
  }
  function toggleAll() {
    if (selected.size === items.length) setSelected(new Set());
    else setSelected(new Set(items.map((i) => i.id)));
  }

  async function restore() {
    setBusy(true);
    try {
      await api.restoreTrash([...selected]);
      await refresh();
    } finally {
      setBusy(false);
    }
  }
  async function purge() {
    if (!confirm(`立即清除选中的 ${selected.size} 项？此操作不可撤销。`)) return;
    setBusy(true);
    try {
      await api.purgeTrash([...selected]);
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="rounded-lg border border-zinc-200 bg-white">
      <header className="flex items-center justify-between border-b border-zinc-200 px-5 py-3">
        <div className="flex items-center gap-3">
          <h2 className="text-sm font-semibold">回收站</h2>
          <input
            placeholder="搜索文件名…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && refresh()}
            className="w-56 rounded border border-zinc-300 px-2 py-1 text-sm outline-none focus:border-blue-500"
          />
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={restore}
            disabled={selected.size === 0 || busy}
            className="rounded bg-blue-50 px-3 py-1.5 text-sm font-medium text-blue-700 hover:bg-blue-100 disabled:opacity-50"
          >
            恢复 ({selected.size})
          </button>
          <button
            onClick={purge}
            disabled={selected.size === 0 || busy}
            className="rounded bg-red-50 px-3 py-1.5 text-sm font-medium text-red-700 hover:bg-red-100 disabled:opacity-50"
          >
            立即清除
          </button>
        </div>
      </header>

      {loading ? (
        <div className="p-12 text-center text-zinc-400">加载中…</div>
      ) : items.length === 0 ? (
        <div className="p-12 text-center">
          <p className="text-base font-medium text-zinc-700">回收站为空</p>
          <p className="mt-2 text-sm text-zinc-500">
            手机删除后的文件会临时存放在这里，30 天后自动清理。
          </p>
        </div>
      ) : (
        <table className="w-full text-sm">
          <thead className="bg-zinc-50 text-xs uppercase text-zinc-500">
            <tr>
              <th className="px-4 py-2 text-left">
                <input
                  type="checkbox"
                  checked={items.length > 0 && selected.size === items.length}
                  onChange={toggleAll}
                />
              </th>
              <th className="px-4 py-2 text-left">原路径</th>
              <th className="px-4 py-2 text-right">大小</th>
              <th className="px-4 py-2 text-left">删除时间</th>
              <th className="px-4 py-2 text-right">剩余天数</th>
            </tr>
          </thead>
          <tbody>
            {items.map((it) => (
              <tr key={it.id} className="border-t border-zinc-100 hover:bg-zinc-50">
                <td className="px-4 py-2">
                  <input type="checkbox" checked={selected.has(it.id)} onChange={() => toggle(it.id)} />
                </td>
                <td className="truncate px-4 py-2 font-mono text-xs">{it.originalRel}</td>
                <td className="px-4 py-2 text-right">{formatBytes(it.size)}</td>
                <td className="px-4 py-2 text-zinc-600">{formatUnix(it.deletedAt)}</td>
                <td className="px-4 py-2 text-right">
                  <DaysLeft days={daysFromNow(it.expireAt)} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function DaysLeft({ days }: { days: number }) {
  const color = days <= 3 ? "text-red-600" : days <= 7 ? "text-amber-600" : "text-zinc-600";
  return <span className={`text-xs ${color}`}>{days} 天</span>;
}
