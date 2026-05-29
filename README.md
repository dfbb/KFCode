# KFCode

**KFCode — ported from [OpenCodeRust](https://github.com/ChrisFeldmeier/OpenCodeRust)**

KFCode is a port of [OpenCodeRust](https://github.com/ChrisFeldmeier/OpenCodeRust) by Chris Feldmeier, itself a Rust implementation of [opencode](https://github.com/sst/opencode) by SST. It provides a full CLI/TUI/Server workflow for local AI coding agents, session management, tool invocation, MCP/LSP integration, and plugin extensions.

## Current status

- Project name: `KFCode`
- Executable: `kfcode`

## Feature overview

- **Interaction modes:** TUI (default), CLI single run, HTTP server, Web/ACP mode
- **Sessions:** Create, continue, fork sessions; import/export
- **Tool system:** Built-in read/write/edit, shell, patch, and related tools
- **Model support:** Multiple providers, agent mode switching
- **Extensibility:** Plugin bridge (including TS plugins), MCP, LSP
- **Terminal:** Improved layout, collapsible sidebar, syntax highlighting, path completion

## Quick start

### 1. Requirements

- Rust stable
- Cargo
- Git (recommended)

### 2. Build

```bash
cargo build -p kfcode-cli
```

### 3. Use this project's binary

To avoid running another KFCode (e.g. npm/global) on your PATH when using KFCode, run from the **repo root**:

- **`./target/debug/kfcode`** after `cargo build -p kfcode-cli`
- **`cargo run -p kfcode-cli --`** to always use this repo's version

### 4. Show help

```bash
./target/debug/kfcode --help
```

or

```bash
cargo run -p kfcode-cli -- --help
```

### 5. How to run

- Default: start TUI:

```bash
cargo run -p kfcode-cli --
```

- Start TUI explicitly:

```bash
cargo run -p kfcode-cli -- tui
```

- Single non-interactive run:

```bash
cargo run -p kfcode-cli -- run "Check this repo for risks"
```

- Start HTTP server:

```bash
cargo run -p kfcode-cli -- serve --port 3000 --hostname 127.0.0.1
```

## CLI commands overview

From the repo root use `./target/debug/kfcode` or `cargo run -p kfcode-cli --`. These commands match the current `./target/debug/kfcode --help`:

- `tui` – Start interactive terminal UI
- `attach` – Attach to a running server
- `run` – Run a single message
- `serve` – Start HTTP server
- `web` – Start headless server and open web UI
- `acp` – Start ACP server
- `models` – List available models
- `session` – Session management
- `stats` – Token/cost statistics
- `db` – Database tools
- `config` – Show configuration
- `auth` – Credential management
- `agent` – Agent management
- `debug` – Debugging and troubleshooting
- `mcp` – MCP management
- `export` / `import` – Export/import sessions
- `github` / `pr` – GitHub-related features
- `upgrade` / `uninstall` – Upgrade and uninstall
- `generate` – Generate OpenAPI spec
- `version` – Show version

Subcommand help:

```bash
./target/debug/kfcode tui --help
./target/debug/kfcode run --help
./target/debug/kfcode serve --help
./target/debug/kfcode session --help
```

## Configuration

Configuration is merged from the following paths in priority order (searched upward):

- `kfcode.jsonc`
- `kfcode.json`
- `.kfcode/kfcode.jsonc`
- `.kfcode/kfcode.json`

Global config default path:

- Linux/macOS: `~/.config/kfcode/kfcode.jsonc` (or `.json`)

See: `docs/kfcode-config.md`

## Repository structure

- `crates/kfcode-cli` – CLI entrypoint (binary: `kfcode`)
- `crates/kfcode-server` – HTTP/SSE/WebSocket server
- `crates/kfcode-tui` – Terminal UI
- `crates/kfcode-session` – Sessions and messages
- `crates/kfcode-tool` – Tool registration and execution
- `crates/kfcode-provider` – Model provider adapters
- `crates/kfcode-plugin` – Plugin system and subprocess bridge
- `crates/kfcode-mcp` – MCP client and registration
- `crates/kfcode-lsp` – LSP support
- `crates/kfcode-storage` – SQLite storage

## Development and validation

```bash
cargo fmt
cargo check
cargo clippy --workspace --all-targets
```

Minimal check (typical):

```bash
cargo check -p kfcode-cli
cargo check -p kfcode-tui
```

## Documentation

- User guide: `USER_GUIDE.md`
- Docs index: `docs/README.md`
- CLI: `docs/kfcode-cli.md`
- TUI: `docs/kfcode-tui.md`
- Server: `docs/kfcode-server.md`
- Tools: `docs/kfcode-tool.md`
- Provider: `docs/kfcode-provider.md`
- Config: `docs/kfcode-config.md`

## Notes

- The executable is named `kfcode`. KFCode is the project name, ported from [OpenCodeRust](https://github.com/ChrisFeldmeier/OpenCodeRust).

## Acknowledgments

KFCode is a port of [OpenCodeRust](https://github.com/ChrisFeldmeier/OpenCodeRust), created and maintained by [Chris Feldmeier](https://github.com/ChrisFeldmeier). OpenCodeRust is itself a Rust implementation of [opencode](https://github.com/sst/opencode) by SST. We are grateful to the original authors for their work, without which KFCode would not exist.

KFCode is distributed under the MIT License. Copyright notices for OpenCodeRust and opencode are retained in [LICENSE](./LICENSE).
