# T18 — LSP 错误路径（server 不响应 / always-error / 不存在的 binary）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证 `LspClient::start` 与后续请求在异常 stub 下返回合适 `LspError`。

**Architecture:** 三种异常：
1. binary 路径不存在 → `LspError::ServerStartError`
2. STUB_MODE=no-response → `LspError::Timeout`（initialize 等待）
3. STUB_MODE=always-error → request 阶段返回 `LspError::JsonRpcError`

第 2、3 共享 STUB_MODE env，所以全文件用 `multi_thread` runtime 并依赖测试逐个串行执行（不要求 `serial_test`，因为本文件内部按顺序声明，cargo 默认在 single test binary 内允许并行——本 plan 显式把这两条测试拆到独立文件 `lsp_errors_*` 以避免串扰）。

**Tech Stack:** `kfcode_lsp::{LspClient, LspError, LspServerConfig}`。

**依赖:** T16

---

### Task 2.8：LSP 错误路径

**Files:**
- Create: `crates/kfcode-lsp/tests/lsp_errors_missing_binary.rs`
- Create: `crates/kfcode-lsp/tests/lsp_errors_no_response.rs`
- Create: `crates/kfcode-lsp/tests/lsp_errors_always_error.rs`

> 三个文件分别独立 cargo test binary，避免共享 process env。

- [ ] **Step 1: missing binary**

写入 `crates/kfcode-lsp/tests/lsp_errors_missing_binary.rs`：

```rust
use kfcode_lsp::{LspClient, LspError, LspServerConfig};
use tempfile::TempDir;

#[tokio::test]
async fn start_fails_when_binary_missing() {
    let root = TempDir::new().unwrap();
    let cfg = LspServerConfig {
        id: "missing".into(),
        command: "/path/that/does/not/exist".into(),
        args: vec![],
        initialization_options: None,
    };
    let res = LspClient::start(cfg, root.path().to_path_buf()).await;
    let err = res.unwrap_err();
    match err {
        LspError::ServerStartError(_) => {}
        other => panic!("expected ServerStartError, got {other:?}"),
    }
}
```

- [ ] **Step 2: no-response（initialize 超时）**

写入 `crates/kfcode-lsp/tests/lsp_errors_no_response.rs`：

```rust
mod common {
    include!("common/mod.rs");
}

use kfcode_lsp::LspError;

#[tokio::test]
async fn start_times_out_when_server_silent() {
    std::env::set_var("STUB_MODE", "no-response");
    let res = common::start_stub_with_mode("no-response").await;
    let err = res.expect_err("initialize must fail");
    match err {
        LspError::Timeout | LspError::InitializeError(_) => {}
        other => panic!("expected Timeout/InitializeError, got {other:?}"),
    }
}
```

> 如 `LspClient::start` 当前**不带 timeout**（永远等 stdin），把这条测试改成显式断言"start 永不返回"是不可行的——本 plan 把测试**标 `#[ignore]`** 并附 issue 链接：
>
> ```rust
> #[tokio::test]
> #[ignore = "LspClient::start has no timeout; tracked for follow-up issue"]
> async fn ...
> ```
>
> 实施时先看 `LspClient::start` 是否有 deadline；没有就标 ignore 并在 commit message 写明。

- [ ] **Step 3: always-error**

写入 `crates/kfcode-lsp/tests/lsp_errors_always_error.rs`：

```rust
mod common {
    include!("common/mod.rs");
}

use kfcode_lsp::LspError;

#[tokio::test]
async fn initialize_fails_when_stub_returns_error() {
    let res = common::start_stub_with_mode("always-error").await;
    let err = res.expect_err("initialize should propagate JSON-RPC error");
    match err {
        LspError::InitializeError(_) | LspError::JsonRpcError(_) => {}
        other => panic!("expected InitializeError/JsonRpcError, got {other:?}"),
    }
}
```

- [ ] **Step 4: 跑测试**

```
cargo test -p kfcode-lsp --test lsp_errors_missing_binary
cargo test -p kfcode-lsp --test lsp_errors_always_error
cargo test -p kfcode-lsp --test lsp_errors_no_response
```

预期：第 1 条 pass、第 3 条 pass（或 ignore）、第 2 条按真实有无 timeout 决定。

- [ ] **Step 5: 提交**

```bash
git add crates/kfcode-lsp/tests/lsp_errors_*.rs
git commit -m "$(cat <<'EOF'
test(lsp): cover error paths via stub binary modes

Three independent test binaries to avoid sharing STUB_MODE process
env: missing-binary fails fast (ServerStartError); always-error
propagates JsonRpcError out of initialize. The no-response timeout
test is feature-flagged on whether LspClient::start has a deadline.
EOF
)"
```
