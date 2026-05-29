# T05 — cli:删除旧升级代码 + 重写 `upgrade` 命令接线

> 属于 `2026-05-29-github-release-upgrade-INDEX.md`。依赖:T02(util 检查 API)、T04(`perform_upgrade`)。可并行:⛔。

**Goal:** 删除 main.rs 中全部旧升级代码(多源 + 委托包管理器),把 `upgrade` 命令改成无参数,接线到 util 的版本检查 + cli 的 `perform_upgrade`。

**Files:**
- Modify: `crates/kfcode-cli/src/main.rs`

---

- [ ] **Step 1: 删除旧升级代码块**

Modify `crates/kfcode-cli/src/main.rs` — 删除从 `enum InstallMethod {`(约 2512 行)到 `handle_upgrade_command` 函数结束 `}`(约 2767 行,即 `async fn handle_uninstall_command` 之前)的**整块连续代码**。

这块包含这些必须一并删除的符号:
- `enum InstallMethod` + `impl InstallMethod`
- `fn command_text`（仅被 detect_install_method 使用）
- `fn detect_install_method`
- `async fn latest_version`
- `fn run_upgrade_process`
- `fn prompt_yes_no`（仅被旧 handle_upgrade_command 使用）
- `async fn handle_upgrade_command`

删除后,文件中 `fn handle_pr_command`（或其它紧邻 2512 之前的函数)与 `async fn handle_uninstall_command` 直接相邻。

验证删除干净:

Run: `grep -nE "InstallMethod|detect_install_method|run_upgrade_process|fn command_text|fn prompt_yes_no|async fn handle_upgrade_command|async fn latest_version" crates/kfcode-cli/src/main.rs`
Expected: 无输出（全部删除）

- [ ] **Step 2: 改 Upgrade 命令定义(去掉 target/method 参数)**

Modify `crates/kfcode-cli/src/main.rs` — 找到命令枚举里的:

```rust
    #[command(about = "Upgrade kfcode to latest or specific version")]
    Upgrade {
        #[arg(value_name = "TARGET")]
        target: Option<String>,
        #[arg(short = 'm', long)]
        method: Option<String>,
    },
```

替换为:

```rust
    #[command(about = "Upgrade kfcode to the latest GitHub release")]
    Upgrade,
```

- [ ] **Step 3: 改命令分发**

Modify `crates/kfcode-cli/src/main.rs` — 找到:

```rust
        Some(Commands::Upgrade { target, method }) => {
            handle_upgrade_command(target, method).await?;
        }
```

替换为:

```rust
        Some(Commands::Upgrade) => {
            handle_upgrade_command().await?;
        }
```

- [ ] **Step 4: 新增无参数的 handle_upgrade_command**

Modify `crates/kfcode-cli/src/main.rs` — 在 `async fn handle_uninstall_command` 之前插入新的实现:

```rust
/// Handles `kfcode upgrade`: checks the latest GitHub release and, only if it is
/// strictly newer than the running version, downloads and self-replaces.
async fn handle_upgrade_command() -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let latest = kfcode_util::upgrade_check::latest_version_cached()
        .await
        .context("检查最新版本失败")?;

    if !kfcode_util::upgrade_check::is_newer(&latest, current) {
        println!("已是最新版 {current}");
        return Ok(());
    }

    println!("发现新版本 {latest}（当前 {current}）,开始升级...");
    upgrade::perform_upgrade().await?;
    println!("已从 {current} 升级到 {latest}");
    Ok(())
}
```

- [ ] **Step 5: 确认 anyhow::Context 已在作用域**

`handle_upgrade_command` 用到 `.context(...)`。检查 main.rs 顶部是否已 `use anyhow::Context;`：

Run: `grep -n "use anyhow::Context" crates/kfcode-cli/src/main.rs`

若无输出,在 main.rs 的 use 区(约第 16 行,`use kfcode_agent::...` 之前的 std/外部 use 区)加一行:

```rust
use anyhow::Context;
```

- [ ] **Step 6: 全量编译**

Run: `cargo build -p kfcode-cli`
Expected: PASS（无未使用警告:upgrade 模块函数现已被调用）

- [ ] **Step 7: 验证 CLI 接口已无旧参数**

Run: `./target/debug/kfcode upgrade --help`
Expected: 帮助里不再出现 `TARGET` 或 `--method`；只描述"Upgrade kfcode to the latest GitHub release"

- [ ] **Step 8: 提交**

```bash
git add crates/kfcode-cli/src/main.rs
git commit -m "feat(cli): replace multi-source upgrade with GitHub-release self-update"
```
