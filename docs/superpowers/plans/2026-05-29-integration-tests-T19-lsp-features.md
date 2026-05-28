# T19 — LSP feature 请求（goto_definition / hover / references）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证三条代表性 LSP 方法（`goto_definition` / `hover` / `references`）能完成请求-响应往返。Default stub 对所有未识别 method 都返回 `result: null`，所以测试断言"返回值为 None / 空 vec、且 client 不报错"。

**Architecture:** 直接在 default stub 上跑，不需要新 mode；如要返回具体内容，扩展 stub 或加新 mode（本 task 不做）。

**Tech Stack:** `kfcode_lsp::LspClient`。

**依赖:** T16 / T17

---

### Task 2.9：LSP feature 请求

**Files:**
- Create: `crates/kfcode-lsp/tests/lsp_features.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-lsp/tests/lsp_features.rs`：

```rust
mod common;

#[tokio::test]
async fn goto_definition_returns_none_for_default_stub() {
    let (client, root) = common::start_default_stub().await;
    let file = root.path().join("a.rs");
    std::fs::write(&file, "fn x() {}\n").unwrap();
    client.open_document(&file, "fn x() {}\n", "rust").await.unwrap();

    let result = client
        .goto_definition(&file, 0, 3)
        .await
        .expect("goto_definition request");
    assert!(result.is_none(), "default stub returns null result");
}

#[tokio::test]
async fn hover_returns_none_for_default_stub() {
    let (client, root) = common::start_default_stub().await;
    let file = root.path().join("a.rs");
    std::fs::write(&file, "fn x() {}\n").unwrap();
    client.open_document(&file, "fn x() {}\n", "rust").await.unwrap();

    let result = client.hover(&file, 0, 3).await.expect("hover request");
    assert!(result.is_none());
}

#[tokio::test]
async fn references_returns_empty_for_default_stub() {
    let (client, root) = common::start_default_stub().await;
    let file = root.path().join("a.rs");
    std::fs::write(&file, "fn x() {}\n").unwrap();
    client.open_document(&file, "fn x() {}\n", "rust").await.unwrap();

    let result = client.references(&file, 0, 3).await.expect("references request");
    assert!(result.is_empty(), "stub returns null → client deserializes to empty vec");
}
```

> `LspClient::references` 真实返回 `Result<Vec<Location>, LspError>`（见 `lib.rs:518`）。如返回的是 `Result<Option<Vec<Location>>, LspError>`，把 `assert!(result.is_empty())` 改成 `assert!(result.is_none() || result.unwrap().is_empty())`。

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-lsp --test lsp_features
```

预期：3 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-lsp/tests/lsp_features.rs
git commit -m "test(lsp): cover goto_definition / hover / references via default stub"
```
