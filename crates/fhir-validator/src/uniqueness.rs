//! Values the R5 definitions make unique within one resource.
//!
//! Element.id: "Unique id for the element within a resource (for internal references)";
//! narrative.html 2.1.4.0.4 extends the scope to the resource's contained resources.
//!
//! Meta.tag and Meta.security (comments): "The list of tags is a set. Uniqueness is based
//! the system/code, and version and display are ignored." (the same for security labels).
use std::collections::HashSet;
use std::sync::Arc;

use fhir_parser::{Node, Resource, Value};
use indexmap::IndexMap;

use crate::diagnostic::Diagnostic;

/// Emits `STRUCTURE_006` (error) at each element id an earlier element of `resource`
/// (instance `path`) or of its contained resources already uses. Resources held elsewhere
/// (Bundle entries, Parameters) are their own scope; resource ids are not element ids.
pub(crate) fn check_element_ids(
    resource: &Resource,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = HashSet::new();
    for (name, node) in &resource.fields {
        let child_path = format!("{path}.{name}");
        if &**name == "contained" {
            for (item, item_path) in items(node, &child_path) {
                if let Value::Object(fields) = &item.value {
                    // The contained resource's own `id` is its resource id.
                    let fields = fields.iter().filter(|(name, _)| &***name != "id");
                    element_ids(fields, &item_path, &mut seen, diagnostics);
                }
            }
        } else {
            element_ids([(name, node)], path, &mut seen, diagnostics);
        }
    }
}

fn element_ids<'a>(
    fields: impl IntoIterator<Item = (&'a Arc<str>, &'a Node)>,
    path: &str,
    seen: &mut HashSet<Arc<str>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (name, node) in fields {
        let child_path = format!("{path}.{name}");
        match &node.value {
            Value::Str(id) if &**name == "id" => {
                if !seen.insert(Arc::clone(id)) {
                    diagnostics.push(
                        Diagnostic::error(
                            "STRUCTURE_006",
                            format!("Duplicate element id '{id}' within the resource"),
                            child_path,
                        )
                        .with_location(node.span),
                    );
                }
            }
            Value::Object(_) | Value::Array(_) => {
                for (item, item_path) in items(node, &child_path) {
                    match &item.value {
                        // An embedded resource (Bundle entry, Parameters value) is its own scope.
                        Value::Object(fields) if !fields.contains_key("resourceType") => {
                            element_ids(fields, &item_path, seen, diagnostics);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

/// Emits `STRUCTURE_007` (error) at each `meta.tag` / `meta.security` coding whose
/// system and code an earlier one of the same list already has.
pub(crate) fn check_meta_sets(resource: &Resource, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    let Some(Value::Object(meta)) = resource.fields.get("meta").map(|node| &node.value) else {
        return;
    };
    for list in ["tag", "security"] {
        let Some(node) = meta.get(list) else {
            continue;
        };
        let mut seen = HashSet::new();
        for (item, item_path) in items(node, &format!("{path}.meta.{list}")) {
            let Value::Object(coding) = &item.value else {
                continue;
            };
            // A coding without a code has no identity to repeat.
            let Some(code) = string(coding, "code") else {
                continue;
            };
            let system = string(coding, "system");
            if !seen.insert((system, code)) {
                diagnostics.push(
                    Diagnostic::error(
                        "STRUCTURE_007",
                        format!(
                            "Duplicate meta.{list} '{}#{code}': the list is a set on system \
                             and code",
                            system.unwrap_or("")
                        ),
                        item_path,
                    )
                    .with_location(item.span),
                );
            }
        }
    }
}

/// The items of a repeating element with their instance paths: an array item by index, a
/// single value (XML grouping, or a non-repeating element) at `path` itself.
fn items<'a>(node: &'a Node, path: &str) -> Vec<(&'a Node, String)> {
    match &node.value {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (item, format!("{path}[{index}]")))
            .collect(),
        _ => vec![(node, path.to_owned())],
    }
}

fn string<'a>(fields: &'a IndexMap<Arc<str>, Node>, name: &str) -> Option<&'a str> {
    match &fields.get(name)?.value {
        Value::Str(s) => Some(s),
        _ => None,
    }
}
