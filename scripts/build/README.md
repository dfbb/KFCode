# KFCode 发布系统

基于 [`dist`](https://github.com/axodotdev/cargo-dist)(原 `cargo-dist`)的跨平台编译与发布流水线。

## 产物

通过 git tag `vX.Y.Z` 触发 GitHub Actions,自动编译并发布:

| 平台 | target | 格式 |
|------|--------|------|
| Windows amd64 | `x86_64-pc-windows-msvc` | `.zip` |
| Linux amd64 | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Mac arm64 | `aarch64-apple-darwin` | `.tar.gz` |

每个 Release 还包含:Homebrew formula(`kfcode.rb`)、shell / PowerShell 一键安装脚本、各产物的 sha256 校验和。

> 注意:仅支持 Apple Silicon Mac。Intel Mac(x86_64)不在覆盖范围内,这类用户 `brew install` 会失败。

## 配置文件位置(工具/平台强约束,不在本目录)

- `dist-workspace.toml`(仓库根)—— dist 配置,工具约定须在根或 Cargo.toml。
- `.github/workflows/release.yml` —— dist 生成的发布工作流,GitHub 硬性要求此路径。
- 修改配置后须运行 `bash scripts/build/check-dist.sh` 重新生成并校验工作流。

## 本目录的脚本

- `release.sh <version>` —— 发版入口:校验工作区干净、改版本号、打 tag、push。
- `package-local.sh` —— 本地为当前平台打包,产出压缩包供本地试装。
- `check-dist.sh` —— CI 前自检:`dist plan` + `dist generate --check`。

## 首次发布前的一次性准备

Homebrew 自动发布需要:

1. 在 GitHub 建好空仓库 **`dfbb/homebrew-tap`**。
2. 生成对该 tap 仓库有写权限的 token(细粒度 PAT 或 classic PAT,勾选 `repo` / `contents:write`)。
3. 在主仓库 `dfbb/KFCode` 的 Settings → Secrets and variables → Actions 中,新增 secret **`HOMEBREW_TAP_TOKEN`** 填入该 token。
   (跨仓库推送默认的 `GITHUB_TOKEN` 权限不足,必须用这个。)

## 发布流程

```bash
# 1. 安装 dist(一次性,本地)
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/axodotdev/cargo-dist/releases/download/v0.32.0/cargo-dist-installer.sh | sh

# 2. 发版前自检
bash scripts/build/check-dist.sh

# 3. 发版(示例:0.1.1)
bash scripts/build/release.sh 0.1.1
```

`release.sh` 会推送 tag `v0.1.1`,GitHub Actions 随即编译三平台、创建 Release、把更新后的 formula 推到 `dfbb/homebrew-tap`。

## 用户安装方式

```bash
# Homebrew(Mac arm64 / Linux amd64)
brew install dfbb/tap/kfcode

# shell 一键安装(Mac / Linux)
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/dfbb/KFCode/releases/latest/download/kfcode-cli-installer.sh | sh

# PowerShell 一键安装(Windows)
irm https://github.com/dfbb/KFCode/releases/latest/download/kfcode-cli-installer.ps1 | iex
```
