# T35 — kfcode-permission 集成测试脚手架

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 准备 permission 测试目录与最小 helper。permission 是纯内存 crate，不需要 tempfile。

**Architecture:** `tests/common/mod.rs` 提供 `make_rule(perm, pattern, action)` 与 `make_ruleset(rules)`。

**Tech Stack:** kfcode-permission。

**依赖:** 无

---

### Task 4.7：permission 脚手架

**Files:**
- Modify: `crates/kfcode-permission/Cargo.toml`（如尚无 dev-deps，加 `tokio-test`）
- Create: `crates/kfcode-permission/tests/common/mod.rs`
- Create: `crates/kfcode-permission/tests/smoke.rs`

- [ ] **Step 1: dev-deps**

修改 `crates/kfcode-permission/Cargo.toml`，确保有：

```toml
[dev-dependencies]
tokio-test = "0.4"
```

- [ ] **Step 2: common helper**

写入 `crates/kfcode-permission/tests/common/mod.rs`：

```rust
#![allow(dead_code)]

use kfcode_permission::{PermissionAction, PermissionRule, PermissionRuleset};

pub fn rule(perm: &str, pattern: &str, action: PermissionAction) -> PermissionRule {
    PermissionRule {
        permission: perm.to_string(),
        pattern: pattern.to_string(),
        action,
    }
}

pub fn ruleset(rules: Vec<PermissionRule>) -> PermissionRuleset {
    rules
}
```

> 上面假设 `PermissionRuleset` 是 `Vec<PermissionRule>` 的别名（spec §3.2 已说"merge 拼接平铺"暗示如此）；如真实是 newtype，按真实定义调整 helper。

- [ ] **Step 3: smoke 测试**

写入 `crates/kfcode-permission/tests/smoke.rs`：

```rust
mod common;

use kfcode_permission::PermissionAction;

#[tokio::test]
async fn rule_helper_constructs() {
    let r = common::rule("read", "*.env", PermissionAction::Ask);
    assert_eq!(r.permission, "read");
}
```

- [ ] **Step 4: 跑测试**

```
cargo test -p kfcode-permission --test smoke
```

预期：1 条 pass。

- [ ] **Step 5: 提交**

```bash
git add crates/kfcode-permission/Cargo.toml crates/kfcode-permission/tests/
git commit -m "test(permission): scaffold integration tests"
```
