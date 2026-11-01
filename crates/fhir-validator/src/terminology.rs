use fhir_definitions::{Registry, StructureDefinition};
use fhir_parser::{Node, Resource, Value};

use crate::diagnostic::Diagnostic;

/// Validate `required` terminology bindings on top-level elements.
///
/// Emits `TERMINOLOGY_001` when a `code`, `Coding` or `CodeableConcept` is
/// provably outside the bound ValueSet. Membership that cannot be decided from
/// the loaded definitions (filters, external code systems) is not reported.
pub fn validate(
    resource: &Resource,
    sd: &StructureDefinition,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let resource_type = resource.resource_type.as_ref();

    for (field_name, node) in &resource.fields {
        let path = format!("{resource_type}.{field_name}");
        let Some(elem) = sd.snapshot.iter().find(|e| e.path.as_ref() == path) else {
            continue;
        };
        let Some(binding) = &elem.binding else {
            continue;
        };
        let (true, Some(vs)) = (binding.strength.as_ref() == "required", &binding.value_set) else {
            continue;
        };
        let [fhir_type] = elem.types.as_slice() else {
            continue;
        };

        let items: Vec<(&Node, String)> = match &node.value {
            Value::Array(a) => a
                .iter()
                .enumerate()
                .map(|(i, n)| (n, format!("{path}[{i}]")))
                .collect(),
            _ => vec![(node, path.clone())],
        };
        for (item, item_path) in items {
            if let Some(bad) = out_of_set(&item.value, fhir_type, vs, registry) {
                diagnostics.push(
                    Diagnostic::error(
                        "TERMINOLOGY_001",
                        format!("Code {bad} at '{item_path}' is not in required ValueSet '{vs}'"),
                        item_path,
                    )
                    .with_location(item.span),
                );
            }
        }
    }
}

/// Return a description of the offending code when `value` is provably not in `vs`.
fn out_of_set(value: &Value, fhir_type: &str, vs: &str, registry: &Registry) -> Option<String> {
    let provably_out = |system: Option<&str>, code: &str| {
        (registry.value_set_contains(vs, system, code) == Some(false)).then(|| code.to_owned())
    };
    match (fhir_type, value) {
        ("code", Value::Str(code)) => provably_out(None, code),
        ("Coding", Value::Object(_)) => coding(value).and_then(|(s, c)| provably_out(s, c)),
        ("CodeableConcept", Value::Object(fields)) => {
            let Value::Array(codings) = &fields.get("coding")?.value else {
                return None;
            };
            // One matching coding is enough; every coding must be provably out to fail.
            let outs: Vec<_> = codings
                .iter()
                .filter_map(|n| coding(&n.value))
                .map(|(s, c)| provably_out(s, c))
                .collect();
            let all_out = !outs.is_empty() && outs.iter().all(Option::is_some);
            all_out.then(|| outs.into_iter().flatten().collect::<Vec<_>>().join(", "))
        }
        _ => None,
    }
}

/// Extract `(system, code)` from a Coding; a Coding without a code is ignored.
fn coding(value: &Value) -> Option<(Option<&str>, &str)> {
    let Value::Object(f) = value else { return None };
    let Value::Str(code) = &f.get("code")?.value else {
        return None;
    };
    let system = match f.get("system").map(|n| &n.value) {
        Some(Value::Str(s)) => Some(s.as_ref()),
        _ => None,
    };
    Some((system, code))
}
