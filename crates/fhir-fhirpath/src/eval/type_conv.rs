use std::sync::{Arc, LazyLock};

use regex_lite::Regex;

use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value, calendar_unit, is_calendar_date};

use super::Evaluator;
use super::helpers::{check_arity, check_arity_range, singleton};

impl Evaluator<'_> {
    /// Handle type conversion and type-checking functions.
    /// Returns `None` if the function name is not in this category.
    pub(super) fn call_type_function(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Option<Result<Collection, EvalError>> {
        self.try_type_conv(name, args, focus, ctx).transpose()
    }

    fn try_type_conv(
        &self,
        name: &str,
        args: &[Expr],
        focus: &[Value],
        ctx: &[Value],
    ) -> Result<Option<Collection>, EvalError> {
        let col: Collection = match name {
            // ── Integer ───────────────────────────────────────────────────
            "toInteger" => convert(name, args, focus, to_integer)?,
            "convertsToInteger" => converts(name, args, focus, to_integer)?,

            // ── Long ──────────────────────────────────────────────────────
            "toLong" => convert(name, args, focus, to_long)?,
            "convertsToLong" => converts(name, args, focus, to_long)?,

            // ── Decimal ───────────────────────────────────────────────────
            "toDecimal" => convert(name, args, focus, to_decimal)?,
            "convertsToDecimal" => converts(name, args, focus, to_decimal)?,

            // ── Boolean ───────────────────────────────────────────────────
            "toBoolean" => convert(name, args, focus, to_boolean)?,
            "convertsToBoolean" => converts(name, args, focus, to_boolean)?,

            // ── String ────────────────────────────────────────────────────
            "toString" => convert(name, args, focus, to_string)?,
            "convertsToString" => converts(name, args, focus, to_string)?,

            // ── Date / DateTime / Time ─────────────────────────────────────
            "toDate" => convert(name, args, focus, to_date)?,
            "convertsToDate" => converts(name, args, focus, to_date)?,
            "toDateTime" => convert(name, args, focus, to_date_time)?,
            "convertsToDateTime" => converts(name, args, focus, to_date_time)?,
            "toTime" => convert(name, args, focus, to_time)?,
            "convertsToTime" => converts(name, args, focus, to_time)?,

            // ── Quantity ──────────────────────────────────────────────────
            "toQuantity" | "convertsToQuantity" => {
                check_arity_range(name, args, 0, 1)?;
                // N1 §5.5.7: without UCUM conversion, only the input's own unit
                // converts; an empty unit argument gives empty (N1 §4.4.1).
                let unit = match args.first() {
                    Some(arg) => match singleton(&self.eval(arg, ctx)?, name)? {
                        None => return Ok(Some(vec![])),
                        Some(Value::String(unit)) => Some(Arc::clone(unit)),
                        Some(v) => {
                            return Err(EvalError::Type(format!(
                                "{name}() unit must be a String, got {}",
                                v.type_name()
                            )));
                        }
                    },
                    None => None,
                };
                let Some(item) = singleton(focus, name)? else {
                    return Ok(Some(vec![]));
                };
                let quantity = to_quantity(item).filter(|q| match (&unit, q) {
                    (Some(unit), Value::Quantity(_, own)) => {
                        calendar_unit(unit).unwrap_or(unit) == &**own
                    }
                    _ => true,
                });
                if name == "toQuantity" {
                    quantity.into_iter().collect()
                } else {
                    vec![Value::Bool(quantity.is_some())]
                }
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

/// toLong is not in N1 (FHIRPath 3.0 STU); it follows toInteger: Integer,
/// Long, Boolean, or a String in the form `(\+|-)?\d+` that fits in 64 bits.
fn to_long(item: &Value) -> Option<Value> {
    match item {
        Value::Integer(n) | Value::Long(n) => Some(Value::Long(*n)),
        Value::Bool(b) => Some(Value::Long(i64::from(*b))),
        Value::String(s) => s.parse().ok().map(Value::Long),
        _ => None,
    }
}

/// N1 §5.5.7: Integer or Decimal (unit `'1'`), Quantity, Boolean (`1.0 '1'`
/// or `0.0 '1'`), or a String matching `QUANTITY`. An unquoted unit must be
/// a calendar duration keyword, the only unquoted unit N1 §4.1.8 allows.
fn to_quantity(item: &Value) -> Option<Value> {
    let (value, unit) = match item {
        Value::Quantity(..) => return Some(item.clone()),
        Value::Integer(n) => (*n as f64, "1"),
        Value::Decimal(d) => (*d, "1"),
        Value::Bool(b) => (f64::from(u8::from(*b)), "1"),
        Value::String(s) => {
            let captures = QUANTITY.as_ref()?.captures(s)?;
            let unit = match (captures.get(3), captures.get(4)) {
                (Some(ucum), _) => ucum.as_str(),
                (None, Some(keyword)) => calendar_unit(keyword.as_str())?,
                (None, None) => "1",
            };
            (finite(captures.get(1)?.as_str())?, unit)
        }
        _ => return None,
    };
    Some(Value::Quantity(value, Arc::from(unit)))
}

/// N1 §5.5.8: the String form of a String, Integer, Decimal, Date, Time,
/// DateTime, Quantity or Boolean; anything else does not convert.
fn to_string(item: &Value) -> Option<Value> {
    let text = match item {
        Value::String(_) => return Some(item.clone()),
        Value::Bool(b) => b.to_string(),
        Value::Integer(n) | Value::Long(n) => n.to_string(),
        Value::Decimal(d) => decimal_text(*d),
        Value::Date(s) | Value::DateTime(s) | Value::Time(s) => s.to_string(),
        Value::Quantity(value, unit) => format!("{value} '{unit}'"),
        _ => return None,
    };
    Some(Value::String(Arc::from(text)))
}

/// A digit string as a Decimal; `None` beyond f64 instead of infinity.
fn finite(digits: &str) -> Option<f64> {
    digits.parse::<f64>().ok().filter(|d| d.is_finite())
}

/// A Decimal as `(\+|-)?\d+(.\d+)?`, always with a fraction so it stays
/// distinct from an Integer (`1.0.toString()` is `'1.0'`).
// shortcut: f64 keeps no literal precision, so `1.50` renders as `1.5`;
// upgrade with a real decimal type.
fn decimal_text(d: f64) -> String {
    let text = d.to_string();
    if text.contains('.') {
        text
    } else {
        format!("{text}.0")
    }
}

/// N1 §5.5.4: Integer, Decimal, Boolean, or a String in the form
/// `(\+|-)?\d+(\.\d+)?`.
fn to_decimal(item: &Value) -> Option<Value> {
    match item {
        Value::Decimal(d) => Some(Value::Decimal(*d)),
        Value::Integer(n) => Some(Value::Decimal(*n as f64)),
        Value::Bool(b) => Some(Value::Decimal(if *b { 1.0 } else { 0.0 })),
        Value::String(s) if full_match(&DECIMAL, s) => finite(s).map(Value::Decimal),
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

const DATE_PART: &str = r"\d{4}(-(0[1-9]|1[0-2])(-(0[1-9]|[12]\d|3[01]))?)?";
// Seconds up to 60 (leap second), as in the R5 time regex.
const TIME_PART: &str = r"([01]\d|2[0-3])(:[0-5]\d(:([0-5]\d|60)(\.\d+)?)?)?";

type Pattern = LazyLock<Option<Regex>>;

static DECIMAL: Pattern = LazyLock::new(|| anchored(r"[+-]?\d+(\.\d+)?"));
// N1 §5.5.7: `(?'value'(\+|-)?\d+(\.\d+)?)\s*('(?'unit'[^']+)'|(?'time'[a-zA-Z]+))?`
static QUANTITY: Pattern =
    LazyLock::new(|| anchored(r"([+-]?\d+(\.\d+)?)\s*(?:'([^']+)'|([a-zA-Z]+))?"));
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
