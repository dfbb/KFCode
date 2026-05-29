# T06 — tui:启动异步检查 + 空闲提示

> 属于 `2026-05-29-github-release-upgrade-INDEX.md`。依赖:T02(util `latest_version_cached`/`is_newer`)。可并行:⛔(可与 T04/T05 并行)。

**Goal:** TUI 启动时异步(不阻塞)检查 GitHub 最新版;若有新版,经 `CustomEvent::UpgradeAvailable` 投递到主事件循环;主循环在用户空闲(prompt 输入框为空)时用 Toast 提示一行,否则丢弃(下次启动再提示)。CLI 不受影响。

**Files:**
- Modify: `crates/kfcode-tui/Cargo.toml`(开启 util 的 upgrade-check feature)
- Modify: `crates/kfcode-tui/src/event.rs`(加 `CustomEvent::UpgradeAvailable`)
- Modify: `crates/kfcode-tui/src/app/app.rs`(spawn 检查 + 处理事件)

---

- [ ] **Step 1: 让 tui 依赖 util 的 upgrade-check feature**

Modify `crates/kfcode-tui/Cargo.toml` — 找到:

```toml
kfcode-util = { path = "../kfcode-util" }
```

改为:

```toml
kfcode-util = { path = "../kfcode-util", features = ["upgrade-check"] }
```

- [ ] **Step 2: 给 CustomEvent 加变体**

Modify `crates/kfcode-tui/src/event.rs` — 在 `pub enum CustomEvent {` 内,`StateChanged(StateChange),` 之后加一个变体:

```rust
    /// A newer release is available; payload is the latest version string.
    UpgradeAvailable(String),
```

- [ ] **Step 3: 编译确认枚举改动不破坏现有 match**

Run: `cargo build -p kfcode-tui`
Expected: 可能出现 `non-exhaustive match` 错误(app.rs 的 `Event::Custom` match)。若有,记下,Step 5 会补分支。若编译器未报(因有通配分支),继续。

- [ ] **Step 4: 新增启动检查线程函数**

Modify `crates/kfcode-tui/src/app/app.rs` — 在文件中 `fn spawn_server_event_listener(` 之前插入:

```rust
/// Spawns a detached thread that checks for a newer release and, if found,
/// sends `CustomEvent::UpgradeAvailable`. Never blocks startup; any failure is
/// silently ignored (a failed update check must not disrupt the TUI).
fn spawn_upgrade_check(event_tx: Sender<Event>) {
    thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(_) => return,
        };
        let latest = match runtime.block_on(kfcode_util::upgrade_check::latest_version_cached()) {
            Ok(v) => v,
            Err(_) => return, // 静默忽略
        };
        let current = env!("CARGO_PKG_VERSION");
        if kfcode_util::upgrade_check::is_newer(&latest, current) {
            let _ = event_tx.send(Event::Custom(CustomEvent::UpgradeAvailable(latest)));
        }
    });
}
```

- [ ] **Step 5: 启动时 spawn 检查线程**

Modify `crates/kfcode-tui/src/app/app.rs` — 在 `App::new` 中,现有这一行之后:

```rust
        spawn_server_event_listener(event_tx.clone(), base_url);
```

加一行:

```rust
        spawn_upgrade_check(event_tx.clone());
```

- [ ] **Step 6: 处理 UpgradeAvailable 事件(空闲才提示)**

Modify `crates/kfcode-tui/src/app/app.rs` — 在 `Event::Custom(event) => match event {` 块内,
通配分支 `_ => {}`(约 717 行)**之前**插入新分支:

```rust
                CustomEvent::UpgradeAvailable(version) => {
                    // 仅在用户空闲(输入框为空)时提示,否则丢弃,留待下次启动再提示。
                    if self.prompt.get_input().is_empty() {
                        let current = env!("CARGO_PKG_VERSION");
                        self.toast.show(
                            ToastVariant::Info,
                            &format!(
                                "有新版本 {version} 可用(当前 {current}),运行 kfcode upgrade 升级"
                            ),
                            6000,
                        );
                    }
                }
```

- [ ] **Step 7: 全量编译 tui**

Run: `cargo build -p kfcode-tui`
Expected: PASS（`spawn_upgrade_check` 已被调用,无未使用警告；新 match 分支已补,无 non-exhaustive 报错）

- [ ] **Step 8: 验证 CLI 不受影响(无升级检查网络行为)**

确认仅 TUI 引入检查:CLI 的 `kfcode serve` 等命令不调用 `spawn_upgrade_check`。

Run: `grep -rn "spawn_upgrade_check\|latest_version_cached" crates/kfcode-cli/src/`
Expected: 仅 `handle_upgrade_command`(T05 新增)里出现 `latest_version_cached`;无 `spawn_upgrade_check`（那是 TUI 专属）。启动检查不在任何 CLI 命令路径上。

- [ ] **Step 9: 提交**

```bash
git add crates/kfcode-tui/Cargo.toml crates/kfcode-tui/src/event.rs crates/kfcode-tui/src/app/app.rs
git commit -m "feat(tui): async startup upgrade check with idle toast notice"
```

---

## 全计划收尾验收(在最后一个完成的 task 之后跑一次)

- [ ] **A1: workspace 全量编译**

Run: `cargo build --workspace`
Expected: PASS

- [ ] **A2: 相关单测全绿**

Run: `cargo test -p kfcode-util --features upgrade-check && cargo test -p kfcode-cli upgrade`
Expected: PASS

- [ ] **A3: 旧符号彻底清除**

Run: `grep -rnE "InstallMethod|detect_install_method|run_upgrade_process|kfcode-ai|kfcode\.ai/install|registry\.npmjs\.org|formulae\.brew\.sh" crates/kfcode-cli/src/`
Expected: 无输出（旧多源/委托逻辑全部删除）

- [ ] **A4: util 默认不引入 reqwest(feature 门控生效)**

Run: `cargo tree -p kfcode-tool | grep reqwest`
Expected: 无输出（kfcode-tool 依赖 util 但未开 upgrade-check,不应拉入 reqwest）

