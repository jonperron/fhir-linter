//! Rules on StructureDefinition instances themselves (profiles, extension and type
//! definitions), beyond the generic structure and invariant checks every resource gets.
//!
//! elementdefinition.html, ElementDefinition.meaningWhenMissing: "Implicit meanings for
//! missing values can only be specified on a resource, data type, or extension definition,
//! and never in a profile that applies to one of these."
use fhir_definitions::Registry;
use fhir_parser::{Node, Resource, Value};
use indexmap::IndexMap;
use std::sync::Arc;

use crate::diagnostic::Diagnostic;

/// Check the StructureDefinition `resource` at instance `path`; other resources pass.
pub(crate) fn validate(
    resource: &Resource,
    registry: &Registry,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if &*resource.resource_type != "StructureDefinition" || !is_profile(resource, registry) {
        return;
    }
    // Only the differential: a profile's snapshot legitimately repeats the base's meanings.
    let Some(Value::Object(differential)) = resource.fields.get("differential").map(|n| &n.value)
    else {
        return;
    };
    for (index, element) in elements(differential.get("element")).enumerate() {
        let Value::Object(fields) = &element.value else {
            continue;
        };
        if let Some(node) = fields.get("meaningWhenMissing") {
            let element_path = string(fields, "path").unwrap_or("(no path)");
            diagnostics.push(
                Diagnostic::error(
                    "DEFINITION_001",
                    format!(
                        "The element definition for {element_path} sets meaningWhenMissing, \
                         which a profile may not set: only a resource, data type or extension \
                         definition can give a missing value an implicit meaning"
                    ),
                    format!("{path}.differential.element[{index}].meaningWhenMissing"),
                )
                .with_location(node.span),
            );
        }
    }
}

/// True for a constraint on a resource, data type or extension definition. A constraint
/// on the base Extension type is itself an extension definition, not a profile.
fn is_profile(resource: &Resource, registry: &Registry) -> bool {
    let fields = &resource.fields;
    if string(fields, "derivation") != Some("constraint") {
        return false;
    }
    // Without a type (required, reported elsewhere) the exemption cannot be decided: stay silent.
    let Some(type_name) = string(fields, "type") else {
        return false;
    };
    let Some(base) = string(fields, "baseDefinition") else {
        return true;
    };
    let base = base.split_once('|').map_or(base, |(url, _)| url);
    // The spec text names the extension definition as the exception, hence the type name.
    let defines_extension = type_name == "Extension"
        && registry
            .core(type_name)
            .is_some_and(|type_sd| &*type_sd.url == base);
    !defines_extension
}

fn string<'a>(fields: &'a IndexMap<Arc<str>, Node>, name: &str) -> Option<&'a str> {
    match &fields.get(name)?.value {
        Value::Str(s) => Some(s),
        _ => None,
    }
}

/// The items of a repeating element: an array, or a single object (XML grouping).
fn elements(node: Option<&Node>) -> impl Iterator<Item = &Node> {
    let items: &[Node] = match node {
        Some(Node {
            value: Value::Array(items),
            ..
        }) => items,
        Some(single) => std::slice::from_ref(single),
        None => &[],
    };
    items.iter()
}
