# KFCode documentation index

This documentation set corresponds to the current code state of **KFCode** (version: `2026.02.23`).

## Quick links

- Project overview: `README.md`
- Installation: `docs/INSTALL.md`
- User guide: `USER_GUIDE.md`
- Build & release: `docs/BUILD.md`
- CLI: `docs/kfcode-cli.md`
- TUI: `docs/kfcode-tui.md`
- Server: `docs/kfcode-server.md`

## Module documentation

- `docs/kfcode-agent.md` – Agent registration, execution, and message handling
- `docs/kfcode-cli.md` – `kfcode` command and subcommands
- `docs/kfcode-command.md` – Slash command registration and rendering
- `docs/kfcode-config.md` – Configuration loading and merging
- `docs/kfcode-core.md` – Event bus and ID infrastructure
- `docs/kfcode-grep.md` – Code and text search abstraction
- `docs/kfcode-lsp.md` – LSP client and registry
- `docs/kfcode-mcp.md` – MCP client, OAuth, transport layer
- `docs/kfcode-permission.md` – Permission rules and decision engine
- `docs/kfcode-plugin.md` – Hook system and TS subprocess bridge
- `docs/kfcode-provider.md` – Multi-provider model adapter layer
- `docs/kfcode-server.md` – HTTP routes, event stream, control endpoints
- `docs/kfcode-session.md` – Session lifecycle and message flow
- `docs/kfcode-storage.md` – SQLite storage and repository layer
- `docs/kfcode-tool.md` – Built-in tools and tool registry
- `docs/kfcode-tui.md` – Terminal UI architecture and interaction
- `docs/kfcode-types.md` – Shared data types across modules
- `docs/kfcode-util.md` – Filesystem, logging, and common utilities
- `docs/kfcode-watcher.md` – Filesystem watcher

## Code and documentation conventions

- The command name is `kfcode`.
- Documentation should follow the source code and `--help` output as the source of truth.
- Behaviour differences or refactoring plans are documented in `docs/overview/` (if that directory exists).
