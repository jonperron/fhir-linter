//! Rules on StructureDefinition instances themselves (profiles, extension and type
//! definitions), beyond the generic structure and invariant checks every resource gets.
//!
//! elementdefinition.html, ElementDefinition.meaningWhenMissing: "Implicit meanings for
//! missing values can only be specified on a resource, data type, or extension definition,
//! and never in a profile that applies to one of these."
//!
//! profiling.html, Slice Cardinality: when an element of cardinality m..n is sliced, "The
//! maximum cardinality on each slice cannot exceed n" and "The sum of the minimum
//! cardinalities must be less or equal to n". ElementDefinition eld-2: "Min <= Max", which
//! also binds a slice's child against the unsliced child it narrows.
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
    if &*resource.resource_type != "StructureDefinition" {
        return;
    }
    // Only the differential: a profile's snapshot legitimately repeats the base's meanings.
    let Some(Value::Object(differential)) = resource.fields.get("differential").map(|n| &n.value)
    else {
        return;
    };
    let differential: Vec<(usize, &Fields)> = elements(differential.get("element"))
        .enumerate()
        .filter_map(|(index, element)| match &element.value {
            Value::Object(fields) => Some((index, fields)),
            _ => None,
        })
        .collect();
    if is_profile(resource, registry) {
        meaning_when_missing(&differential, path, diagnostics);
    }
    slice_cardinality(resource, registry, &differential, path, diagnostics);
}

type Fields = IndexMap<Arc<str>, Node>;

fn meaning_when_missing(
    differential: &[(usize, &Fields)],
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for &(index, fields) in differential {
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

/// Slices against their slicer's maximum (DEFINITION_002), and slice children against the
/// maximum of the unsliced child they narrow (DEFINITION_003).
fn slice_cardinality(
    resource: &Resource,
    registry: &Registry,
    differential: &[(usize, &Fields)],
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let base = string(&resource.fields, "baseDefinition")
        .map(|url| url.split_once('|').map_or(url, |(url, _)| url))
        .and_then(|url| registry.structure_definition(url));
    let by_id = |id: &str| {
        differential
            .iter()
            .find(|(_, fields)| string(fields, "id") == Some(id))
    };
    for &(index, element) in differential {
        // shortcut: slices are matched by id, so elements without one are skipped; match by
        // path and sliceName if id-less differentials turn up.
        let (Some(id), Some(element_path)) = (string(element, "id"), string(element, "path"))
        else {
            continue;
        };
        if let Some(slicing) = element.get("slicing") {
            // An unstated maximum is the base's: the unsliced element of the same path there.
            let max_text = string(element, "max").or_else(|| {
                base.and_then(|sd| sd.element(element_path))
                    .map(|element| &*element.max)
            });
            if let Some(max) = max_text.and_then(bound) {
                let slicer = Slicer {
                    index,
                    id,
                    max,
                    slicing,
                };
                slices_within(differential, &slicer, path, diagnostics);
            }
        }
        // A slice child: the last segment names no slice, an earlier one does.
        let Some(min) = min(element) else {
            continue;
        };
        if id.rsplit('.').next().unwrap_or(id).contains(':') || !id.contains(':') {
            continue;
        }
        let unsliced: Vec<&str> = id
            .split('.')
            .map(|segment| segment.split(':').next().unwrap_or(segment))
            .collect();
        let unsliced = unsliced.join(".");
        // shortcut: only an unsliced child stated in this differential is compared; a bound
        // inherited from the base is a constraint-loosening check, not this rule.
        let Some(max) = by_id(&unsliced).and_then(|(_, fields)| string(fields, "max")) else {
            continue;
        };
        if bound(max).is_some_and(|max| min > max) {
            diagnostics.push(
                Diagnostic::error(
                    "DEFINITION_003",
                    format!(
                        "No instance can meet the cardinality rules for {id}: the minimum \
                         {min} is greater than the maximum {max} (from {unsliced})"
                    ),
                    format!("{path}.differential.element[{index}].min"),
                )
                .with_location(element["min"].span),
            );
        }
    }
}

/// A sliced element of the differential, with its finite maximum.
struct Slicer<'a> {
    index: usize,
    id: &'a str,
    max: u64,
    slicing: &'a Node,
}

/// The slices of `slicer`: each slice's maximum, then the total of their minimums.
fn slices_within(
    differential: &[(usize, &Fields)],
    slicer: &Slicer,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let &Slicer {
        index,
        id,
        max,
        slicing,
    } = slicer;
    // shortcut: reslices (`slice/reslice`) are left out; they belong to their slice.
    let prefix = format!("{id}:");
    let slices = differential.iter().filter(|(_, fields)| {
        string(fields, "id")
            .and_then(|slice_id| slice_id.strip_prefix(&prefix))
            .is_some_and(|name| !name.contains(['.', ':', '/']))
    });
    let mut total = 0u64;
    for &(slice_index, slice) in slices {
        total = total.saturating_add(min(slice).unwrap_or(0));
        let Some(slice_max) = string(slice, "max") else {
            continue;
        };
        let exceeds = slice_max == "*" || bound(slice_max).is_some_and(|slice_max| slice_max > max);
        if exceeds {
            let slice_id = string(slice, "id").unwrap_or(id);
            diagnostics.push(
                Diagnostic::error(
                    "DEFINITION_002",
                    format!(
                        "The slice {slice_id} allows up to {slice_max} repetitions, more than \
                         the {max} that {id} allows"
                    ),
                    format!("{path}.differential.element[{slice_index}].max"),
                )
                .with_location(slice["max"].span),
            );
        }
    }
    if total > max {
        diagnostics.push(
            Diagnostic::error(
                "DEFINITION_002",
                format!(
                    "The slices of {id} need at least {total} repetitions between them (the \
                     total of their minimum cardinalities), but {id} allows at most {max}"
                ),
                format!("{path}.differential.element[{index}]"),
            )
            .with_location(slicing.span),
        );
    }
}

/// A stated `min`: an integer in JSON, a string in XML.
fn min(fields: &Fields) -> Option<u64> {
    match &fields.get("min")?.value {
        Value::Integer(min) => u64::try_from(*min).ok(),
        Value::Str(min) => min.parse().ok(),
        _ => None,
    }
}

/// A finite maximum; `*` (unbounded) and malformed values give none.
fn bound(max: &str) -> Option<u64> {
    max.parse().ok()
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
