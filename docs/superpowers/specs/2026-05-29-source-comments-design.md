# Source Code Comments — Design Spec

Date: 2026-05-29
Topic: Add accurate, appropriate English comments to all source code in the project.

## Goal

Add English documentation comments to every Rust source file in all 19 crates of
the workspace, covering the public API and key non-obvious internal logic. Do
not change any code (identifiers, signatures, visibility, implementation,
imports, or formatting all stay as-is).

Out of scope:
- The 4 TypeScript files under `crates/kfcode-plugin/builtin/` and
  `crates/kfcode-plugin/host/` are skipped.
- Tests under `tests/` directories and `#[cfg(test)]` blocks are skipped.
- Generated/included code is skipped.

## Context

- Workspace has 19 crates and ~208 Rust source files (~90k lines total).
- Existing doc-comment coverage is low: only 53 of 208 files contain any `///`
  or `//!`, ~615 doc-comment lines in total.
- The author's global `CLAUDE.md` says "default to no comments; only add when
  WHY is non-obvious." The user explicitly chose, for this task, to override
  that default with an "API docs + key internal explanations" policy.

## Approach

Three execution paths were considered:

1. Single-agent serial — slowest, heaviest main-context cost.
2. Single-agent serial with read-only subagent helpers — moderate.
3. **Per-crate parallel subagents in dependency-layered batches** — chosen.

The chosen approach runs one subagent per crate, batched by the crate
dependency graph. Each batch runs concurrently; batches run serially.

## Comment Standard (binding for every subagent)

### Required

1. Each `.rs` file gets a top-of-file `//!` module summary (1–3 lines).
2. Every `pub` item gets a `///` doc comment:
   - `pub struct` / `pub enum` / `pub trait` — what the type represents and
     typical usage.
   - `pub fn` / `pub async fn` — what it does; for non-obvious behavior use
     `# Errors`, `# Panics`, or `# Note` sections.
   - `pub mod` — one line.
   - `pub const` / `pub static` — meaning and units (if any).
3. Non-obvious private logic gets `//` line comments that explain **why**, not
   what. Examples: `// SAFETY:`, `// Workaround for ...`, `// Invariant: ...`.

### Forbidden

- No code changes whatsoever (identifiers, signatures, visibility, imports,
  formatting).
- No fenced code blocks ` ``` ` inside doc comments (avoids unintended
  doctests).
- No emoji.
- No Chinese (English-only).
- No comments that restate the type signature ("Returns a String").
- No references to PR numbers, issue numbers, or other file paths.
- No `TODO` or `FIXME` markers.
- Existing comments stay as-is unless they directly contradict a new comment or
  are obviously wrong.

### Templates

Module header (first lines of every file):

```rust
//! <One sentence describing the module's responsibility>.
//!
//! <Optional second paragraph: core types/functions, or relation to other
//! modules.>
```

Struct:

```rust
/// <What this type represents>.
/// <Optional: typical construction, lifetime constraints, concurrency.>
pub struct Foo { ... }
```

Function:

```rust
/// <Verb-led: what it does>.
/// # Errors
/// <When it returns Err.>
pub fn parse(input: &str) -> Result<Foo, Error> { ... }
```

Trait:

```rust
/// <What the abstraction represents>.
/// Implementors must <core contract>.
pub trait Provider { ... }
```

Enum: each `pub` variant gets a single-line `///` directly above it.

### Boundaries

- `tests/` directories and `#[cfg(test)]` modules: skip.
- Generated code: skip.
- Re-exports (`pub use foo::*`): doc comment optional.
- Pre-existing `///` content: keep unless wrong.

## Architecture: Dependency-Layered Batching

Crate internal dependencies (from `Cargo.toml` analysis):

- **Layer 0 (no internal deps)**: `kfcode-core`, `kfcode-types`,
  `kfcode-plugin`, `kfcode-lsp`, `kfcode-watcher`.
- **Layer 1 (depends only on layer 0)**: `kfcode-config`, `kfcode-util`,
  `kfcode-grep`, `kfcode-mcp`, `kfcode-permission`, `kfcode-provider`,
  `kfcode-storage`, `kfcode-command`.
- **Layer 2 (depends on layers 0 and 1)**: `kfcode-tool`, `kfcode-agent`,
  `kfcode-session`, `kfcode-server`, `kfcode-tui`, `kfcode-cli`.

Three batches map directly to these layers. Within a batch, subagents run in
parallel; batches run sequentially.

## Subagent Contract

Each subagent receives a self-contained prompt with:
- Target crate name and path.
- Full comment standard (above).
- Step-by-step workflow.
- Required report format.

Workflow each subagent follows:

1. Read `crates/<crate>/Cargo.toml` to understand crate purpose.
2. List all target `.rs` files under `crates/<crate>/src/`.
3. Add comments file-by-file using `Edit` (never `Write`).
4. Run, from the workspace root:
   - `cargo check -p <crate>`
   - `cargo doc -p <crate> --no-deps`
5. If `check` fails due to introduced comments (rare; usually doctest):
   remove the offending comment and re-run.
6. If `doc` reports broken-link warnings: replace the broken intra-doc link
   with plain text. Do not introduce new types or imports to make a link
   resolve.

Required report (≤200 words):

- `crate`: name
- `files_modified`: count
- `files_skipped`: list with reasons
- `cargo_check`: PASS / FAIL with summary
- `cargo_doc`: PASS with warning count / FAIL with summary
- `notes`: optional

## Batches

### Batch 1 — 5 concurrent subagents (Layer 0)

| ID | Crate |
|----|-------|
| 1.1 | `kfcode-core` |
| 1.2 | `kfcode-types` |
| 1.3 | `kfcode-plugin` |
| 1.4 | `kfcode-lsp` |
| 1.5 | `kfcode-watcher` |

### Batch 2 — 8 concurrent subagents (Layer 1)

| ID | Crate |
|----|-------|
| 2.1 | `kfcode-config` |
| 2.2 | `kfcode-util` |
| 2.3 | `kfcode-grep` |
| 2.4 | `kfcode-mcp` |
| 2.5 | `kfcode-permission` |
| 2.6 | `kfcode-provider` |
| 2.7 | `kfcode-storage` |
| 2.8 | `kfcode-command` |

### Batch 3 — 6 concurrent subagents (Layer 2)

| ID | Crate |
|----|-------|
| 3.1 | `kfcode-tool` |
| 3.2 | `kfcode-agent` |
| 3.3 | `kfcode-session` |
| 3.4 | `kfcode-server` |
| 3.5 | `kfcode-tui` |
| 3.6 | `kfcode-cli` |

## Failure Handling

- A subagent that reports FAIL triggers main-thread inspection of that crate's
  diff. The most likely cause is an unintended fenced code block; remove it.
- If a crate cannot be made to pass within reasonable effort, its doc changes
  are rolled back via `git checkout -- crates/<crate>/` and the crate is marked
  "unprocessed" in the final report.
- Concurrent `cargo check` runs may contend for the `target/` lock. If a
  subagent hits a lock error, it retries.

## Final Verification

After all batches complete:

1. `cargo check --workspace`
2. `cargo doc --workspace --no-deps`
3. `git diff --stat` summary shown to the user.
4. Diff sanity check: every changed line should be a comment (`///`, `//!`,
   `//`) or trivial whitespace next to one. Any non-comment change in a crate
   triggers rollback for that crate.

## Commit

A single commit will be created after the user approves the full diff:

> Add documentation comments across all crates

No push is performed automatically.

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Concurrent `cargo check` lock contention on `target/` | Subagents retry on lock error; each subagent runs its own `check` and `doc` serially within itself. |
| Style drift across subagents | A single binding standard with templates ships in every prompt. |
| A subagent edits non-comment code | Final diff sanity check rolls back any crate with non-comment changes. |
| Doctest failure from accidental fenced code blocks | Standard forbids fenced blocks; final `cargo doc` (not `cargo test`) gates the work. |

## Time and Context Budget

- Batch 1 (~15 files): ~5 minutes.
- Batch 2 (~60 files): ~15 minutes.
- Batch 3 (~130 files): ~30 minutes.
- Total: ~1 hour wall clock; main context absorbs 19 short reports.
