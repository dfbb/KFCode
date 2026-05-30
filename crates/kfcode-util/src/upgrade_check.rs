//! Upgrade-check helpers shared by the CLI and TUI: version parsing/comparison,
//! latest-release lookup, and a rate-limited cache. Network/IO pieces live behind
//! the `upgrade-check` cargo feature.

/// A parsed three-segment version (major, minor, patch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// Parses a `MAJOR.MINOR.PATCH` string into a [`Version`].
///
/// A leading `v` and any pre-release `-suffix` are stripped before parsing
/// (e.g. `v0.1.2-rc.1` → `0.1.2`). Returns `None` if the three numeric segments
/// cannot be parsed; callers treat `None` as "no upgrade available".
pub fn parse_version(raw: &str) -> Option<Version> {
    let trimmed = raw.trim().trim_start_matches('v');
    let core = trimmed.split('-').next().unwrap_or(trimmed);
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(Version { major, minor, patch })
}

/// Returns `true` when `candidate` is strictly newer than `current`.
///
/// Unparseable inputs yield `false` (never triggers an upgrade/downgrade).
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(c), Some(cur)) => (c.major, c.minor, c.patch) > (cur.major, cur.minor, cur.patch),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_prefixed() {
        assert_eq!(parse_version("0.1.2"), Some(Version { major: 0, minor: 1, patch: 2 }));
        assert_eq!(parse_version("v1.20.3"), Some(Version { major: 1, minor: 20, patch: 3 }));
    }

    #[test]
    fn strips_prerelease_suffix() {
        assert_eq!(parse_version("0.1.2-rc.1"), Some(Version { major: 0, minor: 1, patch: 2 }));
    }

    #[test]
    fn rejects_malformed() {
        assert_eq!(parse_version("0.1"), None);
        assert_eq!(parse_version("0.1.2.3"), None);
        assert_eq!(parse_version("abc"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn newer_only_when_strictly_greater() {
        assert!(is_newer("0.1.2", "0.1.1"));
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn not_newer_when_equal_or_older() {
        assert!(!is_newer("0.1.1", "0.1.1")); // equal
        assert!(!is_newer("0.1.0", "0.1.1")); // older
        assert!(!is_newer("0.1.0", "0.2.0")); // local dev build ahead — must not downgrade
    }

    #[test]
    fn unparseable_is_not_newer() {
        assert!(!is_newer("garbage", "0.1.1"));
        assert!(!is_newer("0.1.2", "garbage"));
    }
}

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
        assert!(!cache_is_stale(&now)); // just checked — not stale
        let old = (chrono::Utc::now() - chrono::Duration::hours(25)).to_rfc3339();
        assert!(cache_is_stale(&old)); // older than 24h — stale
        assert!(cache_is_stale("not-a-timestamp")); // unparseable — treat as stale
    }
}

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
    // Follow the redirect from /releases/latest → /releases/tag/vX.Y.Z and
    // extract the version from the final URL. This avoids the GitHub REST API
    // entirely, so there is no rate limit for unauthenticated users.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;
    let resp = client
        .get(format!("https://github.com/{RELEASE_REPO}/releases/latest"))
        .header("User-Agent", "kfcode-cli")
        .send()
        .await?
        .error_for_status()?;
    // Final URL is https://github.com/.../releases/tag/vX.Y.Z
    let final_url = resp.url().to_string();
    let tag = final_url
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("unexpected redirect URL: {final_url}"))?;
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
