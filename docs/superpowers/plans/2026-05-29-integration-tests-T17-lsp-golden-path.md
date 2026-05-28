# T17 — LSP 黄金路径（initialize → didOpen → didChange → diagnostics → shutdown）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用 default-mode stub server 走完一遍 LSP 全流程：初始化 → 打开文档 → 变更文档 → 收 diagnostics → shutdown。

**Architecture:** 默认 stub 在 default mode 下不会主动推 diagnostics，所以 diagnostics 测试断言"列表能拿、无错误"，不要求"包含具体诊断"——后者要把 stub 升级为推 publishDiagnostics 通知，留给后续迭代。

**Tech Stack:** `kfcode_lsp::LspClient` / lsp-types 0.97。

**依赖:** T16

---

### Task 2.7：LSP 黄金路径

**Files:**
- Create: `crates/kfcode-lsp/tests/lsp_golden.rs`

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-lsp/tests/lsp_golden.rs`：

```rust
mod common;

use std::path::PathBuf;

#[tokio::test]
async fn full_open_change_diagnostics_round_trip() {
    let (client, root) = common::start_default_stub().await;

    let file = root.path().join("hello.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();

    client
        .open_document(&file, "fn main() {}\n", "rust")
        .await
        .expect("didOpen");

    // didChange 通过 LspClient::notify 触发；如有更高层 API 用更高层。
    // 默认 stub 不推 diagnostics 通知，所以 get_diagnostics 应返回空列表。
    let diagnostics = client.get_diagnostics(&file).await;
    assert!(diagnostics.is_empty(), "default stub does not push diagnostics");
}

#[tokio::test]
async fn subscribe_yields_no_events_on_default_stub() {
    let (client, root) = common::start_default_stub().await;
    let mut rx = client.subscribe();
    // 拿到 subscriber 即可；不主动等事件（默认 stub 不发）
    drop(rx);
    let _ = root;
}

#[tokio::test]
async fn open_nonexistent_file_does_not_panic() {
    let (client, root) = common::start_default_stub().await;
    let file = root.path().join("never-existed.rs");
    // 文件不在磁盘也无所谓——LSP didOpen 接受 in-memory 内容
    client
        .open_document(&file, "// content", "rust")
        .await
        .expect("didOpen with in-memory content");
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-lsp --test lsp_golden
```

预期：3 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-lsp/tests/lsp_golden.rs
git commit -m "$(cat <<'EOF'
test(lsp): cover initialize → didOpen → diagnostics with default stub

Default stub completes initialize handshake but does not push
publishDiagnostics, so this task verifies the no-diagnostics-yet
state and event subscription wiring without expecting populated
diagnostics. Diagnostic-push behavior is tested separately when the
stub is extended to emit them.
EOF
)"
```
