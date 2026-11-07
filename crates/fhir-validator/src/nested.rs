use fhir_definitions::{ElementDefinition, Registry, StructureDefinition};
use fhir_parser::{Node, Resource, Value};
use indexmap::IndexMap;
use std::sync::Arc;

use crate::diagnostic::Diagnostic;
use crate::type_check::check_type;

/// Validate elements below the top level: backbone elements and complex types.
///
/// Applies the structural (`STRUCTURE_001`), cardinality (`CARDINALITY_001/002`)
/// and primitive type (`TYPE_*`) checks recursively. Terminology, invariants,
/// references and profile layers remain top-level only.
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
    for (name, node) in &resource.fields {
        let path = format!("{resource_type}.{name}");
        let Some((elem, Some(ty))) = resolve(sd, &path, name) else {
            continue;
        };
        descend(node, elem, &ty, sd, registry, &path, diagnostics);
    }
}

/// Names with path punctuation are never valid element names.
pub(crate) fn has_punctuation(name: &str) -> bool {
    name.contains(['.', '[', ']'])
}

/// Find the element for `name` under `parent_path`, and the single type it has
/// for this instance (the suffix picks the type of a `[x]` element). The type is
/// `None` for typeless elements such as `contentReference`; `None` overall means not found.
pub(crate) fn resolve<'a>(
    sd: &'a StructureDefinition,
    path: &str,
    name: &str,
) -> Option<(&'a ElementDefinition, Option<Arc<str>>)> {
    if has_punctuation(name) {
        return None;
    }
    let direct = sd.snapshot.iter().find(|e| e.path.as_ref() == path);
    if let Some(e) = direct {
        return Some((e, e.types.first().cloned()));
    }
    let parent = path.strip_suffix(name)?;
    sd.snapshot.iter().find_map(|e| {
        let base = e.path.strip_prefix(parent)?.strip_suffix("[x]")?;
        let suffix = name.strip_prefix(base)?;
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
            } else if let Some(type_sd) = registry
                .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{ty}"))
                .filter(|t| t.kind.as_ref() == "complex-type")
            {
                walk(fields, type_sd, &type_sd.name, registry, path, diagnostics);
            }
        }
        _ => {}
    }
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
    let prefix = format!("{def_path}.");
    let children = || {
        sd.snapshot
            .iter()
            .filter_map(|e| Some((e, e.path.strip_prefix(&prefix)?)))
            .filter(|(_, rest)| !rest.contains('.'))
    };

    for (elem, rest) in children().filter(|(_, rest)| !rest.ends_with("[x]")) {
        let child_path = format!("{path}.{rest}");
        check_cardinality(elem, present(fields, elem, rest), &child_path, diagnostics);
    }

    for (name, node) in fields {
        let name = name.as_ref();
        if !has_punctuation(name)
            && (name.starts_with('_') || name == "extension" || name == "modifierExtension")
        {
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

        // Choice elements have no fixed name, so the children loop cannot count them.
        if elem.path.ends_with("[x]") {
            check_cardinality(elem, Some(node), &child_path, diagnostics);
        }
        // Element.id is a `string`; R5 snapshots label many as `id` in the fhir-type extension.
        let check_as = if name == "id" { "string" } else { ty.as_ref() };
        check_type(&node.value, check_as, &child_path, node.span, diagnostics);
        descend(node, elem, &ty, sd, registry, &child_path, diagnostics);
    }
}

/// The node that makes `elem` present: its value, or the `_name` primitive extension
/// (which stands in for an absent value only on primitive types, named in lowercase).
pub(crate) fn present<'a>(
    fields: &'a IndexMap<Arc<str>, Node>,
    elem: &ElementDefinition,
    name: &str,
) -> Option<&'a Node> {
    let is_primitive = elem
        .types
        .first()
        .is_some_and(|t| t.starts_with(|c: char| c.is_ascii_lowercase()));
    fields.get(name).or_else(|| {
        fields
            .get(format!("_{name}").as_str())
            .filter(|_| is_primitive)
    })
}

/// Emit `CARDINALITY_001` when a required element is absent and `CARDINALITY_002`
/// when it holds more values than a finite maximum.
pub(crate) fn check_cardinality(
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
