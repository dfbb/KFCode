# T38 — kfcode-command 集成测试脚手架 + builtin

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 准备 command 测试目录，覆盖 `CommandRegistry::new` 内置命令（init/review/commit/test）的存在性。

**Architecture:** 简单测试，无 helper 必要；single test file 即可。

**Tech Stack:** kfcode-command。

**依赖:** 无

---

### Task 4.10：command 脚手架 + builtin

**Files:**
- Modify: `crates/kfcode-command/Cargo.toml`（dev-deps 加 `tempfile`、`tokio-test`）
- Create: `crates/kfcode-command/tests/common/mod.rs`
- Create: `crates/kfcode-command/tests/builtin.rs`

- [ ] **Step 1: dev-deps**

修改 `crates/kfcode-command/Cargo.toml`：

```toml
[dev-dependencies]
tempfile = { workspace = true }
tokio-test = "0.4"
```

- [ ] **Step 2: common helper**

写入 `crates/kfcode-command/tests/common/mod.rs`：

```rust
#![allow(dead_code)]

use kfcode_command::{Command, CommandContext, CommandRegistry, CommandSource};
use std::path::PathBuf;
use tempfile::TempDir;

pub fn fresh_registry() -> CommandRegistry {
    CommandRegistry::new()
}

pub fn make_ctx(cwd: PathBuf) -> CommandContext {
    CommandContext::new(cwd)
}

pub fn fresh_workspace() -> TempDir {
    TempDir::new().expect("tempdir")
}

pub fn make_file_command(name: &str, template: &str, path: PathBuf) -> Command {
    Command {
        name: name.into(),
        description: format!("Test command {name}"),
        template: template.into(),
        source: CommandSource::File(path),
    }
}
```

- [ ] **Step 3: 写测试**

写入 `crates/kfcode-command/tests/builtin.rs`：

```rust
mod common;

#[test]
fn new_registry_has_builtin_commands() {
    let r = common::fresh_registry();
    let list = r.list();
    let names: Vec<&str> = list.iter().map(|c| c.name.as_str()).collect();
    for n in ["init", "review", "commit", "test"] {
        assert!(names.contains(&n), "missing builtin command: {n}; got {names:?}");
    }
}

#[test]
fn get_returns_some_for_builtin() {
    let r = common::fresh_registry();
    assert!(r.get("init").is_some());
    assert!(r.get("does-not-exist").is_none());
}
```

> 内置命令列表来自 `CommandRegistry::new()` 的实现（按 explore 报告 4 个：init/review/commit/test）。如真实数量/名字不同，按实际调整断言。

- [ ] **Step 4: 跑测试**

```
cargo test -p kfcode-command --test builtin
```

预期：2 条 pass。

- [ ] **Step 5: 提交**

```bash
git add crates/kfcode-command/Cargo.toml crates/kfcode-command/tests/
git commit -m "test(command): scaffold + cover CommandRegistry::new builtin commands"
```
