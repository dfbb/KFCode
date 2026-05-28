# T16 — kfcode-lsp 集成测试脚手架（含 stub binary fixture）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 准备 lsp 集成测试目录、helper、与 in-workspace stub LSP server binary（spec §2.7 路径 A 形态 1，首选）。

**Architecture:** 在 `kfcode-lsp/Cargo.toml` 加 `[[bin]]` 段声明 `lsp-test-stub`，源码 `tests/fixtures/stub.rs`（单文件 Rust binary，从 stdin 读 LSP 协议、按 method 分发响应）。测试通过 `env!("CARGO_BIN_EXE_lsp-test-stub")` 拿绝对路径，作为 `LspServerConfig.command` 启动。

**Tech Stack:** tokio process / lsp-types / 自手写 JSON-RPC over stdio。

**依赖:** 无

---

### Task 2.6：lsp 脚手架 + stub binary

**Files:**
- Modify: `crates/kfcode-lsp/Cargo.toml`（新增 `[[bin]]` 与 `[dev-dependencies]`）
- Create: `crates/kfcode-lsp/tests/fixtures/stub.rs`
- Create: `crates/kfcode-lsp/tests/common/mod.rs`
- Create: `crates/kfcode-lsp/tests/smoke.rs`

- [ ] **Step 1: Cargo.toml 加 bin 与 dev-deps**

修改 `crates/kfcode-lsp/Cargo.toml`，在文件末尾追加：

```toml
[[bin]]
name = "lsp-test-stub"
path = "tests/fixtures/stub.rs"
test = false
bench = false

[dev-dependencies]
tempfile = { workspace = true }
tokio-test = "0.4"
```

> `[[bin]]` 把 stub.rs 当作正常 binary 编译；测试启动时通过 `env!("CARGO_BIN_EXE_lsp-test-stub")` 拿到 cargo 编译产物路径。这样无需 node/python 解释器、跨平台。

- [ ] **Step 2: 写 stub binary**

写入 `crates/kfcode-lsp/tests/fixtures/stub.rs`：

```rust
//! Minimal LSP stub server for integration tests.
//!
//! Reads Content-Length-framed JSON-RPC requests from stdin, dispatches
//! by method, writes Content-Length-framed responses to stdout.
//!
//! Behavior controlled via env STUB_MODE:
//!   "default" — answers initialize, ignores didOpen/didChange, returns
//!               empty diagnostics on every didChange.
//!   "no-response" — never writes anything (used to test client timeout).
//!   "always-error" — replies to any request with JSON-RPC error.

use std::io::{BufRead, BufReader, Read, Write};

fn main() {
    let mode = std::env::var("STUB_MODE").unwrap_or_else(|_| "default".to_string());
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    if mode == "no-response" {
        // 把 stdin 读完但不响应
        let mut sink = String::new();
        let _ = stdin.lock().read_to_string(&mut sink);
        return;
    }

    let mut reader = BufReader::new(stdin.lock());
    loop {
        // 读 Content-Length 头
        let mut content_length = 0usize;
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).unwrap_or(0) == 0 {
                return; // stdin closed
            }
            if header == "\r\n" || header.is_empty() {
                break;
            }
            if let Some(rest) = header.trim_end().strip_prefix("Content-Length: ") {
                content_length = rest.parse().unwrap_or(0);
            }
        }
        if content_length == 0 {
            continue;
        }
        let mut buf = vec![0u8; content_length];
        if reader.read_exact(&mut buf).is_err() {
            return;
        }
        let body = String::from_utf8(buf).unwrap_or_default();
        let req: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        // notifications: 没有 id 字段，不需回复
        if id.is_none() {
            continue;
        }

        let response: serde_json::Value = if mode == "always-error" {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": format!("stub: forced error for {method}")}
            })
        } else {
            match method {
                "initialize" => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "capabilities": {
                            "textDocumentSync": 1,
                            "definitionProvider": true,
                            "hoverProvider": true,
                            "referencesProvider": true,
                        },
                        "serverInfo": {"name": "lsp-test-stub", "version": "0"}
                    }
                }),
                "shutdown" => serde_json::json!({"jsonrpc":"2.0","id":id,"result":null}),
                _ => serde_json::json!({"jsonrpc":"2.0","id":id,"result":null}),
            }
        };

        let body = response.to_string();
        write!(stdout, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        stdout.flush().unwrap();
    }
}
```

> 注：`stub.rs` 不能依赖 dev-deps；`serde_json` 已是普通 dep（`crates/kfcode-lsp/Cargo.toml:10`）所以可用。

- [ ] **Step 3: common helper**

写入 `crates/kfcode-lsp/tests/common/mod.rs`：

```rust
#![allow(dead_code)]

use kfcode_lsp::{LspClient, LspError, LspServerConfig};
use std::path::PathBuf;
use tempfile::TempDir;

/// 取 cargo 编译出的 stub 路径。
pub fn stub_path() -> &'static str {
    env!("CARGO_BIN_EXE_lsp-test-stub")
}

/// 启动 stub LSP server（mode=default）并返回 (client, root tempdir)。
pub async fn start_default_stub() -> (LspClient, TempDir) {
    let root_dir = TempDir::new().unwrap();
    let cfg = LspServerConfig {
        id: "stub".into(),
        command: stub_path().into(),
        args: vec![],
        initialization_options: None,
    };
    let client = LspClient::start(cfg, root_dir.path().to_path_buf())
        .await
        .expect("start stub");
    (client, root_dir)
}

/// 用指定 STUB_MODE 启动 stub。tokio process 没有直接 builder 注入 env 的入口；
/// stub binary 自己读 std::env::var("STUB_MODE")，所以测试在调用 LspClient::start
/// 之前用 std::env::set_var 即可——单线程默认 runtime 下安全；如未来并发要起多个
/// 不同 mode 的 stub，请改用本 helper 包装的 process spawn。
pub async fn start_stub_with_mode(mode: &str) -> Result<(LspClient, TempDir), LspError> {
    std::env::set_var("STUB_MODE", mode);
    let root_dir = TempDir::new().unwrap();
    let cfg = LspServerConfig {
        id: "stub".into(),
        command: stub_path().into(),
        args: vec![],
        initialization_options: None,
    };
    let client = LspClient::start(cfg, root_dir.path().to_path_buf()).await?;
    Ok((client, root_dir))
}
```

> 测试函数级别串扰提醒：`start_stub_with_mode` 会改进程级 env，仅供"少量、互斥的 mode 测试"使用；本 plan 默认所有测试都跑 `default` mode（直接 `start_default_stub`），只有 T18 错误测试用 `start_stub_with_mode` 并放在自己的 `#[tokio::test(flavor="multi_thread")]` 串行 suite 里——见 T18。

- [ ] **Step 4: smoke 测试**

写入 `crates/kfcode-lsp/tests/smoke.rs`：

```rust
mod common;

#[tokio::test]
async fn stub_starts_and_completes_initialize() {
    let (_client, _root) = common::start_default_stub().await;
}
```

- [ ] **Step 5: 跑测试**

```
cargo test -p kfcode-lsp --test smoke
```

预期：1 条 pass。Cargo 会先编译 `lsp-test-stub` binary，初次跑稍慢。

- [ ] **Step 6: 提交**

```bash
git add crates/kfcode-lsp/Cargo.toml crates/kfcode-lsp/tests/
git commit -m "$(cat <<'EOF'
test(lsp): scaffold integration tests with in-workspace stub binary

Per spec §2.7 path A form 1: declare an [[bin]] target compiled by
cargo, with source at tests/fixtures/stub.rs. The stub speaks
Content-Length-framed JSON-RPC over stdio and dispatches by method,
configurable via STUB_MODE env var (default / no-response / always-error).
Tests look up the absolute path with env!("CARGO_BIN_EXE_lsp-test-stub"),
so no node/python needed and the suite stays cross-platform.
EOF
)"
```
