// Onboarding：3 步引导 — 选目录 / 提示装 Primitive FTPd / 填连接 + 测试

import { useState } from "react";
import { api, type DeviceForm, type ConnectionResult } from "../lib/api";

interface Props {
  onDone: () => void;
}

export default function Onboarding({ onDone }: Props) {
  const [step, setStep] = useState(1);
  const [form, setForm] = useState<DeviceForm>({
    name: "我的手机",
    host: "",
    port: 1024,
    username: "anonymous",
    password: "",
    backupRoot: "",
  });
  const [test, setTest] = useState<ConnectionResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const next = () => setStep((s) => Math.min(3, s + 1));
  const prev = () => setStep((s) => Math.max(1, s - 1));

  async function doTest() {
    setBusy(true);
    setErr(null);
    try {
      const r = await api.testConnection(form);
      setTest(r);
    } catch (e: unknown) {
      setErr(asError(e));
    } finally {
      setBusy(false);
    }
  }

  async function doSave() {
    setBusy(true);
    setErr(null);
    try {
      await api.saveDevice(form);
      onDone();
    } catch (e: unknown) {
      setErr(asError(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="mx-auto mt-12 max-w-2xl rounded-lg border border-zinc-200 bg-white p-8 shadow-sm">
      <header className="mb-6">
        <h1 className="text-2xl font-bold">欢迎使用 AlbumSync</h1>
        <p className="mt-1 text-sm text-zinc-500">局域网 FTP 相册备份 · 首次配置（步骤 {step}/3）</p>
        <div className="mt-3 flex gap-1">
          {[1, 2, 3].map((s) => (
            <div
              key={s}
              className={`h-1 flex-1 rounded ${s <= step ? "bg-blue-500" : "bg-zinc-200"}`}
            />
          ))}
        </div>
      </header>

      {step === 1 && (
        <div className="space-y-4">
          <h2 className="text-lg font-semibold">第 1 步：选择 PC 端备份目录</h2>
          <p className="text-sm text-zinc-600">
            手机相册会被备份到这个目录下。建议放在容量充足的磁盘。
          </p>
          <label className="block">
            <span className="mb-1 block text-sm font-medium">备份根目录</span>
            <input
              type="text"
              placeholder="例如 D:\PhotoBackup"
              value={form.backupRoot}
              onChange={(e) => setForm({ ...form, backupRoot: e.target.value })}
              className="w-full rounded border border-zinc-300 px-3 py-2 outline-none focus:border-blue-500"
            />
          </label>
          <p className="text-xs text-zinc-500">
            后续可在「设置」页修改。手机端原始目录结构会被原样保留。
          </p>
        </div>
      )}

      {step === 2 && (
        <div className="space-y-4">
          <h2 className="text-lg font-semibold">第 2 步：在手机端启动 FTP 服务</h2>
          <ol className="ml-5 list-decimal space-y-2 text-sm text-zinc-700">
            <li>
              在手机上安装 <strong>Primitive FTPd</strong>（F-Droid / 应用商店免费下载）
            </li>
            <li>打开 App → 设置一个用户名 / 密码（如 albumsync / 你定的密码）</li>
            <li>主界面点「Start」启动 FTP 服务 → 记下显示的 IP 与端口（默认 1024）</li>
            <li>建议在路由器后台为手机绑定静态 IP，避免下次同步因 IP 变化失败</li>
          </ol>
          <div className="rounded bg-amber-50 p-3 text-sm text-amber-900">
            ⚠️ FTP 是明文协议，<strong>仅限局域网使用</strong>。请不要在公网开启。
          </div>
        </div>
      )}

      {step === 3 && (
        <div className="space-y-4">
          <h2 className="text-lg font-semibold">第 3 步：填写连接信息并测试</h2>
          <div className="grid grid-cols-2 gap-3">
            <Field label="别名" value={form.name} onChange={(v) => setForm({ ...form, name: v })} />
            <Field
              label="主机 IP"
              value={form.host}
              placeholder="192.168.1.100"
              onChange={(v) => setForm({ ...form, host: v })}
            />
            <Field
              label="端口"
              type="number"
              value={String(form.port)}
              onChange={(v) => setForm({ ...form, port: parseInt(v) || 1024 })}
            />
            <Field label="用户名" value={form.username} onChange={(v) => setForm({ ...form, username: v })} />
            <Field
              label="密码"
              type="password"
              value={form.password}
              onChange={(v) => setForm({ ...form, password: v })}
            />
          </div>
          <div className="flex gap-2">
            <button
              onClick={doTest}
              disabled={busy || !form.host || !form.username}
              className="rounded bg-zinc-100 px-4 py-2 text-sm font-medium hover:bg-zinc-200 disabled:opacity-50"
            >
              {busy ? "测试中…" : "测试连接"}
            </button>
            {test && (
              <span
                className={`flex items-center text-sm ${test.ok ? "text-green-700" : "text-red-700"}`}
              >
                {test.ok ? "✓ " + (test.serverBanner || "连接成功") : "✗ " + test.error}
              </span>
            )}
          </div>
        </div>
      )}

      {err && <div className="mt-4 rounded bg-red-50 p-3 text-sm text-red-700">{err}</div>}

      <footer className="mt-8 flex justify-between">
        <button
          onClick={prev}
          disabled={step === 1}
          className="rounded px-4 py-2 text-sm text-zinc-600 hover:bg-zinc-100 disabled:opacity-30"
        >
          上一步
        </button>
        {step < 3 ? (
          <button
            onClick={next}
            disabled={(step === 1 && !form.backupRoot)}
            className="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
          >
            下一步
          </button>
        ) : (
          <button
            onClick={doSave}
            disabled={busy || !test?.ok}
            className="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
          >
            {busy ? "保存中…" : "完成并进入主界面"}
          </button>
        )}
      </footer>
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
  type = "text",
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  placeholder?: string;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-sm font-medium">{label}</span>
      <input
        type={type}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded border border-zinc-300 px-3 py-2 outline-none focus:border-blue-500"
      />
    </label>
  );
}

function asError(e: unknown): string {
  if (typeof e === "string") return e;
  if (typeof e === "object" && e !== null && "message" in e) {
    return (e as { message: string }).message;
  }
  return String(e);
}
