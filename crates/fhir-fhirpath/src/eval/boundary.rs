//! `lowBoundary()` and `highBoundary()` (FHIRPath N2 utility functions) and
//! `comparable()` (N2 comparison), as the official FHIRPath suite exercises
//! them. FHIR invariants per-1, rng-2 and ratrng-2 depend on them.
use std::sync::Arc;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};
use crate::{Decimal, Rounding};

use super::Evaluator;
use super::helpers::{
    as_quantity, check_arity, check_arity_range, optional_integer, quantity_values, singleton,
};

/// N2: "at least 8 for Decimal" when no precision is given.
const DECIMAL_DEFAULT_PRECISION: i64 = 8;
/// N2: a precision above the implementation's maximum gives empty.
const DECIMAL_MAX_PRECISION: i64 = 28;

#[derive(Clone, Copy, PartialEq)]
enum Bound {
    Low,
    High,
}

#[derive(Clone, Copy, PartialEq)]
enum Temporal {
    Date,
    DateTime,
    Time,
}

impl Temporal {
    /// Precisions in digits (N2 `precision()`: 4 year .. 17 milliseconds;
    /// 2 hour .. 9 milliseconds for a Time).
    fn precisions(self) -> &'static [i64] {
        match self {
            Self::Date => &[4, 6, 8],
            Self::DateTime => &[4, 6, 8, 10, 12, 14, 17],
            Self::Time => &[2, 4, 6, 9],
        }
    }

    /// N2: "at least 17 for DateTime, and at least 9 for Time". The Date
    /// default is not decided: N2 says both "the greatest precision of the
    /// type" (8) and "4 for Date", so a Date needs an explicit precision.
    fn default_precision(self) -> Option<i64> {
        match self {
            Self::Date => None,
            Self::DateTime => Some(17),
            Self::Time => Some(9),
        }
    }
}

impl Evaluator<'_> {
    /// Handle the boundary and `comparable()` functions.
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_boundary_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        let bound = match name {
            "lowBoundary" => Bound::Low,
            "highBoundary" => Bound::High,
            "comparable" => return Some(self.comparable(args, focus, ctx)),
            "precision" => return Some(precision(name, args, focus)),
            _ => return None,
        };
        Some(self.boundary(name, bound, args, focus, ctx))
    }

    fn boundary(
        &self,
        name: &str,
        bound: Bound,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Collection, EvalError> {
        check_arity_range(name, args, 0, 1)?;
        let precision = match args.first() {
            None => None,
            Some(arg) => match optional_integer(&self.eval(arg, ctx)?, name)? {
                Some(precision) => Some(precision),
                None => return Ok(vec![]),
            },
        };
        let Some(item) = singleton(focus, name)? else {
            return Ok(vec![]);
        };
        Ok(boundary(item, bound, precision).into_iter().collect())
    }

    /// N2: true when both Quantities have units this engine can compare;
    /// empty unless each side is a single Quantity.
    fn comparable(
        &self,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Collection, EvalError> {
        check_arity("comparable", args, 1)?;
        let other = self.eval(&args[0], ctx)?;
        let result = match (focus, other.as_slice()) {
            // `quantity_values` also converts a number; comparable() is on Quantities.
            ([a], [b])
                if [a, b]
                    .iter()
                    .all(|v| as_quantity(v.primitive_value()).is_some()) =>
            {
                quantity_values(a.primitive_value(), b.primitive_value(), false)
                    .map(|values| Value::Bool(values.is_some()))
            }
            _ => None,
        };
        Ok(result.into_iter().collect())
    }
}

fn boundary(item: &Value, bound: Bound, precision: Option<i64>) -> Option<Value> {
    let temporal = |text: &str, kind: Temporal| {
        temporal_boundary(text, kind, bound, precision.or(kind.default_precision())?)
    };
    match item {
        Value::Integer(n) | Value::Long(n) => {
            decimal_boundary(Decimal::from(*n), bound, precision).map(Value::Decimal)
        }
        Value::Decimal(d) => decimal_boundary(*d, bound, precision).map(Value::Decimal),
        Value::Date(text) => temporal(text, Temporal::Date).map(|t| Value::Date(t.into())),
        Value::DateTime(text) => {
            temporal(text, Temporal::DateTime).map(|t| Value::DateTime(t.into()))
        }
        Value::Time(text) => temporal(text, Temporal::Time).map(|t| Value::Time(t.into())),
        other => {
            // N2: the Decimal boundary of the value, with the original unit.
            let (value, unit) = as_quantity(other)?;
            let boundary = decimal_boundary(Decimal::from_f64(value)?, bound, precision)?.to_f64();
            // A unitless FHIR Quantity has unit '1', as in toQuantity().
            Some(Value::Quantity(boundary, Arc::from(unit.unwrap_or("1"))))
        }
    }
}

/// FHIRPath 3.0 `precision()`: the digits of a Date, DateTime or Time
/// (`@2014` is 4, `@T10:30:00.000` is 9), or the decimal places of a Decimal.
fn precision(name: &str, args: &[Expr], focus: &[Value]) -> Result<Collection, EvalError> {
    check_arity(name, args, 0)?;
    let Some(item) = singleton(focus, name)? else {
        return Ok(vec![]);
    };
    let digits = match item.primitive_value() {
        Value::Date(text) | Value::DateTime(text) | Value::Time(text) => {
            // The zone (`Z`, `+hh:mm`, `-hh:mm`) only follows the time part.
            let (date, time) = match text.split_once('T') {
                Some((date, time)) => (date, time.split(['Z', '+', '-']).next().unwrap_or(time)),
                None => (text.as_ref(), ""),
            };
            date.bytes()
                .chain(time.bytes())
                .filter(u8::is_ascii_digit)
                .count()
        }
        Value::Decimal(d) => usize::try_from(d.scale()).unwrap_or(0),
        _ => return Ok(vec![]),
    };
    Ok(vec![Value::Integer(digits as i64)])
}

/// The boundary of a decimal, half a unit of its last written digit away
/// (`9.5e2` is precise to the tens), at `precision` places the way the
/// official suite expects: truncated towards zero, and rounded half up away
/// from zero. Overflow gives empty.
fn decimal_boundary(value: Decimal, bound: Bound, precision: Option<i64>) -> Option<Decimal> {
    let precision = precision.unwrap_or(DECIMAL_DEFAULT_PRECISION);
    if !(0..=DECIMAL_MAX_PRECISION).contains(&precision) {
        return None;
    }
    let negative = value.mantissa() < 0;
    // One digit past the input's own precision holds the half unit.
    let units = value.mantissa().checked_abs()?.checked_mul(10)?;
    let scale = value.scale().checked_add(1)?;
    // Below a negative value the magnitude grows: low(-x) is -high(x).
    let away_from_zero = (bound == Bound::High) != negative;
    let (edge, rounding) = if away_from_zero {
        (units.checked_add(5)?, Rounding::HalfAwayFromZero)
    } else {
        (units - 5, Rounding::TowardZero)
    };
    let cut = Decimal::new(edge, scale).rescale(i32::try_from(precision).ok()?, rounding)?;
    if negative {
        cut.checked_neg()
    } else {
        Some(cut)
    }
}

/// The boundary of a date, date-time or time text at `precision` digits:
/// missing parts take their least (low) or greatest (high) value, and a
/// date-time with a time but no zone takes the widest offset, `+14:00`
/// (low) or `-12:00` (high), as the official suite expects.
fn temporal_boundary(text: &str, kind: Temporal, bound: Bound, precision: i64) -> Option<String> {
    if !kind.precisions().contains(&precision) {
        return None;
    }
    let low = bound == Bound::Low;
    // A part of `width` ASCII digits, or its least (low) or greatest (high) value.
    let pick = |given: Option<&str>, least: &str, greatest: &str, width: usize| {
        let part = given.unwrap_or(if low { least } else { greatest });
        (part.len() == width && part.bytes().all(|b| b.is_ascii_digit())).then(|| part.to_owned())
    };
    let (date, time) = match kind {
        Temporal::Time => ("", text),
        _ => text.split_once('T').unwrap_or((text, "")),
    };
    let mut out = String::new();
    let mut time_precision = precision;
    if kind != Temporal::Time {
        let mut parts = date.split('-');
        let year = pick(parts.next(), "", "", 4)?;
        let month = pick(parts.next(), "01", "12", 2)?;
        let last_day = days_in_month(year.parse().ok()?, month.parse().ok()?)?;
        let day = pick(parts.next(), "01", &format!("{last_day:02}"), 2).filter(|day| {
            day.parse::<u32>()
                .is_ok_and(|d| (1..=last_day).contains(&d))
        })?;
        parts.next().is_none().then_some(())?;
        out.push_str(&year);
        for (part, from) in [(month, 6), (day, 8)] {
            if precision >= from {
                out.push('-');
                out.push_str(&part);
            }
        }
        if precision <= 8 {
            return Some(out);
        }
        out.push('T');
        time_precision -= 8;
    }
    let (clock, zone) = time.split_at(time.find(['Z', '+', '-']).unwrap_or(time.len()));
    let mut parts = (!clock.is_empty())
        .then(|| clock.split(':'))
        .into_iter()
        .flatten();
    let hour = parts.next();
    let has_hour = hour.is_some();
    let hour = pick(hour, "00", "23", 2).filter(|h| at_most(h, 23))?;
    // The suite reads an hour without minutes as hh:00 (FHIR has no hour-only time).
    let minute = parts.next();
    let minute =
        pick(minute.or(has_hour.then_some("00")), "00", "59", 2).filter(|m| at_most(m, 59))?;
    let (second, fraction) = match parts.next() {
        Some(second) => second.split_once('.').unwrap_or((second, "")),
        None => (if low { "00" } else { "59" }, ""),
    };
    let second = pick(Some(second), "", "", 2).filter(|s| at_most(s, 60))?;
    let millis = match fraction {
        "" if low => "000".to_owned(),
        "" => "999".to_owned(),
        given => format!("{:0<3}", given.get(..3).unwrap_or(given)),
    };
    let millis = pick(Some(&millis), "", "", 3)?;
    // No trailing component, and only a DateTime takes a zone.
    parts.next().is_none().then_some(())?;
    (kind == Temporal::DateTime || zone.is_empty()).then_some(())?;
    out.push_str(&hour);
    for (part, from, separator) in [(minute, 4, ':'), (second, 6, ':'), (millis, 9, '.')] {
        if time_precision >= from {
            out.push(separator);
            out.push_str(&part);
        }
    }
    if kind == Temporal::DateTime {
        out.push_str(match zone {
            "" if low => "+14:00",
            "" => "-12:00",
            given if valid_zone(given) => given,
            _ => return None,
        });
    }
    Some(out)
}

/// Whether a digit text is at most `max`.
fn at_most(digits: &str, max: u32) -> bool {
    digits.bytes().all(|b| b.is_ascii_digit())
        && digits.parse::<u32>().is_ok_and(|value| value <= max)
}

/// `Z` or `+hh:mm` / `-hh:mm` up to 14:00, as the FHIR dateTime format allows.
fn valid_zone(zone: &str) -> bool {
    zone == "Z"
        || zone
            .strip_prefix(['+', '-'])
            .and_then(|offset| offset.split_once(':'))
            .is_some_and(|(hours, minutes)| {
                hours.len() == 2
                    && minutes.len() == 2
                    && at_most(hours, 14)
                    && at_most(minutes, if hours == "14" { 0 } else { 59 })
            })
}

fn days_in_month(year: u32, month: u32) -> Option<u32> {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(if leap { 29 } else { 28 }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_temporal_text_has_no_boundary() {
        for (text, kind) in [
            ("2014-02-30", Temporal::Date),
            ("2014-02-29", Temporal::Date),
            ("2014-2-01", Temporal::Date),
            ("12345", Temporal::Date),
            ("2014-01-01T24:00", Temporal::DateTime),
            ("2014-01-01T10::30", Temporal::DateTime),
            ("10:75", Temporal::Time),
            ("10:30:61", Temporal::Time),
            ("1é:30", Temporal::Time),
            ("2014-01-01T10:00+garbage", Temporal::DateTime),
            ("2014-01-01T10:00+15:00", Temporal::DateTime),
            ("2014-01-01T10:00+14:30", Temporal::DateTime),
            ("2014-01-01-05", Temporal::Date),
            ("10:30:00:15", Temporal::Time),
            ("10:30:00.5:15", Temporal::Time),
            ("2014-01-01T10:30:00:15", Temporal::DateTime),
            ("10:30:00Z", Temporal::Time),
            ("10:30:00+01:00", Temporal::Time),
        ] {
            let precision = kind.precisions()[kind.precisions().len() - 1];
            assert_eq!(
                temporal_boundary(text, kind, Bound::Low, precision),
                None,
                "{text}"
            );
        }
    }
}
