# GitHub Release 升级流程重写 — 实现计划索引

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 KFCode 的升级流程整体替换为以 GitHub Release(`dfbb/KFCode/releases`)为唯一版本源的自下载替换方案。

**Architecture:** 拆分两个落点 —— 轻量检查(查 Release 版本 + 三段版本比较 + 缓存)放进 `kfcode-util`(feature `upgrade-check` 门控,cli/tui 共享);重量执行(下载 + sha256 校验 + 解压 + self-replace)放进 `kfcode-cli`。TUI 启动时异步调用 util 检查,空闲时经 `CustomEvent` 投递提示;CLI 提供 `kfcode upgrade` 命令。依赖方向:cli/tui → util(TUI 不反向依赖 cli)。

**Tech Stack:** Rust, reqwest(下载/查询), self-replace(原子替换), flate2+tar(解 tar.gz), zip(解 zip), sha2(校验), std::sync::mpsc(TUI 事件)。

**Spec:** `docs/superpowers/specs/2026-05-29-github-release-upgrade-design.md`

---

## Task 文件与并行性

每个 task 是一个独立文件。"可并行"指该 task 不依赖其他未完成 task 的产物,可由不同 worker 同时进行。

| Task | 文件 | 内容 | 依赖 | 可并行 |
|------|------|------|------|--------|
| T01 | `...-T01-util-version-compare.md` | util:三段版本解析与比较(纯函数 + 单测) | 无 | ✅ 起点,可与 T03 并行 |
| T02 | `...-T02-util-release-check.md` | util:Release 查询 + 缓存读写 + feature 门控 | T01 | ⛔ 依赖 T01 的 `parse_version`/`is_newer` |
| T03 | `...-T03-cli-asset-resolve.md` | cli:平台→triple→asset 名推导(纯函数 + 单测) | 无 | ✅ 可与 T01/T02 并行 |
| T04 | `...-T04-cli-download-replace.md` | cli:下载 + sha256 校验 + 解压 + self-replace | T03 | ⛔ 依赖 T03 的 asset 推导 |
| T05 | `...-T05-cli-wire-command.md` | cli:删旧升级代码 + 重写 `upgrade` 命令接线 | T02, T04 | ⛔ 依赖 T02+T04 |
| T06 | `...-T06-tui-startup-check.md` | tui:`CustomEvent` + 启动异步检查 + 空闲提示 | T02 | ⛔ 依赖 T02;可与 T04/T05 并行 |

**推荐执行顺序:**
- 第一波(并行):**T01** 和 **T03**
- 第二波(并行):**T02**(待 T01)和 **T04**(待 T03)
- 第三波(并行):**T05**(待 T02+T04)和 **T06**(待 T02)

**全部完成后的验收**(在 T05、T06 各自最后一步已覆盖,这里汇总):
- `cargo build --workspace` 通过
- `cargo test -p kfcode-util -p kfcode-cli` 通过
- `cargo build -p kfcode-cli` 后 `./target/debug/kfcode upgrade --help` 不再显示 `--method`/`TARGET`
- 旧符号 `InstallMethod`/`detect_install_method`/`run_upgrade_process` 在 main.rs 中已无残留(`grep` 验证)
