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
    let (a, b) = (a.primitive_value(), b.primitive_value());
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
        (Value::Quantity(xv, xu), Value::Quantity(yv, yu)) => Some(xv == yv && xu == yu),
        // N1 §6.1.1: complex values are equal when all their properties are.
        (Value::Object(x), Value::Object(y)) => objects_match(x, y, false),
        // A complex value never equals a primitive. Without type knowledge an XML
        // primitive with extensions is still an Object; see `resource_to_typed_value`.
        (Value::Object(_), _) | (_, Value::Object(_)) => Some(false),
        _ => temporal_order(a, b).map(|order| order.is_eq()),
    }
}

/// Order for `<`, `<=`, `>`, `>=`: `None` (empty) when date/time precision
/// or zones make it unknown.
pub(super) fn operator_order(
    a: &Value,
    b: &Value,
) -> Result<Option<std::cmp::Ordering>, EvalError> {
    let (a, b) = (a.primitive_value(), b.primitive_value());
    match (a, b) {
        (Value::Date(_) | Value::DateTime(_), Value::Date(_) | Value::DateTime(_))
        | (Value::Time(_), Value::Time(_)) => Ok(temporal_order(a, b)),
        _ => compare_values(a, b).map(Some),
    }
}

/// N1 §6.1.1 `=` on collections: same length and equal item by item, in
/// order; empty when an item comparison is.
pub(super) fn collections_equal(a: &[Value], b: &[Value]) -> Option<bool> {
    if a.len() != b.len() {
        return Some(false);
    }
    let mut result = Some(true);
    for (x, y) in a.iter().zip(b) {
        match values_equal(x, y) {
            Some(false) => return Some(false),
            None => result = None,
            Some(true) => {}
        }
    }
    result
}

/// N1 §6.1.3 `~` on collections: same length and every item equivalent to
/// a distinct item of the other side, in any order. Two empties are
/// equivalent.
pub(super) fn collections_equivalent(a: &[Value], b: &[Value]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut unmatched: Vec<&Value> = b.iter().collect();
    a.iter().all(
        |x| match unmatched.iter().position(|y| values_equivalent(x, y)) {
            Some(i) => {
                unmatched.swap_remove(i);
                true
            }
            None => false,
        },
    )
}

/// Property-wise `=` (or `~`, which skips `id` per the R5 FHIRPath page).
fn objects_match(
    x: &indexmap::IndexMap<Arc<str>, Collection>,
    y: &indexmap::IndexMap<Arc<str>, Collection>,
    equivalent: bool,
) -> Option<bool> {
    let compared = |key: &Arc<str>| !(equivalent && &**key == "id");
    let x_keys = x.keys().filter(|k| compared(k)).count();
    let y_keys = y.keys().filter(|k| compared(k)).count();
    if x_keys != y_keys {
        return Some(false);
    }
    let mut result = Some(true);
    for (key, x_values) in x.iter().filter(|(k, _)| compared(k)) {
        let Some(y_values) = y.get(key) else {
            return Some(false);
        };
        let same = if equivalent {
            Some(collections_equivalent(x_values, y_values))
        } else {
            collections_equal(x_values, y_values)
        };
        match same {
            Some(false) => return Some(false),
            None => result = None,
            Some(true) => {}
        }
    }
    result
}

/// Order of two temporal values of a comparable kind (Date with DateTime,
/// Time with Time); `None` when precision or zones make it unknown.
fn temporal_order(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Date(x) | Value::DateTime(x), Value::Date(y) | Value::DateTime(y)) => {
            super::temporal::compare(x, y, false)
        }
        (Value::Time(x), Value::Time(y)) => super::temporal::compare(x, y, true),
        _ => None,
    }
}

/// Number of digits after the decimal point in the shortest form of `d`.
fn decimal_scale(d: f64) -> i32 {
    let text = d.to_string();
    text.split_once('.')
        .map_or(0, |(_, fraction)| fraction.len() as i32)
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

/// N1 §6.1.3 `~` on single items; never empty.
pub(super) fn values_equivalent(a: &Value, b: &Value) -> bool {
    let (a, b) = (a.primitive_value(), b.primitive_value());
    // N1: "all whitespace characters are treated as equivalent"; runs are not collapsed.
    let normalise = |s: &str| {
        s.chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect::<String>()
            .to_lowercase()
    };
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::String(x), Value::String(y)) => normalise(x) == normalise(y),
        // Decimals are rounded to the precision of the least precise operand.
        // shortcut: f64 has no literal precision, so the shortest decimal form
        // stands in for it; upgrade with a real decimal type.
        (Value::Decimal(_), _) | (_, Value::Decimal(_)) => match (a.as_decimal(), b.as_decimal()) {
            (Some(x), Some(y)) => {
                let scale = 10f64.powi(decimal_scale(x).min(decimal_scale(y)));
                (x * scale).round() == (y * scale).round()
            }
            _ => false,
        },
        (Value::Object(x), Value::Object(y)) => objects_match(x, y, true) == Some(true),
        _ => values_equal(a, b).unwrap_or(false),
    }
}

pub(super) fn compare_values(a: &Value, b: &Value) -> Result<std::cmp::Ordering, EvalError> {
    match (a.primitive_value(), b.primitive_value()) {
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
        // shortcut: sort/min/max order date-times by text (total, but zones are not
        // normalised); operators use `operator_order`. Upgrade when sort needs zones.
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

/// N1 §4.5 singleton evaluation: the single item of `col`, `None` when empty,
/// an error when there are several.
/// A primitive gives its value, and one without a value counts as empty.
pub(super) fn singleton<'a>(col: &'a [Value], what: &str) -> Result<Option<&'a Value>, EvalError> {
    match col {
        [] => Ok(None),
        [v] => Ok(Some(v.primitive_value()).filter(|v| !matches!(v, Value::Null))),
        _ => Err(EvalError::Type(format!(
            "{what} requires a single item, got {}",
            col.len()
        ))),
    }
}

/// N1 §4.5: where a Boolean is expected, one Boolean is itself, any other
/// single item is true, empty is empty and several items are an error.
pub(super) fn singleton_bool(col: &[Value], what: &str) -> Result<Option<bool>, EvalError> {
    Ok(singleton(col, what)?.map(|v| !matches!(v, Value::Bool(false))))
}

/// N1 §5.2.1, §5.5.1: criteria (where, exists, all, iif) must give one
/// Boolean or empty; anything else is an error.
pub(super) fn criteria_bool(col: &[Value], what: &str) -> Result<Option<bool>, EvalError> {
    match singleton(col, what)? {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(v) => Err(EvalError::Type(format!(
            "{what} criteria must be Boolean, got {}",
            v.type_name()
        ))),
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

pub(super) fn three_valued_implies(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(false), _) | (_, Some(true)) => Some(true),
        (Some(true), b) => b,
        (None, _) => None,
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
    let v = v.primitive_value();
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

/// Shared by the `as` operator and `as()` (N1 §6.3.2): the item when it has
/// the type, else empty; more than one item is an error.
pub(super) fn as_type(focus: &[Value], type_name: &str) -> Result<Collection, EvalError> {
    match focus {
        [] => Ok(vec![]),
        [v] => Ok(value_is_type(v, type_name)
            .then(|| v.clone())
            .into_iter()
            .collect()),
        _ => Err(EvalError::Type(format!(
            "as requires a single-item input, got {}",
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
    match col.first().map(Value::primitive_value) {
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
        let children: Vec<&Value> = current.child_elements().collect();
        stack.extend(children.into_iter().rev());
        if !std::ptr::eq(current, v) {
            out.push(current.clone());
        }
    }
}

/// Each item's primitive value (see [`Value::primitive_value`]), dropping
/// primitives that have none; borrowed when no item is a primitive element.
pub(super) fn primitive_values(focus: &[Value]) -> std::borrow::Cow<'_, [Value]> {
    if !focus.iter().any(|v| matches!(v, Value::Primitive(..))) {
        return std::borrow::Cow::Borrowed(focus);
    }
    std::borrow::Cow::Owned(
        focus
            .iter()
            .map(Value::primitive_value)
            .filter(|v| !matches!(v, Value::Null))
            .cloned()
            .collect(),
    )
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
