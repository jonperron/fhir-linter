use fhir_definitions::{Registry, StructureDefinition};
use fhir_parser::{Node, Resource, Value};

use crate::diagnostic::Diagnostic;

/// Validate `Reference.reference` strings on top-level elements.
///
/// Emits `REFERENCE_001` for a relative reference whose type is not a known
/// resource type, and `REFERENCE_002` for a `#id` reference with no matching
/// entry in `contained`. Absolute URLs and `urn:` references are not resolved.
pub fn validate(
    resource: &Resource,
    sd: &StructureDefinition,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let resource_type = resource.resource_type.as_ref();
    let contained_ids: Vec<&str> = match resource.fields.get("contained").map(|n| &n.value) {
        Some(Value::Array(items)) => items.iter().filter_map(|n| string_field(n, "id")).collect(),
        _ => Vec::new(),
    };

    for (field_name, node) in &resource.fields {
        let path = format!("{resource_type}.{field_name}");
        let is_reference = sd
            .snapshot
            .iter()
            .any(|e| e.path.as_ref() == path && e.types.iter().any(|t| t.as_ref() == "Reference"));
        if !is_reference {
            continue;
        }
        let items: Vec<(&Node, String)> = match &node.value {
            Value::Array(a) => a
                .iter()
                .enumerate()
                .map(|(i, n)| (n, format!("{path}[{i}].reference")))
                .collect(),
            _ => vec![(node, format!("{path}.reference"))],
        };
        for (item, ref_path) in items {
            let Some(reference) = string_field(item, "reference") else {
                continue;
            };
            if let Some(problem) = check(reference, &contained_ids, registry) {
                diagnostics.push(
                    Diagnostic::error(problem.0, problem.1, ref_path).with_location(item.span),
                );
            }
        }
    }
}

fn string_field<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    let Value::Object(f) = &node.value else {
        return None;
    };
    match &f.get(key)?.value {
        Value::Str(s) => Some(s),
        _ => None,
    }
}

fn check(
    reference: &str,
    contained: &[&str],
    registry: &Registry,
) -> Option<(&'static str, String)> {
    if let Some(id) = reference.strip_prefix('#') {
        // "#" alone points at the containing resource.
        return (!id.is_empty() && !contained.contains(&id)).then(|| {
            (
                "REFERENCE_002",
                format!("Contained resource '{id}' not found"),
            )
        });
    }
    if reference.contains(':') {
        return None;
    }
    let type_name = reference.split(['/', '?']).next().unwrap_or(reference);
    let url = format!("http://hl7.org/fhir/StructureDefinition/{type_name}");
    let known = registry
        .structure_definition(&url)
        .is_some_and(|sd| sd.kind.as_ref() == "resource" && !sd.is_abstract);
    (!known).then(|| {
        (
            "REFERENCE_001",
            format!("Unknown resource type '{type_name}' in reference '{reference}'"),
        )
    })
}
