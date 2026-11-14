//! Cardinality of the children of one object (`CARDINALITY_001/002/003`).
use fhir_definitions::{ElementDefinition, StructureDefinition};
use fhir_parser::{Node, Value};
use indexmap::IndexMap;
use std::sync::Arc;

use crate::diagnostic::Diagnostic;
use crate::resolve::{is_primitive, resolve};

/// Check the cardinality of every child of `def_path` in one object (instance `path`).
pub(crate) fn check_children_cardinality(
    fields: &IndexMap<Arc<str>, Node>,
    sd: &StructureDefinition,
    def_path: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let run = own_descendants(sd, def_path);
    check_run_cardinality(fields, sd, run, def_path, path, diagnostics);
}

/// [`check_children_cardinality`] over `run`, the elements below `def_path` (see
/// [`own_descendants`]); profiles pass the run they reached, since a slice can repeat a path.
pub(crate) fn check_run_cardinality(
    fields: &IndexMap<Arc<str>, Node>,
    sd: &StructureDefinition,
    run: &[ElementDefinition],
    def_path: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let prefix = format!("{def_path}.");
    for elem in run {
        let Some(rest) = elem.path.strip_prefix(&prefix).filter(|r| !r.contains('.')) else {
            continue;
        };
        // shortcut: slices (choice slices too) are not matched to instances, so a slice's
        // own cardinality is not enforced; revisit when slicing discriminators are evaluated.
        if elem.slice_name.is_some() {
            continue;
        }
        if rest.ends_with("[x]") {
            check_choice(fields, sd, elem, &prefix, path, diagnostics);
            continue;
        }
        let node = present(fields, elem, rest);
        // An absent optional element cannot fail; skip building its path.
        if node.is_none() && elem.min == 0 {
            if let Some(empty) = fields.get(rest) {
                push_empty_array(empty, &format!("{path}.{rest}"), diagnostics);
            }
            continue;
        }
        check_cardinality(elem, node, &format!("{path}.{rest}"), diagnostics);
    }
}

/// Check a `[x]` choice element as a whole: its variants are the fields `resolve`
/// maps to it (a `_variant` counts only for a primitive type), and it holds at most one.
///
/// shortcut: profile type restrictions are not handled (a disallowed variant reads as
/// missing) and choice slices are skipped by the caller; revisit when profile snapshots
/// are walked.
fn check_choice(
    fields: &IndexMap<Arc<str>, Node>,
    sd: &StructureDefinition,
    elem: &ElementDefinition,
    prefix: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let variants: Vec<(&str, &Node)> = fields
        .iter()
        .filter_map(|(key, node)| {
            let name = key.strip_prefix('_');
            let bare = name.unwrap_or(key);
            let (e, ty) = resolve(sd, &format!("{prefix}{bare}"), bare)?;
            // A `_variant` beside its value is the same variant, counted once via the value.
            let extension_only = name.is_none_or(|n| {
                !fields.contains_key(n)
                    && ty.as_deref().is_some_and(is_primitive)
                    && is_extension_shape(node)
            });
            (e.path == elem.path && extension_only).then_some((bare, node))
        })
        .collect();
    match variants.as_slice() {
        [] => {
            let choice_path = format!("{path}.{}", elem.path.strip_prefix(prefix).unwrap_or(""));
            check_cardinality(elem, None, &choice_path, diagnostics);
        }
        [(name, node)] => {
            check_cardinality(elem, Some(node), &format!("{path}.{name}"), diagnostics)
        }
        [(first, _), (second, node), ..] => {
            let second_path = format!("{path}.{second}");
            diagnostics.push(
                Diagnostic::error(
                    "CARDINALITY_002",
                    format!("Choice element '{second_path}' conflicts with '{first}'"),
                    second_path,
                )
                .with_location(node.span),
            );
        }
    }
}

/// The elements below `def_path` without the slices of `def_path` itself, which follow its
/// own children in a snapshot and repeat their paths.
pub(crate) fn own_descendants<'a>(
    sd: &'a StructureDefinition,
    def_path: &str,
) -> &'a [ElementDefinition] {
    without_own_slices(sd.descendants(def_path), def_path)
}

/// `run` (the elements below `path`) up to the first slice of `path` itself.
pub(crate) fn without_own_slices<'a>(
    run: &'a [ElementDefinition],
    path: &str,
) -> &'a [ElementDefinition] {
    let end = run
        .iter()
        .position(|e| e.slice_name.is_some() && *e.path == *path)
        .unwrap_or(run.len());
    &run[..end]
}

/// The elements after `run[k]` that lie below it, its own slices included.
pub(crate) fn subtree(run: &[ElementDefinition], k: usize) -> &[ElementDefinition] {
    let path = &run[k].path;
    let rest = &run[k + 1..];
    let len = rest
        .iter()
        .position(|e| {
            !e.path
                .strip_prefix(path.as_ref())
                .is_some_and(|tail| tail.is_empty() || tail.starts_with('.'))
        })
        .unwrap_or(rest.len());
    &rest[..len]
}

/// An array holding no value: empty or only nulls (R5 json.html: "Objects are never empty").
fn is_empty_array(node: &Node) -> bool {
    let Value::Array(items) = &node.value else {
        return false;
    };
    items.iter().all(|item| matches!(item.value, Value::Null))
}

/// Warn (`CARDINALITY_003`) that an optional element holds an empty or all-null array.
fn push_empty_array(node: &Node, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(
        Diagnostic::warning(
            "CARDINALITY_003",
            format!("Element '{path}' is an array with no values and should be omitted"),
            path.to_owned(),
        )
        .with_location(node.span),
    );
}

/// The node that makes `elem` present: its value, or the `_name` primitive extension
/// (which stands in for an absent value only on primitive types, named in lowercase).
/// An empty or all-null value array is no value.
fn present<'a>(
    fields: &'a IndexMap<Arc<str>, Node>,
    elem: &ElementDefinition,
    name: &str,
) -> Option<&'a Node> {
    fields
        .get(name)
        .filter(|node| !is_empty_array(node))
        .or_else(|| {
            // Only primitives take a `_name`; skip building the key for the rest.
            let primitive = elem.types.first().is_some_and(|t| is_primitive(t));
            primitive
                .then(|| fields.get(format!("_{name}").as_str()))
                .flatten()
                .filter(|node| is_extension_shape(node))
        })
}

/// A `_name` primitive extension is an object, or an array of objects and nulls
/// holding at least one object.
fn is_extension_shape(node: &Node) -> bool {
    match &node.value {
        Value::Object(_) => true,
        Value::Array(items) => {
            items
                .iter()
                .all(|item| matches!(item.value, Value::Object(_) | Value::Null))
                && items
                    .iter()
                    .any(|item| matches!(item.value, Value::Object(_)))
        }
        _ => false,
    }
}

/// Emit `CARDINALITY_001` when a required element is absent and `CARDINALITY_002`
/// when it holds more values than a finite maximum.
fn check_cardinality(
    elem: &ElementDefinition,
    node: Option<&Node>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(node) = node else {
        if elem.min > 0 {
            diagnostics.push(Diagnostic::error(
                "CARDINALITY_001",
                format!("Required element '{path}' is missing (min={})", elem.min),
                path.to_owned(),
            ));
        }
        return;
    };
    let Ok(max) = elem.max.parse::<usize>() else {
        return;
    };
    let count = match &node.value {
        Value::Array(items) => items.len(),
        Value::Null => 0,
        _ => 1,
    };
    if count > max {
        diagnostics.push(
            Diagnostic::error(
                "CARDINALITY_002",
                format!("Element '{path}' has {count} value(s) but maximum is {max}"),
                path.to_owned(),
            )
            .with_location(node.span),
        );
    }
}
