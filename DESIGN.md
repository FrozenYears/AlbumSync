# AlbumSync 设计文档

> 项目代号：AlbumSync  ·  版本：v0.1  ·  日期：2026-06-07
>
> 配套需求文档：[REQUIREMENTS.md](./REQUIREMENTS.md)

---

## 0. 文档维护规则

- 所有重大设计变更（数据模型、模块边界、关键 API、技术栈替换）必须通过 git commit 同步更新本文档。
- 小调整（字段重命名、内部函数签名）允许在代码中先行，PR / commit 描述里说明即可。
- 文档分章自成闭环；查表/查图可只看对应章节，无需通读。

---

## 1. 系统架构

### 1.1 上下文图

```
┌─────────────────────┐    LAN / FTP     ┌──────────────────────────────┐
│  Android phone       │ ◀──────────────▶ │  Windows 11 PC                │
│  Primitive FTPd      │  control + data  │  AlbumSync.exe                │
│  exposes /sdcard     │                  │  Tauri 2 (Rust + React)       │
└─────────────────────┘                  └──────────────────────────────┘
                                                      │
                                                      ▼
                                          ┌──────────────────────┐
                                          │ Windows Credential   │
                                          │  Manager (DPAPI)     │
                                          └──────────────────────┘
```

### 1.2 进程内分层

```
┌──────────────────────────────────────────────────────────────┐
│  Frontend (React + TypeScript)                                │
│  ─ Pages: Onboarding / Device / Sync / History / Trash / Set │
│  ─ Talks to Rust via @tauri-apps/api: invoke + Channel + listen│
└────────────────────────────┬─────────────────────────────────┘
                             │ IPC (tauri commands / events / channels)
┌────────────────────────────▼─────────────────────────────────┐
│  Tauri command layer (Rust)                                   │
│  ─ #[tauri::command] thin handlers, validation, error mapping │
└────────────────────────────┬─────────────────────────────────┘
                             │
┌────────────────────────────▼─────────────────────────────────┐
│  Domain modules                                               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐    │
│  │  config  │ │   ftp    │ │  sync    │ │     trash    │    │
│  │ (keyring)│ │(suppaftp)│ │ engine   │ │ (gc + restore│    │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘    │
│  ┌──────────────────────────────────────────────────────┐    │
│  │              SQLite (sqlx, WAL)                       │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

## 2. 目录结构

```
ef21c03b/                          # 仓库根
├── .gitignore
├── README.md                      # 用户手册（含 Primitive FTPd 配置教程，最后阶段写）
├── REQUIREMENTS.md
├── DESIGN.md                      # 本文件
├── package.json                   # pnpm workspace（如需）
├── pnpm-lock.yaml
├── vite.config.ts
├── tsconfig.json
├── index.html
├── src/                           # 前端
│   ├── main.tsx
│   ├── App.tsx
│   ├── routes.tsx
│   ├── pages/
│   │   ├── Onboarding.tsx
│   │   ├── Device.tsx
│   │   ├── Sync.tsx
│   │   ├── History.tsx
│   │   ├── Trash.tsx
│   │   └── Settings.tsx
│   ├── components/
│   ├── hooks/
│   ├── lib/
│   │   ├── api.ts                 # 封装 invoke / Channel / listen
│   │   └── types.ts               # 与 Rust 端共享的类型（手工对齐）
│   └── styles/
│       └── index.css              # @import "tailwindcss";
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── build.rs
    ├── icons/
    ├── capabilities/
    │   └── default.json
    ├── migrations/                # sqlx migrate!() 读取
    │   └── 0001_init.sql
    └── src/
        ├── main.rs                # 仅 main
        ├── lib.rs                 # 应用装配
        ├── error.rs               # 统一错误类型 + 转 ipc::Error
        ├── config/
        │   ├── mod.rs             # 设备/根目录/保留天数等
        │   └── credential.rs      # keyring 封装
        ├── db/
        │   ├── mod.rs             # 连接池初始化、迁移
        │   ├── models.rs          # FileEntry / TrashEntry / SyncRun 等
        │   └── queries.rs         # SQL 查询集中地
        ├── ftp/
        │   ├── mod.rs             # 连接封装、连接池
        │   ├── walker.rs          # MLSD 递归扫描
        │   └── downloader.rs      # 流式下载 + REST 续传 + .part 原子重命名
        ├── sync/
        │   ├── mod.rs             # SyncEngine：编排 walker + diff + downloader
        │   ├── diff.rs            # 远端清单 vs 本地清单
        │   └── progress.rs        # 节流 + 聚合，写入 Channel
        ├── trash/
        │   ├── mod.rs             # 软删除入站
        │   └── gc.rs              # 后台清理过期项
        ├── tray.rs                # 托盘与窗口关闭拦截
        ├── commands/
        │   ├── mod.rs
        │   ├── device.rs
        │   ├── sync.rs
        │   ├── history.rs
        │   ├── trash.rs
        │   └── settings.rs
        └── events.rs              # 事件名常量 + payload struct
```

## 3. 数据模型

### 3.1 SQLite schema（迁移 `0001_init.sql`）

```sql
-- 1) 设备配置（当前只支持 1 台，但保留多记录能力以备未来扩展）
CREATE TABLE devices (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,                      -- 用户起的别名
    host            TEXT    NOT NULL,                      -- IP
    port            INTEGER NOT NULL DEFAULT 1024,
    username        TEXT    NOT NULL,                      -- 密码不在表里，存 keyring
    is_active       INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
    backup_root     TEXT    NOT NULL,                      -- PC 端备份根目录绝对路径
    created_at      INTEGER NOT NULL,                      -- unix ms
    updated_at      INTEGER NOT NULL
);

-- 2) 本地文件清单（每条 = 备份根下的一个文件）
CREATE TABLE file_index (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id       INTEGER NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    rel_path        TEXT    NOT NULL,                      -- 相对手机 /sdcard 的路径（也用作本地相对路径）
    size            INTEGER NOT NULL,
    mtime_unix      INTEGER NOT NULL,                      -- 远端 MDTM 转 unix 秒
    local_status    TEXT    NOT NULL CHECK (local_status IN ('present','partial','missing')),
    last_synced_at  INTEGER NOT NULL,
    UNIQUE (device_id, rel_path)
);
CREATE INDEX idx_file_device ON file_index(device_id);

-- 3) 同步运行记录
CREATE TABLE sync_runs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id       INTEGER NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    started_at      INTEGER NOT NULL,
    ended_at        INTEGER,                                -- NULL = 进行中
    status          TEXT    NOT NULL CHECK (status IN ('running','succeeded','partial','failed','aborted')),
    added           INTEGER NOT NULL DEFAULT 0,
    updated         INTEGER NOT NULL DEFAULT 0,
    deleted         INTEGER NOT NULL DEFAULT 0,
    failed          INTEGER NOT NULL DEFAULT 0,
    bytes_downloaded INTEGER NOT NULL DEFAULT 0,
    error_summary   TEXT
);

-- 4) 单次运行内的失败明细
CREATE TABLE sync_failures (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id          INTEGER NOT NULL REFERENCES sync_runs(id) ON DELETE CASCADE,
    rel_path        TEXT    NOT NULL,
    reason          TEXT    NOT NULL
);

-- 5) 软删除回收站
CREATE TABLE trash (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id       INTEGER NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    original_rel    TEXT    NOT NULL,                      -- 原相对路径
    trash_rel       TEXT    NOT NULL,                      -- .trash 下的相对路径
    size            INTEGER NOT NULL,
    deleted_at      INTEGER NOT NULL,
    expire_at       INTEGER NOT NULL                       -- deleted_at + 30 days
);
CREATE INDEX idx_trash_expire ON trash(expire_at);

-- 6) 应用配置（KV）
CREATE TABLE app_settings (
    key             TEXT PRIMARY KEY,
    value           TEXT NOT NULL
);
-- 已知 key：
--   retention_days  → "30"
--   auto_start      → "false"
--   concurrency     → "4"
--   include_globs   → JSON array，下文 4.2 说明
--   exclude_globs   → JSON array
```

### 3.2 内存中的核心结构（Rust）

```rust
pub struct FileEntry {
    pub rel_path: String,
    pub size: u64,
    pub mtime: i64,        // unix 秒
}

pub enum DiffItem {
    Add(FileEntry),                 // 远端新增 → 下载
    Update(FileEntry),              // size/mtime 变化 → 重下
    DeleteLocal { rel_path: String, size: u64 },  // 远端没了 → 移入 trash
}

pub struct DeviceConfig {
    pub id: i64,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub backup_root: PathBuf,
    pub keyring_service: String,     // 固定 "albumsync"
}
```

## 4. 关键流程

### 4.1 首次启动 / 引导

1. PC 端启动后检查 `devices` 表是否有 `is_active=1` 行。
2. 无 → 进入 Onboarding：
   - Step 1：选择备份根目录（默认 `<User>\Pictures\PhoneBackup`）。
   - Step 2：图文说明手机端如何装 Primitive FTPd + 启动 + 记下 IP。
   - Step 3：填 host/port/username/password + 点"测试连接"（Rust 端用 suppaftp 拨测）。
   - Step 4：保存到 `devices` 表 + 密码写 keyring。

### 4.2 同步范围筛选

- 默认 `include_globs`：
  ```json
  [
    "DCIM/**/*",
    "Pictures/**/*",
    "Tencent/MicroMsg/WeiXin/**/*",
    "Tencent/QQ_Images/**/*",
    "Movies/**/*"
  ]
  ```
- 默认 `exclude_globs`：
  ```json
  [
    "**/.thumbnails/**",
    "**/cache/**",
    "**/*.tmp"
  ]
  ```
- 扩展名白名单（在 walker 内 hardcode，简单可靠）：
  - 图片：jpg / jpeg / png / heic / heif / webp / gif / bmp / tiff / raw / dng / arw / cr2 / nef
  - 视频：mp4 / mov / 3gp / mkv / avi / m4v / webm
- 同一文件需同时通过 include + 扩展名白名单且不命中 exclude。

### 4.3 单次同步（手动触发）

```
[User clicks "立即同步"]
      │
      ▼
SyncEngine::run(device_id, channel) ────────────────────────────────────────
  1. mark sync_runs row as 'running'
  2. walker.scan_remote()
        ─ open AsyncFtpStream, login, binary mode
        ─ recursive MLSD from /sdcard
        ─ apply include/exclude/ext filters
        ─ return Vec<FileEntry>
  3. local = SELECT * FROM file_index WHERE device_id=?
  4. diff = compute_diff(local, remote)
  5. send Channel::Started { total_files, total_bytes, deletes }
  6. concurrent pool (size=4):
        for each Add / Update:
            downloader.download(entry)
                ─ build local_path, ensure parent dir
                ─ if .part exists: resume_transfer(part_size)
                ─ retr_as_stream → write loop → finalize_retr_stream
                ─ rename .part → final
                ─ INSERT/UPDATE file_index
                ─ send Channel::Progress { bytes, file_name, done, total }
        for each DeleteLocal:
            trash::move_to_trash(entry)
                ─ move file → .trash/{rel}__{ts}/...
                ─ INSERT trash row (expire_at = now + 30d)
                ─ DELETE FROM file_index
  7. mark sync_runs row 'succeeded'/'partial'/'failed'
  8. send Channel::Finished { stats }
```

### 4.4 软删除 GC

- 启动时立刻跑一次 + 每 6 小时跑一次。
- 逻辑：`DELETE FROM trash WHERE expire_at < now`，对每个删除的行再 `fs::remove_file(backup_root + trash_rel)`，悄悄忽略文件已不存在的情况。
- GC 与同步并发安全：用 `tokio::sync::Mutex` 保护"是否正在同步"标志，GC 在同步进行时跳过本次。

### 4.5 关闭窗口 → 最小化到托盘

```rust
main_win.on_window_event(move |event| {
    if let WindowEvent::CloseRequested { api, .. } = event {
        let _ = main_win.hide();
        api.prevent_close();
    }
});
```

### 4.6 托盘菜单

- `显示主窗口` → `get_webview_window("main").show() + set_focus()`
- `立即同步` → invoke 内部命令 `sync_start`（与前端按钮同路径）
- `退出` → `app.exit(0)`
- 双击托盘 = 显示主窗口
- 右键 = 系统弹出菜单

### 4.7 单实例

- 启用 `tauri-plugin-single-instance`，新实例回调中调出现有主窗口。

## 5. 前后端 API 契约

### 5.1 Tauri commands（`#[tauri::command]`）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `get_active_device` | – | `Option<DeviceDto>` | 启动时调用，决定是否进引导 |
| `save_device` | `DeviceForm { name, host, port, username, password, backup_root }` | `DeviceDto` | 写表 + 写 keyring |
| `test_connection` | `DeviceForm` | `ConnectionResult { ok, server_banner?, error? }` | 用 suppaftp 拨测 |
| `delete_device` | `id` | `()` | 同时 keyring 清理 |
| `device_status` | `id` | `DeviceStatus { online, latency_ms? }` | UI 轮询用，内部走 TCP 探测缓存 |
| `sync_start` | `device_id`, `on_event: Channel<SyncEvent>` | `()` | 启动后台任务 + 通过 channel 推进度 |
| `sync_abort` | `device_id` | `()` | 设置取消标志，等待 inflight 完成 |
| `list_sync_runs` | `device_id`, `limit` | `Vec<SyncRunDto>` | History 页 |
| `list_trash` | `device_id`, `search?` | `Vec<TrashItemDto>` | Trash 页 |
| `restore_trash` | `Vec<i64>` | `()` | 文件移回 + 记录 INSERT 回 file_index |
| `purge_trash` | `Vec<i64>` | `()` | 立刻删 |
| `get_settings` | – | `SettingsDto` | retention/concurrency/include/exclude |
| `update_settings` | `SettingsDto` | `()` | 保存到 app_settings 表 |

### 5.2 Channel 事件类型（高频进度）

```rust
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum SyncEvent {
    Started   { run_id: i64, total_files: u64, total_bytes: u64, deletes: u32 },
    Progress  { done_files: u64, done_bytes: u64, current_file: String, speed_bps: u64 },
    FileDone  { rel_path: String, action: FileAction },           // 单个文件结束（用于实时表格更新）
    Failed    { rel_path: String, reason: String },
    Finished  { run_id: i64, added: u32, updated: u32, deleted: u32, failed: u32 },
    Aborted   { run_id: i64 },
}
```

进度节流策略：Rust 端聚合 200ms 一次 `Progress` 推送；`FileDone`/`Failed` 即时发送（频率低）。

### 5.3 全局广播事件（低频）

| 事件名 | payload | 触发场景 |
|---|---|---|
| `device-status-changed` | `{ deviceId, online }` | 后台 TCP 探测变化 |
| `trash-gc-finished` | `{ purged, freedBytes }` | GC 完成 |
| `app-error` | `{ where, message }` | 未捕获异常兜底 |

## 6. 错误处理

- Rust 端定义统一 `AlbumError`（用 `thiserror`），分类：`Ftp`, `Db`, `Io`, `Keyring`, `Config`, `Cancelled`, `Other`。
- 所有 `#[tauri::command]` 返回 `Result<T, AlbumError>`，实现 `serde::Serialize` 后前端直接拿到结构化错误码与文案。
- 同步过程中**单文件失败不中断整个任务**：写入 `sync_failures` 表 + Channel `Failed` 事件，最终 `Finished.failed` 计数；用户在 History 页可下钻看具体哪几张。

## 7. 并发与取消

- 同步任务由 `tokio::task::spawn` 启动。
- 4 路并发用 `tokio::sync::Semaphore::new(4)` 控制下载槽位。
- 取消通过 `tokio_util::sync::CancellationToken`：`sync_abort` 命令调 `token.cancel()`，下载循环每次 `read` 间隙 `token.is_cancelled()` 检查；已写到 `.part` 的字节保留，下次续传。

## 8. 安全

- FTP 协议明文，**只能在局域网内用**，README 中红字提示。
- 密码：
  - 不写 `devices` 表
  - 不写 `app_settings` 表
  - 不写日志
  - `test_connection` 在内存中用完即释放
  - 存储用 `keyring::Entry::new("albumsync", &username)`
- 备份根目录 `..` 防护：写入前用 `path.canonicalize()` 后断言以 `backup_root` 开头，防止远端给出 `../../etc/passwd` 这种相对路径攻击（虽然 Primitive FTPd 不至于这么干，但要兜住）。

## 9. 日志

- 用 `tracing` + `tracing-subscriber`：JSON 行格式，每天滚动，保留 7 天。
- 日志位置：`<app_data_dir>/logs/albumsync-YYYYMMDD.log`。
- 启动日志：版本、配置摘要（脱敏后）、数据库迁移结果。
- 同步日志：每个文件 1 行（成功/失败），运行汇总 1 行。
- **不打印密码 / FTP 完整 URL**（URL 里可能带认证）。

## 10. 配置文件

- 不引入 `config.toml`——所有可变配置存数据库 `app_settings` 表。理由：减少"代码 vs 配置 vs 数据库"三态不一致。
- Tauri 自身用 `tauri.conf.json`（构建期）。

## 11. 测试策略

- **单元**：`diff` 算法（构造 mock 本地+远端清单）、`include/exclude/ext` 过滤、trash 路径生成、keyring 包装。
- **集成**：起一个本地 vsftpd / pyftpdlib 做远端，跑同步引擎；验证：首次全量 → 增量为空 → 修改一文件 → 重下 → 删一文件 → 入 trash。
- **手工 e2e**：用真手机 + Primitive FTPd 走 5.1 流程。

## 12. 技术栈锁定（来自调研）

| 维度 | 选择 |
|---|---|
| 桌面框架 | Tauri 2（>= 2.9.x，以脚手架时 `cargo add` 实际版本为准） |
| 前端 | React 18 + TypeScript 5.x + Vite 5/6 + Tailwind CSS v4 (`@tailwindcss/vite`) |
| 包管理 | pnpm |
| Rust 运行时 | tokio（multi-thread runtime） |
| FTP | `suppaftp`（`tokio` feature） |
| DB | `sqlx`（`runtime-tokio`, `sqlite`, `macros`, `migrate`） |
| 凭据 | `keyring`（最新稳定版） |
| 单实例 | `tauri-plugin-single-instance` |
| 日志 | `tracing` + `tracing-subscriber` |
| 错误 | `thiserror` + `anyhow`（仅命令行入口处兜底） |

> **注**：调研里出现的具体版本号（如 keyring 0.2+、Tauri 2.9.5）作为参考。脚手架阶段以 `cargo add` 拿到的最新稳定版为准，并将实际版本回填进 `Cargo.toml`，本节如有出入以代码为准。

## 13. 开发里程碑

1. **M0 — 仓库与文档**：✅ 已完成（REQUIREMENTS + DESIGN）
2. **M1 — 脚手架可跑**：`pnpm tauri dev` 起空白窗口；后端能 `tracing::info`；数据库初始化通过迁移。
3. **M2 — 后端核心**：FTP 拨测 / MLSD 扫描 / 流式下载 / diff / 软删除全跑通（在测试 FTP 服务器上）。
4. **M3 — 前端可用**：4 个主要页面接通；Channel 进度流通；托盘 + 关闭最小化。
5. **M4 — 端到端**：真手机 Primitive FTPd 走通 5320 张测试；处理边缘情况（断网续传、单文件失败、目录权限）。
6. **M5 — 打包 + README**：NSIS 安装包；README 含手机端图文教程。

每个 M 完成都对应一次 `feat:` 或 `chore:` git commit；M2/M3/M4 单独建 PR 也行（本项目单人无需）。

---

*更改记录*

| 日期 | 变更 | 引用 |
|---|---|---|
| 2026-06-07 | v0.1 初版 | 需求 v0.1 / 技术调研 wf_2d160d42-a80 |
