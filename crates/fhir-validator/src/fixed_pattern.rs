//! `fixed[x]` and `pattern[x]` of profile elements (elementdefinition.html), also used to
//! match `value`/`pattern` slice discriminators.
use fhir_definitions::ElementDefinition;
use fhir_fhirpath::Value as FhirPathValue;
use fhir_parser::{Decimal, Node, Value};
use serde_json::Value as Json;

use crate::diagnostic::Diagnostic;

/// The non-null occurrences of `node` (instance `path`): the items of an array, or the node.
pub(crate) fn occurrences<'a>(node: &'a Node, path: &str) -> Vec<(&'a Node, String)> {
    match &node.value {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .filter(|(_, item)| !matches!(item.value, Value::Null))
            .map(|(i, item)| (item, format!("{path}[{i}]")))
            .collect(),
        Value::Null => Vec::new(),
        _ => vec![(node, path.to_owned())],
    }
}

/// Check each occurrence of `node` (instance `path`) against the `fixed[x]` of `elem`, which
/// it SHALL be exactly (`PROFILE_004`), and its `pattern[x]`, which it SHALL follow
/// (`PROFILE_005`).
pub(crate) fn check_value(
    node: &Node,
    elem: &ElementDefinition,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if elem.fixed.is_none() && elem.pattern.is_none() {
        return;
    }
    for (item, item_path) in occurrences(node, path) {
        let values = fhir_fhirpath::from_parser_value(&item.value);
        if let Some(fixed) = elem.fixed.as_deref() {
            let mut parts = Vec::new();
            fixed_differences(&values, &fixed.value, &item_path, &mut parts);
            for (part_path, expected) in parts {
                let rule = match expected {
                    Some(expected) => format!("must be exactly {expected}"),
                    None => "must be absent".to_owned(),
                };
                diagnostics.push(
                    Diagnostic::error(
                        "PROFILE_004",
                        format!(
                            "Value at '{part_path}' {rule}: the {} {} required by '{}'",
                            fixed.key, fixed.value, elem.path
                        ),
                        part_path,
                    )
                    .with_location(item.span),
                );
            }
        }
        if let Some(pattern) = elem
            .pattern
            .as_deref()
            .filter(|p| !matches_all(&values, &p.value, false))
        {
            diagnostics.push(
                Diagnostic::error(
                    "PROFILE_005",
                    format!(
                        "Value at '{item_path}' does not match the {} {} required by '{}'",
                        pattern.key, pattern.value, elem.path
                    ),
                    item_path.clone(),
                )
                .with_location(item.span),
            );
        }
    }
}

/// The parts of `values` (instance `path`) that are not exactly `expected`, each with what
/// was expected there (none: the part must be absent). fixed[x]: "all values must be an
/// exact match [...] Missing elements/attributes must also be missing", so objects compare
/// property by property and each wrong, missing or extra property is its own part;
/// repetitions compare item by item, and a different number of them is one part on the
/// collection.
fn fixed_differences(
    values: &[FhirPathValue],
    expected: &Json,
    path: &str,
    parts: &mut Vec<(String, Option<String>)>,
) {
    match (values, expected) {
        (values, Json::Array(items)) if values.len() == items.len() => {
            for (index, (value, item)) in values.iter().zip(items).enumerate() {
                fixed_differences(
                    std::slice::from_ref(value),
                    item,
                    &format!("{path}[{index}]"),
                    parts,
                );
            }
        }
        ([FhirPathValue::Object(fields, _)], Json::Object(map)) => {
            for (key, item) in map {
                let child = format!("{path}.{key}");
                match fields.get(key.as_str()) {
                    Some(values) => fixed_differences(values, item, &child, parts),
                    None => parts.push((child, Some(item.to_string()))),
                }
            }
            for key in fields.keys().filter(|key| !map.contains_key(&***key)) {
                parts.push((format!("{path}.{key}"), None));
            }
        }
        (values, expected) if matches_all(values, expected, true) => {}
        _ => parts.push((path.to_owned(), Some(expected.to_string()))),
    }
}

/// Whether the collection `values` equals `expected` (`exact`: arrays item by item) or
/// follows it as a pattern (each pattern array item matching some value). A single expected
/// value needs a single value.
pub(crate) fn matches_all(values: &[FhirPathValue], expected: &Json, exact: bool) -> bool {
    match (values, expected) {
        (values, Json::Array(expected)) if exact => {
            values.len() == expected.len()
                && values
                    .iter()
                    .zip(expected)
                    .all(|(v, e)| matches(v, e, true))
        }
        (values, Json::Array(expected)) => expected
            .iter()
            .all(|e| values.iter().any(|v| matches(v, e, false))),
        ([value], expected) => matches(value, expected, exact),
        _ => false,
    }
}

/// Whether `value` equals `expected` (`exact`: same properties, no extra extension or id on a
/// primitive) or follows it as a pattern (every pattern property present and matching).
/// Numbers compare as written decimals, digits and precision both: a fixed value "SHALL be
/// exactly the value" (elementdefinition.html) and "0.010 is regarded as different to 0.01"
/// (datatypes.html), so `3` and `3.00` are not `3.0`.
fn matches(value: &FhirPathValue, expected: &Json, exact: bool) -> bool {
    if let (FhirPathValue::Primitive(_, children, _), true) = (value, exact)
        && !children.is_empty()
    {
        return false;
    }
    match (value.primitive_value(), expected) {
        (FhirPathValue::Object(fields, _), Json::Object(map)) => {
            (!exact || fields.len() == map.len())
                && map.iter().all(|(key, e)| {
                    fields
                        .get(key.as_str())
                        .is_some_and(|values| matches_all(values, e, exact))
                })
        }
        (FhirPathValue::Bool(b), Json::Bool(e)) => b == e,
        (FhirPathValue::Integer(i) | FhirPathValue::Long(i), Json::Number(e)) => {
            same_decimal(Decimal::from(*i), e)
        }
        (FhirPathValue::Decimal(d), Json::Number(e)) => same_decimal(*d, e),
        (_, Json::String(e)) => value.as_string().is_some_and(|s| *s == **e),
        _ => false,
    }
}

/// Whether `value` is `expected` as written: same digits at the same scale, so `1e2`
/// (precise to hundreds) is not `100`. serde_json's `arbitrary_precision` keeps the
/// definition's number text.
fn same_decimal(value: Decimal, expected: &serde_json::Number) -> bool {
    expected
        .to_string()
        .parse::<Decimal>()
        .is_ok_and(|e| (e.mantissa(), e.scale()) == (value.mantissa(), value.scale()))
}
