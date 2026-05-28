# T32 — tool registry 注册 / 查找 / 列出 / schemas

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证 `ToolRegistry::{register, get, list, list_ids, list_schemas, suggest_tools, execute}` 在未知 tool 时的错误路径。

**Architecture:** 默认 registry + 直接断言。

**Tech Stack:** kfcode-tool。

**依赖:** T31

---

### Task 4.4：tool registry

**Files:**
- Create: `crates/kfcode-tool/tests/registry.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-tool/tests/registry.rs`：

```rust
mod common;

use kfcode_tool::tool::ToolError;

#[tokio::test]
async fn list_returns_all_registered() {
    let r = common::fresh_default_registry().await;
    let ids = r.list_ids().await;
    let list = r.list().await;
    assert_eq!(list.len(), ids.len());
}

#[tokio::test]
async fn get_returns_some_for_known_tool() {
    let r = common::fresh_default_registry().await;
    assert!(r.get("read").await.is_some());
}

#[tokio::test]
async fn get_returns_none_for_unknown() {
    let r = common::fresh_default_registry().await;
    assert!(r.get("definitely-not-a-tool").await.is_none());
}

#[tokio::test]
async fn list_schemas_includes_known_tools() {
    let r = common::fresh_default_registry().await;
    let schemas = r.list_schemas().await;
    let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
    for n in ["read", "write"] {
        assert!(names.contains(&n), "missing schema: {n}");
    }
}

#[tokio::test]
async fn execute_unknown_tool_returns_invalid_arguments_error() {
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let ctx = common::make_ctx(ws.path().to_str().unwrap());
    let res = r.execute("definitely-not-a-tool", serde_json::json!({}), ctx).await;
    let err = res.expect_err("expected error for unknown tool");
    match err {
        ToolError::InvalidArguments(msg) => {
            assert!(msg.contains("not found"), "got: {msg}");
        }
        other => panic!("expected InvalidArguments, got {other:?}"),
    }
}

#[tokio::test]
async fn suggest_tools_returns_nonempty_for_typo() {
    let r = common::fresh_default_registry().await;
    let suggestions = r.suggest_tools("reaad").await;
    assert!(!suggestions.is_empty(), "should suggest tools for typo");
}
```

> `ToolSchema.name` 字段是假设——按 `kfcode_tool::tool::ToolSchema` 真实字段调整（grep `pub struct ToolSchema`）。

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-tool --test registry
```

预期：6 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-tool/tests/registry.rs
git commit -m "test(tool): cover ToolRegistry list/get/list_schemas/execute-unknown/suggest"
```
