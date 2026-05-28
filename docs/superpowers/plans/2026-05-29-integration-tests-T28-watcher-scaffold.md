# T28 — kfcode-watcher 集成测试脚手架

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 准备 watcher 测试目录、helper、与 dev-deps。

**Architecture:** `tests/common/mod.rs` 提供 `tempdir + FileWatcher::new(default config)` helper；测试用 `subscribe()` 拿 broadcast receiver 并 `recv().await`。

**Tech Stack:** notify 7 / tokio broadcast / tempfile。

**依赖:** 无

---

### Task 4.0：watcher 脚手架

**Files:**
- Modify: `crates/kfcode-watcher/Cargo.toml`（新增 `[dev-dependencies]`，已有 tempfile 则跳过）
- Create: `crates/kfcode-watcher/tests/common/mod.rs`
- Create: `crates/kfcode-watcher/tests/smoke.rs`

- [ ] **Step 1: dev-deps**

修改 `crates/kfcode-watcher/Cargo.toml`，确保有：

```toml
[dev-dependencies]
tempfile = { workspace = true }
tokio-test = "0.4"
```

- [ ] **Step 2: common helper**

写入 `crates/kfcode-watcher/tests/common/mod.rs`：

```rust
#![allow(dead_code)]

use kfcode_watcher::{FileWatcher, WatcherConfig};
use std::sync::Arc;
use tempfile::TempDir;

pub fn fresh_watcher() -> Arc<FileWatcher> {
    Arc::new(FileWatcher::new(WatcherConfig::default()).expect("watcher init"))
}

pub fn fresh_tempdir() -> TempDir {
    TempDir::new().expect("tempdir")
}
```

- [ ] **Step 3: smoke 测试**

写入 `crates/kfcode-watcher/tests/smoke.rs`：

```rust
mod common;

#[tokio::test]
async fn watcher_constructs_and_subscribes() {
    let w = common::fresh_watcher();
    let _rx = w.subscribe();
}
```

- [ ] **Step 4: 跑测试**

```
cargo test -p kfcode-watcher --test smoke
```

预期：1 条 pass。

- [ ] **Step 5: 提交**

```bash
git add crates/kfcode-watcher/Cargo.toml crates/kfcode-watcher/tests/
git commit -m "test(watcher): scaffold integration tests"
```
