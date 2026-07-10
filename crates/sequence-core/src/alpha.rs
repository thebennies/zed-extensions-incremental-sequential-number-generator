//! Alphabetical sequence generator: `<letter>:[step]`.
//!
//! Example (from the PRD): `a:1` -> `a`, `b`, `c`, `d`.
//!
//! Letters are treated as base-26 digits (a=0..z=25). Incrementing past `z`
//! rolls over into a second letter using the same "bijective base-26"
//! scheme as spreadsheet column names: `y:1` -> `y`, `z`, `aa`, `ab`, ...
//! Decrementing past `a` has no natural bijective-base-26 negative form, so
//! it instead wraps modulo 26 back into a single letter: `a:-1` -> `a`,
//! `z`, `y`, `x`, ...

use crate::error::SequenceError;
use crate::generator::Generator;

#[derive(Debug, Clone, Copy)]
pub struct AlphaGenerator {
    /// 0-based offset of the start letter (a=0, b=1, ...).
    start: i64,
    step: i64,
    uppercase: bool,
}

impl AlphaGenerator {
    /// `letter` must be a single ASCII alphabetic character; `step` is an
    /// optional signed integer string (defaults to `1`).
    pub fn new(letter: &str, step: Option<&str>) -> Result<Self, SequenceError> {
        let mut chars = letter.chars();
        let ch = chars
            .next()
            .filter(|c| c.is_ascii_alphabetic())
            .ok_or_else(|| SequenceError::InvalidSyntax(format!("not a letter: {letter}")))?;
        if chars.next().is_some() {
            return Err(SequenceError::InvalidSyntax(format!(
                "alpha sequence start must be a single letter: {letter}"
            )));
        }
        let uppercase = ch.is_ascii_uppercase();
        let start = (ch.to_ascii_lowercase() as u8 - b'a') as i64;
        let step = match step {
            Some(s) if !s.is_empty() => s
                .parse::<i64>()
                .map_err(|_| SequenceError::InvalidSyntax(format!("invalid step: {s}")))?,
            _ => 1,
        };
        Ok(Self { start, step, uppercase })
    }
}

/// Converts a non-negative base-26 offset into a letter string, using the
/// same "bijective base-26" scheme as spreadsheet column names: 0=a,
/// 25=z, 26=aa, 27=ab, ... Unlike plain base-26, there is no digit that
/// means "zero" in a non-leading position, which is what lets `z` roll over
/// into `aa` instead of `a0`.
fn offset_to_letters(offset: u64, uppercase: bool) -> String {
    // 1-based bijective numeration: repeatedly take ((n - 1) mod 26) as the
    // next (least-significant) letter, then move to the next "digit" via
    // (n - 1) / 26. This is the standard Excel-column-name algorithm.
    let mut n = offset + 1;
    let mut letters = Vec::new();
    while n > 0 {
        let remainder = ((n - 1) % 26) as u8;
        letters.push(remainder);
        n = (n - 1) / 26;
    }
    letters.reverse();
    letters
        .into_iter()
        .map(|r| {
            let base = if uppercase { b'A' } else { b'a' };
            (base + r) as char
        })
        .collect()
}

impl Generator for AlphaGenerator {
    fn value_at(&self, index: usize) -> String {
        // Saturate instead of panicking on overflow for pathological
        // start/step/index combinations (e.g. huge cursor counts with a
        // huge step) - matches the PRD's "never crash the host" edge case.
        let raw_offset = self
            .start
            .saturating_add(self.step.saturating_mul(index as i64));
        let offset = if raw_offset >= 0 {
            // Positive offsets are used as-is so incrementing past 'z'
            // rolls over into 'aa', 'ab', ... (see module docs).
            raw_offset as u64
        } else {
            // Negative offsets (decrementing past 'a') have no natural
            // bijective-base-26 representation, so wrap modulo 26 into a
            // single trailing letter instead.
            raw_offset.rem_euclid(26) as u64
        };
        offset_to_letters(offset, self.uppercase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_lowercase_sequence() {
        let gen = AlphaGenerator::new("a", Some("1")).unwrap();
        let values: Vec<String> = (0..4).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn uppercase_is_preserved() {
        let gen = AlphaGenerator::new("A", Some("1")).unwrap();
        let values: Vec<String> = (0..3).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["A", "B", "C"]);
    }

    #[test]
    fn default_step_is_one() {
        let gen = AlphaGenerator::new("x", None).unwrap();
        let values: Vec<String> = (0..3).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["x", "y", "z"]);
    }

    #[test]
    fn rolls_over_past_z_into_double_letters() {
        let gen = AlphaGenerator::new("y", Some("1")).unwrap();
        let values: Vec<String> = (0..4).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["y", "z", "aa", "ab"]);
    }

    #[test]
    fn decrementing_past_a_wraps_to_single_letter() {
        let gen = AlphaGenerator::new("a", Some("-1")).unwrap();
        let values: Vec<String> = (0..4).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["a", "z", "y", "x"]);
    }

    #[test]
    fn huge_step_and_index_never_panics() {
        let gen = AlphaGenerator::new("a", Some("9223372036854775807")).unwrap();
        // Must not overflow-panic; exact letters don't matter, just that it returns.
        let _ = gen.value_at(usize::MAX / 2);
    }

    #[test]
    fn rejects_multi_character_start() {
        assert!(AlphaGenerator::new("ab", Some("1")).is_err());
    }
}
