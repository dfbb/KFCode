# T02 — util:Release 查询 + 缓存 + feature 门控

> 属于 `2026-05-29-github-release-upgrade-INDEX.md`。依赖:T01(`parse_version`/`is_newer`)。可并行:⛔。

**Goal:** 在 `kfcode-util` 的 `upgrade_check` 模块上,加 GitHub `/releases/latest` 查询、24h 限频缓存读写。网络依赖(reqwest)放在 `upgrade-check` feature 后,默认关闭。

**Files:**
- Modify: `crates/kfcode-util/Cargo.toml`(加 `[features]` + 可选 reqwest)
- Modify: `crates/kfcode-util/src/upgrade_check.rs`(追加查询 + 缓存)

---

- [ ] **Step 1: 给 util 加 feature 与可选依赖**

Modify `crates/kfcode-util/Cargo.toml` — 在 `[dependencies]` 段末尾(`regex = { workspace = true }` 之后)加 `dirs`(缓存路径用,轻量、无 TLS,放默认依赖)和可选 reqwest,并在文件末尾新增 `[features]` 段:

```toml
dirs = { workspace = true }
reqwest = { workspace = true, optional = true }

[features]
# Enables the GitHub release lookup + cache in `upgrade_check`.
# Off by default so util's other dependents don't pull in reqwest/TLS.
upgrade-check = ["dep:reqwest"]
```

- [ ] **Step 2: 验证默认 build 不引入 reqwest**

Run: `cargo build -p kfcode-util`
Expected: PASS（默认 feature 关闭,reqwest 不参与编译）

Run: `cargo build -p kfcode-util --features upgrade-check`
Expected: PASS（开启后 reqwest 参与,能编译）

- [ ] **Step 3: 追加缓存结构与读写函数(含失败测试)**

Append to `crates/kfcode-util/src/upgrade_check.rs`:

```rust
use std::path::PathBuf;

/// Cached result of the last upgrade check.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpgradeCache {
    /// RFC3339 timestamp of the last network check.
    pub last_check: String,
    /// Latest version string seen (e.g. "0.1.2"), without a leading `v`.
    pub latest_version: String,
}

/// Path to the cache file: `<cache_dir>/kfcode/upgrade-check.json`.
pub fn cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|p| p.join("kfcode").join("upgrade-check.json"))
}

/// Reads the cache, returning `None` if it is missing or unparseable.
pub fn read_cache() -> Option<UpgradeCache> {
    let path = cache_path()?;
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Writes the cache, creating the parent directory if needed. Errors are ignored
/// by callers (a failed cache write must never break startup).
pub fn write_cache(cache: &UpgradeCache) -> std::io::Result<()> {
    let path = cache_path()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no cache dir"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string(cache)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, text)
}

/// Returns true when `last_check` (RFC3339) is more than 24h before now.
pub fn cache_is_stale(last_check: &str) -> bool {
    match chrono::DateTime::parse_from_rfc3339(last_check) {
        Ok(ts) => {
            let age = chrono::Utc::now().signed_duration_since(ts.with_timezone(&chrono::Utc));
            age.num_hours() >= 24
        }
        Err(_) => true,
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn roundtrips_cache_json() {
        let cache = UpgradeCache {
            last_check: "2026-05-29T08:00:00Z".to_string(),
            latest_version: "0.1.2".to_string(),
        };
        let text = serde_json::to_string(&cache).unwrap();
        let back: UpgradeCache = serde_json::from_str(&text).unwrap();
        assert_eq!(back.latest_version, "0.1.2");
        assert_eq!(back.last_check, "2026-05-29T08:00:00Z");
    }

    #[test]
    fn staleness_thresholds() {
        let now = chrono::Utc::now().to_rfc3339();
        assert!(!cache_is_stale(&now)); // 刚检查过,不过期
        let old = (chrono::Utc::now() - chrono::Duration::hours(25)).to_rfc3339();
        assert!(cache_is_stale(&old)); // 超过 24h,过期
        assert!(cache_is_stale("not-a-timestamp")); // 解析失败视为过期
    }
}
```

- [ ] **Step 4: 运行缓存与版本测试**

Run: `cargo test -p kfcode-util --features upgrade-check upgrade_check`
Expected: PASS（T01 的 6 个 + roundtrips_cache_json + staleness_thresholds，共 8 个）

- [ ] **Step 5: 追加 feature 门控的 Release 查询函数**

Append to `crates/kfcode-util/src/upgrade_check.rs`:

```rust
/// GitHub owner/repo that publishes KFCode releases.
pub const RELEASE_REPO: &str = "dfbb/KFCode";

/// Fetches the latest **stable** release version (no leading `v`) from GitHub.
///
/// Uses `/releases/latest`, which excludes prereleases by design. Available only
/// when the `upgrade-check` feature is enabled. A 4s timeout keeps callers from
/// hanging; any network/parse failure surfaces as `Err` (callers ignore it for
/// the silent startup check, or report it for the explicit `upgrade` command).
#[cfg(feature = "upgrade-check")]
pub async fn fetch_latest_version() -> anyhow::Result<String> {
    let url = format!("https://api.github.com/repos/{RELEASE_REPO}/releases/latest");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()?;
    let resp = client
        .get(url)
        .header("User-Agent", "kfcode-cli")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?;
    let json: serde_json::Value = resp.json().await?;
    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("release JSON missing tag_name"))?;
    Ok(tag.trim_start_matches('v').to_string())
}

/// Returns the latest version using the cache when fresh (<24h), otherwise
/// queries GitHub and refreshes the cache. Network/cache-write failures are
/// non-fatal: on error it falls back to any cached value, else returns `Err`.
#[cfg(feature = "upgrade-check")]
pub async fn latest_version_cached() -> anyhow::Result<String> {
    if let Some(cache) = read_cache() {
        if !cache_is_stale(&cache.last_check) {
            return Ok(cache.latest_version);
        }
    }
    match fetch_latest_version().await {
        Ok(version) => {
            let _ = write_cache(&UpgradeCache {
                last_check: chrono::Utc::now().to_rfc3339(),
                latest_version: version.clone(),
            });
            Ok(version)
        }
        Err(e) => match read_cache() {
            Some(cache) => Ok(cache.latest_version),
            None => Err(e),
        },
    }
}
```

- [ ] **Step 6: 确认 anyhow 在 util 可用,编译 feature 版本**

util 已依赖 `anyhow`（见 `crates/kfcode-util/Cargo.toml` 的 `[dependencies]`）。

Run: `cargo build -p kfcode-util --features upgrade-check`
Expected: PASS

- [ ] **Step 7: 提交**

```bash
git add crates/kfcode-util/Cargo.toml crates/kfcode-util/src/upgrade_check.rs
git commit -m "feat(util): add GitHub release lookup + 24h cache behind upgrade-check feature"
```
