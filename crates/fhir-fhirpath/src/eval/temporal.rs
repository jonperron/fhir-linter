//! Date, DateTime and Time comparison per FHIRPath N1 §6.1.1 and §6.2:
//! precision by precision, with seconds and milliseconds as one precision,
//! and time zones normalised to UTC.
//!
//! shortcut: values are parsed from their literal text on each comparison;
//! move to typed temporal values when F10 lands.
use std::cmp::Ordering;

/// Components from year (or hour for a Time) down to seconds, and the
/// zone offset in minutes.
struct Parts {
    fields: Vec<f64>,
    offset: Option<i64>,
}

/// Order of two date/time texts; `None` when they share every common
/// precision but one is more precise, or only one has a zone.
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
    }
    for (x, y) in a.fields.iter().zip(&b.fields) {
        match x.partial_cmp(y)? {
            Ordering::Equal => {}
            other => return Some(other),
        }
    }
    (a.fields.len() == b.fields.len()).then_some(Ordering::Equal)
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
