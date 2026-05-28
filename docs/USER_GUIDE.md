# USER GUIDE - KFCode

This guide is for day-to-day users. It covers startup, common commands, configuration, and troubleshooting.  
The project is named `KFCode`. The CLI command is `kfcode`.

**Use this project's kfcode.** If you have another KFCode (e.g. npm/global) on your PATH, the shell may run that instead. From the **KFCode repo root** use one of:

- **`./target/debug/kfcode`** — after `cargo build -p kfcode-cli`
- **`cargo run -p kfcode-cli --`** — always uses this repo's version

Example: `./target/debug/kfcode tui` or `cargo run -p kfcode-cli -- tui`.  
In the commands below, `kfcode` means this local binary (run from repo root).

## 1. Quick start

From the KFCode repo root:

```bash
cargo run -p kfcode-cli -- --help
```

Start TUI by default:

```bash
cargo run -p kfcode-cli --
```

Same as:

```bash
cargo run -p kfcode-cli -- tui
```

Single non-interactive run:

```bash
cargo run -p kfcode-cli -- run "Summarise the current risks in this repo"
```

Start HTTP server:

```bash
cargo run -p kfcode-cli -- serve --port 3000 --hostname 127.0.0.1
```

## 2. Common commands

Run these from the **KFCode repo root**; use `./target/debug/kfcode` or `cargo run -p kfcode-cli --`.

### 2.1 Session management

```bash
./target/debug/kfcode session list
./target/debug/kfcode session list --format json
./target/debug/kfcode session show <SESSION_ID>
./target/debug/kfcode session delete <SESSION_ID>
```

### 2.2 Models and config

```bash
./target/debug/kfcode models
./target/debug/kfcode models --refresh --verbose
./target/debug/kfcode config
```

### 2.3 Auth management

```bash
./target/debug/kfcode auth list
./target/debug/kfcode auth login --help
./target/debug/kfcode auth logout --help
```

Note: For `auth login` / `auth logout` options, use the `--help` output.

### 2.4 MCP management

```bash
./target/debug/kfcode mcp list
./target/debug/kfcode mcp connect <NAME>
./target/debug/kfcode mcp disconnect <NAME>
./target/debug/kfcode mcp add --help
./target/debug/kfcode mcp auth --help
```

If the local server is not at the default address:

```bash
./target/debug/kfcode mcp --server http://127.0.0.1:3000 list
```

### 2.5 Debug commands

```bash
./target/debug/kfcode debug paths
./target/debug/kfcode debug config
./target/debug/kfcode debug skill
./target/debug/kfcode debug agent
```

## 3. TUI and Run common options

Show full options:

```bash
./target/debug/kfcode tui --help
./target/debug/kfcode run --help
```

Frequently used (both TUI and run):

- `-m, --model <MODEL>` – Set model
- `-c, --continue` – Continue latest session
- `-s, --session <SESSION>` – Continue specific session
- `--fork` – Fork session
- `--agent <AGENT>` – Set agent (default: `build`)
- `--port <PORT>` / `--hostname <HOSTNAME>` – Server address

Additional common options for `run`:

- `--format default|json`
- `-f, --file <FILE>`
- `--thinking`

## 4. Config file locations

The program merges config from multiple locations by priority (searching upward):

- `kfcode.jsonc`
- `kfcode.json`
- `.kfcode/kfcode.jsonc`
- `.kfcode/kfcode.json`

Global config default:

- `~/.config/kfcode/kfcode.jsonc` (or `.json`)

Recommendation: Start with a minimal project-level config, then add provider/mcp/agent/lsp as needed.

## 5. Using Claude (Anthropic) and testing a coding session

Claude is supported via the **anthropic** provider. You can run interactive coding in the TUI or a one-off task with `run`.

### 5.1 Set your API key

**Option A – Environment variable (easiest):**

```bash
export ANTHROPIC_API_KEY="sk-ant-your-key-here"
```

Get a key at [Anthropic Console](https://console.anthropic.com/).

**Option B – Config file:**

Create or edit `kfcode.jsonc` in your project (or `~/.config/kfcode/kfcode.jsonc`):

```jsonc
{
  "model": "anthropic/claude-sonnet-4-20250514",
  "provider": {
    "anthropic": {
      "api_key": "sk-ant-your-key-here"
    }
  }
}
```

Do not commit real API keys; use env vars or a local config that’s in `.gitignore`.

### 5.2 Check that the provider is available

```bash
./target/debug/kfcode auth list
./target/debug/kfcode models --refresh
```

You should see `anthropic` and Claude models in the list.

### 5.3 Run a coding session

**Interactive TUI (recommended for coding):**

```bash
./target/debug/kfcode tui -m anthropic/claude-sonnet-4-20250514
```

Or with default model from config:

```bash
./target/debug/kfcode tui
```

In the TUI you can type prompts, use slash commands, and let the agent edit files and run tools.

**Single task (no TUI):**

```bash
./target/debug/kfcode run "Add a unit test for the login function in src/auth.rs"
```

With a specific model:

```bash
./target/debug/kfcode run -m anthropic/claude-sonnet-4-20250514 "Review this repo for security issues"
```

### 5.4 Useful model IDs (Anthropic)

- `anthropic/claude-sonnet-4-20250514` – Claude Sonnet 4
- `anthropic/claude-3-5-sonnet-20241022` – Claude 3.5 Sonnet
- `anthropic/claude-3-opus-20240229` – Claude 3 Opus

Use `./target/debug/kfcode models` to see the full list for your setup.

## 6. Recommended workflows

### 6.1 Local interactive use

1. `cargo run -p kfcode-cli --` or `./target/debug/kfcode tui`
2. Run tasks in the TUI
3. Use `./target/debug/kfcode session list/show` to review history

### 6.2 Scripts or integration

1. `./target/debug/kfcode serve --port 3000`
2. Use `./target/debug/kfcode run ... --format json` or the server API for integration
3. Use `./target/debug/kfcode stats` to track token/cost

## 7. Troubleshooting

### 7.1 Port in use

- Use another port: `./target/debug/kfcode serve --port 3001`

### 7.2 Model not available

1. `./target/debug/kfcode auth list`
2. `./target/debug/kfcode models --refresh`
3. `./target/debug/kfcode config` to check that provider config is applied

### 7.3 Config issues

1. `./target/debug/kfcode debug paths` – see config search paths
2. `./target/debug/kfcode debug config` – see final merged config

### 7.4 MCP connection problems

1. `./target/debug/kfcode mcp list`
2. `./target/debug/kfcode mcp debug <NAME>`
3. `./target/debug/kfcode mcp connect <NAME>`

## 8. Documentation index

- Project overview: `README.md`
- Docs index: `docs/README.md`
- CLI: `docs/kfcode-cli.md`
- TUI: `docs/kfcode-tui.md`
- Config: `docs/kfcode-config.md`
