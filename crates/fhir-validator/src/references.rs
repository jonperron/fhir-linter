use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use fhir_definitions::{CORE_PREFIX, ElementDefinition, Registry};
use fhir_parser::{Node, Resource, Value};

use crate::diagnostic::Diagnostic;

/// The id of a contained resource, with its `resourceType` when that is a string.
pub(crate) type ContainedId = (Arc<str>, Option<Arc<str>>);

/// The `resourceType` of the Bundle entries with each `fullUrl`; `None` when entries
/// sharing a fullUrl disagree on it.
pub(crate) type BundleTargets = HashMap<Arc<str>, Option<Arc<str>>>;

/// The entries of a Bundle that references inside it resolve to (bundle.html 2.4.5.1: "Look
/// for an entry with a fullUrl that matches the reference.value"). Emits `REFERENCE_007` at
/// each entry whose RESTful `fullUrl` does not end with the resource id (Bundle.entry.fullUrl:
/// "The fullUrl SHALL NOT disagree with the id in the resource").
pub(crate) fn bundle_entries(
    resource: &Resource,
    path: &str,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) -> BundleTargets {
    let entries = match resource.fields.get("entry") {
        Some(Node {
            value: Value::Array(items),
            ..
        }) => items.as_slice(),
        Some(single) => std::slice::from_ref(single),
        None => &[],
    };
    let mut found = BundleTargets::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(full_url) = string_field(entry, "fullUrl") else {
            continue;
        };
        let Value::Object(fields) = &entry.value else {
            continue;
        };
        let entry_resource = fields.get("resource");
        let id = entry_resource.and_then(|node| string_field(node, "id"));
        if let (Some(id), Some(url_id)) = (id, restful_id(full_url, registry))
            && id != url_id
        {
            let message = format!(
                "The fullUrl '{full_url}' is a RESTful URL, so it must end with the resource \
                 id '{id}'"
            );
            diagnostics.push(
                Diagnostic::error("REFERENCE_007", message, format!("{path}.entry[{index}]"))
                    .with_location(entry.span),
            );
        }
        // An entry without a resource type leaves its fullUrl ambiguous, never resolved.
        let resource_type = entry_resource.and_then(|node| string_field(node, "resourceType"));
        found
            .entry(Arc::from(full_url))
            .and_modify(|known: &mut Option<Arc<str>>| {
                if known.as_deref() != resource_type {
                    *known = None;
                }
            })
            .or_insert_with(|| resource_type.map(Arc::from));
    }
    found
}

/// The id part of an absolute URL that matches the R5 RESTful URL regex.
fn restful_id<'a>(url: &'a str, registry: &Registry) -> Option<&'a str> {
    fhir_url_type(url, registry).filter(|_| has_scheme(url))?;
    let segments: Vec<&str> = url.rsplit('/').take(3).collect();
    match segments.as_slice() {
        [_, "_history", id] => Some(id),
        [id, ..] => Some(id),
        [] => None,
    }
}

/// The ids of the resource's `contained` resources, which `#id` references resolve to.
pub(crate) fn contained_ids(resource: &Resource) -> Vec<ContainedId> {
    contained(resource)
        .iter()
        .filter_map(contained_id)
        .collect()
}

/// Emits `REFERENCE_006` (error) at each contained resource whose id an earlier
/// contained resource of `resource` (instance `path`) already uses: narrative.html
/// 2.1.4.0.4 says ids SHALL be unique within the resource, contained resources included.
pub(crate) fn check_duplicate_contained_ids(
    resource: &Resource,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = HashSet::new();
    for (index, item) in contained(resource).iter().enumerate() {
        let Some((id, _)) = contained_id(item) else {
            continue;
        };
        if !seen.insert(Arc::clone(&id)) {
            let message = format!("Duplicate id for contained resource: '{id}'");
            diagnostics.push(
                Diagnostic::error(
                    "REFERENCE_006",
                    message,
                    format!("{path}.contained[{index}]"),
                )
                .with_location(item.span),
            );
        }
    }
}

/// The resource's `contained` resources, in order. A single XML `<contained>`
/// stays an object (the reshape keeps XML grouping).
fn contained(resource: &Resource) -> &[Node] {
    match resource.fields.get("contained") {
        Some(Node {
            value: Value::Array(items),
            ..
        }) => items,
        Some(single) => std::slice::from_ref(single),
        None => &[],
    }
}

/// The id of one contained resource, with its `resourceType` when that is a string.
fn contained_id(node: &Node) -> Option<ContainedId> {
    let Value::Object(fields) = &node.value else {
        return None;
    };
    let Some(Value::Str(id)) = fields.get("id").map(|n| &n.value) else {
        return None;
    };
    let resource_type = match fields.get("resourceType").map(|n| &n.value) {
        Some(Value::Str(resource_type)) => Some(Arc::clone(resource_type)),
        _ => None,
    };
    Some((Arc::clone(id), resource_type))
}

/// Validate the `reference` string of one `Reference` value `item` (instance
/// `path`). The walk in `nested.rs` calls it for every Reference, at every depth.
///
/// Emits `REFERENCE_001` for a relative reference whose type is not a known
/// resource type, and `REFERENCE_002` for a `#id` reference with no matching
/// entry in `contained_ids`, or a bare `#` outside a contained resource
/// (`in_contained`), where it would name a container. Absolute URLs and `urn:`
/// references are not resolved. `REFERENCE_003` reports a `Reference.type` that
/// disagrees with the type of a FHIR URL or `#id` reference, and `REFERENCE_004`
/// (warning) a malformed relative reference. `REFERENCE_005` reports a FHIR URL
/// or `#id` reference whose type is not allowed by the `targetProfile` list of
/// `elem`. A `#id` takes the type of the contained resource it resolves to. An absolute
/// reference whose text equals the fullUrl of `bundle` entries (such as `urn:uuid:`, or a
/// RESTful URL) takes their resource type when it is the same for all, else it is left
/// unresolved; other absolute URLs and `urn:` references are not resolved.
#[allow(clippy::too_many_arguments)] // the reference, its definition and where it resolves
pub(crate) fn check(
    item: &Node,
    elem: &ElementDefinition,
    path: &str,
    contained_ids: &[ContainedId],
    in_contained: bool,
    bundle: &BundleTargets,
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
    // A matching Bundle entry is authoritative (ambiguous matches stay unresolved); else the
    // type a FHIR URL names, or the type of the contained resource a `#id` resolves to.
    let target_type = match bundle.get(reference).filter(|_| has_scheme(reference)) {
        Some(known) => known.as_deref().filter(|t| is_resource_type(registry, t)),
        None => fhir_url_type(reference, registry).or_else(|| {
            // A bare `#` names the container, not a contained resource.
            let id = reference.strip_prefix('#').filter(|id| !id.is_empty())?;
            let (_, resource_type) = contained_ids.iter().find(|(c, _)| c.as_ref() == id)?;
            resource_type
                .as_deref()
                .filter(|t| is_resource_type(registry, t))
        }),
    };
    // references.html 2.1.3.0.1: type and the reference SHALL be consistent.
    let declared = string_field(item, "type");
    if let (Some(declared), Some(url_type)) = (declared, target_type)
        && declared.strip_prefix(CORE_PREFIX).unwrap_or(declared) != url_type
    {
        let message = format!(
            "Reference.type '{declared}' does not match type '{url_type}' \
             of reference '{reference}'"
        );
        diagnostics.push(
            Diagnostic::error("REFERENCE_003", message, ref_path.clone()).with_location(item.span),
        );
    }
    if let Some(url_type) = target_type
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

fn is_resource_type(registry: &Registry, name: &str) -> bool {
    registry
        .core(name)
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
    contained: &[ContainedId],
    in_contained: bool,
    registry: &Registry,
) -> Option<(&'static str, String)> {
    if let Some(id) = reference.strip_prefix('#') {
        // ref-1: "#" alone points at the container (`%rootResource != %resource`).
        if id.is_empty() {
            let message = "Reference '#' points at a containing resource, but this is the root";
            return (!in_contained).then(|| ("REFERENCE_002", message.to_owned()));
        }
        return (!contained.iter().any(|(c, _)| c.as_ref() == id)).then(|| {
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
