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

use anyhow::{anyhow, Context};
use std::path::{Path, PathBuf};

/// Downloads `url` into `dest`.
async fn download_to(client: &reqwest::Client, url: &str, dest: &Path) -> anyhow::Result<()> {
    let bytes = client
        .get(url)
        .header("User-Agent", "kfcode-cli")
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    std::fs::write(dest, &bytes).with_context(|| format!("写入下载文件失败: {}", dest.display()))?;
    Ok(())
}

/// Computes the lowercase hex sha256 of a file's contents.
fn sha256_hex(path: &Path) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex_encode(&hasher.finalize()))
}

/// Minimal lowercase hex encoder (avoids adding the `hex` crate to cli).
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Parses a cargo-dist `.sha256` file (format: "<hex>  <filename>") and returns
/// the lowercase hex digest.
fn parse_sha256_file(contents: &str) -> Option<String> {
    contents.split_whitespace().next().map(|s| s.to_lowercase())
}

#[cfg(test)]
mod download_tests {
    use super::*;

    #[test]
    fn hex_encodes_known_bytes() {
        assert_eq!(hex_encode(&[0x00, 0x0f, 0xff]), "000fff");
    }

    #[test]
    fn parses_sha256_sidecar() {
        let line = "d2c7720dc9b9e38f  kfcode-cli-aarch64-apple-darwin.tar.gz\n";
        assert_eq!(parse_sha256_file(line), Some("d2c7720dc9b9e38f".to_string()));
        assert_eq!(parse_sha256_file(""), None);
    }
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

/// Extracts the kfcode binary from the downloaded archive into `out_dir`,
/// returning the path to the extracted binary. The binary lives at
/// `kfcode-cli-<triple>/<binary_name>` inside the archive.
fn extract_binary(archive: &Path, asset: &PlatformAsset, out_dir: &Path) -> anyhow::Result<PathBuf> {
    let inner_rel = format!("kfcode-cli-{}/{}", asset.triple, asset.binary_name);
    if asset.is_zip {
        let file = std::fs::File::open(archive)?;
        let mut zip = zip::ZipArchive::new(file)?;
        let mut entry = zip
            .by_name(&inner_rel)
            .with_context(|| format!("压缩包内未找到 {inner_rel}"))?;
        let dest = out_dir.join(asset.binary_name);
        let mut out = std::fs::File::create(&dest)?;
        std::io::copy(&mut entry, &mut out)?;
        Ok(dest)
    } else {
        let file = std::fs::File::open(archive)?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(decoder);
        for entry in tar.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            if path.to_string_lossy() == inner_rel {
                let dest = out_dir.join(asset.binary_name);
                entry.unpack(&dest)?;
                return Ok(dest);
            }
        }
        Err(anyhow!("压缩包内未找到 {inner_rel}"))
    }
}

#[cfg(test)]
mod extract_tests {
    use super::*;

    #[test]
    fn extracts_binary_from_targz() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("a.tar.gz");
        // 构造一个含 kfcode-cli-<triple>/kfcode 的 tar.gz
        let asset = resolve_asset_for("linux", "x86_64").unwrap();
        {
            let f = std::fs::File::create(&archive).unwrap();
            let enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
            let mut builder = tar::Builder::new(enc);
            let mut header = tar::Header::new_gnu();
            let data = b"#!/bin/sh\necho hi\n";
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, format!("kfcode-cli-{}/kfcode", asset.triple), &data[..])
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let bin = extract_binary(&archive, &asset, &out).unwrap();
        assert!(bin.exists());
        assert_eq!(std::fs::read(&bin).unwrap(), b"#!/bin/sh\necho hi\n");
    }
}

/// Downloads the latest release's binary for the current platform, verifies its
/// sha256, and atomically replaces the running executable.
///
/// `version` is the target version string without a leading `v` (e.g. `"0.1.2"`).
/// Caller has already decided an upgrade is warranted (newer version available).
/// Download URLs are constructed directly from the version + asset name, avoiding
/// the GitHub REST API and its unauthenticated rate limit.
pub async fn perform_upgrade(version: &str) -> anyhow::Result<()> {
    let asset = resolve_current_asset().ok_or_else(|| {
        anyhow!(
            "当前平台 {}-{} 无对应发布产物,无法自动升级",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;

    let base = format!(
        "https://github.com/{}/releases/download/v{version}",
        kfcode_util::upgrade_check::RELEASE_REPO
    );
    let archive_url = format!("{base}/{}", asset.archive_name);
    let sha_name = format!("{}.sha256", asset.archive_name);
    let sha_url = format!("{base}/{sha_name}");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let tmp = tempfile::tempdir().context("创建临时目录失败")?;
    let archive_path = tmp.path().join(&asset.archive_name);
    download_to(&client, &archive_url, &archive_path)
        .await
        .context("下载发布产物失败")?;

    // 校验 sha256
    let sha_path = tmp.path().join(&sha_name);
    download_to(&client, &sha_url, &sha_path)
        .await
        .context("下载校验文件失败")?;
    let expected = parse_sha256_file(&std::fs::read_to_string(&sha_path)?)
        .ok_or_else(|| anyhow!("无法解析校验文件 {sha_name}"))?;
    let actual = sha256_hex(&archive_path)?;
    if actual != expected {
        return Err(anyhow!("sha256 校验失败: 期望 {expected}, 实际 {actual}"));
    }

    // 解压 + 替换
    let new_binary = extract_binary(&archive_path, &asset, tmp.path())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&new_binary, std::fs::Permissions::from_mode(0o755))?;
    }
    self_replace::self_replace(&new_binary)
        .map_err(|e| anyhow!("替换当前二进制失败(可能无写权限,可改用包管理器升级,如 brew upgrade): {e}"))?;
    Ok(())
}
