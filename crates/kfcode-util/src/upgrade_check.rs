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
        assert!(!is_newer("0.1.1", "0.1.1")); // 相等
        assert!(!is_newer("0.1.0", "0.1.1")); // 更旧
        assert!(!is_newer("0.1.0", "0.2.0")); // 本地/开发版超前,不降级
    }

    #[test]
    fn unparseable_is_not_newer() {
        assert!(!is_newer("garbage", "0.1.1"));
        assert!(!is_newer("0.1.2", "garbage"));
    }
}
