//! Alphabetical sequence generator: `<letter>:[step]`.
//!
//! Example (from the PRD): `a:1` -> `a`, `b`, `c`, `d`.
//!
//! Letters are treated as base-26 digits (A=0..Z=25) so sequences roll over
//! naturally past `z` (`y:1` -> `y`, `z`, `aa`, `ab`, ...), mirroring how
//! spreadsheet column names extend past `Z`.

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
/// 25=z, 26=aa, 27=ab, ...
fn offset_to_letters(mut offset: i64, uppercase: bool) -> String {
    let mut letters = Vec::new();
    loop {
        let remainder = (offset % 26) as u8;
        letters.push(remainder);
        offset = offset / 26 - 1;
        if offset < 0 {
            break;
        }
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
        let raw_offset = self.start + self.step * index as i64;
        // Negative offsets (e.g. decrementing past 'a') wrap using
        // Euclidean modulo so the sequence stays within a-z rather than
        // producing invalid characters.
        let wrapped = raw_offset.rem_euclid(26);
        offset_to_letters(wrapped, self.uppercase)
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
    fn rolls_over_past_z() {
        let gen = AlphaGenerator::new("y", Some("1")).unwrap();
        let values: Vec<String> = (0..4).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["y", "z", "a", "b"]);
    }

    #[test]
    fn rejects_multi_character_start() {
        assert!(AlphaGenerator::new("ab", Some("1")).is_err());
    }
}
