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

/// Find the element for `name` under `parent_path`, and the single type it has
/// for this instance (the suffix picks the type of a `[x]` element). The type is
/// `None` for typeless elements such as `contentReference`; `None` overall means not found.
pub(crate) fn resolve<'a>(
    sd: &'a StructureDefinition,
    path: &str,
    name: &str,
) -> Option<(&'a ElementDefinition, Option<Arc<str>>)> {
    if name.contains(['.', '[', ']']) {
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

    for (elem, rest) in children() {
        if rest.ends_with("[x]")
            || fields.contains_key(rest)
            || fields.contains_key(format!("_{rest}").as_str())
        {
            continue;
        }
        if elem.min > 0 {
            diagnostics.push(Diagnostic::error(
                "CARDINALITY_001",
                format!(
                    "Required element '{path}.{rest}' is missing (min={})",
                    elem.min
                ),
                format!("{path}.{rest}"),
            ));
        }
    }

    for (name, node) in fields {
        let name = name.as_ref();
        if name.starts_with('_') || name == "extension" || name == "modifierExtension" {
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

        if let (Ok(max), Value::Array(items)) = (elem.max.parse::<usize>(), &node.value) {
            if items.len() > max {
                diagnostics.push(
                    Diagnostic::error(
                        "CARDINALITY_002",
                        format!(
                            "Element '{child_path}' has {} value(s) but maximum is {max}",
                            items.len()
                        ),
                        child_path.clone(),
                    )
                    .with_location(node.span),
                );
            }
        }
        check_type(&node.value, &ty, &child_path, node.span, diagnostics);
        descend(node, elem, &ty, sd, registry, &child_path, diagnostics);
    }
}
