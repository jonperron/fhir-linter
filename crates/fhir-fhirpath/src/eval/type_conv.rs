use std::sync::{Arc, LazyLock};

use regex_lite::Regex;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

use super::Evaluator;
use super::helpers::{check_arity, singleton};

impl Evaluator<'_> {
    /// Handle type conversion and type-checking functions.
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_type_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        self.try_type_conv(name, args, focus).transpose()
    }

    #[allow(clippy::too_many_lines)]
    fn try_type_conv(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
    ) -> Result<Option<Collection>, EvalError> {
        let col: Collection = match name {
            // ── Integer ───────────────────────────────────────────────────
            "toInteger" => convert(name, args, focus, to_integer)?,
            "convertsToInteger" => converts(name, args, focus, to_integer)?,

            // ── Long ──────────────────────────────────────────────────────
            "toLong" => {
                check_arity(name, args, 0)?;
                match focus.first() {
                    Some(Value::Long(n)) => vec![Value::Long(*n)],
                    Some(Value::Integer(n)) => vec![Value::Long(*n)],
                    Some(Value::Bool(b)) => vec![Value::Long(if *b { 1 } else { 0 })],
                    Some(Value::String(s)) => match s.parse::<i64>() {
                        Ok(n) => vec![Value::Long(n)],
                        Err(_) => vec![],
                    },
                    None => vec![],
                    Some(v) => {
                        return Err(EvalError::Type(format!(
                            "toLong() cannot convert {}",
                            v.type_name()
                        )));
                    }
                }
            }
            "convertsToLong" => {
                check_arity(name, args, 0)?;
                let result = match focus.first() {
                    Some(Value::Long(_) | Value::Integer(_) | Value::Bool(_)) => true,
                    Some(Value::String(s)) => s.parse::<i64>().is_ok(),
                    _ => false,
                };
                vec![Value::Bool(result)]
            }

            // ── Decimal ───────────────────────────────────────────────────
            "toDecimal" => convert(name, args, focus, to_decimal)?,
            "convertsToDecimal" => converts(name, args, focus, to_decimal)?,

            // ── Boolean ───────────────────────────────────────────────────
            "toBoolean" => convert(name, args, focus, to_boolean)?,
            "convertsToBoolean" => converts(name, args, focus, to_boolean)?,

            // ── String ────────────────────────────────────────────────────
            "convertsToString" => {
                check_arity(name, args, 0)?;
                vec![Value::Bool(!focus.is_empty())]
            }

            // ── Date / DateTime / Time ─────────────────────────────────────
            "toDate" => convert(name, args, focus, to_date)?,
            "convertsToDate" => converts(name, args, focus, to_date)?,
            "toDateTime" => convert(name, args, focus, to_date_time)?,
            "convertsToDateTime" => converts(name, args, focus, to_date_time)?,
            "toTime" => convert(name, args, focus, to_time)?,
            "convertsToTime" => converts(name, args, focus, to_time)?,

            // ── Quantity ──────────────────────────────────────────────────
            "toQuantity" => {
                if args.len() > 1 {
                    return Err(EvalError::Arity {
                        name: name.to_owned(),
                        expected: 1,
                        got: args.len(),
                    });
                }
                focus
                    .iter()
                    .filter_map(|v| match v {
                        Value::Quantity(_, _) => Some(v.clone()),
                        _ => None,
                    })
                    .collect()
            }
            "convertsToQuantity" => {
                check_arity(name, args, 0)?;
                let result = focus
                    .first()
                    .map(|v| matches!(v, Value::Quantity(_, _)))
                    .unwrap_or(false);
                vec![Value::Bool(result)]
            }

            _ => return Ok(None),
        };
        Ok(Some(col))
    }
}

// ── N1 §5.5 conversions ───────────────────────────────────────────────────────
//
// Each `to_*` returns the converted value, or `None` when the item is not
// convertible; `toX()` then gives empty and `convertsToX()` gives false.
// Both give empty on an empty input.

fn convert(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    to: fn(&Value) -> Option<Value>,
) -> Result<Collection, EvalError> {
    check_arity(name, args, 0)?;
    Ok(singleton(focus, name)?.and_then(to).into_iter().collect())
}

fn converts(
    name: &str,
    args: &[Expr],
    focus: &[Value],
    to: fn(&Value) -> Option<Value>,
) -> Result<Collection, EvalError> {
    check_arity(name, args, 0)?;
    Ok(singleton(focus, name)?
        .map(|item| Value::Bool(to(item).is_some()))
        .into_iter()
        .collect())
}

/// N1 §5.5.3: Integer, Boolean, or a String in the form `(\+|-)?\d+`.
fn to_integer(item: &Value) -> Option<Value> {
    match item {
        Value::Integer(n) => Some(Value::Integer(*n)),
        Value::Bool(b) => Some(Value::Integer(i64::from(*b))),
        // `i64::from_str` accepts exactly an optional sign and ASCII digits.
        Value::String(s) => s.parse().ok().map(Value::Integer),
        _ => None,
    }
}

/// N1 §5.5.4: Integer, Decimal, Boolean, or a String in the form
/// `(\+|-)?\d+(\.\d+)?`.
fn to_decimal(item: &Value) -> Option<Value> {
    match item {
        Value::Decimal(d) => Some(Value::Decimal(*d)),
        Value::Integer(n) => Some(Value::Decimal(*n as f64)),
        Value::Bool(b) => Some(Value::Decimal(if *b { 1.0 } else { 0.0 })),
        Value::String(s) if full_match(&DECIMAL, s) => s.parse().ok().map(Value::Decimal),
        _ => None,
    }
}

/// N1 §5.5.2: Boolean; Integer or Decimal 1 and 0; and these Strings, with
/// case ignored.
fn to_boolean(item: &Value) -> Option<Value> {
    let b = match item {
        Value::Bool(b) => *b,
        Value::Integer(1) => true,
        Value::Integer(0) => false,
        Value::Decimal(d) if *d == 1.0 => true,
        Value::Decimal(d) if *d == 0.0 => false,
        Value::String(s) => match s.to_ascii_lowercase().as_str() {
            "true" | "t" | "yes" | "y" | "1" | "1.0" => true,
            "false" | "f" | "no" | "n" | "0" | "0.0" => false,
            _ => return None,
        },
        _ => return None,
    };
    Some(Value::Bool(b))
}

/// N1 §5.5.6: Date; the date part of a DateTime; or a String in the
/// (partial) form `YYYY-MM-DD`.
fn to_date(item: &Value) -> Option<Value> {
    match item {
        Value::Date(_) => Some(item.clone()),
        Value::DateTime(s) => {
            let date = s.split_once('T').map_or(s.as_ref(), |(date, _)| date);
            Some(Value::Date(Arc::from(date)))
        }
        Value::String(s) if full_match(&DATE, s) && is_calendar_date(s) => {
            Some(Value::Date(Arc::clone(s)))
        }
        _ => None,
    }
}

/// N1 §5.5.7: DateTime; a Date (time left unset); or a String in the
/// (partial) form `YYYY-MM-DDThh:mm:ss.fff(+|-)hh:mm`.
fn to_date_time(item: &Value) -> Option<Value> {
    match item {
        Value::DateTime(_) => Some(item.clone()),
        Value::Date(s) => Some(Value::DateTime(Arc::clone(s))),
        Value::String(s) if full_match(&DATE_TIME, s) && is_calendar_date(s) => {
            Some(Value::DateTime(Arc::clone(s)))
        }
        _ => None,
    }
}

/// N1 §5.5.9: Time, or a String in the (partial) form `hh:mm:ss.fff`.
fn to_time(item: &Value) -> Option<Value> {
    match item {
        Value::Time(_) => Some(item.clone()),
        Value::String(s) if full_match(&TIME, s) => Some(Value::Time(Arc::clone(s))),
        _ => None,
    }
}

/// Year 0001-9999 and a day that exists in its month, leap years included.
/// Expects a string already matched by `DATE` or `DATE_TIME` (ASCII digits).
/// Only the date before any `T` is read: in `2015T11:31` there is no month.
fn is_calendar_date(s: &str) -> bool {
    let date = s.split_once('T').map_or(s, |(date, _)| date);
    let field = |range: std::ops::Range<usize>| date.get(range).and_then(|f| f.parse::<u32>().ok());
    let Some(year) = field(0..4).filter(|y| *y > 0) else {
        return false;
    };
    let (Some(month), Some(day)) = (field(5..7), field(8..10)) else {
        return true; // partial date: the regex already bounds the month
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    day <= days_in_month
}

const DATE_PART: &str = r"\d{4}(-(0[1-9]|1[0-2])(-(0[1-9]|[12]\d|3[01]))?)?";
const TIME_PART: &str = r"([01]\d|2[0-3])(:[0-5]\d(:[0-5]\d(\.\d+)?)?)?";

type Pattern = LazyLock<Option<Regex>>;

static DECIMAL: Pattern = LazyLock::new(|| anchored(r"[+-]?\d+(\.\d+)?"));
static DATE: Pattern = LazyLock::new(|| anchored(DATE_PART));
static DATE_TIME: Pattern = LazyLock::new(|| {
    anchored(&format!(
        r"{DATE_PART}(T{TIME_PART}(Z|[+-]((0\d|1[0-3]):[0-5]\d|14:00))?)?"
    ))
});
static TIME: Pattern = LazyLock::new(|| anchored(TIME_PART));

/// The patterns are constants; the conversion tests prove each one compiles.
fn anchored(pattern: &str) -> Option<Regex> {
    Regex::new(&format!(r"\A(?:{pattern})\z")).ok()
}

fn full_match(pattern: &Pattern, s: &str) -> bool {
    pattern.as_ref().is_some_and(|regex| regex.is_match(s))
}
