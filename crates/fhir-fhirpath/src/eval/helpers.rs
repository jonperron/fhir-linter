use std::sync::Arc;

use super::ArithOp;
use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value};

/// Check that a function received the expected number of arguments.
pub(super) fn check_arity(name: &str, args: &[Expr], expected: usize) -> Result<(), EvalError> {
    if args.len() != expected {
        return Err(EvalError::Arity {
            name: name.to_owned(),
            expected,
            got: args.len(),
        });
    }
    Ok(())
}

// ── Arithmetic ────────────────────────────────────────────────────────────────

pub(super) fn numeric_op(lv: &Value, rv: &Value, op: ArithOp) -> Result<Collection, EvalError> {
    if let (Value::String(a), Value::String(b), ArithOp::Add) = (lv, rv, &op) {
        return Ok(vec![Value::String(Arc::from(format!("{a}{b}")))]);
    }
    if let Some(result) = integer_op(lv, rv, &op)? {
        return Ok(vec![result]);
    }
    let a = lv
        .as_decimal()
        .ok_or_else(|| EvalError::Type("arithmetic requires numbers".into()))?;
    let b = rv
        .as_decimal()
        .ok_or_else(|| EvalError::Type("arithmetic requires numbers".into()))?;
    let result = match op {
        ArithOp::Add => a + b,
        ArithOp::Sub => a - b,
        ArithOp::Mul => a * b,
        // N1 §6.6: division by zero gives empty, not an error.
        ArithOp::Div | ArithOp::Mod if b == 0.0 => return Ok(vec![]),
        ArithOp::Div => a / b,
        ArithOp::Mod => a % b,
        ArithOp::DivInt => return Err(EvalError::Type("div is not a numeric_op".into())),
    };
    Ok(vec![Value::Decimal(result)])
}

/// Exact i64 arithmetic for Integer/Long operands; `None` when a Decimal is involved or for `/`.
fn integer_op(lv: &Value, rv: &Value, op: &ArithOp) -> Result<Option<Value>, EvalError> {
    let (a, b, is_long) = match (lv, rv) {
        (Value::Integer(a), Value::Integer(b)) => (*a, *b, false),
        (Value::Long(a), Value::Long(b))
        | (Value::Integer(a), Value::Long(b))
        | (Value::Long(a), Value::Integer(b)) => (*a, *b, true),
        _ => return Ok(None),
    };
    let result = match op {
        ArithOp::Add => a.checked_add(b),
        ArithOp::Sub => a.checked_sub(b),
        ArithOp::Mul => a.checked_mul(b),
        // Zero falls through to the decimal path, which gives empty.
        ArithOp::Mod if b == 0 => return Ok(None),
        ArithOp::Mod => a.checked_rem(b),
        _ => return Ok(None),
    }
    .ok_or_else(|| EvalError::Type("integer overflow".into()))?;
    Ok(Some(if is_long {
        Value::Long(result)
    } else {
        Value::Integer(result)
    }))
}

// ── Equality / comparison ─────────────────────────────────────────────────────

pub(super) fn values_equal(a: &Value, b: &Value) -> Option<bool> {
    match (a, b) {
        (Value::Null, _) | (_, Value::Null) => None,
        (Value::Bool(x), Value::Bool(y)) => Some(x == y),
        (Value::Integer(x), Value::Integer(y)) => Some(x == y),
        (Value::Long(x), Value::Long(y)) => Some(x == y),
        (Value::Long(x), Value::Integer(y)) | (Value::Integer(y), Value::Long(x)) => Some(x == y),
        (Value::Decimal(x), Value::Decimal(y)) => Some(x == y),
        (Value::Integer(x), Value::Decimal(y)) => Some(*x as f64 == *y),
        (Value::Decimal(x), Value::Integer(y)) => Some(*x == *y as f64),
        (Value::Long(x), Value::Decimal(y)) => Some(*x as f64 == *y),
        (Value::Decimal(x), Value::Long(y)) => Some(*x == *y as f64),
        (Value::String(x), Value::String(y)) => Some(x == y),
        (Value::Date(x), Value::Date(y))
        | (Value::DateTime(x), Value::DateTime(y))
        | (Value::Time(x), Value::Time(y)) => Some(x == y),
        (Value::Quantity(xv, xu), Value::Quantity(yv, yu)) => Some(xv == yv && xu == yu),
        _ => None,
    }
}

/// Membership using FHIRPath equality (`1 = 1.0`); complex values fall back to structural equality.
pub(super) fn contains_value(col: &[Value], v: &Value) -> bool {
    col.iter()
        .any(|x| values_equal(x, v).unwrap_or_else(|| x == v))
}

/// First occurrence of each value, in order, under FHIRPath equality.
pub(super) fn distinct(values: impl IntoIterator<Item = Value>) -> Collection {
    let mut seen = Vec::new();
    for v in values {
        if !contains_value(&seen, &v) {
            seen.push(v);
        }
    }
    seen
}

pub(super) fn values_equivalent(a: &Value, b: &Value) -> bool {
    // For ~: null ~ null is true, case-insensitive string comparison
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::String(x), Value::String(y)) => x.to_lowercase() == y.to_lowercase(),
        _ => values_equal(a, b).unwrap_or(false),
    }
}

pub(super) fn compare_values(a: &Value, b: &Value) -> Result<std::cmp::Ordering, EvalError> {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => Ok(x.cmp(y)),
        (Value::Long(x), Value::Long(y)) => Ok(x.cmp(y)),
        (Value::Long(x), Value::Integer(y)) => Ok(x.cmp(y)),
        (Value::Integer(x), Value::Long(y)) => Ok(x.cmp(y)),
        (Value::Decimal(x), Value::Decimal(y)) => {
            Ok(x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal))
        }
        (Value::Integer(x), Value::Decimal(y)) => Ok((*x as f64)
            .partial_cmp(y)
            .unwrap_or(std::cmp::Ordering::Equal)),
        (Value::Decimal(x), Value::Integer(y)) => Ok(x
            .partial_cmp(&(*y as f64))
            .unwrap_or(std::cmp::Ordering::Equal)),
        (Value::Long(x), Value::Decimal(y)) => Ok((*x as f64)
            .partial_cmp(y)
            .unwrap_or(std::cmp::Ordering::Equal)),
        (Value::Decimal(x), Value::Long(y)) => Ok(x
            .partial_cmp(&(*y as f64))
            .unwrap_or(std::cmp::Ordering::Equal)),
        (Value::String(x), Value::String(y)) => Ok(x.cmp(y)),
        (Value::Date(x), Value::Date(y))
        | (Value::DateTime(x), Value::DateTime(y))
        | (Value::Time(x), Value::Time(y)) => Ok(x.cmp(y)),
        (Value::Quantity(xv, xu), Value::Quantity(yv, yu)) => {
            if xu != yu {
                return Err(EvalError::Type(format!(
                    "cannot compare quantities with different units: '{xu}' vs '{yu}'"
                )));
            }
            Ok(xv.partial_cmp(yv).unwrap_or(std::cmp::Ordering::Equal))
        }
        (a, b) => Err(EvalError::Type(format!(
            "cannot compare {} and {}",
            a.type_name(),
            b.type_name()
        ))),
    }
}

// ── Boolean helpers ───────────────────────────────────────────────────────────

pub(super) fn collection_to_bool(col: &Collection) -> Option<bool> {
    match col.as_slice() {
        [] => None,
        [Value::Bool(b)] => Some(*b),
        _ => None,
    }
}

pub(super) fn bool_col(v: Option<bool>) -> Collection {
    match v {
        Some(b) => vec![Value::Bool(b)],
        None => vec![],
    }
}

pub(super) fn three_valued_and(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    }
}

pub(super) fn three_valued_or(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    }
}

// ── Type helpers ──────────────────────────────────────────────────────────────

pub(super) fn value_is_type(v: &Value, type_name: &str) -> bool {
    match type_name {
        "Boolean" | "boolean" | "bool" => matches!(v, Value::Bool(_)),
        "Integer" | "integer" => matches!(v, Value::Integer(_)),
        "Long" | "long" => matches!(v, Value::Long(_)),
        "Decimal" | "decimal" => matches!(v, Value::Decimal(_)),
        "String" | "string" => matches!(v, Value::String(_)),
        "Date" | "date" => matches!(v, Value::Date(_)),
        "DateTime" | "dateTime" => matches!(v, Value::DateTime(_)),
        "Time" | "time" => matches!(v, Value::Time(_)),
        "Quantity" | "quantity" => matches!(v, Value::Quantity(_, _)),
        // shortcut: resources match on their exact resourceType only (no Resource/DomainResource
        // inheritance, no complex datatypes), upgrade when the evaluator gets Registry access.
        _ => matches!(v, Value::Object(fields) if matches!(
            fields.get("resourceType").map(Vec::as_slice),
            Some([Value::String(rt)]) if **rt == *type_name
        )),
    }
}

/// Shared by the `is` operator and `is()` (N1 §6.3.1): empty in, empty out;
/// more than one item is an error.
pub(super) fn is_type(focus: &[Value], type_name: &str) -> Result<Collection, EvalError> {
    match focus {
        [] => Ok(vec![]),
        [v] => Ok(vec![Value::Bool(value_is_type(v, type_name))]),
        _ => Err(EvalError::Type(format!(
            "is requires a single-item input, got {}",
            focus.len()
        ))),
    }
}

// ── String helpers ────────────────────────────────────────────────────────────

pub(super) fn string_or_empty(col: &[Value]) -> Arc<str> {
    col.first()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| Arc::from(""))
}

pub(super) fn string_transform(
    focus: &[Value],
    f: impl Fn(&str) -> String,
) -> Result<Collection, EvalError> {
    match focus.first().and_then(|v| v.as_string()) {
        Some(s) => Ok(vec![Value::String(Arc::from(f(&s)))]),
        None => Ok(vec![]),
    }
}

/// Single-pass JSON string unescape (FHIRPath 3.0 `unescape('json')`).
pub(super) fn json_unescape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => result.push('\\'),
            Some('"') => result.push('"'),
            Some('/') => result.push('/'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }
    result
}

// ── Argument extraction ───────────────────────────────────────────────────────

pub(super) fn single_integer(col: &Collection, context: &str) -> Result<i64, EvalError> {
    match col.first() {
        Some(Value::Integer(n)) => Ok(*n),
        Some(v) => Err(EvalError::Type(format!(
            "{context}: expected Integer, got {}",
            v.type_name()
        ))),
        None => Err(EvalError::Type(format!(
            "{context}: expected Integer, got empty"
        ))),
    }
}

pub(super) fn eval_string_arg(col: &Collection, context: &str) -> Result<Arc<str>, EvalError> {
    match col.first().and_then(|v| v.as_string()) {
        Some(s) => Ok(s),
        None => Err(EvalError::Type(format!("{context}: expected String"))),
    }
}

pub(super) fn eval_type_name(expr: &Expr) -> Result<String, EvalError> {
    // For ofType/is/as(TypeName), the argument is an identifier, not evaluated
    match expr {
        Expr::Ident(name) => Ok(name.clone()),
        _ => Err(EvalError::Type(
            "type function argument must be a type name identifier".into(),
        )),
    }
}

// ── Tree traversal ────────────────────────────────────────────────────────────

pub(super) fn collect_descendants(v: &Value, out: &mut Collection) {
    // Iterative depth-first pre-order, so hostile deep input cannot overflow the stack.
    let mut stack: Vec<&Value> = vec![v];
    while let Some(current) = stack.pop() {
        if let Value::Object(fields) = current {
            let children: Vec<&Value> = fields.values().flatten().collect();
            for child in children.into_iter().rev() {
                stack.push(child);
            }
        }
        if !std::ptr::eq(current, v) {
            out.push(current.clone());
        }
    }
}

/// Check that a function received between `min` and `max` arguments (inclusive).
pub(super) fn check_arity_range(
    name: &str,
    args: &[Expr],
    min: usize,
    max: usize,
) -> Result<(), EvalError> {
    if args.len() < min || args.len() > max {
        return Err(EvalError::Arity {
            name: name.to_owned(),
            expected: if args.len() < min { min } else { max },
            got: args.len(),
        });
    }
    Ok(())
}
