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
///
/// Never panics: all intermediate arithmetic uses `i128` so it cannot
/// overflow, and a target year outside `NaiveDate`'s representable range
/// clamps to [`NaiveDate::MIN`]/[`NaiveDate::MAX`] instead of panicking
/// (matches the PRD's "never crash the host" requirement for adversarial
/// input like `date(2026-01-01):999999999999999m`).
fn add_months(date: NaiveDate, months: i64) -> NaiveDate {
    // i128 cannot overflow for any i32 year combined with any i64 month
    // delta, so this arithmetic is panic-free regardless of input.
    let total_months: i128 = date.year() as i128 * 12 + (date.month() as i128 - 1) + months as i128;
    let year_i128 = total_months.div_euclid(12);
    let month0 = total_months.rem_euclid(12) as u32; // 0-based
    let month = month0 + 1;

    let year = match i32::try_from(year_i128) {
        Ok(year) => year,
        Err(_) => return if year_i128 > 0 { NaiveDate::MAX } else { NaiveDate::MIN },
    };

    let day = date.day();
    // Walk the day down until we land on a valid date for the target month.
    for candidate_day in (1..=day).rev() {
        if let Some(d) = NaiveDate::from_ymd_opt(year, month, candidate_day) {
            return d;
        }
    }
    // Unreachable in practice (every year/month has a 1st), but fall back to
    // a clamp rather than `.expect(..)` so this can truly never panic.
    NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(NaiveDate::MAX)
}

/// Adds `days` days to `date` without panicking, even for adversarial
/// `days` values that would overflow `chrono::TimeDelta` construction or
/// push the result outside `NaiveDate`'s representable range. Clamps to
/// [`NaiveDate::MIN`]/[`NaiveDate::MAX`] on overflow instead.
fn add_days_checked(date: NaiveDate, days: i64) -> NaiveDate {
    let clamp = || if days >= 0 { NaiveDate::MAX } else { NaiveDate::MIN };
    match chrono::TimeDelta::try_days(days) {
        Some(delta) => date.checked_add_signed(delta).unwrap_or_else(clamp),
        None => clamp(),
    }
}

impl Generator for DateGenerator {
    fn value_at(&self, index: usize) -> String {
        // Saturate instead of panicking on overflow for pathological
        // step/index combinations (e.g. a 10,000+ cursor date sequence with
        // a huge step).
        let n = self.step.saturating_mul(index as i64);
        let date = match self.unit {
            Unit::Days => add_days_checked(self.start, n),
            Unit::Weeks => add_days_checked(self.start, n.saturating_mul(7)),
            Unit::Months => add_months(self.start, n),
            Unit::Years => add_months(self.start, n.saturating_mul(12)),
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

    #[test]
    fn adversarial_huge_step_never_panics() {
        // Regression test: this used to panic inside chrono's Add impl /
        // i64 arithmetic overflow. Must now clamp instead of crashing the
        // extension host, per the PRD's edge-case handling.
        let days = DateGenerator::new("2026-01-01", "999999999999999999d").unwrap();
        assert_eq!(days.value_at(2), NaiveDate::MAX.format("%Y-%m-%d").to_string());

        let months = DateGenerator::new("2026-01-01", "999999999999999999m").unwrap();
        assert_eq!(months.value_at(2), NaiveDate::MAX.format("%Y-%m-%d").to_string());

        let years = DateGenerator::new("2026-01-01", "-999999999999999999y").unwrap();
        assert_eq!(years.value_at(2), NaiveDate::MIN.format("%Y-%m-%d").to_string());
    }
}
