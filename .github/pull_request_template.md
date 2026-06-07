<!--
PR 提交前请确保：
  1. 已读 CONTRIBUTING.md
  2. 本地 `cargo fmt && cargo clippy -- -D warnings && pnpm typecheck && pnpm build` 全部通过
  3. commit message 遵循 `类型: 简述` 格式（feat / fix / docs / refactor / chore / test / perf）
-->

## 这次改了什么 / Summary

<!-- 一两句话说明 WHY，不需要描述 WHAT —— diff 已经在说 WHAT 了 -->

## 关联 issue

Closes #

## 测试 / Test plan

<!-- 列出你做了哪些验证，便于 reviewer 重现 -->

- [ ]
- [ ]

## 检查清单 / Checklist

- [ ] 仅改动了 PR 描述里说的范围（没有顺手"美化"无关代码）
- [ ] 新增 / 修改的代码遵循「最小依赖」原则
- [ ] 注释只解释 **WHY**，不解释 WHAT
- [ ] 涉及 Rust ↔ TS 契约改动时，已同时更新 `src-tauri/src/events.rs` 和 `src/lib/api.ts`
- [ ] 涉及用户可见行为改动时，已同步更新 README / CHANGELOG.md
