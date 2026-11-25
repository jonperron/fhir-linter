//! `minValue[x]` and `maxValue[x]` of profile elements (elementdefinition.html): inclusive
//! bounds on the value, reported only where the order is decided.
use std::cmp::Ordering;

use fhir_definitions::{ChoiceValue, ElementDefinition};
use fhir_parser::{Decimal, Node, Value};
use serde_json::Value as Json;

use crate::diagnostic::Diagnostic;

/// Check the occurrence `item` (instance `path`) against the `minValue[x]` and `maxValue[x]`
/// of `elem`: a value outside them is `PROFILE_011`.
pub(crate) fn check(
    item: &Node,
    elem: &ElementDefinition,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let bounds = [
        (
            elem.min_value.as_deref(),
            Ordering::Less,
            "below the minimum",
        ),
        (
            elem.max_value.as_deref(),
            Ordering::Greater,
            "above the maximum",
        ),
    ];
    for (bound, outside, rule) in bounds {
        let Some(bound) = bound.filter(|b| order(&item.value, b) == Some(outside)) else {
            continue;
        };
        diagnostics.push(
            Diagnostic::error(
                "PROFILE_011",
                format!(
                    "Value at '{path}' is {rule} {} {} required by '{}'",
                    bound.key, bound.value, elem.path
                ),
                path.to_owned(),
            )
            .with_location(item.span),
        );
    }
}

/// Order of `value` against `bound`; `None` when it is undecided or the two do not compare.
fn order(value: &Value, bound: &ChoiceValue) -> Option<Ordering> {
    let bound_type = bound
        .key
        .strip_prefix("minValue")
        .or_else(|| bound.key.strip_prefix("maxValue"))?;
    match (value, &bound.value) {
        (Value::Integer(_) | Value::Decimal(_), Json::Number(_)) => {
            Some(number(value)?.cmp(&json_number(&bound.value)?))
        }
        // json.html: integer64 is a JSON string.
        (Value::Str(v), Json::String(b)) if bound_type == "Integer64" => {
            Some(v.parse::<i64>().ok()?.cmp(&b.parse::<i64>().ok()?))
        }
        // FHIRPath N1 §6.1.1 order: undecided (`None`) when the precisions differ and the
        // value overlaps the bound, as in 2000 against 2000-06-15.
        (Value::Str(v), Json::String(b)) => {
            fhir_fhirpath::compare_temporal(v, b, bound_type == "Time")
        }
        (Value::Object(_), Json::Object(_)) => quantity_order(value, &bound.value),
        _ => None,
    }
}

/// Order of two Quantities with the same `system` and `code`, neither with a `comparator`
/// (`< 5` is a range, not a value); a side without `system` or `code` has no comparable unit.
// shortcut: units are compared as written, never converted (9000 g against 10 kg is
// undecided); upgrade by exposing the FHIRPath UCUM factors (`quantity_unit_factor`).
fn quantity_order(value: &Value, bound: &Json) -> Option<Ordering> {
    let Value::Object(fields) = value else {
        return None;
    };
    let field = |key: &str| fields.get(key).map(|node| &node.value);
    if field("comparator").is_some() || bound.get("comparator").is_some() {
        return None;
    }
    let same = |key: &str| {
        matches!(
            (field(key), bound.get(key)),
            (Some(Value::Str(a)), Some(Json::String(b))) if **a == **b
        )
    };
    if !same("system") || !same("code") {
        return None;
    }
    Some(number(field("value")?)?.cmp(&json_number(bound.get("value")?)?))
}

/// The instance number `value`.
fn number(value: &Value) -> Option<Decimal> {
    match value {
        Value::Integer(i) => Some(Decimal::from(*i)),
        Value::Decimal(d) => Some(*d),
        _ => None,
    }
}

/// The definition number `value`, from its text (serde_json `arbitrary_precision`).
fn json_number(value: &Json) -> Option<Decimal> {
    value.as_number()?.to_string().parse().ok()
}
