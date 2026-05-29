# KFCode 发布系统设计(cargo-dist / dist)

日期:2026-05-29
状态:已确认设计,进入实现

## 1. 目标

用 `cargo-dist`(已改名为 `dist`,保留 `cargo dist` 别名)为 KFCode 搭建跨平台编译与发布系统。通过 git tag 触发 GitHub Actions,自动编译三平台二进制、打包、生成 GitHub Release,并自动更新 Homebrew tap。

主二进制:`kfcode`(来自 `crates/kfcode-cli`)。
主仓库:`dfbb/KFCode`。Tap 仓库:`dfbb/homebrew-tap`。

## 2. 构建目标与产物(严格锁定)

| 平台 | target triple | 压缩格式 | 默认 runner |
|------|---------------|----------|-------------|
| Windows amd64 | `x86_64-pc-windows-msvc` | `.zip` | windows |
| Linux amd64 | `x86_64-unknown-linux-gnu` | `.tar.gz` | ubuntu |
| Mac arm64 | `aarch64-apple-darwin` | `.tar.gz` | macos(arm64) |

每平台在对应原生 runner 上编译,规避交叉编译。本项目依赖大量 native crate(`sqlx`+sqlite、`portable-pty`、`syntect` 的 onig、`reqwest` 默认 TLS),原生编译是最稳路径。

**已知局限**:只覆盖 Mac arm64,Intel Mac(x86_64)用户 `brew install` 会失败。这是显式需求选择,formula 与文档中注明仅支持 Apple Silicon。

## 3. 架构与数据流

```
开发者本地                       GitHub Actions (tag 触发)              用户
scripts/build/release.sh
  → push tag vX.Y.Z
                          .github/workflows/release.yml (dist 生成)
                                       │
              ┌────────────────────────┼────────────────────────┐
        windows (msvc)            ubuntu (gnu)              macos (arm64)
          → .zip                   → .tar.gz                  → .tar.gz
              └────────────────────────┼────────────────────────┘
                                       ▼
              GitHub Release(3 压缩包 + sha256 + installer 脚本)
                                       ▼
              自动生成 Formula/kfcode.rb 推送到 dfbb/homebrew-tap
              ┌────────────────────────┴────────────────────────┐
        brew install dfbb/tap/kfcode                curl|sh / irm|iex
```

## 4. 文件落点(混合方案)

工具/平台强约束的文件留在仓库根与 `.github`;所有"人工操作入口"和文档放 `scripts/build/`。

```
仓库根/
├── dist-workspace.toml          # dist 配置(工具约定,须在根)
├── .github/workflows/
│   └── release.yml              # dist 生成,GitHub 硬性要求此路径
└── scripts/build/
    ├── README.md                # 发布流程、token 配置、tap 准备说明
    ├── release.sh               # 发版入口:校验工作区→打 tag→push
    ├── package-local.sh         # 本地验证:dist build 当前平台产出压缩包
    └── check-dist.sh            # CI 前自检:dist plan + dist generate --check
```

## 5. dist 配置要点(dist-workspace.toml)

- `targets`:上表三个 triple。
- `installers`:`["homebrew", "shell", "powershell"]`。
- `tap = "dfbb/homebrew-tap"`,`publish-jobs = ["homebrew"]`。
- `windows-archive = ".zip"`,`unix-archive = ".tar.gz"`(满足格式要求)。
- `ci = ["github"]`。
- `dist-version` 固定为安装时所用版本,保证可复现。

二进制解压后位于压缩包顶层(dist 默认行为),formula `bin.install "kfcode"` 直接可用。

## 6. Homebrew 发布前置条件(需人工准备)

1. 在 GitHub 建空仓库 `dfbb/homebrew-tap`。
2. 生成对该仓库有写权限的 token,在主仓库 `dfbb/KFCode` 配置为 secret `HOMEBREW_TAP_TOKEN`(跨仓库推送默认 `GITHUB_TOKEN` 不足)。

用户安装:`brew install dfbb/tap/kfcode`。

## 7. 安装脚本

启用 `shell`(`curl ... | sh`)与 `powershell`(`irm ... | iex`)installer,dist 自动生成并挂到 Release。

## 8. 已识别风险(不阻塞)

- **Linux 用 glibc(gnu)而非 musl**:产物动态链接,过老发行版可能不兼容。符合"amd64 tar.gz"描述,不改 musl。
- **`reqwest` 默认 TLS**:Linux 上动态链接 libssl,ubuntu runner 预装 libssl-dev,编译运行均可。改 rustls 超出本次 scope,不动。

## 9. 成功标准

1. `dist-workspace.toml` 含三 target + 两种压缩格式 + 三种 installer + tap。
2. `.github/workflows/release.yml` 由 dist 生成,`dist generate --check` 通过。
3. `scripts/build/` 四个文件就位,shell 脚本 `bash -n` 通过。
4. `dist plan` 输出三平台产物正确。
5. workspace 能编译(`cargo build -p kfcode-cli`)。
6. 全部提交到 `main`。

## 10. 发版流程(交付后使用)

```
bash scripts/build/release.sh 0.1.1   # 校验→改版本→打 tag vX.Y.Z→push
# GitHub Actions 自动编译三平台、发 Release、更新 tap
```
