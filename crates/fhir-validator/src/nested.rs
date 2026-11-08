use fhir_definitions::{ElementDefinition, Registry, StructureDefinition};
use fhir_parser::{Node, Resource, Value};
use indexmap::IndexMap;
use std::sync::Arc;

use crate::diagnostic::Diagnostic;
use crate::type_check::{check_type, is_valid_id};

/// Validate the resource's elements at every depth, starting at the top level.
///
/// Applies the structural (`STRUCTURE_001`), cardinality (`CARDINALITY_001/002`)
/// and primitive type (`TYPE_*`) checks recursively into backbone elements and
/// complex types. Terminology, invariants, references and profile layers remain
/// top-level only.
///
/// shortcut: `contentReference` elements (skipped silently) and contained/embedded resources are
/// not descended into; upgrade when recursive structures need validating.
pub fn validate(
    resource: &Resource,
    sd: &StructureDefinition,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let resource_type = resource.resource_type.as_ref();

    // The parser lifts `id` out of `fields` (already rejecting non-strings), without a span.
    if let Some(id) = resource.id.as_deref().filter(|id| !is_valid_id(id)) {
        let path = format!("{resource_type}.id");
        diagnostics.push(Diagnostic::error(
            "TYPE_005",
            format!("Invalid id format at '{path}': '{id}'"),
            path,
        ));
    }

    walk(
        &resource.fields,
        sd,
        resource_type,
        registry,
        resource_type,
        diagnostics,
    );
}

/// Names with path punctuation are never valid element names.
fn has_punctuation(name: &str) -> bool {
    name.contains(['.', '[', ']'])
}

/// Find the element for `name` under `parent_path`, and the single type it has
/// for this instance (the suffix picks the type of a `[x]` element). The type is
/// `None` for typeless elements such as `contentReference`; `None` overall means not found.
fn resolve<'a>(
    sd: &'a StructureDefinition,
    path: &str,
    name: &str,
) -> Option<(&'a ElementDefinition, Option<Arc<str>>)> {
    if has_punctuation(name) {
        return None;
    }
    if let Some(e) = sd.element(path) {
        return Some((e, e.types.first().cloned()));
    }
    let parent = path.strip_suffix(name)?;
    // A type suffix is capitalised, so a choice can only split before an uppercase letter.
    (1..name.len())
        .filter(|&i| name.as_bytes()[i].is_ascii_uppercase())
        .find_map(|i| {
            let (base, suffix) = name.split_at(i);
            let e = sd.element(&format!("{parent}{base}[x]"))?;
            let ty = e.types.iter().find(|t| capitalise(t) == suffix)?;
            Some((e, Some(ty.clone())))
        })
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// Check the value of `elem` (instance `path`), recursing into objects.
fn descend(
    node: &Node,
    elem: &ElementDefinition,
    ty: &str,
    sd: &StructureDefinition,
    registry: &Registry,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &node.value {
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                descend(
                    item,
                    elem,
                    ty,
                    sd,
                    registry,
                    &format!("{path}[{i}]"),
                    diagnostics,
                );
            }
        }
        Value::Object(fields) => {
            if ty == "BackboneElement" || ty == "Element" {
                walk(fields, sd, &elem.path, registry, path, diagnostics);
            } else if let Some(type_sd) = complex_type(ty, registry) {
                walk(fields, type_sd, &type_sd.name, registry, path, diagnostics);
            }
        }
        _ if ty == "BackboneElement" || ty == "Element" || complex_type(ty, registry).is_some() => {
            diagnostics.push(
                Diagnostic::error(
                    "TYPE_001",
                    format!("Expected {ty} object at '{path}'"),
                    path.to_owned(),
                )
                .with_location(node.span),
            );
        }
        _ => {}
    }
}

/// The StructureDefinition of `ty` when it is a complex datatype.
fn complex_type<'a>(ty: &str, registry: &'a Registry) -> Option<&'a StructureDefinition> {
    registry
        .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{ty}"))
        .filter(|t| t.kind.as_ref() == "complex-type")
}

/// Validate the children of one object against the elements under `def_path`.
fn walk(
    fields: &IndexMap<Arc<str>, Node>,
    sd: &StructureDefinition,
    def_path: &str,
    registry: &Registry,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_children_cardinality(fields, sd, def_path, path, diagnostics);

    let prefix = format!("{def_path}.");

    for (name, node) in fields {
        let name = name.as_ref();
        // `_name` primitive extensions only count towards presence; `extension` and
        // `modifierExtension` are typed `Extension` and walked like any other element.
        if !has_punctuation(name) && name.starts_with('_') {
            continue;
        }
        let child_path = format!("{path}.{name}");
        let Some((elem, ty)) = resolve(sd, &format!("{prefix}{name}"), name) else {
            diagnostics.push(
                Diagnostic::error(
                    "STRUCTURE_001",
                    format!("Unknown element '{child_path}'"),
                    child_path,
                )
                .with_location(node.span),
            );
            continue;
        };
        // contentReference element: no type to check here.
        let Some(ty) = ty else { continue };

        // Element.id is a `string`; R5 snapshots label many as `id` in the fhir-type extension.
        let check_as = if name == "id" { "string" } else { ty.as_ref() };
        check_type(&node.value, check_as, &child_path, node.span, diagnostics);
        descend(node, elem, &ty, sd, registry, &child_path, diagnostics);
    }
}

/// Check the cardinality of every child of `def_path` in one object (instance `path`).
pub(crate) fn check_children_cardinality(
    fields: &IndexMap<Arc<str>, Node>,
    sd: &StructureDefinition,
    def_path: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let prefix = format!("{def_path}.");
    for elem in sd.descendants(def_path) {
        let Some(rest) = elem.path.strip_prefix(&prefix).filter(|r| !r.contains('.')) else {
            continue;
        };
        if rest.ends_with("[x]") {
            check_choice(fields, sd, elem, &prefix, path, diagnostics);
            continue;
        }
        let node = present(fields, elem, rest);
        // An absent optional element cannot fail; skip building its path.
        if node.is_none() && elem.min == 0 {
            continue;
        }
        check_cardinality(elem, node, &format!("{path}.{rest}"), diagnostics);
    }
}

/// Check a `[x]` choice element as a whole: its variants are the fields `resolve`
/// maps to it (a `_variant` counts only for a primitive type), and it holds at most one.
///
/// shortcut: profile type restrictions and choice slices are not handled (a disallowed
/// variant reads as missing, any variant satisfies a slice); revisit when profile
/// snapshots are walked.
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

/// Primitive FHIR type names start with a lowercase letter.
fn is_primitive(ty: &str) -> bool {
    ty.starts_with(|c: char| c.is_ascii_lowercase())
}

/// The node that makes `elem` present: its value, or the `_name` primitive extension
/// (which stands in for an absent value only on primitive types, named in lowercase).
fn present<'a>(
    fields: &'a IndexMap<Arc<str>, Node>,
    elem: &ElementDefinition,
    name: &str,
) -> Option<&'a Node> {
    fields.get(name).or_else(|| {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn elem(path: &str, min: u32) -> ElementDefinition {
        ElementDefinition {
            path: Arc::from(path),
            min,
            max: Arc::from("1"),
            types: vec![],
            constraints: vec![],
            binding: None,
        }
    }

    #[test]
    fn children_cardinality_checks_direct_children_only() {
        let sd = StructureDefinition::new(
            Arc::from("http://example.org/StructureDefinition/obs-profile"),
            Arc::from("ObsProfile"),
            Arc::from("resource"),
            false,
            None,
            vec![
                elem("Observation", 0),
                elem("Observation.subject", 1),
                elem("Observation.value[x]", 1),
                elem("Observation.component.code", 1),
            ],
        );
        let mut found = vec![];
        check_children_cardinality(
            &IndexMap::new(),
            &sd,
            "Observation",
            "Observation",
            &mut found,
        );
        let found: Vec<_> = found
            .iter()
            .map(|d| (d.code.as_ref(), d.path.as_str()))
            .collect();
        // A missing required choice is reported at its placeholder path.
        assert_eq!(
            found,
            [
                ("CARDINALITY_001", "Observation.subject"),
                ("CARDINALITY_001", "Observation.value[x]"),
            ],
        );
    }
}
