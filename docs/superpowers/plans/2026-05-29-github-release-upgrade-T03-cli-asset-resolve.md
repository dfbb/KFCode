# T03 — cli:平台 → target triple → asset 名推导

> 属于 `2026-05-29-github-release-upgrade-INDEX.md`。依赖:无。可并行:✅(与 T01/T02)。

**Goal:** 在 cli 新建 `upgrade` 模块,提供"当前平台 → cargo-dist asset 文件名"的纯函数推导。无网络、无 IO,便于单测。后续 T04 在此模块上加下载/解压/替换。

**Files:**
- Create: `crates/kfcode-cli/src/upgrade.rs`
- Modify: `crates/kfcode-cli/src/main.rs`(声明 `mod upgrade;`)

---

- [ ] **Step 1: 创建 upgrade.rs,写平台推导 + 失败测试**

Create `crates/kfcode-cli/src/upgrade.rs`:

```rust
//! Self-update by downloading the matching asset from the latest GitHub release,
//! verifying its sha256, unpacking it, and atomically replacing the running
//! binary. Version lookup/comparison lives in `kfcode_util::upgrade_check`.

/// Describes the release asset for the current platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformAsset {
    /// cargo-dist target triple, e.g. `aarch64-apple-darwin`.
    pub triple: &'static str,
    /// Archive file name in the release, e.g.
    /// `kfcode-cli-aarch64-apple-darwin.tar.gz`.
    pub archive_name: String,
    /// True for `.zip` (Windows), false for `.tar.gz`.
    pub is_zip: bool,
    /// Binary name inside the archive's `kfcode-cli-<triple>/` dir.
    pub binary_name: &'static str,
}

/// Resolves the asset for an explicit `(os, arch)` pair. Returns `None` for
/// platforms KFCode does not publish (e.g. Intel macOS).
///
/// `os`/`arch` use `std::env::consts::OS` / `ARCH` values.
pub fn resolve_asset_for(os: &str, arch: &str) -> Option<PlatformAsset> {
    let triple: &'static str = match (os, arch) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => return None,
    };
    let is_zip = os == "windows";
    let ext = if is_zip { "zip" } else { "tar.gz" };
    Some(PlatformAsset {
        triple,
        archive_name: format!("kfcode-cli-{triple}.{ext}"),
        is_zip,
        binary_name: if is_zip { "kfcode.exe" } else { "kfcode" },
    })
}

/// Resolves the asset for the platform this binary is running on.
pub fn resolve_current_asset() -> Option<PlatformAsset> {
    resolve_asset_for(std::env::consts::OS, std::env::consts::ARCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_three_supported_platforms() {
        let mac = resolve_asset_for("macos", "aarch64").unwrap();
        assert_eq!(mac.triple, "aarch64-apple-darwin");
        assert_eq!(mac.archive_name, "kfcode-cli-aarch64-apple-darwin.tar.gz");
        assert!(!mac.is_zip);
        assert_eq!(mac.binary_name, "kfcode");

        let linux = resolve_asset_for("linux", "x86_64").unwrap();
        assert_eq!(linux.archive_name, "kfcode-cli-x86_64-unknown-linux-gnu.tar.gz");
        assert!(!linux.is_zip);

        let win = resolve_asset_for("windows", "x86_64").unwrap();
        assert_eq!(win.archive_name, "kfcode-cli-x86_64-pc-windows-msvc.zip");
        assert!(win.is_zip);
        assert_eq!(win.binary_name, "kfcode.exe");
    }

    #[test]
    fn unsupported_platforms_return_none() {
        assert!(resolve_asset_for("macos", "x86_64").is_none()); // Intel Mac
        assert!(resolve_asset_for("linux", "aarch64").is_none());
        assert!(resolve_asset_for("freebsd", "x86_64").is_none());
    }
}
```

- [ ] **Step 2: 在 main.rs 声明模块**

Modify `crates/kfcode-cli/src/main.rs` — 在文件顶部 `use` 区之后(第 35 行 `use kfcode_types::...` 之后)加一行模块声明:

```rust
mod upgrade;
```

- [ ] **Step 3: 运行测试**

Run: `cargo test -p kfcode-cli upgrade::tests`
Expected: PASS（resolves_three_supported_platforms / unsupported_platforms_return_none）

注:此时 `mod upgrade;` 会触发"未使用"警告（函数尚未被 main 调用），属正常,T05 接线后消失。

- [ ] **Step 4: 提交**

```bash
git add crates/kfcode-cli/src/upgrade.rs crates/kfcode-cli/src/main.rs
git commit -m "feat(cli): add platform->asset name resolution for self-update"
```
