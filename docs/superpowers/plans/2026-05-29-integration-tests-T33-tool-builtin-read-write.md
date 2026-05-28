# T33 — tool 内置 read / write 工具（tempdir）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用 tempdir 做沙盒，跑 read / write 内置工具的黄金路径与错误路径（不存在文件、二进制文件等）。

**Architecture:** 默认 ctx（无 ask callback → 默认 allow）；输入参数按各工具真实 schema 准备。

**Tech Stack:** kfcode-tool / tempfile。

**依赖:** T31

---

### Task 4.5：read / write 内置工具

**Files:**
- Create: `crates/kfcode-tool/tests/builtin_read_write.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-tool/tests/builtin_read_write.rs`：

```rust
mod common;

#[tokio::test]
async fn write_then_read_round_trips_text() {
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let dir = ws.path().to_str().unwrap();

    // write
    let target = ws.path().join("hello.txt");
    let res = r
        .execute(
            "write",
            serde_json::json!({
                "filePath": target.to_str().unwrap(),
                "content": "hello world"
            }),
            common::make_ctx(dir),
        )
        .await
        .expect("write ok");

    // 文件应在磁盘
    let on_disk = std::fs::read_to_string(&target).expect("read file");
    assert_eq!(on_disk, "hello world");

    // read
    let read_res = r
        .execute(
            "read",
            serde_json::json!({
                "filePath": target.to_str().unwrap()
            }),
            common::make_ctx(dir),
        )
        .await
        .expect("read ok");

    let read_text = serde_json::to_string(&read_res).unwrap();
    assert!(read_text.contains("hello world"), "read result missing content: {read_text}");
}

#[tokio::test]
async fn read_returns_error_for_missing_file() {
    let r = common::fresh_default_registry().await;
    let ws = common::fresh_workspace();
    let missing = ws.path().join("never-existed.txt");
    let res = r
        .execute(
            "read",
            serde_json::json!({"filePath": missing.to_str().unwrap()}),
            common::make_ctx(ws.path().to_str().unwrap()),
        )
        .await;
    let err = res.expect_err("expected error for missing file");
    let _ = err; // 真实错误变体（FileNotFound 或 ExecutionError）由 read 工具决定
}
```

> `write` / `read` 工具的入参字段名（`filePath` / `content`）是按通用约定假设；实施时 grep 真实工具实现确认（`crates/kfcode-tool/src/write.rs` 与 `crates/kfcode-tool/src/read.rs`）。

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-tool --test builtin_read_write
```

预期：2 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-tool/tests/builtin_read_write.rs
git commit -m "test(tool): cover read/write builtin tools in tempdir sandbox"
```
