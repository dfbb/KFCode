# 集成测试约定 Spec

**日期**：2026-05-29
**范围**：为 8 个 crate 补集成测试时统一遵循的工程约定
**目标 crate**：

- 强烈建议补：`kfcode-storage`、`kfcode-mcp`、`kfcode-lsp`、`kfcode-server`
- 建议补：`kfcode-watcher`、`kfcode-tool`、`kfcode-permission`、`kfcode-command`

## 1. 背景与目标

工作区共 19 个 crate。目前**有 3 个 crate 实质上有集成测试 `tests/*.rs`**：`kfcode-provider`、`kfcode-session`、`kfcode-config`。**其余 16 个 crate 都没有 `tests/*.rs`**，其中：`kfcode-plugin/tests/` 只放 fixture（`fixtures/echo-plugin.ts`），不是集成测试 crate；`kfcode-storage` 当前**0 测试**（既无单测也无集成测试）；其余若干 crate 仅有源文件内嵌的 `#[cfg(test)]` 单元测试。

经过覆盖密度与外部边界评估，识别出 8 个 crate 应当补充集成测试。这 8 个 crate 的协议、IO、跨 crate 协作各异，若各自为政会导致：fixture 写法不一致、mock 策略割裂、CI 行为不可预测。

本 spec 在动手补测试**之前**先统一约定。它不规定具体写什么测试（那是后续每个 batch 的实施计划要解决的），只规定**怎么写、放哪、用什么工具**。

## 2. 约定（必须遵循）

### 2.1 目录与文件组织

- 每个目标 crate 在 `crates/<name>/tests/` 下组织集成测试
- **按功能切面拆多个文件**：例 `tests/session_crud.rs` / `tests/migration.rs` / `tests/concurrency.rs`，单文件控制在数百行内
- 每个 crate 的共享 helper 放在 `tests/common/mod.rs`，子模块按需放 `tests/common/<topic>.rs`
- **唯一的 `dead_code` 写法**：在 `tests/common/mod.rs` 顶部放 crate-level `#![allow(dead_code)]`；测试文件**只**写 `mod common;`，不再加任何 `allow` 属性
- 不要使用 `tests/common.rs` 这种平铺写法——Rust 会把它当作独立测试 crate 编译
- 已有 3 个 `tests/*.rs` 老文件保持原样不动，约定仅约束新写测试

### 2.2 命名

- 测试函数采用**行为式命名，不带 `test_` 前缀**：`loads_session_from_disk` / `rejects_invalid_token` / `round_trips_message`
- 函数名应能读成英文短句，明确表达"这个测试在断言什么行为"
- 测试文件命名 `<切面>.rs`（如 `session_crud.rs`），不再使用 `<crate>_integration.rs` 的旧命名
- 已有老文件不改名

### 2.3 异步与运行时

- 异步测试统一使用 `#[tokio::test]`
- 默认使用单线程 runtime（`#[tokio::test]` 默认行为）；仅在测试**确实需要多线程**（如并发竞态）时显式写 `#[tokio::test(flavor = "multi_thread")]`
- 同步纯逻辑测试继续用 `#[test]`

### 2.4 依赖与 mock 策略

mock 工具按通信类型分别选择：

- **HTTP 请求/响应（含 OAuth 端点、模型 API、MCP HTTP transport 的 RPC 调用）**：使用 `wiremock`
- **SSE 长连接（如 MCP 的 `EventSource` 长连接，见 `crates/kfcode-mcp/src/transport.rs:297`）**：用 `axum` 起本地测试 server，手动写 `text/event-stream` 响应；wiremock 对长连接 SSE 控制粒度不够
- **WebSocket（如 server 的 PTY `pty_connect` axum upgrade，见 `crates/kfcode-server/src/routes.rs:3463`）**：用 `axum::serve` 起本地测试 server + `tokio-tungstenite` 客户端连入，**不**用 wiremock
- **本机 in-process server 生命周期**：必须返回 shutdown guard。`tokio::spawn` 出来的 `JoinHandle` 在 drop 时**不会**停止任务，只会 detach；测试结束时必须显式 `handle.abort()` 或通过 `oneshot` / `tokio_util::sync::CancellationToken` 触发 graceful shutdown。helper 应当返回一个持有 `JoinHandle` 的 RAII guard，其 `Drop` impl 调用 `abort()`，避免测试遗忘。
- 临时目录使用 `tempfile`（已在工作区使用）
- **不引入** `rstest` / `test-case` / `proptest` / `serial_test` 等额外测试框架——标准 `#[test]` / `#[tokio::test]` 够用
- **不引入** coverage 工具（`tarpaulin` / `llvm-cov`）配置——本约定不关心覆盖率数字
- 多个 crate 都要用的 dev-dep（`wiremock`、`tempfile`、`tokio-tungstenite` 等）建议在 workspace 根的 `[workspace.dependencies]` 统一版本，各 crate 在 `[dev-dependencies]` 下用 `{ workspace = true }` 引用——版本一致、互不污染（workspace 列出依赖只是声明可用版本，不显式 `workspace = true` 引用的 crate 不会拉取）

### 2.5 真实依赖与持久化状态隔离

默认原则：**能 in-process 真实就用真实，需要走外网的一律 mock**。但**任何测试都不得读写用户真实的 `data_dir` / `data_local_dir` / `home_dir`**。

**关于注入点的可见性**：Rust 集成测试位于 `crates/<name>/tests/`，把 crate 当作普通外部依赖编译——单纯 `#[cfg(test)]` 标注的内部 API **不可见**。所以下文要求新增的"test-only 入口"必须满足下列两条之一：

- **作为正常公开 API（默认选项）**：函数名清晰表达用途（如 `Database::open_at`），不靠 `#[cfg(test)]` 隐藏，作为 crate 长期支持的一部分；裸 `cargo test --workspace` 直接覆盖
- **通过 cargo feature 暴露**：crate 内定义 `test-utils` feature，相关入口用 `#[cfg(feature = "test-utils")]` 门控；测试本身用 `#[cfg(feature = "test-utils")]` 门控以匹配。**重要：feature 不会被 `cargo test --workspace` 自动启用**，且 `[[test]]` 的 `required-features` 只决定"feature 未启用时跳过该测试"，不会自行启用 feature。所以选这条路的 batch 必须：
  1. 在 crate 测试代码顶部用 `#![cfg(feature = "test-utils")]` 让裸 `cargo test --workspace` 时跳过、不报错
  2. 在该 batch 的本地运行命令明确写成 `cargo test -p <crate> --features test-utils`（也可以加 workspace 级 alias）
  3. 在 crate `README` / 模块文档里记一句"feature gated 测试需手动启用"，避免后续维护者困惑

每个 crate 的 plan 阶段决定走哪条路；**强烈优先正常公开 API**——更直观、可被 workspace 默认覆盖；只有当注入点确实只对测试有意义、放进公开 API 会污染稳定接口时才用 `test-utils` feature。

具体规则：

- **数据库**：不调用 `Database::new()`（它走 `dirs::data_local_dir()`，见 `crates/kfcode-storage/src/database.rs:148`）。集成测试有两种合法选择：
  1. **单连接 in-memory**：`Database::in_memory()`（`crates/kfcode-storage/src/database.rs:75`），但其 pool `max_connections=1`，**无法测多连接并发/锁行为**——并发场景必须用方式 2
  2. **tempdir 多连接**：按 §2.8 增加按上述可见性规则暴露的入口（如 `Database::open_at(path: &Path) -> Result<Self, _>`），复用 `Database::new()` 的连接逻辑但不走 `get_database_path()`，测试用 tempfile 路径打开，可配置多连接 pool
- **MCP OAuth 凭据**：`crates/kfcode-mcp/src/auth.rs:52` 的 `auth_file_path()` 写入 `dirs::data_dir()/kfcode/mcp-auth.json`。**禁用全局 setter**（如 `set_auth_path_for_testing`）——它和 `HOME` env 一样在并行测试里有串扰风险。允许的方案按优先级：
  1. **每次调用显式传 path / 注入 store**：把 auth 模块改造成"接受 `&Path` 或 `Arc<dyn AuthStore>` 参数"的形态，每个测试构造各自独立的 store；**首选这条**
  2. 若改造成本不可接受，保留全局 path 状态时必须用 **scoped guard + mutex 串行**：定义 `set_auth_path_scoped(path) -> Guard`，进入时锁全局 `Mutex`、guard `Drop` 时还原前值；用此入口的所有测试自动通过 mutex 串行，但因此**不能并行**——明确写入 plan 并在测试代码里加注释提醒
- **server 启动**：`crates/kfcode-server/src/server.rs` 直接 `Database::new()`。集成测试**必须**为 server 提供注入 `Database` 实例的入口（test constructor 或 builder，按上述可见性规则暴露），而非依赖默认路径
- **文件系统**：使用 tempdir，不 mock
- **HTTP / OAuth provider / 外部模型 API**：一律使用 `wiremock` 起本地 mock server，不联外网
- **LSP server**：见 2.7
- **MCP server**：HTTP RPC 用 wiremock；SSE 用本地 axum server
- **外部命令**（如测 bash 工具）：使用 `echo` 这类无副作用的命令；CI 环境不一定有 `bash` 时按 2.6 标 `#[ignore]`

如果某个 crate 当前**没有**注入路径/连接的能力（如 `kfcode-server`、`kfcode-mcp::auth`），按 2.8 的"为可测性修源代码"补一个最小注入点，按本节可见性规则暴露。

### 2.6 `#[ignore]` 用法

`#[ignore]` 用于**外部环境依赖**或**确认 flaky 且短期内无法稳定**的测试，不是慢测试的兜底。默认 `cargo test --workspace` 跑下面"应当 ignore"以外的、且不依赖 §2.5 `test-utils` feature 的全部测试；`cargo test --workspace -- --ignored` 跑被 `#[ignore]` 标记的；`test-utils` feature 门控的测试需按 §2.5 显式 `cargo test -p <crate> --features test-utils` 启动。

何时**应当** `#[ignore]`：

- 依赖外部 binary（`rust-analyzer`、`bash`、`ripgrep` 之类）且不在所有开发/CI 环境都可用
- 已确认存在平台相关 flaky（如 macOS 文件系统事件时序），且修复需要单独立项
- 显著超出 suite budget 的端到端长流程（**非**指单测级 1 秒，而是几十秒/几分钟级别），且短流程已足够覆盖该切面

何时**不应** `#[ignore]`：

- 仅因为"测试比较慢"——确定性的集成测试本来就比单测慢，cargo test 默认就该跑
- 仅因为"懒得修 flaky"——应当先定位 flaky 根因，修不了再 `#[ignore]` 并附 issue 链接

`#[ignore]` 的测试必须在测试函数旁加注释说明原因，例：

```rust
#[tokio::test]
#[ignore = "依赖 rust-analyzer binary，本地按需运行"]
async fn handles_real_lsp_initialize() { /* ... */ }
```

### 2.7 LSP 测试策略

`LspClient::start`（`crates/kfcode-lsp/src/lib.rs:201`）当前固定通过 `Command::new(...).spawn()` 启动外部进程，**没有** in-process transport 注入点。集成测试有两条可选路径，二选一：

- **路径 A：fixture binary / script**——给 `LspServerConfig` 一个能跑通的可执行 stub。**注意：`tests/fixtures/` 下的 `.rs` 文件不会被 cargo 自动编译**，所以下面三种 stub 形态按可执行性排序：
  1. **workspace 内 helper binary（首选）**——在 `kfcode-lsp/Cargo.toml` 加 `[[bin]]` 段（如 `name = "lsp-test-stub"`、`path = "tests/fixtures/stub.rs"`、`required-features = ["test-utils"]` 配合 §2.5 的 feature 路径，或直接无门控暴露），cargo 在 `cargo test -p kfcode-lsp` 时自动编译；测试通过 `env!("CARGO_BIN_EXE_lsp-test-stub")` 拿到路径并交给 `LspClient::start`。**不依赖外部解释器，跨平台**
  2. **当前 test binary self-spawn**——把 stub 逻辑写成测试内的一个函数，测试启动时 `Command::new(std::env::current_exe()).args(["--test-stub-mode", ...])` 重新拉起当前 test binary 走 stub 分支；只在没有 helper binary 路径时使用，stub 与测试代码耦合较紧
  3. **node / python 脚本**——仅在前两种确实不可行时使用；按 §2.6 视为"外部 binary 依赖"必须 `#[ignore]`，且测试启动前 `which::which("node")` 检查，缺解释器时 skip 而非 fail
- **路径 B：为 LSP 增加 test-only transport/process 注入接口**——在 `kfcode-lsp` 内引入"接受外部 `ChildStdin`/`ChildStdout` 或抽象 `LspTransport` trait"的接口，允许测试用 in-process 通道驱动握手与诊断流。优点：测试稳定快速、无需外部进程；缺点：需要改源码（属于 §2.8 允许的范围），按 §2.5 可见性规则用正常公开 API 或 `test-utils` feature 暴露

batch 2 实施时由 plan 选定其中一条；首选路径 A 形态 1 或路径 B（这两条都不依赖外部解释器）。OAuth 之外不依赖真实 `rust-analyzer`；如确需真实 server，标 `#[ignore]`。

### 2.8 源代码修改边界

集成测试不是只读活动。允许的源代码修改：

- **修复测试发现的真实 bug**：例如 `kfcode-storage` 当前未配置 SQLite `PRAGMA foreign_keys=ON`（见 `crates/kfcode-storage/src/database.rs`），外键约束测试会暴露这点；按 bug 修复处理，独立提交
- **为可测性增加最小注入点**：例如为 server 增加接受外部 `Database` 的 constructor、为 MCP auth 增加路径注入；遵循"加最少接口、不破坏现有 API"
- **修复 spec 切面发现的实现缺陷**：例如 `kfcode-command` 命令名冲突直接覆盖（无错误），若 plan 决定该作为 bug 修复，独立提交并改测试切面

不允许的修改：

- 为通过测试随意扩大公开 API（无端把 private 改 pub）
- 为绕开 flaky 删除断言或弱化断言
- 与本次集成测试任务无关的重构

每个源代码修改在所属 batch PR 内独立成 commit，并在 commit message 写明动机（"测试 X 发现 Y bug" / "为 Z 测试增加注入点"）。

## 3. 切面清单

每个 crate 的集成测试至少应覆盖下列切面（不规定具体测试函数数量与名字，由 batch 实施时按风险分配）。覆盖目标：黄金路径必测，关键错误路径必测，边界按"出过 bug 容易藏哪里"选测。

### 3.1 强烈建议补的 4 个

#### `kfcode-storage`（SQLite 数据层）

按真实 schema（`crates/kfcode-storage/src/schema.rs`）测试，表为 `sessions` / `messages` / `parts` / `todos` / `permissions` / `session_shares`。

- 黄金路径：按真实可用 repo 调用，不直接拼业务 SQL。当前 `crates/kfcode-storage/src/lib.rs` 把 `pub mod repository` 整体公开（`crates/kfcode-storage/src/lib.rs:5`），所以 `SessionRepository` / `MessageRepository` / `TodoRepository` 通过顶层 re-export 拿到，`PartRepository` / `ShareRepository`（`crates/kfcode-storage/src/repository.rs:725` 等）通过 `kfcode_storage::repository::PartRepository` 直接可访问——**不需要因 re-export 缺失而跳过**，re-export 只是 ergonomics（plan 阶段可顺手补 re-export，不补也不影响测试可达性）。`permissions` 表**当前没有 repository**，对它的覆盖只限于 schema/migration 级（建表存在、列与索引正确），允许在测试中写最小 SQL 直接断言 `PRAGMA table_info(permissions)` 等元信息——除非 plan 决定在本批次补 `PermissionRepository` 后再走 repo API
- 错误路径：唯一约束冲突、外键违反；JSON 字段反序列化失败的处理——当前 repository 多处对损坏 JSON 走 `.and_then(... .ok())` **静默降级为 `None` / default**（见 `crates/kfcode-storage/src/repository.rs:56`、`:84`、`:510`）。**默认按现状测**：写入合法 JSON → 读出一致；写入损坏 JSON（绕过 setter 直接 SQL UPDATE 或在 fixture 写脏数据）→ 读出降级值且 row 仍可用、不 panic。如果 plan 认为静默降级是 bug，按 §2.8 改成显式错误并独立提交，同时把测试切面切到"返回 `DatabaseError::Deserialization`"
- 边界：空字符串、超长字段、unicode；多连接并发写同一行（按 §2.5 用 tempdir 打开多连接 pool，不要用 in-memory 单连接）
- 事务：commit / rollback 行为（`Database::transaction()`），错误传播
- 迁移：当前迁移机制是启动时跑一遍 `ALL_MIGRATIONS`（`crates/kfcode-storage/src/schema.rs:182`），**没有 schema_version 表**。集成测试覆盖："空库初始化能跑完所有迁移"、"重复打开同一个 tempfile DB 不重建/不破坏数据"。如要测"版本号校验/部分迁移"，需先按 §2.8 加 schema_version 表，作为 bug 修复独立提交，不在本 spec 默认范围
- 真实依赖：默认 `Database::in_memory()`；并发与多连接相关测试用 §2.5 方式 2 的 tempdir 入口
- **预期发现的实现缺陷**：当前源码未见 `PRAGMA foreign_keys=ON`（SQLite 默认关闭外键）；外键约束相关测试要求先在源代码启用 PRAGMA，按 §2.8 作为 bug 修复独立提交

#### `kfcode-mcp`（MCP 协议 + OAuth）

- 黄金路径：HTTP transport 的 `initialize` → `tools/list` → `tools/call` 全流程（按 `crates/kfcode-mcp/src/client.rs` 的 `connect_http` / `send_request` 真实顺序）；SSE transport 的 streaming 消息
- 错误路径：HTTP 4xx/5xx、SSE 断流、JSON-RPC error envelope、超时
- OAuth：authorization code 流程、token refresh、token 过期时拒绝；凭据存储路径必须按 2.5 隔离（不写入用户真实 `data_dir`）
- 边界：大 payload、并发请求、重连重试
- 真实依赖：HTTP RPC 用 wiremock 模拟 MCP server 与 OAuth provider；SSE 长连接用本地 `axum` server（见 2.4）

#### `kfcode-lsp`（LSP JSON-RPC 客户端）

- 黄金路径：`initialize` → `didOpen` → `didChange` → `diagnostics` → `shutdown`
- 错误路径：server 不响应、返回 error、协议版本不匹配
- 能力协商：capabilities 字段子集匹配
- 文档同步：增量更新、版本号一致性
- 真实依赖：按 2.7 选定路径 A（fixture binary/script）或路径 B（test-only 注入接口），不假设 in-process stub 可用；不依赖真实 `rust-analyzer`，如确需则按 2.6 标 `#[ignore]`

#### `kfcode-server`（HTTP / WebSocket）

router 在 `crates/kfcode-server/src/routes.rs:43` 起按 route group 挂载（`/session` / `/provider` / `/config` / `/mcp` / `/file` / `/find` / `/permission` / `/project` / `/pty` / `/question` / `/tui` / `/global` / `/experimental` / `/plugin`）。**不要承诺"每条 route 都返回 200"**——许多路由依赖具体 fixture（已存在的 session/项目/MCP 配置等）。

- 黄金路径：每个 route group **挑选 1-2 条代表性路由**走通（如 `POST /session` 创建 + `GET /session/:id` 取回；`GET /provider` 列模型；`GET /config`；`GET /mcp` 列已配置 server）；不强求覆盖每条路由
- 错误路径：按现有 `ApiError` 真实映射覆盖（`crates/kfcode-server/src/error.rs`）——400 BadRequest / InvalidRequest、404 NotFound / SessionNotFound、502 ProviderError、500 InternalError；非法 JSON 入参；未知 path。**401 / 403 暂不在范围**——当前 `ApiError` 没有 unauthorized/forbidden 分支，相关测试在后续实现鉴权后再补
- 跨切面：CORS 行为、并发连接
- **测试隔离（关键）**：`kfcode-server` 内有大量进程级全局状态——CORS whitelist、session run status、MCP manager、PTY manager、permission/question/TUI queues 等（`crates/kfcode-server/src/server.rs:426`、`crates/kfcode-server/src/routes.rs:2507`），**集成测试间不天然隔离**。规则：
  1. 每个测试创建新 `Database`（按 §2.5 入口）和新 `ServerState`，不复用全局 singleton
  2. 涉及 session / project / MCP / PTY 的测试为每条记录使用唯一 ID（如 `format!("test-{}-{}", function_name, uuid)`），结束时显式清理（删除 session、关闭 PTY、清空 queue）；不要假设并行测试会自动看到彼此的清理
  3. 当某全局状态没有 reset/clear API 时，按 §2.8 增加最小 reset hook（如 `clear_for_testing()` 或返回 `Drop` guard 的 scoped setter）；按 §2.5 可见性规则暴露
  4. 触碰真正进程级、无法分隔的全局（如 `OnceLock` 初始化的 manager）时，相关测试族用 `#[tokio::test(flavor = "multi_thread")]` + `Mutex` 串行；如串行成本过高，标 `#[ignore]` 留给单独运行
- 集成：与 `kfcode-storage` / `kfcode-session` 的真实协作（不 mock 下游 crate）；server 必须接受外部注入的 `Database` 实例（按 §2.5、§2.8 增加注入点），不调用默认的 `Database::new()`
- WebSocket：当前**只有 PTY WebSocket**（`crates/kfcode-server/src/routes.rs:3466` 的 `WebSocketUpgrade` → `handle_pty_websocket`）。测试覆盖：握手成功、按 PTY 协议（subscription + cursor）订阅、收到 PTY 输出帧、客户端正常关闭。**不要**写"泛化 ping/pong"测试——server 不实现这种行为
- 真实依赖：用 `axum::serve` + 端口 0 起测试 server、`tokio-tungstenite` 客户端连入；按 §2.4 的 shutdown guard 规则管理生命周期

### 3.2 建议补的 4 个

#### `kfcode-watcher`（文件系统监听）

- 黄金路径：单路径监听、文件创建/修改/删除事件正确触发
- 多路径：多目录同时监听、事件聚合
- 错误路径：监听不存在的路径、权限不足
- 边界：高频写入下的事件丢失/合并、忽略规则（如 `.git/`）
- 真实依赖：tempdir + 真 fs
- flaky 风险：默认不 `#[ignore]`；若实际跑出 flaky 再按 2.6 标记

#### `kfcode-tool`（工具注册表 / 执行）

- 黄金路径：工具注册、按名查找、执行内置工具（read / edit / bash 等代表性几个）
- 错误路径：未注册工具调用、参数 schema 校验失败、执行错误透传
- 边界：参数为空、超长字符串、unicode 路径
- 权限协作：`ToolRegistry::execute` **不直接**调用 `kfcode_permission::disabled`；权限只通过 `ToolContext::ask_permission` callback 介入（默认无 callback 时 `ask_permission` 直接 allow，见 `crates/kfcode-tool/src/registry.rs:86`、`crates/kfcode-tool/src/tool.rs:375`）。集成测试切面应当是：构造 deny callback → 验证 tool 调用收到 `ToolError` 并不执行副作用；以及构造 allow callback → 验证正常透传。"被禁用工具应被拒绝"这种基于 `disabled()` 的过滤测试应放在真正调用 `kfcode_permission::disabled` 的上层（如 agent 或 CLI 层），不在 `kfcode-tool` crate 范围
- 真实依赖：tempdir 跑 read / edit 真实工具；bash 工具因环境差异按 §2.6 标 `#[ignore]`

#### `kfcode-permission`（权限引擎）

- 黄金路径：`evaluate` 对 allow / deny 规则的匹配；`disabled` 对工具集合的过滤（**两者都基于 `crates/kfcode-permission/src/ruleset.rs:154` 的 `wildcard_match`**）；引擎的 pending → response 流程（`approved` 缓存命中 / 不命中，**基于 `crates/kfcode-permission/src/engine.rs:253` 中独立的 `wildcard_match` 实现**——这是与 `ruleset.rs` 并存的另一份实现，需各自单测）
- 匹配器：两份 `wildcard_match` 实现分别覆盖：精确等于、`prefix*`、`*suffix`、`*middle*`、`*` 全通配；不支持复杂 glob（spec 不假设支持）。`ruleset.rs` matcher 用于 `evaluate` / `disabled`；`engine.rs` matcher 用于 pending/approval 缓存。如果两份实现行为发生分歧（`*foo*bar` 这类边缘 pattern），按 §2.8 视为 bug，统一到一份
- 错误路径：未知 permission / 未注册 session（`PermissionError::NotFound`）、规则冲突时按"最后命中"或现有顺序语义（按实现确认）
- 边界：空 ruleset、规则数量极大、unicode pattern；edit 家族 alias（`edit`/`write`/`patch`/`multiedit`）映射到 `"edit"` 权限
- 跨场景：`build_agent_ruleset` 对 `"build"` / `"plan"` / `"explore"` 三种 agent 的差异（`crates/kfcode-permission/src/ruleset.rs:246` 起），未知 agent 名走默认分支
- 真实依赖：纯内存，无 IO

#### `kfcode-command`（命令系统）

按当前实现（`crates/kfcode-command/src/lib.rs`）测试现有行为，不假设未实现的特性：

- 黄金路径：`load_from_directory` 从 `.kfcode/commands/*.md` 加载（flat glob，**无嵌套子命令**）；`get` / `list` 取命令；`parse` 解析 slash-command 字符串与位置参数
- 描述提取：`extract_description` 从首行 `#` 标题或首条非空行（按真实实现）；当前**没有 frontmatter 解析**
- 命令名冲突：当前直接 `register` 覆盖（无错误）——按现状写测试断言"后注册的覆盖前者"；如 plan 决定改成报错，按 2.8 走 bug 修复
- 错误路径：缺失目录返回 `Ok(())` 不报错；非法 UTF-8 文件名 fallback 到 `"unknown"`
- 边界：参数透传、unicode 命令名
- 跨 crate：与 `kfcode-tool` / `kfcode-permission` 的协作以代码实际有的为准（plan 阶段确认）
- 真实依赖：tempdir 放命令文件

## 4. 最小可复用模板

下列模板使用 repo 真实 API（`Database::in_memory()` 等），**可以直接编译**。仅作起点参考，新写测试时按实际场景扩展，不要照抄断言内容。

### 4.1 wiremock 骨架（用于 HTTP 端点 mock）

下面只是**最小 wiremock 起停骨架**，不是 MCP HTTP transport 的完整跑通模板——`McpClient::http`（`crates/kfcode-mcp/src/client.rs:228`）的 `connect_http` 内部会先发 `initialize` 再发 `tools/list`，需要 mock 出符合协议的两条响应序列；具体的 MCP HTTP 端到端 fixture 由 batch 2 的 plan 决定。

```rust
// 通用 wiremock helper（按 crate 放在各自 tests/common/mod.rs 中）
#![allow(dead_code)]

use wiremock::{matchers::{method, path}, Mock, MockServer, ResponseTemplate};

pub async fn mock_post_json(server: &MockServer, route: &str, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}
```

> MCP 协议序列示例：`initialize`、`tools/list`、`tools/call` 都是同一路径上的 POST，**必须按 JSON-RPC `method` 字段匹配请求体，不能靠 mount 顺序**——wiremock 不保证按 mount 顺序选 mock。使用 wiremock 的 body matcher（如 `body_partial_json(serde_json::json!({ "method": "initialize" }))`）或自定义 `Match` 实现；每条 mock 单独 mount，测试运行时 wiremock 按 matcher 命中分发。具体 JSON 形状以 `crates/kfcode-mcp/src/protocol.rs` 与 `client.rs` 中的发送顺序为准。

### 4.2 in-memory SQLite 测试模板

```rust
// crates/kfcode-storage/tests/common/mod.rs
#![allow(dead_code)]

use kfcode_storage::Database;

pub async fn fresh_db() -> Database {
    Database::in_memory().await.expect("init in-memory db")
}
```

```rust
// crates/kfcode-storage/tests/session_crud.rs
mod common;

#[tokio::test]
async fn migrations_run_on_fresh_in_memory_db() {
    let _db = common::fresh_db().await;
    // 模板仅展示 fixture 起步；具体 CRUD 断言按 plan 阶段写入。
}
```

> 不要使用 `Database::new()`：它走 `dirs::data_local_dir()`（`crates/kfcode-storage/src/database.rs:148`），会污染用户真实数据目录，违反 §2.5。
> 并发/多连接场景需要 §2.5 方式 2 的 tempdir 多连接入口，由 batch 1 plan 增加。

### 4.3 axum 本地 server 模板（用于 SSE / WebSocket / 端到端 HTTP）

普通 HTTP 黄金路径优先用 `tower::ServiceExt::oneshot` 直接调用 `Router`，**不**起真实 server、不需要 `reqwest`。但 `ServiceExt::oneshot` 在 `tower 0.5` 里属于 `tower::util` 模块，需要启用 `util` feature——workspace 当前 `tower = "0.5"` 没启用任何 feature（`Cargo.toml:64`），server crate 也只是 `tower = { workspace = true }`（`crates/kfcode-server/Cargo.toml:28`），因此 batch 3 必须在 server 的 `[dev-dependencies]` 里**显式覆盖**为带 feature 的版本：

```toml
# crates/kfcode-server/Cargo.toml
[dev-dependencies]
tower = { workspace = true, features = ["util"] }
```

> 不要为这一个 feature 改动 workspace 全局 `tower` 声明（其他 crate 用不到 `util`，加上去会扩大编译图）。

```rust
// crates/kfcode-server/tests/health.rs（HTTP 直调，无需绑端口）
use axum::{body::Body, http::Request};
use tower::ServiceExt; // require tower feature "util"

#[tokio::test]
async fn responds_to_health() {
    let app = build_app(); // 项目自身的 Router 构造（注入 test Database）
    let res = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
    assert_eq!(&bytes[..], b"ok");
}
```

batch 3 plan 二选一：

- **路线 A：启用 `tower` 的 `util` feature**，用上面的 `ServiceExt::oneshot` 在内存中调用 `Router`，无需端口；适合 REST 路由的黄金路径与错误路径
- **路线 B：起真实端口**，用 `TestServer` guard + `tokio-tungstenite`（WebSocket）/ `reqwest`（HTTP）连入；WebSocket、SSE 长连接、需要真实 TCP 行为时必须用这条。`reqwest` 当前不在 `kfcode-server/Cargo.toml`，需在 `[dev-dependencies]` 加 `reqwest = { workspace = true }`（workspace 已声明 `reqwest = { version = "0.12", features = ["json", "stream"] }`）

> 不要寄望 `axum::Router::call_with_state` 这类内部 API——在 axum 0.8 里它是 `pub(crate)`，外部测试无法调用。两条路线之外没有更轻的方案。

需要真实端口/连接（WebSocket、SSE 长连接、外部客户端）时才起 server，并返回 RAII `TestServer` guard，drop 时自动 abort，避免泄漏后台任务（见 §2.4 关于 `JoinHandle` 不会自动停止任务的说明）：

```rust
// crates/kfcode-server/tests/common/mod.rs（仅 WebSocket / SSE 用）
#![allow(dead_code)]

use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub struct TestServer {
    pub addr: SocketAddr,
    handle: JoinHandle<()>,
}

impl TestServer {
    pub async fn spawn(app: Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Self { addr, handle }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
```

> WebSocket 测试用 `tokio-tungstenite` 连 `ws://{addr}/...`；按真实 server 的 PTY 协议（`pty_connect` 路径 + `subscription` + `cursor`）发送，参见 §3.1 `kfcode-server` 切面。

## 5. 不在范围内（YAGNI 声明）

- **不引入 CI 配置**（GitHub Actions 等）。本 spec 只产出测试代码与本地运行约定（`cargo test --workspace` 跑稳定的，`-- --ignored` 跑外部环境/已知 flaky/超长流程）。CI 配置作为后续独立话题。
- **不为通过测试随意扩大公开 API**。源代码修改边界由 2.8 规定：允许修真实 bug、加最小注入点；不允许把 private 改 pub 只为测试方便、不允许无关重构。
- **不重写已有 3 个 `tests/*.rs`**（`kfcode-provider` / `kfcode-session` / `kfcode-config` 老文件保持原样）；`kfcode-plugin/tests/fixtures/` 的 fixture 也不动。
- **不补"不必补"那 7 个 crate 的集成测试**（`kfcode-types` / `kfcode-core` / `kfcode-util` / `kfcode-grep` / `kfcode-cli` / `kfcode-tui` / `kfcode-agent`）；`kfcode-plugin` 也暂不补 `tests/*.rs`（前置评估未列入"建议补"）。
- **不引入额外测试框架**（`rstest` / `test-case` / `proptest`）。
- **不引入 coverage 工具配置**（`tarpaulin` / `llvm-cov`）。

## 6. 实施 batch 划分

后续每个 batch 各自走 `writing-plans` → 实施 → PR，互不阻塞。

| Batch | crate | 理由 |
|---|---|---|
| 1 | `kfcode-storage` | 0 单测、数据正确性最关键、无外部依赖、可独立完成 |
| 2 | `kfcode-mcp` + `kfcode-lsp` | 协议层，wiremock + 本地 axum server + LSP 注入策略（按 2.7）的经验可在两者间复用 |
| 3 | `kfcode-server` | 端到端层，依赖 storage 已稳定后才好做"真实跨 crate 协作"测试 |
| 4 | `kfcode-watcher` + `kfcode-tool` + `kfcode-permission` + `kfcode-command` | 独立度高，可并行；单个 crate 测试规模小 |

batch 1 完成后再 brainstorm + plan batch 2，依次推进。

## 7. 工作区操作

- 实施时为每个 batch 单独开 git worktree（沿用 `superpowers:using-git-worktrees`），避免主分支被半成品阻塞
- 每个 batch 完成后按 `superpowers:finishing-a-development-branch` 流程合入
