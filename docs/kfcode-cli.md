# kfcode-cli

`kfcode-cli` provides the workspace’s unified executable entrypoint (binary name: `kfcode`).

## Command scope

- Start TUI
- Start or attach to server
- Run single tasks (`run`)
- Invoke session, model, MCP, debug, and other subcommands

## Top-level subcommands

As of `kfcode --help` (2026-02-23):

- `tui`
- `attach`
- `run`
- `serve`
- `web`
- `acp`
- `models`
- `session`
- `stats`
- `db`
- `config`
- `auth`
- `agent`
- `debug`
- `mcp`
- `export`
- `import`
- `github`
- `pr`
- `upgrade`
- `uninstall`
- `generate`
- `version`

## Common options

### `kfcode tui`

- `-m, --model <MODEL>`
- `-c, --continue`
- `-s, --session <SESSION>`
- `--fork`
- `--agent <AGENT>` (default: `build`)
- `--port <PORT>`, `--hostname <HOSTNAME>`

### `kfcode run`

- `MESSAGE...`
- `--command <COMMAND>`
- `-f, --file <FILE>`
- `--format <default|json>`
- `--thinking`
- `--agent <AGENT>` / `--model <MODEL>`

## Source entrypoint

- `crates/kfcode-cli/src/main.rs`

## Development notes

- After changing subcommand behaviour, update `--help` and then the docs
- Prefer consistent naming between CLI args and server/config fields

## Validation

```bash
cargo check -p kfcode-cli
./target/debug/kfcode --help
```
