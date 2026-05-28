# Source Code Comments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add accurate English doc comments to every Rust source file across all 19 workspace crates without changing any code.

**Architecture:** Dispatch one subagent per crate, batched in three layers by dependency order. Each subagent uses a single shared prompt template so the comment style stays consistent. Verification is `cargo check -p <crate>` + `cargo doc -p <crate> --no-deps` per crate, then a workspace-wide pass at the end.

**Tech Stack:** Rust 2021 workspace (cargo), Rustdoc, git.

**Spec:** `docs/superpowers/specs/2026-05-29-source-comments-design.md`

---

## File Counts (.rs in src/, excluding tests/)

| Layer | Crate | Files |
|------|-------|-------|
| 0 | kfcode-core | 3 |
| 0 | kfcode-types | 4 |
| 0 | kfcode-plugin | 7 |
| 0 | kfcode-lsp | 1 |
| 0 | kfcode-watcher | 1 |
| 1 | kfcode-config | 4 |
| 1 | kfcode-util | 4 |
| 1 | kfcode-grep | 2 |
| 1 | kfcode-mcp | 7 |
| 1 | kfcode-permission | 4 |
| 1 | kfcode-provider | 33 |
| 1 | kfcode-storage | 4 |
| 1 | kfcode-command | 1 |
| 2 | kfcode-tool | 27 |
| 2 | kfcode-agent | 4 |
| 2 | kfcode-session | 15 |
| 2 | kfcode-server | 8 |
| 2 | kfcode-tui | 75 |
| 2 | kfcode-cli | 1 |
| | **Total** | **205** |

---

## Subagent Prompt Template

This exact template is used for every subagent dispatch. Only `<CRATE_NAME>` changes per call.

````
You are documenting one crate of a Rust workspace at /Users/dfbb/Sites/kfcode/kfcode.

## Your crate
crates/<CRATE_NAME>/

## Task
Add English doc comments to every .rs file under crates/<CRATE_NAME>/src/.
Do NOT change any code. Only insert comment lines.

## Comment standard (binding)

### Required
1. Each .rs file gets a top-of-file `//!` module summary (1–3 lines).
2. Every `pub` item gets a `///` doc comment:
   - pub struct / pub enum / pub trait — what the type represents and typical usage.
   - pub fn / pub async fn — verb-led, what it does. Use `# Errors`, `# Panics`, or `# Note` sections only when behavior is non-obvious.
   - pub mod — one line.
   - pub const / pub static — meaning and units (if any).
3. Non-obvious private logic gets `//` line comments that explain WHY, not what. Examples: `// SAFETY:`, `// Workaround for ...`, `// Invariant: ...`.

### Forbidden
- No code changes. Identifiers, signatures, visibility, imports, formatting, blank lines all stay exactly as-is.
- No fenced code blocks ``` inside doc comments (would create doctests).
- No emoji.
- No Chinese; English only.
- No comments that just restate the type signature.
- No references to PR numbers, issue numbers, or other file paths.
- No TODO or FIXME.
- Existing /// or //! content stays unless it directly contradicts what you're adding or is obviously wrong.

### Templates

Module header (first lines):
//! <One sentence describing this module's responsibility>.
//!
//! <Optional second paragraph.>

Struct:
/// <What this type represents>.
pub struct Foo { ... }

Function:
/// <Verb-led: what it does>.
/// # Errors
/// <When it returns Err.>
pub fn parse(input: &str) -> Result<Foo, Error> { ... }

Trait:
/// <What the abstraction represents>.
/// Implementors must <core contract>.
pub trait Provider { ... }

Enum: each pub variant gets a single-line /// directly above it.

### Boundaries
- Skip tests/ directories and #[cfg(test)] modules.
- Skip generated code.
- Re-exports (pub use foo::*): doc optional.

## Workflow
1. Read crates/<CRATE_NAME>/Cargo.toml to learn the crate's purpose.
2. List every .rs file under crates/<CRATE_NAME>/src/.
3. Add comments file-by-file using the Edit tool. NEVER use Write.
4. From /Users/dfbb/Sites/kfcode/kfcode run:
   - cargo check -p <CRATE_NAME>
   - cargo doc -p <CRATE_NAME> --no-deps
5. If cargo check fails because of a comment you added (rare; usually a fenced code block triggering doctest), remove the offending comment and rerun.
6. If cargo doc reports a broken intra-doc link, replace the broken link with plain text. Do not add imports or change types to make a link resolve.

## Required report (≤200 words, this exact structure)

crate: <CRATE_NAME>
files_modified: <count>
files_skipped: <list with reason>
cargo_check: PASS | FAIL <error summary>
cargo_doc: PASS <warning count> | FAIL <error summary>
notes: <optional>

## Hard rules
- Use only Edit (not Write) for source files.
- Never edit anything outside crates/<CRATE_NAME>/.
- Never run git commands.
````

---

### Task 1: Capture baseline and confirm clean tree

**Files:**
- Read only

- [ ] **Step 1: Confirm working tree is clean**

Run: `git status --short`
Expected: empty output. If anything is dirty, stop and tell the user.

- [ ] **Step 2: Confirm baseline `cargo check --workspace` passes**

Run: `cargo check --workspace 2>&1 | tail -20`
Expected: `Finished` line, no errors. Save the output for comparison.

- [ ] **Step 3: Confirm baseline `cargo doc --workspace --no-deps` passes**

Run: `cargo doc --workspace --no-deps 2>&1 | tail -30`
Expected: `Finished` or `Documenting` lines, no errors. Note current warning count.

- [ ] **Step 4: Record baselines**

If either baseline already fails before this work begins, stop and report. The plan assumes a green baseline; new failures must be attributable to comment additions.

---

### Task 2: Batch 1 — dispatch 5 Layer-0 subagents in parallel

**Files:**
- Create: `crates/kfcode-core/src/**/*.rs` doc comments
- Create: `crates/kfcode-types/src/**/*.rs` doc comments
- Create: `crates/kfcode-plugin/src/**/*.rs` doc comments
- Create: `crates/kfcode-lsp/src/**/*.rs` doc comments
- Create: `crates/kfcode-watcher/src/**/*.rs` doc comments

- [ ] **Step 1: Dispatch all five subagents in a single message**

Send one message containing five `Agent` tool calls in parallel. Each uses `subagent_type: "general-purpose"` and a `description` like `Document <crate>`. Each `prompt` field is the **Subagent Prompt Template** above with `<CRATE_NAME>` replaced.

The five crates: `kfcode-core`, `kfcode-types`, `kfcode-plugin`, `kfcode-lsp`, `kfcode-watcher`.

- [ ] **Step 2: Collect five reports**

Each subagent returns a report in the required format. Verify all five reports were received.

- [ ] **Step 3: Check each report**

For each report, read `cargo_check` and `cargo_doc` lines. Any FAIL: jump to Step 5.

- [ ] **Step 4: Diff sanity check**

Run: `git diff --stat crates/kfcode-core crates/kfcode-types crates/kfcode-plugin crates/kfcode-lsp crates/kfcode-watcher`
Then for each modified file:

Run: `git diff -U0 -- <file> | grep -E '^[+-]' | grep -vE '^(\+\+\+|---|\+///|\+//!|\+//|\+ *$)' | head`
Expected: empty output for every file. Any non-comment additions or any deletions in a crate trigger rollback for that crate (`git checkout -- crates/<crate>/`) and re-run that crate's subagent.

- [ ] **Step 5: Resolve any FAIL reports**

For each failing crate:
- Read the subagent's error summary.
- If it's a fenced code block triggering doctest: find and remove the fence.
- If it's a broken intra-doc link: replace with plain text.
- Re-run `cargo check -p <crate>` and `cargo doc -p <crate> --no-deps`.
- If still failing after one attempt: roll back that crate (`git checkout -- crates/<crate>/`) and note it as unprocessed.

- [ ] **Step 6: Commit Batch 1**

```bash
git add crates/kfcode-core crates/kfcode-types crates/kfcode-plugin crates/kfcode-lsp crates/kfcode-watcher
git commit -m "Add doc comments to layer-0 crates"
```

---

### Task 3: Batch 2 — dispatch 8 Layer-1 subagents in parallel

**Files:**
- Create: doc comments under each of the 8 Layer-1 crates' `src/`

- [ ] **Step 1: Dispatch all eight subagents in a single message**

Send one message containing eight `Agent` tool calls in parallel. The eight crates: `kfcode-config`, `kfcode-util`, `kfcode-grep`, `kfcode-mcp`, `kfcode-permission`, `kfcode-provider`, `kfcode-storage`, `kfcode-command`.

Each prompt is the **Subagent Prompt Template** with `<CRATE_NAME>` replaced. `subagent_type: "general-purpose"` for each.

- [ ] **Step 2: Collect eight reports**

Verify all eight reports received.

- [ ] **Step 3: Check each report**

For each: any FAIL → Step 5.

- [ ] **Step 4: Diff sanity check**

Run: `git diff --stat crates/kfcode-config crates/kfcode-util crates/kfcode-grep crates/kfcode-mcp crates/kfcode-permission crates/kfcode-provider crates/kfcode-storage crates/kfcode-command`

For each modified file:
Run: `git diff -U0 -- <file> | grep -E '^[+-]' | grep -vE '^(\+\+\+|---|\+///|\+//!|\+//|\+ *$)' | head`
Expected: empty. Non-comment changes → rollback that crate.

- [ ] **Step 5: Resolve any FAIL reports**

Same procedure as Task 2 Step 5.

- [ ] **Step 6: Commit Batch 2**

```bash
git add crates/kfcode-config crates/kfcode-util crates/kfcode-grep crates/kfcode-mcp crates/kfcode-permission crates/kfcode-provider crates/kfcode-storage crates/kfcode-command
git commit -m "Add doc comments to layer-1 crates"
```

---

### Task 4: Batch 3 — dispatch 6 Layer-2 subagents in parallel

**Files:**
- Create: doc comments under each of the 6 Layer-2 crates' `src/`

- [ ] **Step 1: Dispatch all six subagents in a single message**

Send one message containing six `Agent` tool calls in parallel. The six crates: `kfcode-tool`, `kfcode-agent`, `kfcode-session`, `kfcode-server`, `kfcode-tui`, `kfcode-cli`.

Note: `kfcode-tui` has 75 files; expect this subagent to take the longest. Do not split it — keeping it in one subagent preserves style consistency across the TUI.

- [ ] **Step 2: Collect six reports**

Verify all six reports received.

- [ ] **Step 3: Check each report**

Any FAIL → Step 5.

- [ ] **Step 4: Diff sanity check**

Run: `git diff --stat crates/kfcode-tool crates/kfcode-agent crates/kfcode-session crates/kfcode-server crates/kfcode-tui crates/kfcode-cli`

For each modified file:
Run: `git diff -U0 -- <file> | grep -E '^[+-]' | grep -vE '^(\+\+\+|---|\+///|\+//!|\+//|\+ *$)' | head`
Expected: empty. Non-comment changes → rollback that crate.

- [ ] **Step 5: Resolve any FAIL reports**

Same procedure as Task 2 Step 5.

- [ ] **Step 6: Commit Batch 3**

```bash
git add crates/kfcode-tool crates/kfcode-agent crates/kfcode-session crates/kfcode-server crates/kfcode-tui crates/kfcode-cli
git commit -m "Add doc comments to layer-2 crates"
```

---

### Task 5: Workspace-wide verification

**Files:**
- Read only

- [ ] **Step 1: Run `cargo check --workspace`**

Run: `cargo check --workspace 2>&1 | tail -20`
Expected: `Finished` line, no new errors vs Task 1 baseline.

- [ ] **Step 2: Run `cargo doc --workspace --no-deps`**

Run: `cargo doc --workspace --no-deps 2>&1 | tail -50`
Expected: `Finished`. Warning count may rise from the Task 1 baseline (new doc comments mean new opportunities for warnings); no errors.

- [ ] **Step 3: Show summary**

Run: `git log --oneline -5` to show the three batch commits, then `git diff HEAD~3 --stat` for an aggregate view.

---

### Task 6: Final user review and aggregate commit

**Files:**
- Read only (final commit creates no new files)

- [ ] **Step 1: Show the user the aggregate diff stat**

Run: `git diff HEAD~3 --stat`

- [ ] **Step 2: Ask the user to confirm**

Wait for the user to acknowledge the diff before proceeding. Do not push.

- [ ] **Step 3: (Optional) squash if user requests**

If the user prefers a single commit:
```bash
git reset --soft HEAD~3
git commit -m "Add documentation comments across all crates"
```
Otherwise leave the three batch commits in place.

- [ ] **Step 4: Stop**

Do not push. The user pushes manually when ready.

---

## Self-Review

**Spec coverage:**
- Goal (English doc comments to all Rust crates, no code changes) → Tasks 2, 3, 4.
- Comment standard → embedded in subagent prompt template, used by Tasks 2, 3, 4.
- Out-of-scope (TS files, tests, generated code) → noted in subagent prompt under "Boundaries".
- Three-batch dependency-layered execution → Tasks 2, 3, 4.
- Per-crate `cargo check` + `cargo doc` → in subagent workflow.
- Workspace-wide final verification → Task 5.
- Diff sanity check → Tasks 2/3/4 Step 4.
- Failure handling (rollback per crate) → Tasks 2/3/4 Step 5.
- Single commit at the end OR per batch → Task 6 Step 3 (optional squash).
- No automatic push → Task 6 Step 4.

**Placeholder scan:** No TBD/TODO. All commands are concrete. Subagent prompt is fully specified.

**Type consistency:** No types or signatures defined; this plan operates entirely on comment text.

**Scope:** Single-purpose plan (add comments). No mixed concerns.
