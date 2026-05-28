# T01 — kfcode-storage 集成测试脚手架

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 `kfcode-storage` 准备集成测试目录、共享 helper 与 dev-dep。

**Architecture:** 在 `crates/kfcode-storage/tests/` 下建 `common/mod.rs` 提供 `fresh_db()`（内存库）和后续 task 用到的辅助函数；`Cargo.toml` 已有 `tokio-test`，本 task 仅再加 `tempfile`。

**Tech Stack:** sqlx 0.8 / tokio / tempfile。

---

### Task 1.0：脚手架

**Files:**
- Modify: `crates/kfcode-storage/Cargo.toml:22-23`（追加 `tempfile` 到 dev-deps）
- Create: `crates/kfcode-storage/tests/common/mod.rs`
- Create: `crates/kfcode-storage/tests/smoke.rs`

- [ ] **Step 1: 在 dev-dependencies 添加 tempfile**

修改 `crates/kfcode-storage/Cargo.toml`，把 dev-dep 段改为：

```toml
[dev-dependencies]
tokio-test = "0.4"
tempfile = { workspace = true }
```

`tempfile` 已在 workspace 声明（前置 batch 0 spec 已确认）；如果 workspace `[workspace.dependencies]` 内还没列 `tempfile`，先在 workspace 根 `Cargo.toml` 加上 `tempfile = "3"`。

- [ ] **Step 2: 创建 common helper**

写入 `crates/kfcode-storage/tests/common/mod.rs`：

```rust
#![allow(dead_code)]

use kfcode_storage::{Database, DatabaseError};

/// 创建一个跑完所有迁移的 in-memory 数据库（单连接）。
pub async fn fresh_db() -> Database {
    Database::in_memory()
        .await
        .expect("init in-memory db failed")
}

/// 暴露 DatabaseError 给测试做匹配断言（避免每个测试都 use）。
pub use kfcode_storage::DatabaseError as DbErr;
```

- [ ] **Step 3: 创建 smoke 测试，验证脚手架能编译能跑**

写入 `crates/kfcode-storage/tests/smoke.rs`：

```rust
mod common;

#[tokio::test]
async fn fresh_db_initializes_without_error() {
    let _db = common::fresh_db().await;
}
```

- [ ] **Step 4: 跑测试**

```
cargo test -p kfcode-storage --test smoke
```

预期：`test fresh_db_initializes_without_error ... ok`，1 passed。

- [ ] **Step 5: 提交**

```bash
git add crates/kfcode-storage/Cargo.toml crates/kfcode-storage/tests/
git commit -m "$(cat <<'EOF'
test(storage): scaffold integration tests

Add tempfile dev-dep and tests/common/mod.rs with fresh_db helper.
Smoke test verifies in-memory Database::in_memory() works under the
new test harness.
EOF
)"
```
