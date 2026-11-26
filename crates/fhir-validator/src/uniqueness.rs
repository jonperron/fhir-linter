//! Values the R5 definitions make unique within one resource.
//!
//! Element.id: "Unique id for the element within a resource (for internal references)";
//! narrative.html 2.1.4.0.4 extends the scope to the resource's contained resources.
//!
//! Meta.tag and Meta.security (comments): "The list of tags is a set. Uniqueness is based
//! the system/code, and version and display are ignored." (the same for security labels).
use std::collections::HashSet;
use std::sync::Arc;

use fhir_definitions::Registry;
use fhir_parser::{Node, Resource, Value};

use crate::diagnostic::Diagnostic;
use crate::embedded::child_element;
use crate::resolve::{items, string};

/// Emits `STRUCTURE_006` (error) at each element id an earlier element of `resource`
/// (instance `path`) or of its contained resources already uses. Resources held elsewhere
/// (Bundle entries, Parameters) are their own scope; resource ids are not element ids.
///
/// ElementDefinition ids are left out: StructureDefinition sdf-16 and sdf-17 make them
/// unique per snapshot and per differential, and every R5 definition repeats each id in both.
pub(crate) fn check_element_ids(
    resource: &Resource,
    registry: &Registry,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut walk = IdWalk {
        registry,
        seen: HashSet::new(),
        diagnostics,
    };
    for (name, node) in &resource.fields {
        let child_path = format!("{path}.{name}");
        if &**name != "contained" {
            walk.fields([(name, node)], &resource.resource_type, path);
            continue;
        }
        for (item, item_path) in items(node, &child_path) {
            let Value::Object(fields) = &item.value else {
                continue;
            };
            // Without a resource type nothing below is a known element (reported elsewhere).
            let Some(definition) = string(fields, "resourceType") else {
                continue;
            };
            // The contained resource's own `id` is its resource id.
            let fields = fields.iter().filter(|(name, _)| &***name != "id");
            walk.fields(fields, definition, &item_path);
        }
    }
}

struct IdWalk<'a, 'd> {
    registry: &'a Registry,
    seen: HashSet<Arc<str>>,
    diagnostics: &'d mut Vec<Diagnostic>,
}

impl IdWalk<'_, '_> {
    /// The ids in `fields`, the children of an object at definition path `definition`. Below
    /// an element the Registry does not know (STRUCTURE_001) nothing is an Element, so no id.
    fn fields<'n>(
        &mut self,
        fields: impl IntoIterator<Item = (&'n Arc<str>, &'n Node)>,
        definition: &str,
        path: &str,
    ) {
        for (name, node) in fields {
            let child_path = format!("{path}.{name}");
            match &node.value {
                Value::Str(id) if &**name == "id" => {
                    if !self.seen.insert(Arc::clone(id)) {
                        self.diagnostics.push(
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
                    // `_name` holds the id and extensions of primitive `name`.
                    let bare = name.strip_prefix('_').unwrap_or(name);
                    let Some((child_definition, ty)) =
                        child_element(self.registry, definition, bare)
                    else {
                        continue;
                    };
                    if &*ty == "ElementDefinition" {
                        continue;
                    }
                    for (item, item_path) in items(node, &child_path) {
                        match &item.value {
                            // An embedded resource (Bundle entry, Parameters) is its own scope.
                            Value::Object(fields) if !fields.contains_key("resourceType") => {
                                self.fields(fields, &child_definition, &item_path);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
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
