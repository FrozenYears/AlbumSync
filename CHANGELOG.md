# 变更日志 / Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与 [Semantic Versioning](https://semver.org/spec/v2.0.0.html)。

## [未发布 / Unreleased]

### 待办
- 远端根目录字段（避免不同 FTP 服务器 chroot 差异时需手改 include glob）
- 暗色主题
- M5：NSIS 打包为正式安装包

## [0.1.0] — 2026-06-07

首个可用版本。在真实设备上完成了第一次 60 GB / 12786 文件镜像同步。

### 新增
- **架构**：Tauri 2 + React 19 + TypeScript 5.8 + Vite 7 + Tailwind v4
- **同步引擎**：FTP MLSD 递归扫描 → diff → 并发下载（默认 4 路）+ 断点续传（`.part` 文件 + REST）
- **镜像同步**：本地缺失的文件下载；本地多出的文件移入 `.trash/` 软删除
- **30 天回收站**：可搜索/筛选 / 批量恢复 / 立即清除；每 6 小时自动 GC 过期项
- **数据持久化**：SQLite（WAL + busy_timeout）保存设备 / 文件清单 / 同步历史 / 回收站 / 应用设置
- **凭据管理**：FTP 密码通过 `keyring` 存入 Windows Credential Manager（DPAPI）
- **托盘集成**：关闭主窗口最小化到托盘；右键菜单立即同步 / 显示 / 退出；单实例锁
- **进度事件**：Tauri Channel API 高频推送进度（200ms 节流），低频用 emit 广播
- **设置页**：可视化修改设备配置、并发数、回收站保留天数、include/exclude glob
- **健康检查**：每 5s TCP 探活，UI 实时反映设备在线 / 延迟
- **包管理**：pnpm 11（`allowBuilds` 白名单） + cargo（USTC mirror）
- **构建脚手架**：`scripts/with-msvc.bat`（自动配 VS Build Tools + Windows SDK 环境）

### 修复（实战发现的非平凡问题）
- **MLSD `Modify=YYYYMMDDHHMMSS.fff` 解析失败**：Primitive FTPd 输出毫秒小数，suppaftp 8 解析不了 → 加预处理 `strip_modify_fraction`
- **MLSD 绝对路径返回空**：Primitive FTPd 不支持 `MLSD <abs-path>` → 改用 `CWD <dir>; MLSD(None)`，对所有 RFC 3659 实现都通用
- **进度显示 NaN**：serde `rename_all = "camelCase"` 只重命名 enum variant，不重命名 struct variant 内部字段 → 改用 `rename_all_fields = "camelCase"`
- **Ctrl+C 退出报错**：Tauri 内部已有 Windows console ctrl handler，自加的 `tokio::signal::ctrl_c()` 与之冲突 → 移除
- **中止同步按钮无反馈**：UI 等不到后端 Aborted 事件 → 800ms 超时兜底重置 UI
- **设置页缺少 FTP 修改入口**：补 `DeviceSection` 组件
- **`with-msvc.bat` PATH 爆炸**：vcvars64 反复 prepend 同样的目录到调用者 PATH，最终超过 Windows 8KB 限制 → `setlocal/endlocal` 隔离

### 已知限制
- 仅支持 Windows 11 x64
- FTP 是明文协议，仅限信任的局域网
- 仅支持 1 台设备（v0.x 设计取舍）
- 端口 2222 等场景需手动确认服务器吐的是 `220 ` 而非 `SSH-2.0-`（不少 Android FTP App 默认开 SFTP）

### 调研留底（issue 形式跟踪，后续版本逐步消化）
- FTP 控制连接失败路径不发 QUIT，server slot 可能被占用至自然 idle timeout
- walker / downloader 内部 FTP 命令无 per-call timeout，网络抖动可能造成 hang
- 上述两条叠加时 UI 的中止按钮实质打断不了正在 hang 的 await

[未发布 / Unreleased]: https://github.com/FrozenYears/AlbumSync/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/FrozenYears/AlbumSync/releases/tag/v0.1.0
