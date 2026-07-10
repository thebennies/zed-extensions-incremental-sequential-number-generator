//! Date sequence generator: `date([start_date]):[step][unit]`.
//!
//! Examples (from the PRD):
//! - `date(2026-07-07):1d` -> `2026-07-07`, `2026-07-08`, `2026-07-09`
//! - `date(2026-01):1m`    -> `2026-01`, `2026-02`, `2026-03`
//!
//! Supported start-date precisions: `YYYY-MM-DD` (day precision) and
//! `YYYY-MM` (month precision, output re-truncated to `YYYY-MM`).
//! Supported step units: `d` (days), `w` (weeks), `m` (months), `y` (years).

use chrono::{Datelike, NaiveDate};

use crate::error::SequenceError;
use crate::generator::Generator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Precision {
    Day,
    Month,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Days,
    Weeks,
    Months,
    Years,
}

impl Unit {
    fn parse(ch: char) -> Result<Self, SequenceError> {
        match ch {
            'd' => Ok(Unit::Days),
            'w' => Ok(Unit::Weeks),
            'm' => Ok(Unit::Months),
            'y' => Ok(Unit::Years),
            other => Err(SequenceError::InvalidSyntax(format!("unknown date unit: {other}"))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DateGenerator {
    start: NaiveDate,
    precision: Precision,
    step: i64,
    unit: Unit,
}

impl DateGenerator {
    /// `start_date` is the text inside `date(...)`; `step_spec` is the text
    /// after the following `:` (e.g. `1d`, `-2w`, `1m`, `1y`).
    pub fn new(start_date: &str, step_spec: &str) -> Result<Self, SequenceError> {
        let (start, precision) = parse_start_date(start_date)?;
        let (step, unit) = parse_step_spec(step_spec)?;
        Ok(Self { start, precision, step, unit })
    }
}

fn parse_start_date(raw: &str) -> Result<(NaiveDate, Precision), SequenceError> {
    let parts: Vec<&str> = raw.split('-').collect();
    match parts.as_slice() {
        [y, m, d] => {
            let (y, m, d) = (
                y.parse::<i32>().map_err(|_| bad_date(raw))?,
                m.parse::<u32>().map_err(|_| bad_date(raw))?,
                d.parse::<u32>().map_err(|_| bad_date(raw))?,
            );
            let date = NaiveDate::from_ymd_opt(y, m, d).ok_or_else(|| bad_date(raw))?;
            Ok((date, Precision::Day))
        }
        [y, m] => {
            let (y, m) = (
                y.parse::<i32>().map_err(|_| bad_date(raw))?,
                m.parse::<u32>().map_err(|_| bad_date(raw))?,
            );
            let date = NaiveDate::from_ymd_opt(y, m, 1).ok_or_else(|| bad_date(raw))?;
            Ok((date, Precision::Month))
        }
        _ => Err(SequenceError::InvalidSyntax(format!("unsupported date shape: {raw}"))),
    }
}

fn bad_date(raw: &str) -> SequenceError {
    SequenceError::InvalidValue(format!("invalid start date: {raw}"))
}

fn parse_step_spec(raw: &str) -> Result<(i64, Unit), SequenceError> {
    if raw.is_empty() {
        return Ok((1, Unit::Days));
    }
    let unit_char = raw
        .chars()
        .last()
        .ok_or_else(|| SequenceError::InvalidSyntax(format!("empty step spec: {raw}")))?;
    let unit = Unit::parse(unit_char)?;
    let number_part = &raw[..raw.len() - unit_char.len_utf8()];
    let step = if number_part.is_empty() {
        1
    } else {
        number_part
            .parse::<i64>()
            .map_err(|_| SequenceError::InvalidSyntax(format!("invalid step number: {raw}")))?
    };
    Ok((step, unit))
}

/// Adds `step * multiplier` months to `date`, clamping the day-of-month to
/// the target month's length (e.g. Jan 31 + 1 month -> Feb 28/29, never an
/// invalid date).
fn add_months(date: NaiveDate, months: i64) -> NaiveDate {
    let total_months = date.year() as i64 * 12 + (date.month() as i64 - 1) + months;
    let year = total_months.div_euclid(12) as i32;
    let month0 = total_months.rem_euclid(12) as u32; // 0-based
    let month = month0 + 1;
    let day = date.day();
    // Walk the day down until we land on a valid date for the target month.
    for candidate_day in (1..=day).rev() {
        if let Some(d) = NaiveDate::from_ymd_opt(year, month, candidate_day) {
            return d;
        }
    }
    // Unreachable in practice: every (year, month) has at least a 1st.
    NaiveDate::from_ymd_opt(year, month, 1).expect("month always has a 1st day")
}

impl Generator for DateGenerator {
    fn value_at(&self, index: usize) -> String {
        let n = self.step * index as i64;
        let date = match self.unit {
            Unit::Days => self.start + chrono::Duration::days(n),
            Unit::Weeks => self.start + chrono::Duration::weeks(n),
            Unit::Months => add_months(self.start, n),
            Unit::Years => add_months(self.start, n * 12),
        };
        match self.precision {
            Precision::Day => date.format("%Y-%m-%d").to_string(),
            Precision::Month => date.format("%Y-%m").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_precision_sequence() {
        let gen = DateGenerator::new("2026-07-07", "1d").unwrap();
        let values: Vec<String> = (0..3).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["2026-07-07", "2026-07-08", "2026-07-09"]);
    }

    #[test]
    fn month_precision_sequence() {
        let gen = DateGenerator::new("2026-01", "1m").unwrap();
        let values: Vec<String> = (0..3).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["2026-01", "2026-02", "2026-03"]);
    }

    #[test]
    fn month_step_clamps_end_of_month() {
        let gen = DateGenerator::new("2026-01-31", "1m").unwrap();
        let values: Vec<String> = (0..3).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["2026-01-31", "2026-02-28", "2026-03-31"]);
    }

    #[test]
    fn week_and_year_units() {
        let weeks = DateGenerator::new("2026-01-01", "1w").unwrap();
        assert_eq!(weeks.value_at(1), "2026-01-08");

        let years = DateGenerator::new("2026-01-01", "1y").unwrap();
        assert_eq!(years.value_at(1), "2027-01-01");
    }

    #[test]
    fn rejects_invalid_calendar_date() {
        assert!(DateGenerator::new("2026-13-40", "1d").is_err());
    }

    #[test]
    fn rejects_unknown_unit() {
        assert!(DateGenerator::new("2026-01-01", "1x").is_err());
    }
}
