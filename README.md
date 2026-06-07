# AlbumSync

[![CI](https://github.com/FrozenYears/AlbumSync/actions/workflows/ci.yml/badge.svg)](https://github.com/FrozenYears/AlbumSync/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/FrozenYears/AlbumSync?include_prereleases&sort=semver)](https://github.com/FrozenYears/AlbumSync/releases)
[![License](https://img.shields.io/github/license/FrozenYears/AlbumSync)](./LICENSE)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB)](https://v2.tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)](https://www.rust-lang.org/)
[![React 19](https://img.shields.io/badge/React-19-61DAFB?logo=react)](https://react.dev/)

> 局域网内将 Android 手机相册照片自动备份到 Windows 11 PC 的桌面软件。
>
> 镜像同步 + 30 天软删除，安装包 < 20 MB，零第三方云。

> [!WARNING]
> **项目状态：early-alpha**。可用，但仍在迭代关键路径（per-call timeout、连接复用、远端根目录可配置）。生产环境请保留另一份独立备份。

## 这是什么

- **手机端零开发**：复用免费开源的 [Primitive FTPd](https://github.com/wolpi/prim-ftpd) 作为 FTP 服务器
- **PC 端 GUI**：Tauri 2 + React，一个安装包就能跑
- **镜像同步**：远端有/没有的状态实时反映到本地；手机删除的文件移入"回收站"保留 30 天
- **断点续传**：网络中断后下次同步从中断点继续

## 目录

- [系统要求](#系统要求)
- [快速上手](#快速上手)
- [日常使用](#日常使用)
- [安全说明](#安全说明)
- [常见问题](#常见问题)
- [架构与设计](#架构与设计)
- [开发与构建](#开发与构建)
- [贡献](#贡献)
- [许可](#许可)

## 系统要求

- Windows 11 x64
- WebView2 Runtime（Win11 自带 Edge 即可）
- 手机：Android 7.0+

## 快速上手

### 1) 在手机上启动 FTP 服务

1. 在 F-Droid 或应用商店安装 **Primitive FTPd**（免费）。
2. 打开 App → 进入 **Configuration** 设置：
   - 勾选 **Anonymous login** 关闭；设一个用户名 + 密码（建议至少 8 位）
   - 端口建议保留默认 `1024`（高位端口不需要 root）
3. 回到主界面点 **Start server** → 屏幕上会显示当前 IP，例如 `192.168.1.123:1024`。
4. （强烈推荐）在路由器后台为这台手机绑定**静态 IP**，避免下次 IP 变化导致 AlbumSync 找不到设备。
5. （省电策略）在系统设置里把 Primitive FTPd 设为**不被系统省电杀死**，否则手机锁屏后服务可能被杀。

> [!TIP]
> 不少 Android FTP App 的"默认端口"其实是 SFTP（如 2222）。判断方法：用 PowerShell 运行
> `$c=New-Object Net.Sockets.TcpClient("<手机IP>",<端口>); $c.GetStream().Read((New-Object byte[] 64),0,64); $c.Close()`
> 应输出以 `220 ` 开头的横幅。看到 `SSH-2.0-` 即是 SFTP，AlbumSync 当前版本只支持明文 FTP。

### 2) 安装并配置 AlbumSync

1. 从 [Releases](https://github.com/FrozenYears/AlbumSync/releases) 下载 `AlbumSync_<版本>_x64-setup.exe`，双击安装。
2. 首次启动会进入引导：
   - **第 1 步**：选 PC 上的备份目录（默认建议 `用户目录\Pictures\PhoneBackup`）。
   - **第 2 步**：手机端配置图文复习。
   - **第 3 步**：填手机 IP / 端口 / 用户名 / 密码 → 点 **测试连接** → 通过后 **完成**。
3. 主界面显示设备状态（绿点 = 在线）。点 **立即同步**，确认风险后开始备份。
   - 首次同步可能很慢，取决于相册大小（5000 张照片约 20-40 分钟）。
   - 同步过程中可关闭主窗口（最小化到托盘），进程继续运行。

> [!IMPORTANT]
> **同步前请关闭 Windows Explorer 里所有打开本机 → 手机 FTP 的窗口**。许多 FTP 服务器默认 per-IP 并发上限是 1，被 Explorer 占住会让 AlbumSync 看似"连接超时"。

## 日常使用

- **同步**：右下角托盘图标 → 双击主窗口 → 点「立即同步」。或托盘右键菜单 → 「立即同步」。
- **回收站**：在主窗口左侧导航点「回收站」，可搜索、批量恢复、立即清除。
  - 列表里"剩余天数 ≤ 3"的项会变红，提醒你 30 天到期。
- **历史**：每次同步的统计（耗时/新增/失败）。
- **退出**：仅托盘右键菜单 → 「退出 AlbumSync」 才真正关闭进程。

## 安全说明

- **FTP 是明文协议**，**仅限在你信任的局域网内使用**。请不要把手机的 FTP 端口暴露到公网。
- AlbumSync 把手机密码存到 Windows Credential Manager（DPAPI 加密），不会写入任何文件。
- 软件本身不联网（除手机 FTP 外）。源码可在 GitHub 审查。

## 常见问题

**Q: 同步时报"连接超时"？**
A: 三步排查：① 关掉 Explorer 里所有打开此 FTP 地址的窗口；② 确认手机和电脑在同一 Wi-Fi、`ping <手机IP>` 通；③ 用上面 [快速上手](#快速上手) 里的 PowerShell 一行命令确认服务器吐的是 `220 ` 而非 `SSH-2.0-`。

**Q: 手机重启后 IP 变了？**
A: 在路由器后台为手机绑定静态 IP。或在 AlbumSync 设备页修改 IP 后重新「测试连接」。

**Q: 同步显示"0 个文件" / 进度不动？**
A: 多数情况是 FTP 服务器根目录不是 `/sdcard` 而是更上一层（如 `Internal storage/`），导致默认 include glob 全部 miss。临时方案：进**设置 → 同步范围 → 包含路径**，给每行前面加 `Internal storage/` 前缀。

**Q: 进度显示 `NaN KB`？**
A: 这是 v0.1.0 之前的版本问题（serde 字段名 snake/camel 不匹配），v0.1.0 已修复。请升级。

**Q: 手机删的照片在 PC 这里多久会真删？**
A: 默认 30 天。可在「设置」页改成 7-365 天之间任意值。期间随时可在「回收站」恢复。

**Q: 想完全关闭软件？**
A: 关闭主窗口只是最小化到托盘。**托盘右键 → 「退出 AlbumSync」** 才真正退出。

**Q: 想删除某个手机的所有设置？**
A: 在「设置」页 → 「删除当前设备」按钮（会同步清掉 Windows Credential Manager 里的密码）。备份的文件不会被删。

## 架构与设计

- 需求基线：[REQUIREMENTS.md](./REQUIREMENTS.md)
- 详细设计（分层架构 / SQLite schema / 同步流程 / 错误处理 / 依赖原则）：[DESIGN.md](./DESIGN.md)
- 变更日志：[CHANGELOG.md](./CHANGELOG.md)

## 开发与构建

详见 [`CONTRIBUTING.md`](./CONTRIBUTING.md)。

简版：

```bash
# 一次性环境（Windows）
rustup default stable-x86_64-pc-windows-msvc
# 装 VS Build Tools + Windows SDK，详见 CONTRIBUTING.md

# 拉代码 + 装依赖
git clone https://github.com/FrozenYears/AlbumSync.git
cd AlbumSync
pnpm install

# 起开发 server（用脚本自动注入 MSVC 环境）
scripts\with-msvc.bat pnpm tauri dev
```

## 贡献

欢迎 PR / issue。请先读 [`CONTRIBUTING.md`](./CONTRIBUTING.md) 和 [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md)。

bug 反馈请通过 [Issue](https://github.com/FrozenYears/AlbumSync/issues/new/choose)；安装求助请走 [Discussions](https://github.com/FrozenYears/AlbumSync/discussions)。

## 致谢

- [Primitive FTPd](https://github.com/wolpi/prim-ftpd) — 手机端 FTP 服务器
- [Tauri](https://v2.tauri.app/) — 桌面框架
- [suppaftp](https://github.com/veeso/suppaftp) — Rust FTP 客户端

## 许可

[Apache License 2.0](./LICENSE)
