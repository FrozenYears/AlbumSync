import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function App() {
  const [pong, setPong] = useState<string>("...");

  useEffect(() => {
    invoke<string>("ping").then(setPong).catch((e) => setPong(`error: ${e}`));
  }, []);

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", padding: 24 }}>
      <h1 style={{ margin: 0 }}>AlbumSync</h1>
      <p style={{ color: "#666" }}>局域网 FTP 相册备份工具 · 脚手架就绪</p>
      <p>IPC 健康检查：<code>{pong}</code></p>
    </main>
  );
}
