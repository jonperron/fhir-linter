use fhir_definitions::{Registry, StructureDefinition};
use fhir_parser::{Node, Resource, Value};

use crate::diagnostic::Diagnostic;

/// Validate `Reference.reference` strings on top-level elements.
///
/// Emits `REFERENCE_001` for a relative reference whose type is not a known
/// resource type, and `REFERENCE_002` for a `#id` reference with no matching
/// entry in `contained`. Absolute URLs and `urn:` references are not resolved.
/// `REFERENCE_003` reports a `Reference.type` that disagrees with the type of a
/// FHIR URL reference.
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
            .element(&path)
            .is_some_and(|e| e.types.iter().any(|t| t.as_ref() == "Reference"));
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
                    Diagnostic::error(problem.0, problem.1, ref_path.clone())
                        .with_location(item.span),
                );
            }
            // references.html 2.1.3.0.1: type and a FHIR URL reference SHALL be consistent.
            let declared = string_field(item, "type");
            if let (Some(declared), Some(url_type)) = (declared, fhir_url_type(reference, registry))
                && declared.strip_prefix(CORE_SD).unwrap_or(declared) != url_type
            {
                let message = format!(
                    "Reference.type '{declared}' does not match type '{url_type}' \
                     of reference '{reference}'"
                );
                diagnostics.push(
                    Diagnostic::error("REFERENCE_003", message, ref_path).with_location(item.span),
                );
            }
        }
    }
}

const CORE_SD: &str = "http://hl7.org/fhir/StructureDefinition/";

fn is_resource_type(registry: &Registry, name: &str) -> bool {
    registry
        .structure_definition(&format!("{CORE_SD}{name}"))
        .is_some_and(|sd| sd.kind.as_ref() == "resource" && !sd.is_abstract)
}

/// The resource type of a reference that fully matches the R5 literal reference regex
/// `((http|https)://([A-Za-z0-9\-\\\.\:\%\$]*\/)+)?(Type)\/[A-Za-z0-9\-\.]{1,64}`
/// `(\/_history\/[A-Za-z0-9\-\.]{1,64})?`, where `Type` is a concrete resource type.
fn fhir_url_type<'a>(reference: &'a str, registry: &Registry) -> Option<&'a str> {
    let (absolute, path) = match reference.split_once("://") {
        Some(("http" | "https", rest)) => (true, rest),
        Some(_) => return None,
        None => (false, reference),
    };
    let segments: Vec<&str> = path.split('/').collect();
    let (base, type_name) = match segments.as_slice() {
        [base @ .., type_name, id, "_history", version] if is_id(id) && is_id(version) => {
            (base, *type_name)
        }
        [base @ .., type_name, id] if is_id(id) => (base, *type_name),
        _ => return None,
    };
    let base_ok = match absolute {
        true => {
            let base_char = |c: char| c.is_ascii_alphanumeric() || "-\\.:%$".contains(c);
            !base.is_empty() && base.iter().all(|s| s.chars().all(base_char))
        }
        false => base.is_empty(),
    };
    (base_ok && is_resource_type(registry, type_name)).then_some(type_name)
}

/// `[A-Za-z0-9\-\.]{1,64}`: the id and version parts of a literal reference.
fn is_id(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-.".contains(c))
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
    (!is_resource_type(registry, type_name)).then(|| {
        (
            "REFERENCE_001",
            format!("Unknown resource type '{type_name}' in reference '{reference}'"),
        )
    })
}
