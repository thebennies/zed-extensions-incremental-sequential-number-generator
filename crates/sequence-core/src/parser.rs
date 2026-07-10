//! Top-level dispatch: inspects the raw input string and decides which
//! generator (numeric, alphabetical, date, or word-list) it describes.
//!
//! Detection order (first match wins), matching the PRD's parsing engine
//! description:
//! 1. `date(...)` function syntax
//! 2. Comma-separated word list
//! 3. `[start]:[step]:[format]` numeric/alpha syntax
//!
//! Any failure to parse - unrecognized shape, bad number, invalid calendar
//! date, etc. - surfaces as [`SequenceError`]. Per the PRD's edge-case
//! handling, callers should use [`generate_sequence`] rather than
//! [`parse`] directly: it never fails, falling back to a basic `1, 2, 3, ...`
//! sequence on any error so the extension never aborts or crashes.

use crate::alpha::AlphaGenerator;
use crate::date::DateGenerator;
use crate::error::SequenceError;
use crate::generator::{generate_n, Generator};
use crate::numeric::{looks_numeric, NumericGenerator};
use crate::wordlist::WordListGenerator;

/// Parses `input` into a boxed [`Generator`]. See module docs for detection
/// rules. Returns [`SequenceError`] on any unrecognized or malformed input.
pub fn parse(input: &str) -> Result<Box<dyn Generator>, SequenceError> {
    let input = input.trim();

    if let Some(rest) = input.strip_prefix("date(") {
        return parse_date(rest);
    }

    if input.contains(',') {
        return Ok(Box::new(WordListGenerator::new(input)?));
    }

    parse_colon_syntax(input)
}

fn parse_date(rest: &str) -> Result<Box<dyn Generator>, SequenceError> {
    let close = rest
        .find(')')
        .ok_or_else(|| SequenceError::InvalidSyntax("unterminated date(...)".to_string()))?;
    let start_date = &rest[..close];
    let after = &rest[close + 1..];
    let step_spec = after.strip_prefix(':').unwrap_or(after).trim();
    Ok(Box::new(DateGenerator::new(start_date, step_spec)?))
}

fn parse_colon_syntax(input: &str) -> Result<Box<dyn Generator>, SequenceError> {
    if input.is_empty() {
        return Ok(Box::new(NumericGenerator::new(None, None, None)?));
    }

    let mut parts = input.splitn(3, ':');
    let first = parts.next().unwrap_or("");
    let second = parts.next();
    let third = parts.next();

    if is_single_letter(first) {
        return Ok(Box::new(AlphaGenerator::new(first, second)?));
    }

    if looks_numeric(first) && second.map_or(true, looks_numeric) {
        return Ok(Box::new(NumericGenerator::new(
            Some(first).filter(|s| !s.is_empty()),
            second,
            third,
        )?));
    }

    Err(SequenceError::InvalidSyntax(format!("unrecognized sequence syntax: {input}")))
}

fn is_single_letter(token: &str) -> bool {
    let mut chars = token.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic()) && chars.next().is_none()
}

/// The default fallback sequence used whenever [`parse`] fails: plain
/// integers starting at 1, incrementing by 1.
fn default_generator() -> NumericGenerator {
    NumericGenerator::new(None, None, None).expect("default numeric sequence is always valid")
}

/// Generates `count` values from `input`. Never fails: on any parse error,
/// silently falls back to the default `1, 2, 3, ...` sequence rather than
/// aborting the caller (per the PRD's "Invalid Syntax" edge case).
pub fn generate_sequence(input: &str, count: usize) -> Vec<String> {
    match parse(input) {
        Ok(generator) => generate_n(generator.as_ref(), count),
        Err(_) => generate_n(&default_generator(), count),
    }
}

/// Same as [`generate_sequence`] but returns the parse error instead of
/// silently falling back, for callers (e.g. tests, or a "strict" UI mode)
/// that want to surface the failure to the user.
pub fn try_generate_sequence(input: &str, count: usize) -> Result<Vec<String>, SequenceError> {
    let generator = parse(input)?;
    Ok(generate_n(generator.as_ref(), count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_number_defaults() {
        assert_eq!(generate_sequence("", 4), vec!["1", "2", "3", "4"]);
    }

    #[test]
    fn numeric_with_step_and_format() {
        assert_eq!(
            generate_sequence("1:2:%02d", 4),
            vec!["01", "03", "05", "07"]
        );
    }

    #[test]
    fn numeric_negative_step() {
        assert_eq!(generate_sequence("10:-1", 4), vec!["10", "9", "8", "7"]);
    }

    #[test]
    fn alpha_sequence() {
        assert_eq!(generate_sequence("a:1", 4), vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn date_day_sequence() {
        assert_eq!(
            generate_sequence("date(2026-07-07):1d", 3),
            vec!["2026-07-07", "2026-07-08", "2026-07-09"]
        );
    }

    #[test]
    fn date_month_sequence() {
        assert_eq!(
            generate_sequence("date(2026-01):1m", 3),
            vec!["2026-01", "2026-02", "2026-03"]
        );
    }

    #[test]
    fn word_list_sequence() {
        assert_eq!(
            generate_sequence("apple, banana, cherry", 3),
            vec!["apple", "banana", "cherry"]
        );
    }

    #[test]
    fn word_list_wraps_around() {
        assert_eq!(
            generate_sequence("apple, banana, cherry", 5),
            vec!["apple", "banana", "cherry", "apple", "banana"]
        );
    }

    #[test]
    fn invalid_syntax_falls_back_to_default_numeric() {
        assert_eq!(generate_sequence("date(not-a-date)garbage::", 4), vec!["1", "2", "3", "4"]);
        assert_eq!(generate_sequence("$$$not valid$$$", 3), vec!["1", "2", "3"]);
    }

    #[test]
    fn try_generate_sequence_surfaces_errors() {
        assert!(try_generate_sequence("$$$not valid$$$", 3).is_err());
        assert!(try_generate_sequence("1:2:%02d", 3).is_ok());
    }

    #[test]
    fn large_cursor_count_generates_quickly() {
        let values = generate_sequence("0:1", 10_000);
        assert_eq!(values.len(), 10_000);
        assert_eq!(values[9_999], "9999");
    }
}
