# KFCode 升级流程重写设计(基于 GitHub Release)

日期:2026-05-29
状态:已确认设计,进入实现

## 1. 目标

把现有的「多源版本检查 + 委托包管理器」升级流程,**整体替换**为以 GitHub Release
(`dfbb/KFCode/releases`)为唯一版本源的自下载替换方案。GitHub Release 是 KFCode
当前唯一真实的发布渠道(由 cargo-dist 流水线产出),升级逻辑应与之对齐。

## 2. 审核:现有升级流程的问题

现有实现位于 `crates/kfcode-cli/src/main.rs`,是「多源混合 + 委托包管理器」模式:

- **多版本源不一致**:brew 查 `formulae.brew.sh`、npm 查 `registry.npmjs.org`、
  scoop 查 scoop bucket、其余查 GitHub Release。版本可能彼此不同步。
- **大量虚假/死代码**:`kfcode-ai`(npm)、`formulae.brew.sh/.../kfcode.json`、
  scoop manifest、`https://kfcode.ai/install` 均不存在,是占位 URL。
- **字符串相等比较版本**,非 semver。
- **升级靠委托**:实际升级调 `npm install` / `brew upgrade` / `curl|bash`,
  kfcode 自身不下载替换二进制。
- **安装方式探测脆弱**:靠路径包含判断 + 跑各包管理器探测。

## 3. 要删除的旧代码(`crates/kfcode-cli/src/main.rs`)

- `enum InstallMethod` + `parse`/`as_str`(~2512-2549)
- `detect_install_method()`(~2560-2618)
- `latest_version()` 的 brew/npm/scoop 分支(~2620-2678)
- `run_upgrade_process()` 委托包管理器逻辑(~2680-2716)
- `Upgrade` 命令的 `target` / `method` 参数(~243-249)
- 升级流程里 `prompt_yes_no` 的使用(命令本身若它处复用则保留)

## 4. CLI 接口(简化到极致)

```
kfcode upgrade
```

- 无参数、无 `--check`、无 `--yes`、无 `target`、无 `--method`。
- 行为:检查 GitHub Release 最新版,已是最新则提示退出(方案 A),否则下载替换到最新。

## 5. 新模块:`crates/kfcode-cli/src/upgrade.rs`

升级逻辑自成一体,从 ~3000 行的 main.rs 抽出为独立模块。职责:

1. 查询最新 Release 版本(GitHub API)
2. 运行时探测平台 → target triple → asset 名
3. 下载 archive + sha256,校验
4. 解压取出 `kfcode` 二进制
5. self-replace 原子替换当前二进制

## 6. `kfcode upgrade` 数据流

```
1. 查最新版本
   GET https://api.github.com/repos/dfbb/KFCode/releases/latest
   Header: User-Agent: kfcode-cli, Accept: application/vnd.github+json
   tag_name 去 v 前缀 → "0.1.2"

2. 比较版本(方案 A)
   current = env!("CARGO_PKG_VERSION")(编译时固化)
   current == latest → 打印"已是最新版"，退出 0;否则继续

3. 定位本平台 asset
   运行时探测 OS/arch → target triple → asset 文件名:
     macOS arm64   → aarch64-apple-darwin    → kfcode-cli-aarch64-apple-darwin.tar.gz
     Linux amd64   → x86_64-unknown-linux-gnu → kfcode-cli-x86_64-unknown-linux-gnu.tar.gz
     Windows amd64 → x86_64-pc-windows-msvc   → kfcode-cli-x86_64-pc-windows-msvc.zip
   从 assets[] 按文件名匹配，取 browser_download_url
   无匹配(如 Intel Mac)→ 明确报错退出

4. 下载 archive + 对应 .sha256 到临时目录，算 sha256 比对，不符则中止

5. 解压取二进制:tar.gz / zip → 取出 kfcode-cli-<triple>/kfcode(.exe)

6. self-replace 原子替换 std::env::current_exe()

   打印"已从 0.1.1 升级到 0.1.2"
```

**错误处理**:每步失败给清晰中文提示并非零退出;清理临时文件;替换用原子操作。

**关键边界**:
- 本平台无 asset(Intel Mac):报错"当前平台 <triple> 无对应发布产物"
- 网络失败 / API 限流:提示并退出
- 无写权限(二进制在系统目录、非 root):捕获权限错误,提示用对应方式升级(如 `brew upgrade` 或加权限)

## 7. 依赖(方案 X)

下载用现有 `reqwest`;新增/显式声明:

| crate | 用途 | 现状 |
|-------|------|------|
| `self-replace` | 原子替换当前二进制(含 Windows 自删除) | 新增 |
| `flate2` | 解 gzip | lock 中已间接存在,需显式声明 |
| `tar` | 解 tar | 新增 |
| `zip` | 解 Windows .zip | 新增 |
| `sha2` | sha256 校验 | workspace 已有 |

不使用 `self_update` 一站式库:它对 archive 内部结构有自身假设,定制 sha256 校验不灵活。

## 8. 启动时检查 + 空闲提示(仅 TUI)

**异步非阻塞**:启动时 spawn 独立任务查版本,主流程/TUI 立即正常启动,绝不因检查阻塞。

**限频缓存**:`<cache_dir>/kfcode/upgrade-check.json`
```json
{ "last_check": "2026-05-29T08:00:00Z", "latest_version": "0.1.2" }
```
- 距 last_check < 24h → 用缓存,不联网
- ≥ 24h → 后台异步查一次并更新缓存,**失败静默**(不影响启动)

**结果投递 + 空闲提示**(TUI):
- 检查任务完成后,经新增的 `Event::UpgradeAvailable(version)` 送入现有事件循环
  (TUI 已是 channel 模型:后台线程 poll crossterm → `event_tx_input` → 主循环 `event_rx`)
- 主循环收到时:**prompt 输入框为空(空闲)** → 显示提示横幅;**用户正在输入** → 丢弃,不打断
- "否则下次启动再显示":只要未升级,下次启动空闲时会再次提示

**提示文案**(一行):`有新版本 0.1.2 可用(当前 0.1.1),运行 kfcode upgrade 升级`

**CLI 模式**:完全不提示。升级检查与提示是 TUI 专属,保持 CLI 纯净(适合脚本/CI,无意外网络行为)。

## 9. 版本比较

自写三段 `u32` 比较,不引入 semver crate(版本号由 release.sh 保证为规整 X.Y.Z):
```
parse_version("0.1.2") -> (0, 1, 2)，逐段比较
```

## 10. 成功标准

1. 旧的 InstallMethod / detect_install_method / 多源 latest_version / run_upgrade_process 全部删除。
2. 新 `upgrade.rs` 模块:查 Release → 校验 sha256 → 解压 → self-replace,全链路可编译。
3. `kfcode upgrade` 无参数,已是最新则提示退出,否则升级到最新。
4. 本平台无 asset / 网络失败 / 无写权限,均有清晰中文报错。
5. TUI 启动异步检查,空闲(输入框空)才提示,不阻塞启动;CLI 不提示。
6. 24h 限频缓存生效,检查失败静默。
7. 三段版本比较正确;workspace 全量编译通过;新增单元测试覆盖版本比较与 asset 名推导。

