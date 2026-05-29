# T04 — cli:下载 + sha256 校验 + 解压 + self-replace

> 属于 `2026-05-29-github-release-upgrade-INDEX.md`。依赖:T03(`PlatformAsset`/`resolve_current_asset`)。可并行:⛔。

**Goal:** 在 cli 的 `upgrade.rs` 上,加从 Release 下载本平台 archive、校验 sha256、解压取出二进制、self-replace 原子替换的完整执行链。重依赖(self-replace/tar/zip/flate2/sha2/tempfile)只在 cli。

**Files:**
- Modify: `crates/kfcode-cli/Cargo.toml`(加依赖)
- Modify: `crates/kfcode-cli/src/upgrade.rs`(追加下载/校验/解压/替换)

---

- [ ] **Step 1: 给 cli 加执行依赖**

Run（自动选兼容版本并写入 Cargo.toml + Cargo.lock）:

```bash
cargo add --package kfcode-cli self-replace tar zip
cargo add --package kfcode-cli sha2 flate2 tempfile
```

Expected: `cargo add` 成功，`crates/kfcode-cli/Cargo.toml` 的 `[dependencies]` 出现
`self-replace`、`tar`、`zip`、`sha2`、`flate2`、`tempfile`。（`reqwest` 已存在。）

- [ ] **Step 2: 给 util 依赖开启 upgrade-check feature(供 T05 调用版本检查)**

Modify `crates/kfcode-cli/Cargo.toml` — 找到现有这一行:

```toml
kfcode-util = { path = "../kfcode-util" }
```

改为开启 feature:

```toml
kfcode-util = { path = "../kfcode-util", features = ["upgrade-check"] }
```

- [ ] **Step 3: 编译确认依赖就位**

Run: `cargo build -p kfcode-cli`
Expected: PASS

- [ ] **Step 4: 追加下载 + sha256 校验函数(含测试)**

Append to `crates/kfcode-cli/src/upgrade.rs`:

```rust
use anyhow::{anyhow, Context};
use std::path::{Path, PathBuf};

/// One asset entry from the GitHub release JSON.
struct ReleaseAsset {
    name: String,
    url: String,
}

/// Fetches the latest release JSON and returns its assets.
async fn fetch_release_assets(client: &reqwest::Client) -> anyhow::Result<Vec<ReleaseAsset>> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        kfcode_util::upgrade_check::RELEASE_REPO
    );
    let json: serde_json::Value = client
        .get(url)
        .header("User-Agent", "kfcode-cli")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let assets = json
        .get("assets")
        .and_then(|a| a.as_array())
        .ok_or_else(|| anyhow!("release JSON missing assets array"))?;
    let mut out = Vec::new();
    for a in assets {
        if let (Some(name), Some(url)) = (
            a.get("name").and_then(|v| v.as_str()),
            a.get("browser_download_url").and_then(|v| v.as_str()),
        ) {
            out.push(ReleaseAsset { name: name.to_string(), url: url.to_string() });
        }
    }
    Ok(out)
}

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
```

- [ ] **Step 5: 运行下载辅助函数的测试**

Run: `cargo test -p kfcode-cli upgrade::download_tests`
Expected: PASS（hex_encodes_known_bytes / parses_sha256_sidecar）

- [ ] **Step 6: 追加解压函数(含测试)**

Append to `crates/kfcode-cli/src/upgrade.rs`:

```rust
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
    use std::io::Write;

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
```

- [ ] **Step 7: 运行解压测试**

Run: `cargo test -p kfcode-cli upgrade::extract_tests`
Expected: PASS（extracts_binary_from_targz）

- [ ] **Step 8: 追加升级编排函数(下载→校验→解压→替换)**

Append to `crates/kfcode-cli/src/upgrade.rs`:

```rust
/// Downloads the latest release's binary for the current platform, verifies its
/// sha256, and atomically replaces the running executable.
///
/// Caller has already decided an upgrade is warranted (newer version available).
pub async fn perform_upgrade() -> anyhow::Result<()> {
    let asset = resolve_current_asset().ok_or_else(|| {
        anyhow!(
            "当前平台 {}-{} 无对应发布产物,无法自动升级",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let assets = fetch_release_assets(&client)
        .await
        .context("获取最新 Release 信息失败")?;
    let archive_asset = assets
        .iter()
        .find(|a| a.name == asset.archive_name)
        .ok_or_else(|| anyhow!("Release 中缺少本平台产物 {}", asset.archive_name))?;
    let sha_name = format!("{}.sha256", asset.archive_name);
    let sha_asset = assets
        .iter()
        .find(|a| a.name == sha_name)
        .ok_or_else(|| anyhow!("Release 中缺少校验文件 {sha_name}"))?;

    let tmp = tempfile::tempdir().context("创建临时目录失败")?;
    let archive_path = tmp.path().join(&asset.archive_name);
    download_to(&client, &archive_asset.url, &archive_path)
        .await
        .context("下载发布产物失败")?;

    // 校验 sha256
    let sha_path = tmp.path().join(&sha_name);
    download_to(&client, &sha_asset.url, &sha_path)
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
```

- [ ] **Step 9: 全量编译 cli**

Run: `cargo build -p kfcode-cli`
Expected: PASS（`perform_upgrade` 暂未被调用会有未使用警告,T05 接线后消失）

- [ ] **Step 10: 提交**

```bash
git add crates/kfcode-cli/Cargo.toml crates/kfcode-cli/src/upgrade.rs Cargo.lock
git commit -m "feat(cli): download+verify+unpack+self-replace for GitHub release upgrade"
```
