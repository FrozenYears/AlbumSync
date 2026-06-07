// Settings：配置表单

import { useEffect, useState } from "react";
import { api, type SettingsForm } from "../lib/api";

export default function SettingsPage() {
  const [form, setForm] = useState<SettingsForm | null>(null);
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  useEffect(() => {
    api.getSettings().then(setForm);
  }, []);

  if (!form) return <div className="p-12 text-center text-zinc-400">加载中…</div>;

  async function save() {
    setBusy(true);
    setMsg(null);
    try {
      await api.updateSettings(form!);
      setMsg("已保存");
      setTimeout(() => setMsg(null), 2000);
    } catch (e) {
      setMsg(`保存失败：${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-4">
      <section className="rounded-lg border border-zinc-200 bg-white p-6">
        <h2 className="text-base font-semibold">同步行为</h2>
        <div className="mt-4 grid grid-cols-2 gap-4">
          <NumField
            label="回收站保留天数"
            value={form.retentionDays}
            onChange={(v) => setForm({ ...form, retentionDays: v })}
            min={1}
            max={365}
          />
          <NumField
            label="并发下载数"
            value={form.concurrency}
            onChange={(v) => setForm({ ...form, concurrency: v })}
            min={1}
            max={16}
          />
        </div>
      </section>

      <section className="rounded-lg border border-zinc-200 bg-white p-6">
        <h2 className="text-base font-semibold">同步范围</h2>
        <p className="mt-1 text-sm text-zinc-500">使用 glob 通配。每行一条。</p>
        <div className="mt-4 grid grid-cols-2 gap-4">
          <GlobList
            label="包含路径"
            value={form.includeGlobs}
            onChange={(v) => setForm({ ...form, includeGlobs: v })}
          />
          <GlobList
            label="排除路径"
            value={form.excludeGlobs}
            onChange={(v) => setForm({ ...form, excludeGlobs: v })}
          />
        </div>
      </section>

      <div className="flex items-center justify-between">
        {msg && <span className="text-sm text-green-700">{msg}</span>}
        <button
          onClick={save}
          disabled={busy}
          className="ml-auto rounded bg-blue-600 px-5 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {busy ? "保存中…" : "保存"}
        </button>
      </div>
    </div>
  );
}

function NumField({
  label,
  value,
  onChange,
  min,
  max,
}: {
  label: string;
  value: number;
  onChange: (v: number) => void;
  min: number;
  max: number;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-sm font-medium">{label}</span>
      <input
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(e) => onChange(Math.max(min, Math.min(max, parseInt(e.target.value) || min)))}
        className="w-full rounded border border-zinc-300 px-3 py-2 outline-none focus:border-blue-500"
      />
    </label>
  );
}

function GlobList({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string[];
  onChange: (v: string[]) => void;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-sm font-medium">{label}</span>
      <textarea
        rows={6}
        value={value.join("\n")}
        onChange={(e) =>
          onChange(
            e.target.value
              .split("\n")
              .map((s) => s.trim())
              .filter(Boolean),
          )
        }
        className="w-full rounded border border-zinc-300 px-3 py-2 font-mono text-xs outline-none focus:border-blue-500"
      />
    </label>
  );
}
