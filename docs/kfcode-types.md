# kfcode-types

`kfcode-types` defines shared data structures across crates to reduce duplication and serialization drift.

## Responsibilities

- Unified message model
- Unified session model
- Unified todo model

## Module structure

- `message.rs` – Message body, message fragment types
- `session.rs` – Session and metadata types
- `todo.rs` – Todo list types
- `lib.rs` – Re-exports

## Design principles

- Data structures should be stable
- Prefer backward compatibility when changing external structure
- Keep JSON serialization field semantics consistent

## Typical use

- `kfcode-session` uses these types in session flow
- `kfcode-storage` uses them as persistence boundary
- `kfcode-server` exposes them at the API layer

## Validation

```bash
cargo check -p kfcode-types
```
