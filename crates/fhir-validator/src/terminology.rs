use fhir_definitions::{Registry, StructureDefinition};
use fhir_parser::{Node, Resource, Value};

use crate::diagnostic::Diagnostic;

/// Validate `required` and `extensible` terminology bindings on top-level elements.
///
/// Emits `TERMINOLOGY_001` when a `code`, `Coding` or `CodeableConcept` is
/// provably outside the bound ValueSet: an error for `required`, a warning for
/// `extensible`. Membership that cannot be decided from the loaded definitions
/// (filters, external code systems) is not reported.
pub fn validate(
    resource: &Resource,
    sd: &StructureDefinition,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let resource_type = resource.resource_type.as_ref();

    for (field_name, node) in &resource.fields {
        let path = format!("{resource_type}.{field_name}");
        let Some(elem) = sd.element(&path) else {
            continue;
        };
        let Some(binding) = &elem.binding else {
            continue;
        };
        let Some(vs) = &binding.value_set else {
            continue;
        };
        let strength = binding.strength.as_ref();
        let required = match strength {
            "required" => true,
            "extensible" => false,
            _ => continue,
        };
        // The spec sets no validator severity for extensible bindings: report them as warnings.
        let report = if required {
            Diagnostic::error
        } else {
            Diagnostic::warning
        };
        let [fhir_type] = elem.types.as_slice() else {
            continue;
        };

        for (item, item_path) in entries(node, &path) {
            if let Some(bad) = out_of_set(&item.value, fhir_type, vs, required, registry) {
                let message =
                    format!("Code {bad} at '{item_path}' is not in {strength} ValueSet '{vs}'");
                diagnostics
                    .push(report("TERMINOLOGY_001", message, item_path).with_location(item.span));
            }
        }
    }
}

/// Return a description of the offending code when `value` is provably not in `vs`.
///
/// Under a `required` binding a CodeableConcept with no coded Coding is also out.
fn out_of_set(
    value: &Value,
    fhir_type: &str,
    vs: &str,
    required: bool,
    registry: &Registry,
) -> Option<String> {
    let provably_out = |system: Option<&str>, code: &str| {
        (registry.value_set_contains(vs, system, code) == Some(false)).then(|| code.to_owned())
    };
    match (fhir_type, value) {
        ("code", Value::Str(code)) => provably_out(None, code),
        ("Coding", Value::Object(_)) => coding(value).and_then(|(s, c)| provably_out(s, c)),
        ("CodeableConcept", Value::Object(fields)) => {
            let codings = fields
                .get("coding")
                .map(|n| entries(n, ""))
                .unwrap_or_default();
            // One matching coding is enough; every coding must be provably out to fail.
            let outs: Vec<_> = codings
                .iter()
                .filter_map(|(n, _)| coding(&n.value))
                .map(|(s, c)| provably_out(s, c))
                .collect();
            if outs.is_empty() {
                // terminologies.html 4.1.4.2.1: text is no substitute for the required code.
                // Extensible bindings (4.1.4.2.2) accept text alone when no code fits.
                return required.then(|| "none (no Coding has a code)".to_owned());
            }
            outs.iter()
                .all(Option::is_some)
                .then(|| outs.into_iter().flatten().collect::<Vec<_>>().join(", "))
        }
        _ => None,
    }
}

/// The values of a possibly repeating element with their paths.
///
/// A single value stays an object: XML keeps one repeating element unwrapped.
fn entries<'a>(node: &'a Node, path: &str) -> Vec<(&'a Node, String)> {
    match &node.value {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, n)| (n, format!("{path}[{i}]")))
            .collect(),
        _ => vec![(node, path.to_owned())],
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
