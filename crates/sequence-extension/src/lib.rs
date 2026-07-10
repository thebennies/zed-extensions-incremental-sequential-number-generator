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
//! The closest real integration point is a **slash command**, so this
//! extension registers `/sequence <spec> <count>` (see `extension.toml`),
//! which runs the exact same [`sequence_core::generate_sequence`] engine the
//! PRD specifies and returns the generated values as newline-separated text
//! that Zed inserts. All the parsing/generation logic (numeric, alpha, date,
//! word-list, format strings, wrap-around, graceful fallback on invalid
//! input) is implemented in full per the PRD in the `sequence-core` crate;
//! only the *last mile* (multi-cursor buffer injection) differs from the
//! PRD's sketch, because that capability does not exist in the current
//! extension API. If/when Zed adds a buffer-editing extension API, this
//! crate is the only file that would need to change.

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

        let (spec, count) = parse_args(&args)?;
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

/// Splits the raw slash-command arguments into `(spec, count)`.
///
/// Zed splits slash-command input on whitespace before handing it to the
/// extension, so `/sequence apple, banana, cherry 5` arrives as
/// `["apple,", "banana,", "cherry", "5"]`. The **last** argument is always
/// the cursor count; everything before it is rejoined with spaces to
/// reconstruct the sequence spec (this matters for word lists and any
/// spec containing spaces).
///
/// Falls back to `count = 1` if the last argument isn't a valid number,
/// per the PRD's "gracefully abort / fall back" edge-case guidance -
/// this function itself never errors on a malformed count, only on a
/// completely empty argument list (nothing to generate).
fn parse_args(args: &[String]) -> Result<(String, usize), String> {
    let (last, rest) = args
        .split_last()
        .ok_or_else(|| "usage: /sequence <spec> <count>".to_string())?;

    match last.parse::<usize>() {
        Ok(count) if !rest.is_empty() => Ok((rest.join(" "), count.max(1))),
        // No explicit count was given (only a spec) - default to a single value.
        _ => Ok((args.join(" "), 1)),
    }
}

zed::register_extension!(SequenceExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_spec_and_trailing_count() {
        let args = vec!["1:2:%02d".to_string(), "4".to_string()];
        assert_eq!(parse_args(&args).unwrap(), ("1:2:%02d".to_string(), 4));
    }

    #[test]
    fn rejoins_word_list_split_by_whitespace() {
        let args = vec![
            "apple,".to_string(),
            "banana,".to_string(),
            "cherry".to_string(),
            "5".to_string(),
        ];
        assert_eq!(
            parse_args(&args).unwrap(),
            ("apple, banana, cherry".to_string(), 5)
        );
    }

    #[test]
    fn defaults_to_count_one_without_explicit_count() {
        let args = vec!["a:1".to_string()];
        assert_eq!(parse_args(&args).unwrap(), ("a:1".to_string(), 1));
    }

    #[test]
    fn empty_args_is_an_error() {
        assert!(parse_args(&[]).is_err());
    }
}
