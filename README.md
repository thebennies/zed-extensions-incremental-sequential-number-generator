# ⚠️ ARCHIVED — Incremental & Sequential Number Generator

This repository is **archived** and no longer actively maintained.

## Why it is archived

The project was designed as a [Zed](https://zed.dev) editor extension that would read the active editor's multi-line cursor positions and insert one generated value per cursor. Zed's extension API does not expose multi-line cursor access or multi-cursor buffer edits to third-party extensions — the `Extension` trait in `zed_extension_api` 0.7.0 has no such methods, and extensions run inside a WebAssembly sandbox ([docs.rs: `Extension` trait, v0.7.0](https://docs.rs/zed_extension_api/0.7.0/zed_extension_api/trait.Extension.html)). Because of that, the project fell back to a `/sequence` **slash-command workaround**. The slash command works technically, but it is not an intuitive workflow for this use case, and — as of API v0.7.0 — it has no reachable UI surface in Zed either: the official "Developing Extensions" docs no longer list slash commands as a supported extension capability ([Zed Docs: Developing Extensions](https://zed.dev/docs/extensions/developing-extensions)), and user reports confirm extension-provided slash commands are "no longer usable from any part of the zed ui" after text threads were removed ([zed-industries/zed#53760](https://github.com/zed-industries/zed/issues/53760)). With neither a proper multi-line cursor API nor a usable workaround, the Zed-extension part of this project cannot function as originally intended.

---

*Everything below this line is the original README, kept for historical reference. It describes the working CLI/Task workflow and the (now unreachable) slash-command extension.*

---

# Incremental & Sequential Number Generator

**A companion for multiple cursors in the [Zed](https://zed.dev) editor.**
Generate sequential numbers, incremental counters, date ranges, alphabetical
lists, and custom word sequences on demand — one value per line, ready to
paste across your multiple cursors/selections.

**Use it via the `sequence` CLI + the Zed Task in `.zed/tasks.json`** (see
[Usage](#usage) below) — that is the supported, working integration today.
The repo also ships a Zed extension registering a `/sequence` slash command
for completeness, but **it currently has no working UI surface**: Zed's
Agent Panel does not invoke extension-provided slash commands (see
["Important note on scope"](#important-note-on-scope-how-this-maps-to-zeds-real-extension-api)
for the full explanation). Don't install the dev extension expecting
`/sequence` to appear in the Agent Panel chat — use the CLI/Task instead.

## Features

Trigger the sequence generator with a single, compact input string and it
expands into one value per cursor/selection:

| Kind          | Syntax                              | Example input            | Example output                          |
|---------------|--------------------------------------|---------------------------|------------------------------------------|
| Numeric       | `[start]:[step]:[format]`            | `1:2:%02d`                | `01`, `03`, `05`, `07`                    |
| Numeric       | `[start]:[step]`                     | `10:-1`                   | `10`, `9`, `8`, `7`                       |
| Alphabetical  | `<letter>:[step]`                    | `a:1`                     | `a`, `b`, `c`, `d`                        |
| Date          | `date([start_date]):[step][unit]`    | `date(2026-07-07):1d`     | `2026-07-07`, `2026-07-08`, `2026-07-09`  |
| Date (month)  | `date([YYYY-MM]):[step]m`            | `date(2026-01):1m`        | `2026-01`, `2026-02`, `2026-03`           |
| Word list     | `word, word, word`                   | `apple, banana, cherry`   | `apple`, `banana`, `cherry` (loops if more cursors than words) |
| Default       | *(empty / invalid input)*            | *(anything unparsable)*   | `1`, `2`, `3`, `4`, ...                   |

Date step units: `d` (days), `w` (weeks), `m` (months, clamped to the target
month's length), `y` (years).

## Usage

### Option A: run the CLI directly in a terminal

```bash
cargo run --quiet -p sequence-cli --bin sequence -- "1:2:%02d" 4
# 01
# 03
# 05
# 07

cargo run --quiet -p sequence-cli --bin sequence -- apple, banana, cherry 5 --copy
# apple
# banana
# cherry
# apple
# banana
# (also best-effort copies the output to your system clipboard)
```

The **last** argument is always the count; everything before it is rejoined
with spaces to form the spec (this is what lets a word list like `apple,
banana, cherry` survive being split into multiple shell arguments). Run
`sequence --help` for the full flag reference.

Once you have the output, select it and copy it (or pass `--copy`), create
your N cursors/selections in Zed (`Cmd/Ctrl+Click`, or `Cmd/Ctrl+D` to select
the next occurrence repeatedly), and paste — Zed distributes one line per
cursor across a multi-line paste onto multiple selections, same as most
editors.

### Option B: run it as a Zed Task (`.zed/tasks.json`)

This repo includes a `.zed/tasks.json` with a ready-made task. Zed's tasks
system doesn't support free-text input prompts (no VS Code-style
`${input:...}`), so the supported workflow is:

1. Open the task modal: command palette → `task: spawn`.
2. Select **"Sequence: generate (edit spec/count before running)"**.
3. Press `tab` to load its command into an editable prompt.
4. Edit the spec/count at the end of the command (`... -- "1:2:%02d" 4`) to
   whatever you need, then press Enter to run it.
5. Output appears in Zed's integrated terminal panel; copy it from there.

A second task, **"Sequence: generate from selected text as spec (count
4)"**, defaults its spec to whatever text you currently have selected in the
buffer (via the `$ZED_SELECTED_TEXT` task variable) — handy for iterating on
a spec you're editing inline before running it.

## Project layout

```
.
├── extension.toml                     # Zed extension manifest (slash command registration)
├── Cargo.toml                         # The extension's own [package] manifest (see below) + [workspace]
├── src/lib.rs                         # `Extension` impl exposing `/sequence <spec> <count>`
├── .zed/tasks.json                    # Zed Task wiring up the CLI (the supported usage path)
└── crates/
    ├── sequence-core/                 # Pure-Rust generation engine, no Zed/Wasm dependency
    │   └── src/
    │       ├── lib.rs                 # Public API: generate_sequence(), parse(), Generator trait
    │       ├── args.rs                # Shared "last arg is the count" convention (CLI + extension)
    │       ├── parser.rs              # Top-level dispatch + fallback-on-error logic
    │       ├── numeric.rs             # `[start]:[step]:[format]` numeric sequences
    │       ├── alpha.rs               # `<letter>:[step]` alphabetical sequences
    │       ├── date.rs                # `date(...):[step][unit]` date sequences (chrono)
    │       ├── wordlist.rs            # Comma-separated word-list sequences with wrap-around
    │       ├── generator.rs           # Shared `Generator` trait + batch generation helper
    │       └── error.rs               # `SequenceError`
    └── sequence-cli/                  # Standalone CLI (binary name `sequence`) - the supported entry point
        └── src/main.rs                # argv parsing, --copy clipboard best-effort, delegates to sequence-core
```

**Why the extension crate lives at the repo root instead of under
`crates/`:** Zed's extension builder reads `<extension_dir>/Cargo.toml`
directly and requires it to be an actual package manifest (a `[package]`
table with `package.name`) — a workspace-only manifest fails to parse there
with `failed to compile Rust extension`. So the root `Cargo.toml` here is
*both* the extension's package manifest *and* a workspace root that pulls in
`crates/sequence-core` as a member via `[workspace] members = [...]`. This
keeps `sequence-core` independently testable (zero I/O, zero Zed dependency)
while satisfying Zed's layout requirement for the crate it actually builds.

## Important note on scope: how this maps to Zed's real extension API

The original spec describes a `sequence::Insert` **editor command** that
reads the active editor's selections and performs a single batched
`editor.edit()` across all cursors, similar to VS Code extensions. Two
layers of Zed's real API make that infeasible today, in increasing order of
how far we got before hitting each one:

1. **No buffer/selection editing API for extensions.** `zed_extension_api`
   (as of `"0.7"`) runs extensions in a sandboxed WebAssembly module
   (compiled to the `wasm32-wasip2` target, per Zed's
   `extension_builder.rs`) that can register language servers, slash
   commands, context servers, indexed-docs providers, debug adapters, and
   similar — but cannot directly read editor selections or mutate buffer
   contents. That capability is only available to Zed's native Rust core.
2. **The fallback we picked - a slash command - doesn't work either,** at
   least not from Zed's current primary chat UI. This extension still
   registers `/sequence <spec> <count>` (`extension.toml` +
   `Extension::run_slash_command` in `src/lib.rs`), and it builds and
   installs fine, but **Zed's Agent Panel never calls it**: its `/`
   commands come from a separate built-in/skill system, not from installed
   extensions. Confirmed against Zed's own "Developing Extensions" docs,
   which list current extension capabilities as Languages, Themes,
   Debuggers, Snippets, and MCP servers — slash commands are absent from
   that list despite the WIT interface still technically existing.

Given both constraints, the **actually-working** integration is the
standalone CLI in `crates/sequence-cli` plus the Zed Task in
`.zed/tasks.json` (see [Usage](#usage) above). The full parsing/generation
engine — numeric/alpha/date/word-list, formatting, wrap-around, graceful
fallback on invalid input, 10,000+ cursor performance — is implemented
exactly as specified in `sequence-core`, and is shared unchanged by the CLI,
the (currently unreachable) slash command, and any future integration: if
Zed ever adds a buffer/selection-editing extension API, or its Agent Panel
starts calling extension slash commands again, only the relevant entry
point file would need to change — `sequence-core`'s public API
(`generate_sequence`) already returns exactly the `Vec<String>` (one item
per cursor) any of those would consume directly.

## Development

```bash
# Run the full workspace test suite (sequence-core + sequence-cli + the
# extension's own delegation, 45 tests total):
cargo test --workspace

# Run just the generation engine's tests:
cargo test -p sequence-core

# Run just the CLI's argument-handling tests:
cargo test -p sequence-cli

# Run the CLI directly without installing anything:
cargo run -p sequence-cli --bin sequence -- "1:2:%02d" 4

# Build the actual Wasm artifact Zed's "Install Dev Extension" flow builds
# (kept working and tested, even though its slash command currently has no
# reachable UI - see "Important note on scope" above):
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2 --target-dir ./target --release
# Expected artifact: target/wasm32-wasip2/release/sequence_extension.wasm
```

Note: the workspace root's `Cargo.toml` sets `default-members = ["."]` (just
the extension package), so a plain `cargo build`/`cargo run` without `-p`
only touches the extension crate — matching exactly what Zed's builder does
and keeping `sequence-cli` out of that build path entirely. Always pass
`-p sequence-cli` (or `--workspace`) when you want to build/run/test the
CLI.

### Installing the dev extension locally in Zed (optional; slash command is currently unreachable)

1. Open Zed → `zed: extensions` → **Install Dev Extension**.
2. Select this repository's root directory (the one containing
   `extension.toml` and `Cargo.toml`).
3. Zed builds the root package to `wasm32-wasip2` and loads it — but per
   "Important note on scope" above, its `/sequence` slash command will not
   appear anywhere in the Agent Panel. Use the CLI/Task from
   [Usage](#usage) instead.

## Edge cases handled

- **Cursor count > sequence length** (word lists): wraps via modulo instead
  of erroring or truncating.
- **Invalid syntax**: never panics or aborts — falls back to a default
  `1, 2, 3, ...` numeric sequence (see `sequence_core::generate_sequence`).
- **Large cursor counts**: generation pre-allocates its output `Vec` and is
  a linear, allocation-light pass per cursor; a 10,000-cursor case is covered
  by tests in both `numeric.rs` and `parser.rs`.
- **End-of-month date arithmetic**: adding months/years clamps the day to
  the target month's actual length (e.g. Jan 31 + 1 month → Feb 28/29,
  never an invalid calendar date).

## License

MIT — see [LICENSE](LICENSE).
