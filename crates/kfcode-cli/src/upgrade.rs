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
