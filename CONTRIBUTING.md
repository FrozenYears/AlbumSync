# 开发指南

> 项目级开发环境约定。用户手册见后续 README.md（M5 阶段产出）。

## 前置依赖

| 软件 | 版本 | 检查命令 |
|---|---|---|
| Visual Studio 2022 Community | 17.x，含 "C++ build tools" workload | `vswhere -latest -property displayName` |
| Rust（rustup） | stable-x86_64-pc-windows-msvc | `rustc --version` |
| Node.js | >= 22 | `node -v` |
| pnpm | 11.5.2（packageManager 已锁定） | `corepack pnpm --version` |
| git | 任意 | `git --version` |

## 一次性环境配置

```bash
# 1) 安装 Rust（用户级，不写系统 PATH）
winget install --id Rustlang.Rustup --silent --accept-source-agreements --accept-package-agreements
rustup default stable-x86_64-pc-windows-msvc

# 2) 激活 corepack（自带于 Node 22+）→ 让 pnpm 命令可用
corepack enable --install-directory "$(npm config get prefix)"
```

> rustup 装到 `%USERPROFILE%\.cargo` 与 `.rustup`，是用户级而非系统级。

## 项目级配置（已落库）

- **`rust-toolchain.toml`** 锁定 Rust 工具链版本与组件。
- **`pnpm-workspace.yaml`** 配置 pnpm 11+ 的 `allowBuilds` 白名单（仅 esbuild / @tauri-apps/cli 允许执行 install 脚本）。
- **`.cargo/config.toml`**：
  - 指向中科大 crates.io sparse 镜像（解决国内 crates.io 访问慢）
  - 显式指定 MSVC `link.exe` 绝对路径（绕过 git bash 里 `/usr/bin/link` 的 MSYS2 coreutils 同名命令劫持）

## 首次拉取后

```bash
pnpm install        # 装 React/Vite/Tauri CLI（约 130 个包）
cd src-tauri
cargo check         # 拉 Rust 依赖并通过类型检查（首次 5-10 分钟）
```

## 日常开发

```bash
pnpm tauri dev      # 起 Vite + Tauri 双进程，热重载
```

主窗口启动后：

- Rust 端日志走 `tracing`，输出到终端
- 前端日志走 DevTools（右键窗口 → Inspect）
- 修改 `src/` 触发 Vite HMR
- 修改 `src-tauri/` 触发 cargo 重新编译并重启窗口

## 编译验证

```bash
pnpm typecheck         # tsc --noEmit，仅类型检查
cd src-tauri
cargo check            # Rust 类型检查
cargo clippy --all-targets --no-deps -- -D warnings   # Rust lint，警告即错误
cargo fmt --check      # 格式
```

## 打包发布

```bash
pnpm tauri build       # NSIS 安装包，产物在 src-tauri/target/release/bundle/nsis/
```

## 仓库约定

- 分支：`main` 直推，单人项目无需 PR。多人协作时按 `feat/`、`fix/`、`docs/` 前缀创建。
- Commit 信息格式（Conventional Commits）：`<type>: <短句>`，type 取 `feat`/`fix`/`docs`/`chore`/`refactor`/`test`。
- 重大设计变更先改 `DESIGN.md`，再改代码。
- 新增依赖必须在 `DESIGN.md` 12.2 / 12.3 节登记"不可替代理由"。

## 故障排除

| 现象 | 原因 | 解决 |
|---|---|---|
| `cargo check` 链接错误 `extra operand` | git bash 的 `/usr/bin/link` 劫持 | 已通过 `.cargo/config.toml` 显式 linker 路径修复 |
| `pnpm install` 报 `ERR_PNPM_IGNORED_BUILDS` | pnpm 11 默认阻止非白名单 build script | 已通过 `pnpm-workspace.yaml` 的 `allowBuilds` 修复 |
| crates.io 下载慢 | 国内访问 | 已切到中科大 sparse 镜像 |
| VS 升级后链接失败 | `link.exe` 路径中的版本号变了 | 用 `vswhere` 重新查找，更新 `.cargo/config.toml` |
