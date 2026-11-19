//! Date, DateTime and Time comparison per FHIRPath N1 §6.1.1 and §6.2:
//! precision by precision, with seconds and milliseconds as one precision,
//! and time zones normalised to UTC.
//!
//! shortcut: values keep their text and are parsed on each comparison; store
//! the parsed parts if profiling shows the cost.
use std::cmp::Ordering;
use std::sync::Arc;

use crate::error::EvalError;
use crate::value::Value;

/// Components from year (or hour for a Time) down to seconds, and the
/// zone offset in minutes.
#[derive(Clone)]
struct Parts {
    fields: Vec<f64>,
    offset: Option<i64>,
}

/// Order of two date/time texts; `None` when they share every common
/// precision but one is more precise, when only one has a zone (a date
/// against a zoned date-time answers when every possible offset agrees).
/// Unparseable text compares as `None` too.
pub(super) fn compare(a: &str, b: &str, is_time: bool) -> Option<Ordering> {
    let (mut a, mut b) = (parse(a, is_time)?, parse(b, is_time)?);
    // A zone only matters once a date-time has an hour; Time takes no zone.
    let has_hour = |p: &Parts| !is_time && p.fields.len() >= 4;
    if has_hour(&a) && has_hour(&b) {
        match (a.offset, b.offset) {
            (Some(a_offset), Some(b_offset)) => {
                to_utc(&mut a, a_offset);
                to_utc(&mut b, b_offset);
            }
            (None, None) => {}
            // The official suite expects empty when only one side has a zone.
            _ => return None,
        }
    } else if is_time && a.offset.is_some() != b.offset.is_some() {
        // A Time takes no zone, so a zoned one is malformed.
        return None;
    } else if let (Some(zone), None) | (None, Some(zone)) = (a.offset, b.offset) {
        // A zoned date-time against a date: N1 §6.1.1 makes the date's zone
        // "an implementation decision", so none is assumed. The local date of
        // the date-time only moves forward as the offset grows, so the two
        // extremes agreeing means every offset agrees.
        let order_at = |date_offset: i64| {
            let (mut a, mut b) = (a.clone(), b.clone());
            let zoned = if a.offset.is_some() { &mut a } else { &mut b };
            to_utc(zoned, zone - date_offset);
            order(&a.fields, &b.fields)
        };
        let (west, east) = (order_at(-MAX_OFFSET), order_at(MAX_OFFSET));
        return (west == east).then_some(west).flatten();
    }
    order(&a.fields, &b.fields)
}

/// The widest zone offset FHIR allows, in minutes (dateTime regex: 14:00).
const MAX_OFFSET: i64 = 14 * 60;

/// Precision-by-precision order of two field lists.
fn order(a: &[f64], b: &[f64]) -> Option<Ordering> {
    for (x, y) in a.iter().zip(b) {
        match x.partial_cmp(y)? {
            Ordering::Equal => {}
            other => return Some(other),
        }
    }
    (a.len() == b.len()).then_some(Ordering::Equal)
}

fn parse(text: &str, is_time: bool) -> Option<Parts> {
    let (date, time) = if is_time {
        ("", text)
    } else {
        text.split_once('T').unwrap_or((text, ""))
    };
    let (time, offset) = split_zone(time)?;
    let mut fields = Vec::new();
    for part in date.split('-').filter(|p| !p.is_empty()) {
        fields.push(part.parse::<u32>().ok()?.into());
    }
    for part in time.split(':').filter(|p| !p.is_empty()) {
        fields.push(part.parse::<f64>().ok()?);
    }
    (!fields.is_empty()).then_some(Parts { fields, offset })
}

/// Split `10:00:00+02:00` into the time and its offset in minutes.
fn split_zone(time: &str) -> Option<(&str, Option<i64>)> {
    if let Some(rest) = time.strip_suffix('Z') {
        return Some((rest, Some(0)));
    }
    let Some(at) = time.rfind(['+', '-']) else {
        return Some((time, None));
    };
    let (hours, minutes) = time[at + 1..].split_once(':')?;
    let magnitude = hours.parse::<i64>().ok()? * 60 + minutes.parse::<i64>().ok()?;
    let sign = if time[at..].starts_with('-') { -1 } else { 1 };
    Some((&time[..at], Some(sign * magnitude)))
}

/// Shift a date-time that has an hour to UTC, carrying across days.
fn to_utc(parts: &mut Parts, offset: i64) {
    let field = |i: usize| parts.fields.get(i).copied().unwrap_or(0.0) as i64;
    let total =
        days_from_civil(field(0), field(1), field(2)) * 1440 + field(3) * 60 + field(4) - offset;
    let (year, month, day) = civil_from_days(total.div_euclid(1440));
    let minute_of_day = total.rem_euclid(1440);
    let shifted = [year, month, day, minute_of_day / 60, minute_of_day % 60];
    // zip stops at the value's own precision, so an hour-only value stays one.
    for (slot, value) in parts.fields.iter_mut().zip(shifted) {
        *slot = value as f64;
    }
}

/// Days since 1970-01-01 (H. Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Inverse of [`days_from_civil`].
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Calendar duration units from year to millisecond, with the UCUM codes
/// N1 §4.1.8 lets stand for them. UCUM `a` and `mo` are definite durations,
/// not calendar ones, so they are absent (testPlusDate14, testPlusDate16).
const DURATION_UNITS: [&[&str]; 7] = [
    &["year"],
    &["month"],
    &["day", "d"],
    &["hour", "h"],
    &["minute", "min"],
    &["second", "s"],
    &["millisecond", "ms"],
];

/// Milliseconds in a day, hour, minute, second and millisecond: the units
/// from [`DURATION_UNITS`] index 2 on.
const UNIT_MILLIS: [i64; 5] = [86_400_000, 3_600_000, 60_000, 1000, 1];

/// `value + amount unit` for a Date, DateTime or Time (N2 §6.6.7; `-` passes
/// a negated amount, §6.6.8). The result keeps the value's precision and
/// zone text. Amounts above seconds lose their decimals, and a unit finer
/// than the value converts to the value's precision, truncated.
pub(super) fn add_duration(value: &Value, amount: f64, unit: &str) -> Result<Value, EvalError> {
    let (text, is_time) = match value {
        Value::Date(text) | Value::DateTime(text) => (text, false),
        Value::Time(text) => (text, true),
        _ => {
            return Err(EvalError::Type(
                "date arithmetic requires a date or time".into(),
            ));
        }
    };
    let bad_unit = || EvalError::Type(format!("'{unit}' is not a calendar duration"));
    let (index, amount) = match unit {
        "week" | "wk" => (2, amount.trunc() * 7.0),
        _ => {
            let index = DURATION_UNITS
                .iter()
                .position(|names| names.contains(&unit))
                .ok_or_else(bad_unit)?;
            (index, if index < 5 { amount.trunc() } else { amount })
        }
    };
    if is_time && index < 3 {
        // not settled by N2: a Time has no calendar to add days or months to.
        return Err(EvalError::Type(format!("cannot add {unit} to a Time")));
    }
    let mut fields = TemporalFields::parse(text, is_time)
        .ok_or_else(|| EvalError::Type(format!("invalid date/time: {text}")))?;
    let out_of_range = || EvalError::Type(format!("{text} + {amount} {unit} is out of range"));
    if !amount.is_finite() {
        return Err(out_of_range());
    }
    if index < 2 {
        let months = (amount as i64)
            .checked_mul(if index == 0 { 12 } else { 1 })
            .ok_or_else(out_of_range)?;
        // A year-precision value takes whole years only.
        let months = if fields.precision == 0 {
            months / 12 * 12
        } else {
            months
        };
        fields.add_months(months).ok_or_else(out_of_range)?;
    } else if fields.precision < 2 {
        // not settled by N2: a day has no fixed length in months or years.
        return Err(EvalError::Type(format!(
            "cannot add {unit} to {text}, which has no day"
        )));
    } else {
        // R5 allows second 60, but civil-time arithmetic would turn
        // 23:59:60 into the next day, so a shift past the day is an error.
        // shortcut: leap seconds are refused, not carried; N2 is silent.
        if fields.fields[5] == 60 {
            return Err(out_of_range());
        }
        let step = UNIT_MILLIS[fields.precision - 2];
        // Rounding to a nano-step first keeps 1.005 s at 1005 ms, not 1004.
        let steps = ((amount * UNIT_MILLIS[index - 2] as f64 / step as f64) * 1e9).round() / 1e9;
        let steps = steps.trunc();
        // 1e18 ms is far past 9999 years yet keeps the sum inside i64.
        if !steps.is_finite() || (steps * step as f64).abs() > 1e18 {
            return Err(out_of_range());
        }
        fields.add_millis(steps as i64 * step);
    }
    fields.format(text, is_time).map(Arc::from).map_or_else(
        || Err(out_of_range()),
        |text| {
            Ok(match value {
                Value::Date(_) => Value::Date(text),
                Value::DateTime(_) => Value::DateTime(text),
                _ => Value::Time(text),
            })
        },
    )
}

/// The year-to-millisecond fields of a date/time text, the index of its
/// most precise field and its zone text. A Time sits on 1970-01-01.
struct TemporalFields<'a> {
    fields: [i64; 7],
    precision: usize,
    zone: &'a str,
}

impl<'a> TemporalFields<'a> {
    fn parse(text: &'a str, is_time: bool) -> Option<Self> {
        let (date, time) = if is_time {
            ("1970-01-01", text)
        } else {
            text.split_once('T').unwrap_or((text, ""))
        };
        let (clock, _) = split_zone(time)?;
        let mut fields = [0, 1, 1, 0, 0, 0, 0];
        let mut count = 0;
        let (whole, fraction) = clock.split_once('.').unwrap_or((clock, ""));
        let parts = date
            .split('-')
            .chain(whole.split(':'))
            .filter(|p| !p.is_empty());
        for part in parts {
            if !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            *fields.get_mut(count)? = part.parse().ok()?;
            count += 1;
        }
        // A clock needs a full date (R5 dateTime); `2014T10:00` would shift
        // its hour into the month.
        if !is_time && count < 3 && !clock.is_empty() {
            return None;
        }
        if !fraction.is_empty() {
            // shortcut: the fraction is written back as three digits (`.5` gives
            // `.500`, sub-millisecond digits dropped); keep the original digits if
            // a caller needs them.
            let millis: String = fraction.chars().chain("00".chars()).take(3).collect();
            fields[6] = millis.parse().ok()?;
            count = 7;
        }
        let precision = count.checked_sub(1)?;
        let [year, month, day, hour, minute, second, _] = fields;
        let month_length = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
            2 => 28,
            _ => return None,
        };
        if !(1..=9999).contains(&year) || !(1..=month_length).contains(&day) {
            return None;
        }
        if hour > 23 || minute > 59 || second > 60 {
            return None;
        }
        let zone = &time[clock.len()..];
        Some(Self {
            fields,
            precision,
            zone,
        })
    }

    /// Shift by whole months, clamping the day to the end of the month.
    fn add_months(&mut self, months: i64) -> Option<()> {
        let total = self.fields[0]
            .checked_mul(12)?
            .checked_add(self.fields[1] - 1)?
            .checked_add(months)?;
        let (year, month) = (total.div_euclid(12), total.rem_euclid(12) + 1);
        if !(1..=9999).contains(&year) {
            return None;
        }
        let next_month_start = if month == 12 {
            days_from_civil(year + 1, 1, 1)
        } else {
            days_from_civil(year, month + 1, 1)
        };
        let month_length = next_month_start - days_from_civil(year, month, 1);
        self.fields[0] = year;
        self.fields[1] = month;
        self.fields[2] = self.fields[2].min(month_length);
        Some(())
    }

    /// Shift by milliseconds, carrying into the date. A Time wraps at
    /// midnight as the carried days are dropped on output.
    fn add_millis(&mut self, millis: i64) {
        let [year, month, day, hour, minute, second, milli] = self.fields;
        let total = days_from_civil(year, month, day) * 86_400_000
            + hour * 3_600_000
            + minute * 60_000
            + second * 1000
            + milli
            + millis;
        let (year, month, day) = civil_from_days(total.div_euclid(86_400_000));
        let of_day = total.rem_euclid(86_400_000);
        self.fields = [
            year,
            month,
            day,
            of_day / 3_600_000,
            of_day / 60_000 % 60,
            of_day / 1000 % 60,
            of_day % 1000,
        ];
    }

    /// The text of the shifted value in its original shape; `None` when the
    /// year leaves 0001-9999.
    fn format(&self, original: &str, is_time: bool) -> Option<String> {
        if !(1..=9999).contains(&self.fields[0]) && !is_time {
            return None;
        }
        let mut text = String::new();
        let first_clock_field = if is_time { 3 } else { 0 };
        for (index, field) in self.fields.iter().enumerate() {
            if index < first_clock_field || index > self.precision {
                continue;
            }
            let separator = match index {
                0 => "",
                1 | 2 => "-",
                3 if is_time => "",
                3 => "T",
                6 => ".",
                _ => ":",
            };
            let width = match index {
                0 => 4,
                6 => 3,
                _ => 2,
            };
            text.push_str(&format!("{separator}{field:0width$}"));
        }
        if self.precision < 3 && original.contains('T') {
            text.push('T');
        }
        text.push_str(self.zone);
        Some(text)
    }
}

/// The current UTC date-time as DateTime text with milliseconds and a
/// `+00:00` zone, for `now()` and `today()` (N2 §5.10).
///
/// shortcut: UTC, not the local zone, as std cannot read it; add a zone
/// source to `Environment` if callers need local dates.
///
/// `None` when the system clock is before 1970 or past year 9999, which no
/// FHIRPath DateTime can hold.
pub(super) fn now_text() -> Option<String> {
    let since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    let millis = i64::try_from(since_epoch.as_millis()).ok()?;
    let (year, month, day) = civil_from_days(millis.div_euclid(86_400_000));
    if year > 9999 {
        return None;
    }
    let millis_of_day = millis.rem_euclid(86_400_000);
    let (hour, minute) = (millis_of_day / 3_600_000, millis_of_day / 60_000 % 60);
    let (second, millisecond) = (millis_of_day / 1000 % 60, millis_of_day % 1000);
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millisecond:03}+00:00"
    ))
}
