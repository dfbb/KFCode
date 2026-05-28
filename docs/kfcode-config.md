# kfcode-config

`kfcode-config` handles config file discovery, loading, parsing, and merging. It is the configuration entrypoint for runtime behaviour.

## Responsibilities

- Search for config files (project and global)
- Parse JSON/JSONC (comments supported)
- Map config to strongly typed structures
- Provide well-known paths and defaults

## Module structure

- `loader.rs` – Config loading, path lookup, merge flow
- `schema.rs` – Config structure definitions
- `wellknown.rs` – Common directory/file path constants

## Config paths (common)

- Project: `kfcode.jsonc` / `kfcode.json`
- Project extension: `.kfcode/kfcode.jsonc` / `.kfcode/kfcode.json`
- Global: `~/.config/kfcode/kfcode.jsonc` (or `.json`)

## Usage notes

- When adding new config fields, define default behaviour
- Merge behaviour should remain predictable
- Changes to provider/mcp/agent fields should be reflected in docs

## Validation

```bash
cargo check -p kfcode-config
```
