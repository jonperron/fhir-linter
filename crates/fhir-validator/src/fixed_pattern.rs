//! `fixed[x]`, `pattern[x]` and value-discriminated slices of profile elements
//! (elementdefinition.html, profiling.html#slicing).
use fhir_definitions::{ChoiceValue, Discriminator, ElementDefinition};
use fhir_parser::{Node, Value};
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
        let checks = [
            (elem.fixed.as_deref(), true, "PROFILE_004", "is not exactly"),
            (
                elem.pattern.as_deref(),
                false,
                "PROFILE_005",
                "does not match",
            ),
        ];
        for (expected, exact, code, verb) in checks {
            let Some(expected) = expected.filter(|e| !matches(&item.value, &e.value, exact)) else {
                continue;
            };
            diagnostics.push(
                Diagnostic::error(
                    code,
                    format!(
                        "Value at '{item_path}' {verb} the {} {} required by '{}'",
                        expected.key, expected.value, elem.path
                    ),
                    item_path.clone(),
                )
                .with_location(item.span),
            );
        }
    }
}

/// Whether `value` equals `expected` (`exact`: same properties, arrays item by item) or
/// follows it as a pattern (every pattern property present and matching; each pattern array
/// item matching some instance item). Numbers compare by value, as FHIR decimals.
///
/// shortcut: both sides arrive as `f64` (parser and serde_json), so decimals beyond 15-17
/// significant digits can compare equal; upgrade when the parser keeps decimal text.
fn matches(value: &Value, expected: &Json, exact: bool) -> bool {
    match (value, expected) {
        // The XML reshape keeps a lone repeating child scalar: it is a one-item array.
        (value, Json::Array(expected)) if !matches!(value, Value::Array(_) | Value::Null) => {
            match expected.as_slice() {
                [only] => matches(value, only, exact),
                _ => !exact && expected.iter().all(|e| matches(value, e, false)),
            }
        }
        (Value::Str(s), Json::String(e)) => s.as_ref() == e,
        (Value::Bool(b), Json::Bool(e)) => b == e,
        (Value::Integer(i), Json::Number(e)) => match e.as_i64() {
            Some(e) => *i == e,
            None => e.as_f64() == Some(*i as f64),
        },
        (Value::Decimal(d), Json::Number(e)) => e.as_f64() == Some(*d),
        (Value::Object(fields), Json::Object(map)) => {
            (!exact || fields.len() == map.len())
                && map.iter().all(|(key, e)| {
                    fields
                        .get(key.as_str())
                        .is_some_and(|n| matches(&n.value, e, exact))
                })
        }
        (Value::Array(items), Json::Array(expected)) if exact => {
            items.len() == expected.len()
                && items
                    .iter()
                    .zip(expected)
                    .all(|(n, e)| matches(&n.value, e, true))
        }
        (Value::Array(items), Json::Array(expected)) => expected
            .iter()
            .all(|e| items.iter().any(|n| matches(&n.value, e, false))),
        _ => false,
    }
}

/// Check the cardinality of each slice of `elem` (sliced element whose snapshot elements
/// below it, slices included, are `run`; instance `path`, value `node`) by counting the
/// occurrences that match it (`PROFILE_006`).
///
/// shortcut: only slicings whose discriminators are all `value`/`pattern` on a plain path
/// (or `$this`) fixed in every slice are evaluated, and a matched occurrence is not checked
/// against the rest of its slice (nor closed or ordered slicing; reslices count as slices);
/// revisit with full slicing support.
pub(crate) fn check_slices(
    node: Option<&Node>,
    elem: &ElementDefinition,
    run: &[ElementDefinition],
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // No discriminator: repetitions cannot be told apart, so slices are not counted.
    let Some(slicing) = elem
        .slicing
        .as_ref()
        .filter(|s| !s.discriminators.is_empty())
    else {
        return;
    };
    let items: Vec<&Node> = node
        .map(|n| occurrences(n, path).into_iter().map(|(n, _)| n).collect())
        .unwrap_or_default();
    let starts: Vec<usize> = (0..run.len())
        .filter(|&i| run[i].slice_name.is_some() && run[i].path == elem.path)
        .collect();
    let mut slices = Vec::with_capacity(starts.len());
    for (k, &start) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(run.len());
        let (slice, children) = (&run[start], &run[start + 1..end]);
        let Some(rules) = slicing
            .discriminators
            .iter()
            .map(|d| discriminator_value(slice, children, d))
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        slices.push((slice, rules));
    }
    for (slice, rules) in slices {
        let count = items
            .iter()
            .filter(|item| {
                rules.iter().all(|(segments, expected, exact)| {
                    values_at(item, segments)
                        .into_iter()
                        .any(|v| matches(v, &expected.value, *exact))
                })
            })
            .count();
        let name = slice.slice_name.as_deref().unwrap_or_default();
        let message = if count < slice.min as usize {
            format!(
                "Slice '{name}' of '{path}' needs at least {} matching value(s), found {count}",
                slice.min
            )
        } else if slice.max.parse::<usize>().is_ok_and(|max| count > max) {
            format!(
                "Slice '{name}' of '{path}' allows at most {} matching value(s), found {count}",
                slice.max
            )
        } else {
            continue;
        };
        let mut diagnostic = Diagnostic::error("PROFILE_006", message, path.to_owned());
        if let Some(node) = node {
            diagnostic = diagnostic.with_location(node.span);
        }
        diagnostics.push(diagnostic);
    }
}

/// The path segments a `value`/`pattern` discriminator reads, and the fixed (exact) or
/// pattern value the slice gives that path; `None` when this slicing is not evaluated.
fn discriminator_value<'a>(
    slice: &'a ElementDefinition,
    children: &'a [ElementDefinition],
    discriminator: &'a Discriminator,
) -> Option<(Vec<&'a str>, &'a ChoiceValue, bool)> {
    if !matches!(discriminator.kind.as_ref(), "value" | "pattern") {
        return None;
    }
    let (segments, target) = match discriminator.path.as_ref() {
        "$this" => (Vec::new(), slice),
        path if path.contains(['(', '[', '$']) => return None,
        path => {
            let full = format!("{}.{path}", slice.path);
            let target = children
                .iter()
                .find(|e| e.slice_name.is_none() && *e.path == *full)?;
            (path.split('.').collect(), target)
        }
    };
    match (&target.fixed, &target.pattern) {
        (Some(fixed), _) => Some((segments, fixed, true)),
        (None, Some(pattern)) => Some((segments, pattern, false)),
        (None, None) => None,
    }
}

/// The values reached from `node` along `segments`, arrays flattened (FHIRPath navigation).
fn values_at<'a>(node: &'a Node, segments: &[&str]) -> Vec<&'a Value> {
    let mut current = vec![&node.value];
    for segment in segments {
        current = current
            .into_iter()
            .filter_map(|v| match v {
                Value::Object(fields) => fields.get(*segment),
                _ => None,
            })
            .flat_map(|n| match &n.value {
                Value::Array(items) => items.iter().map(|i| &i.value).collect(),
                v => vec![v],
            })
            .collect();
    }
    current
}
