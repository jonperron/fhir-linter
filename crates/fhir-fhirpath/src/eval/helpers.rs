use std::sync::Arc;

use super::{ArithOp, Evaluator};
use crate::ast::Expr;
use crate::error::EvalError;
use crate::value::{Collection, Value, quantity_unit_factor};

/// Largest collection an evaluation may build. N1 sets no limit; this one
/// stops exponential growth (a `select($this.combine($this))` chain, or
/// `aggregate($total.combine($total))`) before it exhausts memory.
pub(super) const MAX_COLLECTION_LEN: usize = 100_000;

/// `CollectionTooLarge` once a collection passes [`MAX_COLLECTION_LEN`].
pub(super) fn check_collection_len(len: usize) -> Result<(), EvalError> {
    if len > MAX_COLLECTION_LEN {
        return Err(EvalError::CollectionTooLarge(MAX_COLLECTION_LEN));
    }
    Ok(())
}

/// Longest String, in characters, an evaluation may build: R5's string
/// maximum (1,048,576). Stops a `select($this + $this)` chain or
/// `aggregate($total + $total, 'a')` from doubling a string until memory runs out.
pub(super) const MAX_STRING_LEN: usize = 1_048_576;

/// `StringTooLong` once a length in characters passes [`MAX_STRING_LEN`].
pub(super) fn check_string_len(chars: usize) -> Result<(), EvalError> {
    if chars > MAX_STRING_LEN {
        return Err(EvalError::StringTooLong(MAX_STRING_LEN));
    }
    Ok(())
}

/// Most characters of String values one evaluation may build, in total.
/// [`MAX_STRING_LEN`] and [`MAX_COLLECTION_LEN`] alone still allow 100,000
/// strings of 1,048,576 characters. Freed strings are not given back: an
/// evaluation is short-lived, so the count bounds what it allocated.
/// Only text derived from the input is charged. Literals (at most
/// `MAX_INPUT_LEN` characters), `type()` names and Date, DateTime and Time
/// values have a fixed small size, and charging them on each item of a
/// `where` would skip real invariants on large collections.
pub(super) const MAX_EVAL_STRING_CHARS: usize = 16_777_216;

impl Evaluator<'_> {
    /// Count `chars` newly built characters against
    /// [`MAX_EVAL_STRING_CHARS`]; `StringBudgetExceeded` once past it.
    pub(super) fn charge_chars(&self, chars: usize) -> Result<(), EvalError> {
        let used = self.string_chars.get().saturating_add(chars);
        self.string_chars.set(used);
        if used > MAX_EVAL_STRING_CHARS {
            return Err(EvalError::StringBudgetExceeded(MAX_EVAL_STRING_CHARS));
        }
        Ok(())
    }

    /// Charge every String (and Quantity unit) a function just built into
    /// `col`. A text held elsewhere too (`toString()` of a String returns
    /// the same `Arc`) allocated nothing, so it is not charged.
    pub(super) fn charged(&self, col: Collection) -> Result<Collection, EvalError> {
        let chars = col
            .iter()
            .map(|v| match v {
                Value::String(s) | Value::Quantity(_, s) if Arc::strong_count(s) == 1 => {
                    s.chars().count()
                }
                _ => 0,
            })
            .fold(0, usize::saturating_add);
        self.charge_chars(chars)?;
        Ok(col)
    }

    /// A built String as a value: `StringTooLong` past [`MAX_STRING_LEN`],
    /// and charged against [`MAX_EVAL_STRING_CHARS`].
    pub(super) fn string_value(&self, s: &str) -> Result<Value, EvalError> {
        let chars = s.chars().count();
        check_string_len(chars)?;
        self.charge_chars(chars)?;
        Ok(Value::String(Arc::from(s)))
    }

    pub(super) fn string_transform(
        &self,
        focus: &[Value],
        name: &str,
        f: impl Fn(&str) -> String,
    ) -> Result<Collection, EvalError> {
        string_input(focus, name)?
            .map(|s| self.string_value(&f(&s)))
            .into_iter()
            .collect()
    }
}

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
    if let Some(result) = integer_op(lv, rv, &op)? {
        return Ok(vec![result]);
    }
    let temporal = lv.primitive_value();
    if matches!(
        temporal,
        Value::Date(_) | Value::DateTime(_) | Value::Time(_)
    ) && matches!(op, ArithOp::Add | ArithOp::Sub)
        && let Some((amount, unit)) = as_quantity(rv)
    {
        let amount = if matches!(op, ArithOp::Sub) {
            -amount
        } else {
            amount
        };
        let unit = unit.ok_or_else(|| EvalError::Type("a duration needs a unit".into()))?;
        return Ok(vec![super::temporal::add_duration(temporal, amount, unit)?]);
    }
    if matches!(op, ArithOp::Mul | ArithOp::Div)
        && let Some(result) = super::quantity::quantity_product(lv, rv, &op)
    {
        return Ok(result);
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
    // shortcut: N1 Decimal is exact; computing on the operands scaled to whole
    // numbers avoids binary error (`1.8 - 1.2`, `1.2 mod 0.1`) until Decimal
    // carries its own scale. Larger values keep the plain f64 result.
    if matches!(op, ArithOp::Add | ArithOp::Sub | ArithOp::Mod) {
        let factor = 10f64.powi(decimal_scale(a).max(decimal_scale(b)));
        let (x, y) = ((a * factor).round(), (b * factor).round());
        // Below 2^52 each, the scaled operands and their sum are exact integers.
        if x.abs() < 4.5e15 && y.abs() < 4.5e15 {
            let exact = match op {
                ArithOp::Add => x + y,
                ArithOp::Sub => x - y,
                _ => x % y,
            };
            // `+ 0.0` drops a -0.0.
            return Ok(vec![Value::Decimal(exact / factor + 0.0)]);
        }
    }
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

/// A FHIR Quantity Object as a `System.Quantity` value; `None` without a unit.
pub(super) fn object_quantity(v: &Value) -> Option<Value> {
    match as_quantity(v)? {
        (value, Some(unit)) => Some(Value::Quantity(value, Arc::from(unit))),
        _ => None,
    }
}

/// The value and unit of a Quantity literal, or of a FHIR Quantity whose
/// children are typed (R5 fhirpath.html §2.1.9.1: FHIR Quantity maps to
/// System.Quantity). The unit is the UCUM `code`, else the `unit` text, else
/// none. A non-UCUM code or a `comparator` is not a System.Quantity value.
/// shortcut: equality has no type hook to ask whether an Object's FHIR type
/// derives from Quantity, so a FHIR Quantity (or a descendant: Age, Duration...)
/// is recognised by its typed `value: decimal` and its Quantity element names;
/// upgrade when comparisons can reach the type hook.
pub(super) fn as_quantity(v: &Value) -> Option<(f64, Option<&str>)> {
    let fields = match v {
        Value::Quantity(value, unit) => return Some((*value, Some(unit))),
        Value::Object(fields, _) => fields,
        _ => return None,
    };
    let quantity_element = |key: &Arc<str>| {
        matches!(
            &**key,
            "value" | "unit" | "system" | "code" | "id" | "extension"
        )
    };
    if !fields.keys().all(quantity_element) {
        return None;
    }
    let text = |key: &str| match fields.get(key).map(Vec::as_slice) {
        Some([item]) => item.primitive_value().as_string(),
        _ => None,
    };
    let value = match fields.get("value").map(Vec::as_slice)? {
        [Value::Primitive(value, _, Some(ty))] if &**ty == "decimal" => match &**value {
            // XML primitives are text, typed by their element.
            Value::String(text) => text.parse().ok().filter(|v: &f64| v.is_finite())?,
            other => other.as_decimal()?,
        },
        _ => return None,
    };
    // An extension-only primitive has no value: treat it as absent.
    let field = |key: &str| {
        fields
            .get(key)
            .and_then(|items| items.first())
            .map(Value::primitive_value)
            .filter(|v| !matches!(v, Value::Null))
    };
    let unit = match field("code") {
        Some(_) if text("system").as_deref() != Some("http://unitsofmeasure.org") => return None,
        Some(code) => Some(code),
        None => field("unit"),
    };
    match unit {
        None => Some((value, None)),
        Some(Value::String(unit)) => Some((value, Some(unit))),
        Some(_) => None,
    }
}

/// N1 §6.1, §6.2 on two Quantities: `Some(Some((x, y, factor)))` when `b`'s
/// unit converts into `a`'s (`y * factor` is `b` in `a`'s unit), `Some(None)`
/// when they cannot be compared (no conversion, or a unit on one side only),
/// `None` when either is not a Quantity.
fn quantity_operands(a: &Value, b: &Value, equivalent: bool) -> Option<Option<(f64, f64, f64)>> {
    let ((x, x_unit), (y, y_unit)) = (as_quantity(a)?, as_quantity(b)?);
    let factor = match (x_unit, y_unit) {
        (None, None) => Some(1.0),
        (Some(x_unit), Some(y_unit)) => quantity_unit_factor(y_unit, x_unit, equivalent)
            .map(|ratio| ratio.numer() as f64 / ratio.denom() as f64),
        _ => None,
    };
    Some(factor.map(|factor| (x, y, factor)))
}

/// `y * factor`, snapped to `x` when they differ only by float noise.
/// shortcut: f64 conversion with a 1e-12 relative tolerance; replace with an
/// exact decimal comparison once Quantity values are decimals.
fn convert_amount(x: f64, y: f64, factor: f64) -> f64 {
    if factor == 1.0 {
        return y;
    }
    let converted = y * factor;
    let close =
        converted.is_finite() && (converted - x).abs() <= 1e-12 * x.abs().max(converted.abs());
    if close { x } else { converted }
}

/// [`quantity_operands`] with `b` converted into `a`'s unit: `Some(Some((x, y)))`
/// when comparable, `Some(None)` when not, `None` when either is not a Quantity.
pub(super) fn quantity_values(
    a: &Value,
    b: &Value,
    equivalent: bool,
) -> Option<Option<(f64, f64)>> {
    let operands = quantity_operands(a, b, equivalent)?;
    // An overflowing conversion has no comparable value: empty, not equal.
    Some(operands.and_then(|(x, y, factor)| {
        Some(convert_amount(x, y, factor))
            .filter(|c| c.is_finite())
            .map(|c| (x, c))
    }))
}

/// N1 §6.3.1 `~` on two comparable Quantities: equal once both are rounded
/// to the precision of the least precise one, measured in `x`'s unit so the
/// result does not depend on the operand order (`4 'g' ~ 4040 'mg'`).
/// shortcut: f64 rounding, so float noise can flip a value lying on a .5
/// boundary; upgrade with the decimal type.
fn quantities_equivalent(x: f64, y: f64, factor: f64) -> bool {
    let step = |value: f64| 10f64.powi(-decimal_scale(value));
    let step = step(x).max(step(y) * factor.abs());
    (x / step).round() == (convert_amount(x, y, factor) / step).round()
}

pub(super) fn values_equal(a: &Value, b: &Value) -> Option<bool> {
    let (a, b) = (a.primitive_value(), b.primitive_value());
    if let Some(values) = quantity_values(a, b, false) {
        return values.map(|(x, y)| x == y);
    }
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
        // N1 §6.1.1: complex values are equal when all their properties are.
        (Value::Object(x, _), Value::Object(y, _)) => objects_match(x, y, false),
        // A complex value never equals a primitive. Without type knowledge an XML
        // primitive with extensions is still an Object; see `resource_to_typed_value`.
        (Value::Object(..), _) | (_, Value::Object(..)) => Some(false),
        // N2 §6.1.1 "Otherwise, equals returns false": a Date or DateTime
        // never converts to a Time (testDateNotEqualTimeMinute).
        (Value::Date(_) | Value::DateTime(_), Value::Time(_))
        | (Value::Time(_), Value::Date(_) | Value::DateTime(_)) => Some(false),
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
    if let Some(values) = quantity_values(a, b, false) {
        return Ok(values.map(|(x, y)| x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)));
    }
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

/// `distinct()`: the last occurrence of each value, under FHIRPath equality.
///
/// N1 leaves the order of `distinct()` open; the official suite keeps the last
/// occurrence. Returns `None` when two date/times cannot be compared (different
/// precision, N1 §6.1.1), so the caller yields empty.
pub(super) fn distinct_last(focus: &[Value]) -> Option<Collection> {
    let mut kept = Vec::new();
    for (index, value) in focus.iter().enumerate() {
        let mut repeated = false;
        for later in &focus[index + 1..] {
            // Same fallback as `contains_value`: complex values compare structurally.
            repeated = match values_equal(value, later) {
                Some(equal) => equal,
                None if is_temporal_pair(value, later) => return None,
                None => value == later,
            };
            if repeated {
                break;
            }
        }
        if !repeated {
            kept.push(value.clone());
        }
    }
    Some(kept)
}

/// `isDistinct()`: `false` at the first repeated value; `None` (empty) when no
/// value repeats but two date/times cannot be compared.
pub(super) fn is_distinct(focus: &[Value]) -> Option<bool> {
    let mut uncomparable = false;
    for (index, value) in focus.iter().enumerate() {
        for later in &focus[index + 1..] {
            match values_equal(value, later) {
                Some(true) => return Some(false),
                Some(false) => {}
                None if is_temporal_pair(value, later) => uncomparable = true,
                None if value == later => return Some(false),
                None => {}
            }
        }
    }
    (!uncomparable).then_some(true)
}

fn is_temporal_pair(a: &Value, b: &Value) -> bool {
    matches!(
        (a.primitive_value(), b.primitive_value()),
        (
            Value::Date(_) | Value::DateTime(_),
            Value::Date(_) | Value::DateTime(_),
        ) | (Value::Time(_), Value::Time(_))
    )
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
    if let Some(operands) = quantity_operands(a, b, true) {
        return operands.is_some_and(|(x, y, factor)| quantities_equivalent(x, y, factor));
    }
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
        (Value::Object(x, _), Value::Object(y, _)) => objects_match(x, y, true) == Some(true),
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
        (lhs @ Value::Quantity(_, xu), rhs @ Value::Quantity(_, yu)) => {
            match quantity_values(lhs, rhs, false) {
                Some(Some((x, y))) => Ok(x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)),
                _ => Err(EvalError::Type(format!(
                    "cannot compare quantities with different units: '{xu}' vs '{yu}'"
                ))),
            }
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

/// R5 primitive types other than `string` whose `value` is `System.String`
/// (`profiles-types.json`; checked by `tests/string_subtype_tests.rs`).
const FHIR_STRING_SUBTYPES: [&str; 12] = [
    "base64Binary",
    "canonical",
    "code",
    "id",
    "markdown",
    "oid",
    "positiveInt",
    "unsignedInt",
    "uri",
    "url",
    "uuid",
    "xhtml",
];

/// Whether `v` has the type `type_name`. A value that carries its FHIR type
/// answers through FHIR inheritance when `is_subtype` is set and `type_name`
/// is a FHIR type. Otherwise the System types apply, and asking a String for
/// one of [`FHIR_STRING_SUBTYPES`] is an error rather than a guessed `false`.
pub(super) fn value_is_type(
    v: &Value,
    type_name: &str,
    is_subtype: Option<&super::IsSubtype<'_>>,
) -> Result<bool, EvalError> {
    // FHIR data (typed primitives, objects, resolve() results) is never a
    // System value, and a System value (a literal, a function result) never
    // a FHIR one.
    let is_fhir_data = matches!(
        v,
        Value::Primitive(..) | Value::Object(..) | Value::UnresolvedResource { .. }
    );
    match type_name.split_once('.') {
        // Every type, FHIR ones included, derives from System.Any.
        Some(("System", "Any")) => return Ok(true),
        Some(("System", name)) => {
            // System type names are capitalised (`System.Boolean`, not `boolean`).
            return Ok(!is_fhir_data
                && name.starts_with(|c: char| c.is_ascii_uppercase())
                && system_is_type(v, name) == Some(true));
        }
        Some(("FHIR", name)) => {
            if is_subtype.is_some_and(|is_subtype| !is_subtype(name, name)) {
                return Err(EvalError::Type(format!("unknown type FHIR.{name}")));
            }
            return Ok(is_fhir_data && value_is_type(v, name, is_subtype)?);
        }
        _ => {}
    }
    // N1 §10.1: the FHIR model is searched first, then System, so with the
    // type hook an unqualified System name that is no FHIR type is System's.
    if let Some(is_subtype) = is_subtype
        && !is_subtype(type_name, type_name)
        && type_name.starts_with(|c: char| c.is_ascii_uppercase())
        && (type_name == "Any" || system_is_type(&Value::Null, type_name).is_some())
    {
        return value_is_type(v, &format!("System.{type_name}"), None);
    }
    if let (
        Value::Primitive(_, _, Some(fhir_type)) | Value::Object(_, Some(fhir_type)),
        Some(is_subtype),
    ) = (v, is_subtype)
        && is_subtype(type_name, type_name)
    {
        return Ok(is_subtype(fhir_type, type_name));
    }
    if let (
        Value::UnresolvedResource {
            resource_type,
            reference,
        },
        Some(is_subtype),
    ) = (v, is_subtype)
        && is_subtype(type_name, type_name)
    {
        // `Resource/1` may be any resource: whether it is a Patient is unknown.
        if type_name != &**resource_type && is_subtype(type_name, resource_type) {
            return Err(EvalError::Type(format!(
                "cannot tell whether '{reference}' is a {type_name}",
            )));
        }
        return Ok(is_subtype(resource_type, type_name));
    }
    // Without the type hook, an object is only its own FHIR type (N1 §10.1:
    // the model is searched before System, so a FHIR Quantity is a Quantity).
    if let Value::Object(_, Some(fhir_type)) = v
        && **fhir_type == *type_name
    {
        return Ok(true);
    }
    let v = v.primitive_value();
    if matches!(v, Value::String(_)) && FHIR_STRING_SUBTYPES.contains(&type_name) {
        return Err(EvalError::Type(format!(
            "the FHIR type of a String is unknown, cannot test it for {type_name}"
        )));
    }
    if let Some(system) = system_is_type(v, type_name) {
        return Ok(system);
    }
    // An unqualified name that is neither a System nor a FHIR type is an
    // error; names in another namespace keep the old answer.
    if is_subtype.is_some_and(|is_subtype| {
        !type_name.contains('.') && type_name != "Any" && !is_subtype(type_name, type_name)
    }) {
        return Err(EvalError::Type(format!("unknown type {type_name}")));
    }
    Ok(false)
}

/// Whether the plain value `v` has the System type `type_name` (or its FHIR
/// primitive spelling), `None` when `type_name` names no System type.
fn system_is_type(v: &Value, type_name: &str) -> Option<bool> {
    Some(match type_name {
        "Boolean" | "boolean" | "bool" => matches!(v, Value::Bool(_)),
        "Integer" | "integer" => matches!(v, Value::Integer(_)),
        "Long" | "long" => matches!(v, Value::Long(_)),
        "Decimal" | "decimal" => matches!(v, Value::Decimal(_)),
        "String" | "string" => matches!(v, Value::String(_)),
        "Date" | "date" => matches!(v, Value::Date(_)),
        "DateTime" | "dateTime" => matches!(v, Value::DateTime(_)),
        "Time" | "time" => matches!(v, Value::Time(_)),
        "Quantity" | "quantity" => matches!(v, Value::Quantity(_, _)),
        _ => return None,
    })
}

/// `type()` (N1 §10.2): the item's type as an object with `namespace` and
/// `name`. FHIR data answers in the FHIR namespace, plain values in System.
/// shortcut: no `baseType` and no ClassInfo `element`, as the type hook only
/// answers subtype questions; upgrade when the environment exposes the
/// Registry's type definitions.
pub(super) fn type_info(v: &Value) -> Result<Value, EvalError> {
    let (namespace, name): (&str, Arc<str>) = match v {
        Value::Primitive(_, _, Some(fhir_type)) => ("FHIR", Arc::clone(fhir_type)),
        Value::UnresolvedResource { resource_type, .. } => ("FHIR", Arc::clone(resource_type)),
        Value::Object(_, Some(fhir_type)) => ("FHIR", Arc::clone(fhir_type)),
        Value::Object(_, None) => {
            return Err(EvalError::Type(
                "the FHIR type of an object is unknown".into(),
            ));
        }
        Value::Primitive(_, _, None) | Value::Null => {
            return Err(EvalError::Type(
                "the FHIR type of a primitive is unknown".into(),
            ));
        }
        plain => ("System", Arc::from(plain.type_name())),
    };
    Ok(Value::Object(
        Arc::new(
            [
                (
                    Arc::from("namespace"),
                    vec![Value::String(Arc::from(namespace))],
                ),
                (Arc::from("name"), vec![Value::String(name)]),
            ]
            .into_iter()
            .collect(),
        ),
        None,
    ))
}

/// Fail on an unknown type name before looking at the input, so an empty
/// input does not hide it (`Null` matches no type, it only runs the check).
pub(super) fn check_type_name(
    type_name: &str,
    is_subtype: Option<&super::IsSubtype<'_>>,
) -> Result<(), EvalError> {
    value_is_type(&Value::Null, type_name, is_subtype).map(drop)
}

/// Shared by the `is` operator and `is()` (N1 §6.3.1): empty in, empty out;
/// more than one item is an error.
pub(super) fn is_type(
    focus: &[Value],
    type_name: &str,
    is_subtype: Option<&super::IsSubtype<'_>>,
) -> Result<Collection, EvalError> {
    check_type_name(type_name, is_subtype)?;
    match focus {
        [] => Ok(vec![]),
        [v] => Ok(vec![Value::Bool(value_is_type(v, type_name, is_subtype)?)]),
        _ => Err(EvalError::Type(format!(
            "is requires a single-item input, got {}",
            focus.len()
        ))),
    }
}

/// Shared by the `as` operator and `as()` (N1 §6.3.2): the item when it has
/// the type, else empty; more than one item is an error.
pub(super) fn as_type(
    focus: &[Value],
    type_name: &str,
    is_subtype: Option<&super::IsSubtype<'_>>,
) -> Result<Collection, EvalError> {
    check_type_name(type_name, is_subtype)?;
    match focus {
        [] => Ok(vec![]),
        [v] => Ok(value_is_type(v, type_name, is_subtype)?
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

/// N1 §5.6 input of a string function: `None` when empty; several items or
/// an item that is not a String are an error.
pub(super) fn string_input(focus: &[Value], name: &str) -> Result<Option<Arc<str>>, EvalError> {
    match singleton(focus, name)? {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(Arc::clone(s))),
        Some(v) => Err(EvalError::Type(format!(
            "{name}() requires a String input, got {}",
            v.type_name()
        ))),
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
            Some('u') => {
                let hex = read_hex4(&mut chars);
                let unit = (hex.len() == 4)
                    .then(|| u32::from_str_radix(&hex, 16).ok())
                    .flatten();
                let decoded = match unit {
                    // A high surrogate joins the `\uDC00`-`\uDFFF` escape after it.
                    Some(high @ 0xD800..=0xDBFF) => {
                        let mut ahead = chars.clone();
                        let low = (ahead.next() == Some('\\') && ahead.next() == Some('u'))
                            .then(|| u32::from_str_radix(&read_hex4(&mut ahead), 16).ok())
                            .flatten()
                            .filter(|low| (0xDC00..=0xDFFF).contains(low));
                        low.and_then(|low| {
                            chars = ahead;
                            char::from_u32(0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00))
                        })
                    }
                    unit => unit.and_then(char::from_u32),
                };
                match decoded {
                    Some(c) => result.push(c),
                    None => {
                        result.push_str("\\u");
                        result.push_str(&hex);
                    }
                }
            }
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }
    result
}

/// Up to 4 hex digits of a `\u` escape; stops at the first other character.
fn read_hex4(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut hex = String::with_capacity(4);
    while hex.len() < 4
        && let Some(digit) = chars.next_if(char::is_ascii_hexdigit)
    {
        hex.push(digit);
    }
    hex
}

// ── Argument extraction ───────────────────────────────────────────────────────

pub(super) fn single_integer(col: &Collection, context: &str) -> Result<i64, EvalError> {
    optional_integer(col, context)?
        .ok_or_else(|| EvalError::Type(format!("{context}: expected Integer, got empty")))
}

/// An Integer argument that may be empty (N1 §4.4.1); several items or a
/// non-Integer item are an error (N1 §5).
pub(super) fn optional_integer(col: &[Value], context: &str) -> Result<Option<i64>, EvalError> {
    match singleton(col, context)? {
        None => Ok(None),
        Some(Value::Integer(n)) => Ok(Some(*n)),
        Some(v) => Err(EvalError::Type(format!(
            "{context}: expected Integer, got {}",
            v.type_name()
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
    // For ofType/is/as(TypeName), the argument is a type specifier, not evaluated:
    // an identifier or a qualified name (`System.Integer`, `FHIR.Patient`).
    match expr {
        Expr::Ident(name) => Ok(name.clone()),
        Expr::Dot(namespace, name) => Ok(format!(
            "{}.{}",
            eval_type_name(namespace)?,
            eval_type_name(name)?
        )),
        _ => Err(EvalError::Type(
            "type function argument must be a type name identifier".into(),
        )),
    }
}

// ── Tree traversal ────────────────────────────────────────────────────────────

/// An error when an item is a [`Value::UnresolvedResource`]: its elements are
/// unknown, so reading or comparing them must not look like empty.
pub(super) fn check_known_content(items: &[Value]) -> Result<(), EvalError> {
    match items.iter().find_map(|item| match item {
        Value::UnresolvedResource { reference, .. } => Some(reference),
        _ => None,
    }) {
        Some(reference) => Err(EvalError::Type(format!(
            "the content of '{reference}' is unknown: resolve() only knows its type",
        ))),
        None => Ok(()),
    }
}

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
