//! Error type for the sequence parsing engine.

use std::fmt;

/// Any problem detected while parsing a user-supplied sequence spec.
///
/// This is intentionally *not* fatal to the caller: per the PRD's edge-case
/// handling, an [`InvalidSyntax`](SequenceError::InvalidSyntax) error should
/// be caught by the caller and result in a graceful fallback to a basic
/// `1, 2, 3, ...` numeric sequence rather than aborting or panicking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceError {
    /// The input string did not match any supported generator syntax.
    InvalidSyntax(String),
    /// The input matched a generator's shape but contained an invalid value
    /// (e.g. `date(2026-13-40)`, an out-of-range month/day).
    InvalidValue(String),
}

impl fmt::Display for SequenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SequenceError::InvalidSyntax(msg) => write!(f, "invalid sequence syntax: {msg}"),
            SequenceError::InvalidValue(msg) => write!(f, "invalid sequence value: {msg}"),
        }
    }
}

impl std::error::Error for SequenceError {}
