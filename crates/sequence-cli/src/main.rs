//! `sequence` - standalone CLI for the sequence-core engine.
//!
//! This exists because Zed's Agent Panel does not invoke extension-provided
//! slash commands (see `src/lib.rs` at the repo root for the full
//! explanation), so the extension crate alone has no working UI surface in
//! current Zed. This CLI is the supported way to actually use the engine
//! today: run it directly in a terminal, or via the Zed Task defined in
//! `.zed/tasks.json` (which drops output into Zed's integrated terminal for
//! you to copy and paste across multiple cursors/selections).
//!
//! ```text
//! sequence <spec> <count> [--copy]
//! ```
//!
//! `<spec>` may be split across multiple shell-quoted arguments (matters for
//! word lists like `apple, banana, cherry`); the **last** argument is always
//! the count. `--copy` may appear anywhere and best-effort-copies the output
//! to the system clipboard (macOS `pbcopy`, Linux `xclip`/`xsel`/`wl-copy`,
//! Windows `clip`) - if no clipboard tool is found, the output is still
//! printed to stdout and the CLI exits successfully.

use std::env;
use std::io::Write as _;
use std::process::{self, Command, Stdio};

const USAGE: &str = "\
usage: sequence <spec> <count> [--copy]

examples:
  sequence \"1:2:%02d\" 4          -> 01\\n03\\n05\\n07
  sequence \"10:-1\" 4             -> 10\\n9\\n8\\n7
  sequence \"a:1\" 4               -> a\\nb\\nc\\nd
  sequence \"date(2026-07-07):1d\" 3
  sequence apple, banana, cherry 5 --copy

flags:
  --copy   best-effort copy the output to the system clipboard
  -h, --help   show this message";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        process::exit(if args.is_empty() { 1 } else { 0 });
    }

    let (args, want_copy) = extract_copy_flag(args);

    match build_output(&args) {
        Ok(text) => {
            println!("{text}");
            if want_copy {
                if try_copy_to_clipboard(&text) {
                    eprintln!("(copied to clipboard)");
                } else {
                    eprintln!("(--copy requested but no clipboard tool was found; output printed above)");
                }
            }
        }
        Err(err) => {
            eprintln!("error: {err}");
            eprintln!("{USAGE}");
            process::exit(1);
        }
    }
}

/// Removes every `--copy` occurrence from `args`, returning the remaining
/// arguments plus whether `--copy` was present at all.
fn extract_copy_flag(args: Vec<String>) -> (Vec<String>, bool) {
    let mut want_copy = false;
    let remaining: Vec<String> = args
        .into_iter()
        .filter(|a| {
            if a == "--copy" {
                want_copy = true;
                false
            } else {
                true
            }
        })
        .collect();
    (remaining, want_copy)
}

/// Parses `args` and generates the sequence, joined with newlines - one
/// value per line, ready to paste across multiple cursors/selections.
fn build_output(args: &[String]) -> Result<String, String> {
    let (spec, count) = sequence_core::parse_spec_and_count(args)?;
    let values = sequence_core::generate_sequence(&spec, count);
    Ok(values.join("\n"))
}

/// Best-effort clipboard copy. Tries, in order, the clipboard tool most
/// likely to exist for the current platform; returns `false` (never panics
/// or errors loudly) if none are available or the copy otherwise fails,
/// since `--copy` is a convenience, not a correctness requirement.
fn try_copy_to_clipboard(text: &str) -> bool {
    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else if cfg!(target_os = "windows") {
        &[("clip", &[])]
    } else {
        &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ]
    };

    for (program, program_args) in candidates {
        if copy_via(program, program_args, text) {
            return true;
        }
    }
    false
}

fn copy_via(program: &str, program_args: &[&str], text: &str) -> bool {
    let child = Command::new(program)
        .args(program_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    let mut child = match child {
        Ok(child) => child,
        Err(_) => return false, // program not found / not spawnable - try the next candidate.
    };

    let write_ok = child
        .stdin
        .take()
        .map(|mut stdin| stdin.write_all(text.as_bytes()).is_ok())
        .unwrap_or(false);

    matches!(child.wait(), Ok(status) if status.success() && write_ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_copy_flag_removes_all_occurrences_anywhere() {
        let args = vec![
            "--copy".to_string(),
            "1:2:%02d".to_string(),
            "4".to_string(),
            "--copy".to_string(),
        ];
        let (remaining, copy) = extract_copy_flag(args);
        assert!(copy);
        assert_eq!(remaining, vec!["1:2:%02d".to_string(), "4".to_string()]);
    }

    #[test]
    fn extract_copy_flag_false_when_absent() {
        let args = vec!["a:1".to_string(), "3".to_string()];
        let (remaining, copy) = extract_copy_flag(args.clone());
        assert!(!copy);
        assert_eq!(remaining, args);
    }

    #[test]
    fn build_output_generates_newline_joined_values() {
        let args = vec!["1:2:%02d".to_string(), "4".to_string()];
        assert_eq!(build_output(&args).unwrap(), "01\n03\n05\n07");
    }

    #[test]
    fn build_output_rejoins_word_list_across_args() {
        let args = vec![
            "apple,".to_string(),
            "banana,".to_string(),
            "cherry".to_string(),
            "5".to_string(),
        ];
        assert_eq!(
            build_output(&args).unwrap(),
            "apple\nbanana\ncherry\napple\nbanana"
        );
    }

    #[test]
    fn build_output_errors_on_empty_args() {
        assert!(build_output(&[]).is_err());
    }

    #[test]
    fn copy_via_nonexistent_program_returns_false_not_panic() {
        assert!(!copy_via("this-program-should-never-exist-xyz", &[], "text"));
    }
}
