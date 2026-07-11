# Incremental & Sequential Number Generator

**A companion for multiple cursors in the [Zed](https://zed.dev) editor.**
Generate sequential numbers, incremental counters, date ranges, alphabetical
lists, and custom word sequences on demand via the `/sequence` slash command
- one value per line, ready to paste across your multiple cursors/selections.
(See ["Important note on scope"](#important-note-on-scope-how-this-maps-to-zeds-real-extension-api)
below: today this generates a pasteable text block rather than typing directly
into N cursors, because Zed's extension API doesn't expose buffer editing to
extensions yet.)

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

## Project layout

```
.
├── extension.toml                     # Zed extension manifest (slash command registration)
├── Cargo.toml                         # The extension's own [package] manifest (see below) + [workspace]
├── src/lib.rs                         # `Extension` impl exposing `/sequence <spec> <count>`
└── crates/
    └── sequence-core/                 # Pure-Rust generation engine, no Zed/Wasm dependency
        └── src/
            ├── lib.rs                 # Public API: generate_sequence(), parse(), Generator trait
            ├── parser.rs              # Top-level dispatch + fallback-on-error logic
            ├── numeric.rs             # `[start]:[step]:[format]` numeric sequences
            ├── alpha.rs               # `<letter>:[step]` alphabetical sequences
            ├── date.rs                # `date(...):[step][unit]` date sequences (chrono)
            ├── wordlist.rs            # Comma-separated word-list sequences with wrap-around
            ├── generator.rs           # Shared `Generator` trait + batch generation helper
            └── error.rs               # `SequenceError`
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
`editor.edit()` across all cursors, similar to VS Code extensions.

**Zed's actual `zed_extension_api` (as of `zed_extension_api = "0.7"`) does
not expose that capability.** Extensions run in a sandboxed WebAssembly
module (compiled to the `wasm32-wasip2` target, per Zed's
`extension_builder.rs`) and can register language servers, slash commands,
context servers, indexed-docs providers, debug adapters, and similar — but
they cannot directly read editor selections or mutate buffer contents. That
kind of direct buffer/multi-cursor editing is only available to Zed's native
Rust core, not to Wasm extensions.

Given that constraint, this extension implements the **entire parsing and
generation engine exactly as specified** (numeric/alpha/date/word-list,
formatting, wrap-around, graceful fallback on invalid input, 10,000+ cursor
performance) in `sequence-core`, and exposes it through the closest real
integration point available today: a slash command,
**`/sequence <spec> <count>`**, which returns the generated values as
newline-separated text for Zed to insert. For example:

```
/sequence 1:2:%02d 4
```

produces:

```
01
03
05
07
```

and

```
/sequence apple, banana, cherry 5
```

produces:

```
apple
banana
cherry
apple
banana
```

If Zed's extension API later adds buffer/selection editing capabilities,
`src/lib.rs` is the only file that would need to change — `sequence-core`'s
public API (`generate_sequence`) already returns exactly the `Vec<String>`
(one item per cursor) that a future `editor.edit()`-based implementation
would consume directly.

## Development

```bash
# Run the full, fast native test suite for the generation engine:
cargo test -p sequence-core

# Run the extension wrapper's argument-parsing tests:
cargo test -p sequence-extension

# Build the actual Wasm artifact Zed loads - this mirrors exactly what
# Zed's "Install Dev Extension" flow runs under the hood:
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2 --target-dir ./target --release
# Expected artifact: target/wasm32-wasip2/release/sequence_extension.wasm
```

### Installing locally in Zed

1. Open Zed → `zed: extensions` → **Install Dev Extension**.
2. Select this repository's root directory (the one containing
   `extension.toml` and `Cargo.toml`).
3. Zed builds the root package to `wasm32-wasip2` and loads it; the
   `/sequence` slash command becomes available in the Assistant panel.

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
