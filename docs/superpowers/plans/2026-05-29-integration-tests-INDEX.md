# 集成测试实施计划 — INDEX

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement these plans task-by-task. Each task lives in its own plan file and uses checkbox (`- [ ]`) syntax for tracking.

**Spec**：`docs/superpowers/specs/2026-05-29-integration-testing-conventions-design.md`
**Goal**：按 spec 为 8 个 crate 补集成测试。
**Architecture**：每个 task 一个独立 plan 文件，命名 `2026-05-29-integration-tests-T<NN>-<slug>.md`；按 4 个 batch 顺序推进，batch 内可并行。
**Tech Stack**：Rust（workspace），sqlx + SQLite，tokio + axum，wiremock，tempfile。

---

## Batch 顺序与依赖

| Batch | 内容 | 依赖 |
|---|---|---|
| 1 | `kfcode-storage` 集成测试 + 两项源码修复 | 无 |
| 2 | `kfcode-mcp` + `kfcode-lsp` 集成测试 | 无（与 batch 1 并行可行，但建议在 batch 1 后） |
| 3 | `kfcode-server` 集成测试 + Database 注入改造 | batch 1（依赖 storage 注入入口） |
| 4 | `kfcode-watcher` / `kfcode-tool` / `kfcode-permission` / `kfcode-command` 集成测试 | 无 |

每个 batch 在独立 git worktree 内执行，完成后 PR 合入 main。

---

## Task 列表

### Batch 1 — kfcode-storage

- T01：`2026-05-29-integration-tests-T01-storage-scaffold.md` — dev-dep 与 `tests/common/` 骨架
- T02：`2026-05-29-integration-tests-T02-storage-pragma-foreign-keys.md` — 源码修复 `PRAGMA foreign_keys=ON`
- T03：`2026-05-29-integration-tests-T03-storage-open-at.md` — 源码新增 `Database::open_at(path)`
- T04：`2026-05-29-integration-tests-T04-storage-session-repo.md` — `SessionRepository` CRUD
- T05：`2026-05-29-integration-tests-T05-storage-message-repo.md` — `MessageRepository` CRUD
- T06：`2026-05-29-integration-tests-T06-storage-todo-repo.md` — `TodoRepository` CRUD
- T07：`2026-05-29-integration-tests-T07-storage-parts-share.md` — `PartRepository` + `ShareRepository`（公开模块路径）
- T08：`2026-05-29-integration-tests-T08-storage-permissions-schema.md` — `permissions` 表 schema/migration（最小 SQL）
- T09：`2026-05-29-integration-tests-T09-storage-behavior.md` — 事务、并发、JSON 容错降级、迁移幂等

### Batch 2 — kfcode-mcp + kfcode-lsp

- T10：`2026-05-29-integration-tests-T10-mcp-scaffold.md` — mcp dev-dep + `tests/common/`
- T11：`2026-05-29-integration-tests-T11-mcp-auth-injection.md` — auth.rs 显式 path 注入改造
- T12：`2026-05-29-integration-tests-T12-mcp-http-golden-path.md` — wiremock 全协议序列（initialize/tools/list/tools/call）
- T13：`2026-05-29-integration-tests-T13-mcp-http-errors.md` — HTTP 4xx/5xx + JSON-RPC error envelope + 超时
- T14：`2026-05-29-integration-tests-T14-mcp-sse.md` — SSE transport（本地 axum server）
- T15：`2026-05-29-integration-tests-T15-mcp-oauth.md` — OAuth flow（wiremock + 注入 path）
- T16：`2026-05-29-integration-tests-T16-lsp-scaffold.md` — lsp dev-dep + `[[bin]]` helper stub fixture
- T17：`2026-05-29-integration-tests-T17-lsp-golden-path.md` — initialize → didOpen → didChange → diagnostics → shutdown
- T18：`2026-05-29-integration-tests-T18-lsp-errors.md` — server 不响应、返回 error、协议不匹配
- T19：`2026-05-29-integration-tests-T19-lsp-features.md` — goto_definition / hover / references

### Batch 3 — kfcode-server

- T20：`2026-05-29-integration-tests-T20-server-scaffold.md` — dev-dep（tower util、reqwest、tokio-tungstenite）+ `tests/common/`
- T21：`2026-05-29-integration-tests-T21-server-database-injection.md` — `ServerState` Database 注入改造
- T22：`2026-05-29-integration-tests-T22-server-global-health.md` — `/global/health` 黄金路径（用 oneshot）
- T23：`2026-05-29-integration-tests-T23-server-config.md` — `/config` GET + CORS 行为
- T24：`2026-05-29-integration-tests-T24-server-session.md` — `/session` POST/GET + 跨 storage 协作 + 测试隔离
- T25：`2026-05-29-integration-tests-T25-server-provider-file.md` — `/provider` GET + `/file/content` GET
- T26：`2026-05-29-integration-tests-T26-server-errors.md` — `ApiError` 400/404/502/500 全分支
- T27：`2026-05-29-integration-tests-T27-server-pty-websocket.md` — `/pty/{id}/connect` WebSocket（真端口 + tokio-tungstenite）

### Batch 4 — watcher + tool + permission + command

- T28：`2026-05-29-integration-tests-T28-watcher-scaffold.md` — dev-dep + `tests/common/`
- T29：`2026-05-29-integration-tests-T29-watcher-single-path.md` — 单路径 create/modify/delete
- T30：`2026-05-29-integration-tests-T30-watcher-multi-path-errors.md` — 多路径 + 不存在路径错误 + 重复 watch
- T31：`2026-05-29-integration-tests-T31-tool-scaffold.md` — dev-dep + `tests/common/`
- T32：`2026-05-29-integration-tests-T32-tool-registry.md` — register / get / list / list_schemas
- T33：`2026-05-29-integration-tests-T33-tool-builtin-read-write.md` — 内置 read / write / edit 工具（tempdir）
- T34：`2026-05-29-integration-tests-T34-tool-permission-callback.md` — deny / allow callback 行为
- T35：`2026-05-29-integration-tests-T35-permission-scaffold.md` — `tests/common/`
- T36：`2026-05-29-integration-tests-T36-permission-ruleset.md` — `evaluate` / `disabled` / `build_agent_ruleset`
- T37：`2026-05-29-integration-tests-T37-permission-engine.md` — `PermissionEngine` pending → respond → approved
- T38：`2026-05-29-integration-tests-T38-command-scaffold.md` — `tests/common/` + builtin 测试
- T39：`2026-05-29-integration-tests-T39-command-load-parse.md` — `load_from_directory` + `parse` + 描述提取 + 名冲突覆盖

---

## 共同约定（全部 batch 适用）

- 测试函数名行为式、不带 `test_` 前缀（spec §2.2）
- 每个测试文件 `mod common;`；`tests/common/mod.rs` 顶部 `#![allow(dead_code)]`（spec §2.1）
- 异步测试用 `#[tokio::test]`，默认单线程；并发场景才用 `multi_thread`（spec §2.3）
- 不引入 `rstest` / `serial_test` / `proptest` / coverage 工具（spec §2.4）
- 不读写用户真实 `data_dir` / `data_local_dir` / `home_dir`（spec §2.5）
- 源码修改限于真实 bug、可测性注入点、spec 切面缺陷（spec §2.8），单独 commit

每个 plan 文件遵循 superpowers 的 plan 模板：开头有 header（Goal / Architecture / Tech Stack）、TDD 步骤、commit 步骤。
