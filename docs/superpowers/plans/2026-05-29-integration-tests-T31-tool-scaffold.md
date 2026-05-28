# T31 — kfcode-tool 集成测试脚手架

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 准备 tool 集成测试目录、helper 与 dev-deps。

**Architecture:** `tests/common/mod.rs` 提供 `fresh_default_registry()` 与 `make_ctx(tempdir)`。tool 默认 registry 包含 21 个内置工具。

**Tech Stack:** kfcode-tool / tempfile / tokio。

**依赖:** 无

---

### Task 4.3：tool 脚手架

**Files:**
- Modify: `crates/kfcode-tool/Cargo.toml`（dev-deps 加 tempfile，tokio-test）
- Create: `crates/kfcode-tool/tests/common/mod.rs`
- Create: `crates/kfcode-tool/tests/smoke.rs`

- [ ] **Step 1: dev-deps**

修改 `crates/kfcode-tool/Cargo.toml`：

```toml
[dev-dependencies]
tempfile = { workspace = true }
tokio-test = "0.4"
```

- [ ] **Step 2: common helper**

写入 `crates/kfcode-tool/tests/common/mod.rs`：

```rust
#![allow(dead_code)]

use kfcode_tool::registry::{create_default_registry, ToolRegistry};
use kfcode_tool::tool::ToolContext;
use tempfile::TempDir;

pub async fn fresh_default_registry() -> ToolRegistry {
    create_default_registry().await
}

pub fn make_ctx(directory: &str) -> ToolContext {
    ToolContext::new(
        "ses-test".into(),
        "msg-test".into(),
        directory.into(),
    )
}

pub fn fresh_workspace() -> TempDir {
    TempDir::new().expect("tempdir")
}
```

- [ ] **Step 3: smoke 测试**

写入 `crates/kfcode-tool/tests/smoke.rs`：

```rust
mod common;

#[tokio::test]
async fn default_registry_has_builtin_tools() {
    let r = common::fresh_default_registry().await;
    let ids = r.list_ids().await;
    assert!(!ids.is_empty(), "default registry must have tools");
    // sanity check：read / write / bash 三个常用工具应在
    for id in ["read", "write", "bash"] {
        assert!(ids.contains(&id.to_string()), "missing tool: {id}; got {ids:?}");
    }
}
```

- [ ] **Step 4: 跑测试**

```
cargo test -p kfcode-tool --test smoke
```

预期：1 条 pass。

- [ ] **Step 5: 提交**

```bash
git add crates/kfcode-tool/Cargo.toml crates/kfcode-tool/tests/
git commit -m "test(tool): scaffold integration tests"
```
