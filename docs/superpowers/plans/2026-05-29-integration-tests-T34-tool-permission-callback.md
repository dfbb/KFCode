# T34 — tool 权限 callback 行为（deny / allow）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证 `ToolContext::ask_permission` callback 透传到工具行为：deny callback 必须让 tool 失败，且不写文件副作用；allow callback 必须让正常路径通过。spec §3.2 tool 权限协作的核心切面。

**Architecture:** 把 `ctx.ask` 设成自定义 closure；用 write 工具试跑——deny 时文件**不应**被创建，allow 时正常。

**Tech Stack:** kfcode-tool / tempfile。

**依赖:** T31 / T33

---

### Task 4.6：tool 权限 callback

**Files:**
- Create: `crates/kfcode-tool/tests/permission_callback.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-tool/tests/permission_callback.rs`：

```rust
mod common;

use std::sync::Arc;
use kfcode_tool::tool::{PermissionRequest, ToolError};

fn deny_callback() -> kfcode_tool::tool::AskCallback {
    Arc::new(|_req: PermissionRequest| {
        Box::pin(async move {
            Err::<(), _>(ToolError::PermissionDenied("denied by test".into()))
        })
    })
}

fn allow_callback() -> kfcode_tool::tool::AskCallback {
    Arc::new(|_req: PermissionRequest| {
        Box::pin(async move { Ok::<(), ToolError>(()) })
    })
}

#[tokio::test]
async fn write_aborts_when_callback_denies() {
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let target = ws.path().join("nope.txt");

    let mut ctx = common::make_ctx(ws.path().to_str().unwrap());
    ctx.ask = Some(deny_callback());

    let res = r
        .execute(
            "write",
            serde_json::json!({
                "filePath": target.to_str().unwrap(),
                "content": "should not write"
            }),
            ctx,
        )
        .await;
    let err = res.expect_err("write must fail when permission denied");
    match err {
        ToolError::PermissionDenied(_) => {}
        other => panic!("expected PermissionDenied, got {other:?}"),
    }
    assert!(!target.exists(), "denied write must not create the file");
}

#[tokio::test]
async fn write_proceeds_when_callback_allows() {
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let target = ws.path().join("ok.txt");

    let mut ctx = common::make_ctx(ws.path().to_str().unwrap());
    ctx.ask = Some(allow_callback());

    r.execute(
        "write",
        serde_json::json!({
            "filePath": target.to_str().unwrap(),
            "content": "ok"
        }),
        ctx,
    )
    .await
    .expect("write should succeed when allowed");
    assert!(target.exists());
}

#[tokio::test]
async fn no_callback_defaults_to_allow() {
    // 默认 ctx 无 ask callback；write 必须成功（spec §3.2 tool 权限协作描述）
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let target = ws.path().join("default.txt");

    let ctx = common::make_ctx(ws.path().to_str().unwrap());
    r.execute(
        "write",
        serde_json::json!({
            "filePath": target.to_str().unwrap(),
            "content": "x"
        }),
        ctx,
    )
    .await
    .expect("default ctx should allow");
    assert!(target.exists());
}
```

> 如 `write` 工具的 `PermissionRequest` 不通过 `ctx.ask` 而是另一条路径（如 `external_directory` 检查），该测试会显示"write 不调用 ask"——这与 spec §3.2 描述一致：默认 ctx 直接 allow。把第 1 条断言放宽：write 在 deny callback 下若仍成功，意味着 write 没走 ctx.ask；测试转为"用一个**确实**走 ctx.ask 的工具"——`bash` 或 `task` 可能合适。本 plan 默认假设 write 走 ctx.ask；实施前 grep `crates/kfcode-tool/src/write.rs` 看是否调用 `ctx.ask_permission`。

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-tool --test permission_callback
```

预期：3 条 pass（或按 write 真实是否走 ctx.ask 调整 deny 测试到合适工具）。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-tool/tests/permission_callback.rs
git commit -m "test(tool): cover ToolContext.ask permission callback (deny/allow/default-allow)"
```
