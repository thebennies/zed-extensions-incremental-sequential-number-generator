//! # sequence-core
//!
//! Pure-Rust, allocation-conscious sequence generation engine powering the
//! *Incremental & Sequential Number Generator* Zed extension.
//!
//! This crate has **no dependency on `zed_extension_api`** and no I/O, so it
//! compiles and tests natively (`cargo test -p sequence-core`) as well as to
//! `wasm32-wasip1` for the extension host. It supports four generator kinds
//! per the PRD:
//!
//! - Numeric: `[start]:[step]:[format]`, e.g. `1:2:%02d`
//! - Alphabetical: `<letter>:[step]`, e.g. `a:1`
//! - Date: `date([start_date]):[step][unit]`, e.g. `date(2026-07-07):1d`
//! - Word list: comma-separated, e.g. `apple, banana, cherry`
//!
//! The single public entry point most callers need is [`generate_sequence`],
//! which never fails - on invalid input it falls back to a plain `1, 2, 3,
//! ...` sequence, matching the PRD's edge-case handling requirement.

mod alpha;
mod args;
mod date;
mod error;
mod generator;
mod numeric;
mod parser;
mod wordlist;

pub use alpha::AlphaGenerator;
pub use args::parse_spec_and_count;
pub use date::DateGenerator;
pub use error::SequenceError;
pub use generator::{generate_n, Generator};
pub use numeric::NumericGenerator;
pub use parser::{generate_sequence, parse, try_generate_sequence};
pub use wordlist::WordListGenerator;
