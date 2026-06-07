// App.tsx — 主布局：左侧导航 + 路由切换

import { useEffect, useState } from "react";
import { api, listenAppReady, listenTraySync, type DeviceDto } from "./lib/api";
import { useRoute, type Route } from "./lib/router";
import Onboarding from "./pages/Onboarding";
import SyncPage from "./pages/Sync";
import HistoryPage from "./pages/History";
import TrashPage from "./pages/Trash";
import SettingsPage from "./pages/Settings";

type AppState =
  | { kind: "loading" }
  | { kind: "onboarding" }
  | { kind: "ready"; device: DeviceDto };

export default function App() {
  const [state, setState] = useState<AppState>({ kind: "loading" });
  const [route, navigate] = useRoute();

  // 启动后等 app-ready 再尝试取设备（Rust 端 setup 是异步的）
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let mounted = true;
    (async () => {
      try {
        await api.ping();
      } catch {
        /* ignore */
      }
      const tryLoad = async () => {
        try {
          const d = await api.getActiveDevice();
          if (!mounted) return;
          setState(d ? { kind: "ready", device: d } : { kind: "onboarding" });
        } catch {
          // DB 还没就绪
          setTimeout(tryLoad, 300);
        }
      };
      const un = await listenAppReady(() => tryLoad());
      unlisten = un;
      tryLoad();
    })();
    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  // 托盘"立即同步"事件 → 切到 Sync 页
  useEffect(() => {
    let un: (() => void) | undefined;
    listenTraySync(() => navigate("sync")).then((u) => {
      un = u;
    });
    return () => un?.();
  }, [navigate]);

  if (state.kind === "loading")
    return <div className="flex h-full items-center justify-center text-zinc-500">初始化…</div>;
  if (state.kind === "onboarding")
    return <Onboarding onDone={() => location.reload()} />;

  return (
    <div className="flex h-full">
      <Sidebar route={route} onNavigate={navigate} deviceName={state.device.name} />
      <main className="flex-1 overflow-auto p-6">
        {route === "sync" && <SyncPage device={state.device} />}
        {route === "history" && <HistoryPage device={state.device} />}
        {route === "trash" && <TrashPage device={state.device} />}
        {route === "settings" && <SettingsPage />}
        {route === "onboarding" && <Onboarding onDone={() => location.reload()} />}
      </main>
    </div>
  );
}

function Sidebar({
  route,
  onNavigate,
  deviceName,
}: {
  route: Route;
  onNavigate: (r: Route) => void;
  deviceName: string;
}) {
  const items: { id: Route; label: string }[] = [
    { id: "sync", label: "同步" },
    { id: "history", label: "历史" },
    { id: "trash", label: "回收站" },
    { id: "settings", label: "设置" },
  ];
  return (
    <aside className="w-56 shrink-0 border-r border-zinc-200 bg-white p-4">
      <div className="mb-6">
        <h1 className="text-lg font-bold">AlbumSync</h1>
        <p className="mt-0.5 truncate text-xs text-zinc-500">{deviceName}</p>
      </div>
      <nav className="space-y-1">
        {items.map((it) => (
          <button
            key={it.id}
            onClick={() => onNavigate(it.id)}
            className={`block w-full rounded px-3 py-2 text-left text-sm transition ${
              route === it.id
                ? "bg-blue-50 font-medium text-blue-700"
                : "text-zinc-700 hover:bg-zinc-100"
            }`}
          >
            {it.label}
          </button>
        ))}
      </nav>
      <footer className="mt-8 text-xs text-zinc-400">
        <p>v0.1.0</p>
      </footer>
    </aside>
  );
}
