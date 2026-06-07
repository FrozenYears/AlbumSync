// Settings：配置表单（设备 + 同步行为 + 同步范围）

import { useEffect, useState } from "react";
import {
  api,
  type ConnectionResult,
  type DeviceDto,
  type DeviceForm,
  type SettingsForm,
} from "../lib/api";

export default function SettingsPage() {
  const [settings, setSettings] = useState<SettingsForm | null>(null);
  const [device, setDevice] = useState<DeviceDto | null>(null);
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  useEffect(() => {
    api.getSettings().then(setSettings);
    api.getActiveDevice().then(setDevice);
  }, []);

  async function saveSettings() {
    setBusy(true);
    setMsg(null);
    try {
      await api.updateSettings(settings!);
      setMsg("已保存");
      setTimeout(() => setMsg(null), 2000);
    } catch (e) {
      setMsg(`保存失败：${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  if (!settings) return <div className="p-12 text-center text-zinc-400">加载中…</div>;

  return (
    <div className="space-y-4">
      {device && <DeviceSection device={device} onSaved={(d) => setDevice(d)} />}

      <section className="rounded-lg border border-zinc-200 bg-white p-6">
        <h2 className="text-base font-semibold">同步行为</h2>
        <div className="mt-4 grid grid-cols-2 gap-4">
          <NumField
            label="回收站保留天数"
            value={settings.retentionDays}
            onChange={(v) => setSettings({ ...settings, retentionDays: v })}
            min={1}
            max={365}
          />
          <NumField
            label="并发下载数"
            value={settings.concurrency}
            onChange={(v) => setSettings({ ...settings, concurrency: v })}
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
            value={settings.includeGlobs}
            onChange={(v) => setSettings({ ...settings, includeGlobs: v })}
          />
          <GlobList
            label="排除路径"
            value={settings.excludeGlobs}
            onChange={(v) => setSettings({ ...settings, excludeGlobs: v })}
          />
        </div>
      </section>

      <div className="flex items-center justify-between">
        {msg && <span className="text-sm text-green-700">{msg}</span>}
        <button
          onClick={saveSettings}
          disabled={busy}
          className="ml-auto rounded bg-blue-600 px-5 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {busy ? "保存中…" : "保存同步设置"}
        </button>
      </div>
    </div>
  );
}

function DeviceSection({
  device,
  onSaved,
}: {
  device: DeviceDto;
  onSaved: (d: DeviceDto) => void;
}) {
  const [form, setForm] = useState<DeviceForm>({
    name: device.name,
    host: device.host,
    port: device.port,
    username: device.username,
    password: "",
    backupRoot: device.backupRoot,
  });
  const [test, setTest] = useState<ConnectionResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  async function doTest() {
    if (!form.password) {
      setMsg("测试连接需要输入密码");
      return;
    }
    setBusy(true);
    setMsg(null);
    try {
      const r = await api.testConnection(form);
      setTest(r);
    } catch (e) {
      setMsg(`测试失败：${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  async function doSave() {
    if (!form.password) {
      setMsg("请输入 FTP 密码（修改设备配置需要重新填一次）");
      return;
    }
    setBusy(true);
    setMsg(null);
    try {
      const updated = await api.saveDevice(form);
      onSaved(updated);
      setForm((f) => ({ ...f, password: "" }));
      setMsg("已保存设备配置");
      setTimeout(() => setMsg(null), 2500);
    } catch (e) {
      setMsg(`保存失败：${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="rounded-lg border border-zinc-200 bg-white p-6">
      <header className="mb-4 flex items-center justify-between">
        <h2 className="text-base font-semibold">FTP 服务器</h2>
        <span className="text-xs text-zinc-500">
          密码存于 Windows 凭据管理器，每次修改需重新输入
        </span>
      </header>
      <div className="grid grid-cols-2 gap-3">
        <TextField label="别名" value={form.name} onChange={(v) => setForm({ ...form, name: v })} />
        <TextField label="主机 IP" value={form.host} onChange={(v) => setForm({ ...form, host: v })} />
        <NumField
          label="端口"
          value={form.port}
          onChange={(v) => setForm({ ...form, port: v })}
          min={1}
          max={65535}
        />
        <TextField
          label="用户名"
          value={form.username}
          onChange={(v) => setForm({ ...form, username: v })}
        />
        <TextField
          label="密码"
          type="password"
          value={form.password}
          onChange={(v) => setForm({ ...form, password: v })}
        />
        <TextField
          label="本地备份目录"
          value={form.backupRoot}
          onChange={(v) => setForm({ ...form, backupRoot: v })}
        />
      </div>
      <div className="mt-4 flex items-center gap-3">
        <button
          onClick={doTest}
          disabled={busy || !form.host || !form.password}
          className="rounded bg-zinc-100 px-3 py-1.5 text-sm font-medium hover:bg-zinc-200 disabled:opacity-50"
        >
          {busy ? "处理中…" : "测试连接"}
        </button>
        {test && (
          <span className={`text-sm ${test.ok ? "text-green-700" : "text-red-700"}`}>
            {test.ok ? "✓ " + (test.serverBanner || "连接成功") : "✗ " + test.error}
          </span>
        )}
        <div className="ml-auto flex items-center gap-3">
          {msg && <span className="text-sm text-zinc-600">{msg}</span>}
          <button
            onClick={doSave}
            disabled={busy || !form.password || !form.host || !form.username}
            className="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
          >
            保存设备配置
          </button>
        </div>
      </div>
    </section>
  );
}

function TextField({
  label,
  value,
  onChange,
  type = "text",
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-sm font-medium">{label}</span>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded border border-zinc-300 px-3 py-2 outline-none focus:border-blue-500"
      />
    </label>
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
