use std::sync::Arc;

use fhir_definitions::{ElementDefinition, Registry};
use fhir_parser::{Node, Resource, Value};

use crate::diagnostic::Diagnostic;

/// The ids of the resource's `contained` resources, which `#id` references resolve to.
/// A single XML `<contained>` stays an object (the reshape keeps XML grouping).
pub(crate) fn contained_ids(resource: &Resource) -> Vec<Arc<str>> {
    let id = |node: &Node| match &node.value {
        Value::Object(fields) => match fields.get("id").map(|n| &n.value) {
            Some(Value::Str(id)) => Some(Arc::clone(id)),
            _ => None,
        },
        _ => None,
    };
    match resource.fields.get("contained") {
        Some(Node {
            value: Value::Array(items),
            ..
        }) => items.iter().filter_map(id).collect(),
        Some(single) => id(single).into_iter().collect(),
        None => Vec::new(),
    }
}

/// Validate the `reference` string of one `Reference` value `item` (instance
/// `path`). The walk in `nested.rs` calls it for every Reference, at every depth.
///
/// Emits `REFERENCE_001` for a relative reference whose type is not a known
/// resource type, and `REFERENCE_002` for a `#id` reference with no matching
/// entry in `contained_ids`, or a bare `#` outside a contained resource
/// (`in_contained`), where it would name a container. Absolute URLs and `urn:`
/// references are not resolved. `REFERENCE_003` reports a `Reference.type` that
/// disagrees with the type of a FHIR URL reference, and `REFERENCE_004`
/// (warning) a malformed relative reference. `REFERENCE_005` reports a FHIR URL
/// reference whose type is not allowed by the `targetProfile` list of `elem`.
pub(crate) fn check(
    item: &Node,
    elem: &ElementDefinition,
    path: &str,
    contained_ids: &[Arc<str>],
    in_contained: bool,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(reference) = string_field(item, "reference") else {
        return;
    };
    let ref_path = format!("{path}.reference");
    if let Some(problem) = check_target(reference, contained_ids, in_contained, registry) {
        diagnostics.push(
            Diagnostic::error(problem.0, problem.1, ref_path.clone()).with_location(item.span),
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
            Diagnostic::error("REFERENCE_003", message, ref_path.clone()).with_location(item.span),
        );
    }
    if let Some(url_type) = fhir_url_type(reference, registry)
        && !target_allowed(elem, url_type, registry)
    {
        let message = format!(
            "Reference '{reference}' points to a {url_type}, which the target profiles of \
             '{}' do not allow",
            elem.path
        );
        diagnostics.push(
            Diagnostic::error("REFERENCE_005", message, ref_path.clone()).with_location(item.span),
        );
    }
    // The R5 literal reference regex is not worded as a SHALL: a warning.
    if malformed_relative(reference, registry) {
        let message = format!(
            "Relative reference '{reference}' does not match 'Type/id' or \
             'Type/id/_history/version'"
        );
        diagnostics
            .push(Diagnostic::warning("REFERENCE_004", message, ref_path).with_location(item.span));
    }
}

/// A relative reference to a known type that fails the R5 literal reference regex.
///
/// Conditional references (`Type?query`) and unknown types (`REFERENCE_001`) are left out.
fn malformed_relative(reference: &str, registry: &Registry) -> bool {
    let relative =
        !reference.starts_with('#') && !has_scheme(reference) && !reference.contains('?');
    let type_name = reference.split('/').next().unwrap_or(reference);
    relative
        && is_resource_type(registry, type_name)
        && fhir_url_type(reference, registry).is_none()
}

/// Whether the `Reference` targetProfile list of `elem` allows a reference to `url_type`.
///
/// No list allows any type. Each profile counts for the type its snapshot root names, which
/// allows that type and its descendants (`Resource` allows every resource). A profile the
/// registry cannot resolve allows any type: nothing is known about its target.
fn target_allowed(elem: &ElementDefinition, url_type: &str, registry: &Registry) -> bool {
    let Some(targets) = elem
        .type_profiles
        .iter()
        .find(|t| t.code.as_ref() == "Reference")
        .filter(|t| !t.target_profiles.is_empty())
    else {
        return true;
    };
    let ancestors: Vec<&str> = registry.type_ancestors(url_type).collect();
    targets.target_profiles.iter().any(|url| {
        registry.structure_definition(url).is_none_or(|sd| {
            sd.snapshot
                .first()
                .is_none_or(|root| ancestors.contains(&root.path.as_ref()))
        })
    })
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

/// Whether `reference` starts with a URI scheme (RFC 3986: `ALPHA *( ALPHA / DIGIT / "+" / "-" /
/// "." ) ":"`), making it absolute. A ':' after a '/' does not make one.
fn has_scheme(reference: &str) -> bool {
    reference.split_once(':').is_some_and(|(scheme, _)| {
        scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    })
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

fn check_target(
    reference: &str,
    contained: &[Arc<str>],
    in_contained: bool,
    registry: &Registry,
) -> Option<(&'static str, String)> {
    if let Some(id) = reference.strip_prefix('#') {
        // ref-1: "#" alone points at the container (`%rootResource != %resource`).
        if id.is_empty() {
            let message = "Reference '#' points at a containing resource, but this is the root";
            return (!in_contained).then(|| ("REFERENCE_002", message.to_owned()));
        }
        return (!contained.iter().any(|c| c.as_ref() == id)).then(|| {
            (
                "REFERENCE_002",
                format!("Contained resource '{id}' not found"),
            )
        });
    }
    if has_scheme(reference) {
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
