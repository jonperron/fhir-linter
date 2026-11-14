use fhir_definitions::{ElementDefinition, Registry};
use fhir_parser::{Node, Value};

use crate::diagnostic::Diagnostic;

/// Validate the `required` or `extensible` binding of `elem` on its value `node`
/// (instance `path`), whose type is `fhir_type`. The walk in `nested.rs` calls it
/// for every element, at every depth.
///
/// Emits `TERMINOLOGY_001` when a `code`, `Coding` or `CodeableConcept` is
/// provably outside the bound ValueSet: an error for `required`, a warning for
/// `extensible`. Membership that cannot be decided from the loaded definitions
/// (filters, external code systems) is not reported. A bound Coding with a
/// code but no system also gets a `TERMINOLOGY_002` warning.
pub(crate) fn check(
    node: &Node,
    elem: &ElementDefinition,
    fhir_type: &str,
    path: &str,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(binding) = &elem.binding else {
        return;
    };
    let Some(vs) = &binding.value_set else {
        return;
    };
    let strength = binding.strength.as_ref();
    let required = match strength {
        "required" => true,
        "extensible" => false,
        _ => return,
    };
    // The spec sets no validator severity for extensible bindings: report them as warnings.
    let report = if required {
        Diagnostic::error
    } else {
        Diagnostic::warning
    };

    for (item, item_path) in entries(node, path) {
        let codings = codings(item, fhir_type, &item_path);
        if let Some(bad) = out_of_set(item, fhir_type, &codings, vs, required, registry) {
            let message = format!("{bad} at '{item_path}' is not in {strength} ValueSet '{vs}'");
            diagnostics
                .push(report("TERMINOLOGY_001", message, item_path).with_location(item.span));
        }
        for (coding_node, coding_path) in codings {
            if let Some((None, code)) = coding(&coding_node.value) {
                let message = format!(
                    "Coding '{code}' at '{coding_path}' has no system, \
                     so it cannot be a member of ValueSet '{vs}'"
                );
                diagnostics.push(
                    Diagnostic::warning("TERMINOLOGY_002", message, coding_path)
                        .with_location(coding_node.span),
                );
            }
        }
    }
}

/// Return a description of the offending code when `item` is provably not in `vs`.
///
/// A Coding without a system is never a member: its code has no defined meaning.
/// Under a `required` binding a CodeableConcept with no coded Coding is also out.
fn out_of_set(
    item: &Node,
    fhir_type: &str,
    codings: &[(&Node, String)],
    vs: &str,
    required: bool,
    registry: &Registry,
) -> Option<String> {
    let provably_out = |system: Option<&str>, code: &str| {
        (registry.value_set_contains(vs, system, code) == Some(false)).then(|| code.to_owned())
    };
    match (fhir_type, &item.value) {
        ("code", Value::Str(code)) => provably_out(None, code).map(|c| format!("Code {c}")),
        ("Coding" | "CodeableConcept", Value::Object(_)) => {
            // One matching coding is enough; every coding must be provably out to fail.
            let outs: Vec<_> = codings
                .iter()
                .filter_map(|(n, _)| coding(&n.value))
                .map(|(system, code)| match system {
                    None => Some(code.to_owned()),
                    Some(_) => provably_out(system, code),
                })
                .collect();
            if outs.is_empty() {
                // terminologies.html 4.1.4.2.1: text is no substitute for the required code.
                // Extensible bindings (4.1.4.2.2) accept text alone when no code fits.
                let missing = match fhir_type {
                    "Coding" => "Coding with no code",
                    _ => "CodeableConcept with no coded Coding", // the arm admits only these two
                };
                return required.then(|| missing.to_owned());
            }
            outs.iter().all(Option::is_some).then(|| {
                let codes: Vec<_> = outs.into_iter().flatten().collect();
                format!("Code {}", codes.join(", "))
            })
        }
        _ => None,
    }
}

/// The Codings of a `Coding` or `CodeableConcept` item, with their paths.
fn codings<'a>(item: &'a Node, fhir_type: &str, item_path: &str) -> Vec<(&'a Node, String)> {
    match (fhir_type, &item.value) {
        ("Coding", Value::Object(_)) => vec![(item, item_path.to_owned())],
        ("CodeableConcept", Value::Object(fields)) => fields
            .get("coding")
            .map(|n| entries(n, &format!("{item_path}.coding")))
            .unwrap_or_default(),
        _ => Vec::new(),
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
