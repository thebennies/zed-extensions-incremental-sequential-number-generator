//! Zed extension entry point for the "Incremental & Sequential Number
//! Generator".
//!
//! ## A note on scope vs. the original PRD
//!
//! The PRD's technical architecture section sketches a `sequence::Insert`
//! *editor command* that reads the active `Editor`'s selections and performs
//! a batched `editor.edit()` across all cursors. Zed's real
//! `zed_extension_api` (WASM-sandboxed, see
//! <https://zed.dev/docs/extensions>) does not currently expose that kind of
//! arbitrary buffer/multi-cursor mutation API to extensions - extensions can
//! register language servers, slash commands, context servers, indexed-docs
//! providers, and similar, but cannot directly manipulate editor selections
//! or perform buffer edits from Wasm.
//!
//! The closest real integration point available to a Wasm extension is a
//! **slash command**, so this extension still registers `/sequence <spec>
//! <count>` (see `extension.toml`) for completeness and for any future/
//! alternate Zed surface that does call `run_slash_command`. **However:**
//! as of the current "Agent Panel" redesign, Zed's primary AI chat UI does
//! **not** invoke extension-provided slash commands at all - its `/`
//! commands are sourced from a separate built-in/skill system. Confirmed
//! directly against Zed's docs, whose "Developing Extensions" page lists
//! current extension capabilities as Languages, Themes, Debuggers,
//! Snippets, and MCP servers - slash commands are absent. So this code
//! path is currently **not reachable from the Agent Panel**; it is kept
//! only because the WIT interface still technically exists and this may
//! change again.
//!
//! The supported, working way to use this engine today is the standalone
//! CLI in `crates/sequence-cli` (binary name `sequence`), invoked directly
//! or via the Zed Task in `.zed/tasks.json`. See the README's "Usage"
//! section. All the parsing/generation logic (numeric, alpha, date,
//! word-list, format strings, wrap-around, graceful fallback on invalid
//! input) lives in `sequence-core` and is shared by both entry points
//! unchanged - only the *last mile* differs.

use zed_extension_api::{
    self as zed, Extension, SlashCommand, SlashCommandOutput, SlashCommandOutputSection, Worktree,
};

/// Name of the single slash command this extension registers, must match
/// `extension.toml`'s `[slash_commands.sequence]` table key.
const COMMAND_NAME: &str = "sequence";

struct SequenceExtension;

impl Extension for SequenceExtension {
    fn new() -> Self {
        SequenceExtension
    }

    fn run_slash_command(
        &self,
        command: SlashCommand,
        args: Vec<String>,
        _worktree: Option<&Worktree>,
    ) -> Result<SlashCommandOutput, String> {
        if command.name != COMMAND_NAME {
            return Err(format!("unknown slash command: {}", command.name));
        }

        let (spec, count) = sequence_core::parse_spec_and_count(&args)?;
        let values = sequence_core::generate_sequence(&spec, count);
        let text = values.join("\n");
        let section_label = format!("sequence({spec}) x{count}");
        let range = 0..text.len();

        Ok(SlashCommandOutput {
            text,
            sections: vec![SlashCommandOutputSection {
                range: range.into(),
                label: section_label,
            }],
        })
    }
}

zed::register_extension!(SequenceExtension);

// Argument-splitting behavior ("last token is the count, rejoin the rest as
// the spec") lives in `sequence_core::parse_spec_and_count` and is tested
// there - the CLI binary (`crates/sequence-cli`) shares the exact same
// function so both entry points behave identically. Nothing left to test
// here beyond that delegation.
