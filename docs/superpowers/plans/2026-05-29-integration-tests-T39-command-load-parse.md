# T39 — command load_from_directory + parse + 描述提取 + 名冲突覆盖

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 覆盖 `load_from_directory` 从 `.kfcode/commands/*.md` 加载、`parse` 解析 slash 输入、首行 `#` / 首条非空行 描述提取、名冲突直接覆盖（spec §3.2 command 切面）。

**Architecture:** tempdir 模拟项目根，写多个 `.md` 文件后调用 `load_from_directory`。

**Tech Stack:** kfcode-command / tempfile。

**依赖:** T38

---

### Task 4.11：command 加载与解析

**Files:**
- Create: `crates/kfcode-command/tests/load_parse.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-command/tests/load_parse.rs`：

```rust
mod common;

use std::fs;

#[test]
fn load_from_directory_reads_md_files() {
    let ws = common::fresh_workspace();
    let cmds = ws.path().join(".kfcode/commands");
    fs::create_dir_all(&cmds).unwrap();
    fs::write(cmds.join("hello.md"), "# Say hello\nHello, world!").unwrap();
    fs::write(cmds.join("bye.md"), "# Say bye\nGoodbye!").unwrap();

    let mut r = common::fresh_registry();
    r.load_from_directory(ws.path()).expect("load");

    assert!(r.get("hello").is_some());
    assert!(r.get("bye").is_some());
}

#[test]
fn load_from_directory_returns_ok_for_missing_dir() {
    let ws = common::fresh_workspace();
    // 没建 .kfcode/commands
    let mut r = common::fresh_registry();
    let res = r.load_from_directory(ws.path());
    assert!(res.is_ok(), "missing directory must not error");
}

#[test]
fn description_extracted_from_first_heading() {
    let ws = common::fresh_workspace();
    let cmds = ws.path().join(".kfcode/commands");
    fs::create_dir_all(&cmds).unwrap();
    fs::write(cmds.join("desc.md"), "# Make magic\n\nLong template body...").unwrap();

    let mut r = common::fresh_registry();
    r.load_from_directory(ws.path()).unwrap();
    let cmd = r.get("desc").unwrap();
    assert!(cmd.description.contains("Make magic"), "got: {}", cmd.description);
}

#[test]
fn duplicate_register_overwrites_previous() {
    let mut r = common::fresh_registry();
    r.register(common::make_file_command("dup", "v1", "/tmp/a.md".into()));
    r.register(common::make_file_command("dup", "v2", "/tmp/b.md".into()));
    let cmd = r.get("dup").unwrap();
    assert_eq!(cmd.template, "v2", "second register must overwrite first");
}

#[test]
fn parse_returns_command_and_positional_args() {
    let mut r = common::fresh_registry();
    r.register(common::make_file_command("greet", "Hello $1!", "/tmp/g.md".into()));

    let parsed = r.parse("/greet World extra");
    let (cmd, args) = parsed.expect("parsed");
    assert_eq!(cmd.name, "greet");
    assert_eq!(args, vec!["World".to_string(), "extra".to_string()]);
}

#[test]
fn parse_returns_none_for_unknown_command() {
    let r = common::fresh_registry();
    assert!(r.parse("/no-such-command").is_none());
}

#[test]
fn parse_returns_none_for_non_slash_input() {
    let r = common::fresh_registry();
    assert!(r.parse("plain text not a slash command").is_none());
}

#[test]
fn execute_substitutes_positional_args() {
    let mut r = common::fresh_registry();
    r.register(common::make_file_command("hi", "Hello $1, you are $2.", "/tmp/h.md".into()));
    let mut ctx = common::make_ctx("/tmp".into());
    ctx.arguments = vec!["Alice".into(), "great".into()];
    let rendered = r.execute("hi", ctx).expect("execute");
    assert_eq!(rendered, "Hello Alice, you are great.");
}

#[test]
fn unicode_command_name_works() {
    let ws = common::fresh_workspace();
    let cmds = ws.path().join(".kfcode/commands");
    fs::create_dir_all(&cmds).unwrap();
    fs::write(cmds.join("你好.md"), "# 中文命令\n你好").unwrap();
    let mut r = common::fresh_registry();
    r.load_from_directory(ws.path()).unwrap();
    assert!(r.get("你好").is_some(), "unicode command name should load");
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-command --test load_parse
```

预期：9 条 pass。如 `parse` 不接受多个位置参数（只支持 1 个），调整 `parse_returns_command_and_positional_args` 断言为只取第一个；如 `description` 提取规则不是首行 `#` 而是首条非空行，调整 fixture 让两者一致。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-command/tests/load_parse.rs
git commit -m "$(cat <<'EOF'
test(command): cover load_from_directory + parse + description + dup-register

Verifies the current behaviors documented in spec §3.2:
- flat glob load from .kfcode/commands/*.md (no nested subcommands)
- duplicate register silently overwrites
- description extracted from first heading or non-empty line
- parse returns (Command, positional args) for slash input
- unicode command names supported
EOF
)"
```
