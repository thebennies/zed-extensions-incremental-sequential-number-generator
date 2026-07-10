//! Numeric sequence generator: `[start]:[step]:[format]`.
//!
//! Examples (from the PRD):
//! - `1:2:%02d`  -> `01`, `03`, `05`, `07`
//! - `10:-1`     -> `10`, `9`, `8`, `7`
//! - (empty)     -> `1`, `2`, `3`, `4`  (defaults)

use crate::error::SequenceError;
use crate::generator::Generator;

/// A parsed `%0Nd`-style zero-padding format directive.
///
/// Only zero-padded integer widths are supported (the only case the PRD's
/// examples exercise). Any other `%...` directive is a syntax error so the
/// caller can fall back to the default sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PadFormat {
    width: usize,
}

impl PadFormat {
    fn parse(raw: &str) -> Result<Self, SequenceError> {
        let body = raw
            .strip_prefix('%')
            .ok_or_else(|| SequenceError::InvalidSyntax(format!("format must start with '%': {raw}")))?;
        let digits = body
            .strip_suffix('d')
            .ok_or_else(|| SequenceError::InvalidSyntax(format!("format must end with 'd': {raw}")))?;
        // Accept both "%03d" (leading zero flag) and "%3d" (bare width) -
        // both mean "zero-pad to this width" for our purposes.
        let digits = digits.strip_prefix('0').unwrap_or(digits);
        if digits.is_empty() {
            // "%d" / "%0d" - no padding requested.
            return Ok(PadFormat { width: 0 });
        }
        let width: usize = digits
            .parse()
            .map_err(|_| SequenceError::InvalidSyntax(format!("invalid format width: {raw}")))?;
        Ok(PadFormat { width })
    }

    fn apply(&self, value: i64) -> String {
        if self.width == 0 {
            return value.to_string();
        }
        let negative = value < 0;
        let magnitude = value.unsigned_abs();
        let digits = format!("{magnitude:0width$}", width = self.width);
        if negative {
            format!("-{digits}")
        } else {
            digits
        }
    }
}

/// Either an integer or floating-point numeric sequence, decided by whether
/// `start`/`step` contain a decimal point.
#[derive(Debug, Clone, Copy)]
enum Number {
    Int(i64),
    Float(f64),
}

impl Number {
    fn parse(raw: &str) -> Result<Self, SequenceError> {
        if raw.contains('.') {
            raw.parse::<f64>()
                .map(Number::Float)
                .map_err(|_| SequenceError::InvalidSyntax(format!("invalid number: {raw}")))
        } else {
            raw.parse::<i64>()
                .map(Number::Int)
                .map_err(|_| SequenceError::InvalidSyntax(format!("invalid number: {raw}")))
        }
    }
}

/// Generator for `[start]:[step]:[format]` numeric sequences.
#[derive(Debug, Clone)]
pub struct NumericGenerator {
    start: Number,
    step: Number,
    format: Option<PadFormat>,
}

impl NumericGenerator {
    pub fn new(start: Option<&str>, step: Option<&str>, format: Option<&str>) -> Result<Self, SequenceError> {
        let start = match start {
            Some(s) if !s.is_empty() => Number::parse(s)?,
            _ => Number::Int(1),
        };
        let step = match step {
            Some(s) if !s.is_empty() => Number::parse(s)?,
            _ => Number::Int(1),
        };
        let format = match format {
            Some(f) if !f.is_empty() => Some(PadFormat::parse(f)?),
            _ => None,
        };
        Ok(Self { start, step, format })
    }
}

impl Generator for NumericGenerator {
    fn value_at(&self, index: usize) -> String {
        let index = index as i64;
        match (self.start, self.step) {
            (Number::Int(start), Number::Int(step)) => {
                let value = start.saturating_add(step.saturating_mul(index));
                match self.format {
                    Some(fmt) => fmt.apply(value),
                    None => value.to_string(),
                }
            }
            _ => {
                let start = match self.start {
                    Number::Int(v) => v as f64,
                    Number::Float(v) => v,
                };
                let step = match self.step {
                    Number::Int(v) => v as f64,
                    Number::Float(v) => v,
                };
                let value = start + step * index as f64;
                // Trim trailing zeros for a clean float display (e.g. 1.5, not 1.500000).
                let mut text = format!("{value}");
                if !text.contains('.') && !text.contains('e') {
                    // keep integral floats bare, matches user expectation
                }
                if text.ends_with(".0") {
                    text.truncate(text.len() - 2);
                }
                text
            }
        }
    }
}

/// A pure numeric token is empty, or an optional leading `-`/`+` followed by
/// digits and at most one `.`.
pub fn looks_numeric(token: &str) -> bool {
    if token.is_empty() {
        return true;
    }
    let token = token.strip_prefix(['-', '+']).unwrap_or(token);
    if token.is_empty() {
        return false;
    }
    let mut seen_dot = false;
    for ch in token.chars() {
        if ch == '.' {
            if seen_dot {
                return false;
            }
            seen_dot = true;
        } else if !ch.is_ascii_digit() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sequence_is_1_2_3_4() {
        let gen = NumericGenerator::new(None, None, None).unwrap();
        let values: Vec<String> = (0..4).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["1", "2", "3", "4"]);
    }

    #[test]
    fn step_and_zero_padded_format() {
        let gen = NumericGenerator::new(Some("1"), Some("2"), Some("%02d")).unwrap();
        let values: Vec<String> = (0..4).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["01", "03", "05", "07"]);
    }

    #[test]
    fn negative_step_decrements() {
        let gen = NumericGenerator::new(Some("10"), Some("-1"), None).unwrap();
        let values: Vec<String> = (0..4).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["10", "9", "8", "7"]);
    }

    #[test]
    fn float_sequence() {
        let gen = NumericGenerator::new(Some("0.5"), Some("0.5"), None).unwrap();
        let values: Vec<String> = (0..3).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["0.5", "1", "1.5"]);
    }

    #[test]
    fn invalid_format_is_rejected() {
        let err = NumericGenerator::new(Some("1"), Some("1"), Some("%02x")).unwrap_err();
        assert!(matches!(err, SequenceError::InvalidSyntax(_)));
    }

    #[test]
    fn large_cursor_count_is_fast_and_correct() {
        let gen = NumericGenerator::new(Some("0"), Some("1"), None).unwrap();
        let values = crate::generator::generate_n(&gen, 10_000);
        assert_eq!(values.len(), 10_000);
        assert_eq!(values[0], "0");
        assert_eq!(values[9_999], "9999");
    }
}
