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
//!
//! ElementDefinition.contentReference: "ContentReferences can only be defined in
//! specializations, not constrained types, and they cannot be changed".
//!
//! ElementDefinition.orderMeaning: "if absent in the base type, a profile cannot assert
//! meaning".
//!
//! ElementDefinition.isModifier: "Once set, it cannot be changed in derived profiles (except
//! in the special case of the defining a new extension)."
//!
//! Extension contentReferenceProfile: "The value refers to an element definition in a
//! profile, not to the profile itself, using the same format as the contentReference (a
//! canonical URL for the profile, followed by '#' and the element id)."
//!
//! references.html, Canonical URLs: only the resources that have canonical URLs "are allowed
//! to be the target of references to a canonical URLs".
use fhir_definitions::{ElementDefinition, Registry, StructureDefinition};
use fhir_parser::{Node, Resource, Value};
use indexmap::IndexMap;
use std::collections::HashSet;
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
    extension_contexts(resource, registry, path, diagnostics);
    content_reference_profiles(resource, registry, path, diagnostics);
    // Only the differential: a profile's snapshot legitimately repeats the base's meanings.
    let Some(Value::Object(differential)) = resource.fields.get("differential").map(|n| &n.value)
    else {
        return;
    };
    let differential = element_fields(differential.get("element"));
    if is_profile(resource, registry) {
        meaning_when_missing(&differential, path, diagnostics);
        if let Some(base) = base_definition(resource, registry) {
            modifiers(base, &differential, path, diagnostics);
        }
    }
    slice_cardinality(resource, registry, &differential, path, diagnostics);
    // Constraints only: the R5 core specialization NutritionIntake itself lists the
    // non-canonical ClinicalUseDefinition as an instantiatesCanonical target.
    if string(&resource.fields, "derivation") == Some("constraint") {
        canonical_targets(registry, &differential, path, diagnostics);
        if let Some(base) = base_definition(resource, registry) {
            content_references(base, &differential, path, diagnostics);
            order_meanings(base, &differential, path, diagnostics);
        }
    }
}

/// The element path a contentReference names: what follows its last `#`.
fn reference_target(reference: &str) -> &str {
    reference
        .rsplit_once('#')
        .map_or(reference, |(_, target)| target)
}

/// orderMeaning asserted by a constraint on an element whose base has none
/// (DEFINITION_007). ElementDefinition.orderMeaning: "This element can only be asserted on
/// repeating elements and can only be introduced when defining resources or data types. It
/// can be further refined profiled elements but if absent in the base type, a profile cannot
/// assert meaning."
fn order_meanings(
    base: &StructureDefinition,
    differential: &[(usize, &Fields)],
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for &(index, fields) in differential {
        let (Some(node), Some(element)) = (fields.get("orderMeaning"), base_element(base, fields))
        else {
            continue;
        };
        if element.order_meaning {
            continue;
        }
        diagnostics.push(
            Diagnostic::error(
                "DEFINITION_007",
                format!(
                    "The element definition for {} asserts an orderMeaning its base does not \
                     have: if absent in the base type, a profile cannot assert meaning",
                    element.path
                ),
                format!("{path}.differential.element[{index}].orderMeaning"),
            )
            .with_location(node.span),
        );
    }
}

/// isModifier changed by a profile (DEFINITION_008). ElementDefinition.isModifier: "Only the
/// definition of an element can set IsModifier true - either the specification itself or
/// where an extension is originally defined. Once set, it cannot be changed in derived
/// profiles (except in the special case of the defining a new extension)."
fn modifiers(
    base: &StructureDefinition,
    differential: &[(usize, &Fields)],
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for &(index, fields) in differential {
        let (Some(node), Some(element)) = (fields.get("isModifier"), base_element(base, fields))
        else {
            continue;
        };
        // A boolean in JSON; XML text the reshape could not type stays a string.
        let stated = match &node.value {
            Value::Bool(stated) => *stated,
            Value::Str(text) if matches!(&**text, "true" | "false") => &**text == "true",
            _ => continue,
        };
        if stated == element.is_modifier {
            continue;
        }
        diagnostics.push(
            Diagnostic::error(
                "DEFINITION_008",
                format!(
                    "The element definition for {} sets isModifier to {stated} but its base \
                     has {}: once set, isModifier cannot be changed in derived profiles",
                    element.path, element.is_modifier
                ),
                format!("{path}.differential.element[{index}].isModifier"),
            )
            .with_location(node.span),
        );
    }
}

/// The StructureDefinition `baseDefinition` names (any `|version` dropped), when the
/// Registry holds it.
fn base_definition<'a>(
    resource: &Resource,
    registry: &'a Registry,
) -> Option<&'a StructureDefinition> {
    let url = string(&resource.fields, "baseDefinition")?;
    registry.structure_definition(url.split_once('|').map_or(url, |(url, _)| url))
}

/// The base element of the same path as differential element `fields`. Elements the base
/// does not hold under that path (a renamed choice, a path-less element) give none, so the
/// checks that compare with the base stay silent on them.
fn base_element<'a>(
    base: &'a StructureDefinition,
    fields: &Fields,
) -> Option<&'a ElementDefinition> {
    base.element(string(fields, "path")?)
}

/// contentReference set or changed by a constraint (DEFINITION_006). ElementDefinition
/// .contentReference: "ContentReferences can only be defined in specializations, not
/// constrained types, and they cannot be changed".
fn content_references(
    base: &StructureDefinition,
    differential: &[(usize, &Fields)],
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for &(index, fields) in differential {
        let (Some(node), Some(element)) =
            (fields.get("contentReference"), base_element(base, fields))
        else {
            continue;
        };
        // Compare the element paths: the Registry keeps a local `#path` as the bare path, and
        // a derived profile's snapshot writes the same target as `<core url>#path`.
        let stated = string(fields, "contentReference").map(reference_target);
        if stated.is_some() && stated == element.content_reference.as_deref().map(reference_target)
        {
            continue;
        }
        diagnostics.push(
            Diagnostic::error(
                "DEFINITION_006",
                format!(
                    "The element definition for {} sets a contentReference its base does not \
                     have: content references can only be defined in specializations, not in \
                     constraints",
                    element.path
                ),
                format!("{path}.differential.element[{index}].contentReference"),
            )
            .with_location(node.span),
        );
    }
}

type Fields = IndexMap<Arc<str>, Node>;

const CONTENT_REFERENCE_PROFILE: &str =
    "http://hl7.org/fhir/StructureDefinition/contentReferenceProfile";

/// `contentReferenceProfile` values that do not name an element of a known profile
/// (DEFINITION_009), in the differential and the snapshot. The extension definition: "The
/// value refers to an element definition in a profile, not to the profile itself, using the
/// same format as the contentReference (a canonical URL for the profile, followed by '#' and
/// the element id)." A snapshot value repeating the differential element of the same id is
/// the same statement and is not reported twice.
fn content_reference_profiles(
    resource: &Resource,
    registry: &Registry,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let views = ["differential", "snapshot"].map(|view| {
        let element = match resource.fields.get(view).map(|n| &n.value) {
            Some(Value::Object(fields)) => fields.get("element"),
            _ => None,
        };
        (view, element_fields(element))
    });
    let differential_values: HashSet<(&str, &str)> = views[0]
        .1
        .iter()
        .filter_map(|&(_, fields)| Some((string(fields, "id")?, fields)))
        .flat_map(|(id, fields)| stated(fields).into_iter().map(move |(_, _, v)| (id, v)))
        .collect();
    let own_url = string(&resource.fields, "url");
    for (view, view_elements) in &views {
        for &(element_index, fields) in view_elements {
            let id = string(fields, "id");
            for (index, extension, value) in stated(fields) {
                if *view == "snapshot"
                    && id.is_some_and(|id| differential_values.contains(&(id, value)))
                {
                    continue;
                }
                let Some(problem) = reference_problem(resource, registry, own_url, &views, value)
                else {
                    continue;
                };
                let owner = string(fields, "path").or(id).unwrap_or("an element");
                diagnostics.push(
                    Diagnostic::error(
                        "DEFINITION_009",
                        format!("The contentReferenceProfile '{value}' on {owner} {problem}"),
                        format!("{path}.{view}.element[{element_index}].extension[{index}]"),
                    )
                    .with_location(extension.span),
                );
            }
        }
    }
}

/// Why `value` names no element of a known profile, or none when it resolves or cannot be
/// decided. A canonical equal to the definition's own `url` resolves against the definition
/// itself (its snapshot and differential ids, then its base), not the Registry.
fn reference_problem(
    resource: &Resource,
    registry: &Registry,
    own_url: Option<&str>,
    views: &[(&str, Vec<(usize, &Fields)>)],
    value: &str,
) -> Option<String> {
    // A canonical's `|version` is a string: it cannot be empty.
    let Some((canonical, id)) = value
        .split_once('#')
        .filter(|(c, id)| !c.is_empty() && !id.is_empty() && !c.ends_with('|'))
    else {
        return Some(
            "is not a valid element reference: the value must be a canonical URL for the \
             profile, followed by '#' and the element id"
                .to_owned(),
        );
    };
    let canonical = canonical.split_once('|').map_or(canonical, |(url, _)| url);
    // shortcut: slice ids are not checked (the Registry indexes elements by path, not id);
    // resolve them once ElementDefinition keeps its id.
    if id.contains(':') {
        return None;
    }
    let found =
        if own_url.map(|url| url.split_once('|').map_or(url, |(url, _)| url)) == Some(canonical) {
            let stated = views
                .iter()
                .flat_map(|(_, elements)| elements)
                .any(|&(_, fields)| string(fields, "id") == Some(id));
            // No declared base: the definition holds all its elements. A declared base the
            // Registry does not hold leaves the answer undecided.
            stated
                || match string(&resource.fields, "baseDefinition") {
                    None => false,
                    Some(_) => base_definition(resource, registry)
                        .is_none_or(|base| base.element(id).is_some()),
                }
        } else if let Some(profile) = registry.structure_definition(canonical) {
            profile.element(id).is_some()
        } else if registry.needs_snapshot(canonical) {
            true
        } else {
            return Some(format!(
                "refers to the profile {canonical}, which could not be found"
            ));
        };
    (!found).then(|| format!("refers to the element {id}, which does not exist in {canonical}"))
}

/// The `contentReferenceProfile` extensions of element `fields`: position, node and value.
fn stated(fields: &Fields) -> Vec<(usize, &Node, &str)> {
    elements(fields.get("extension"))
        .enumerate()
        .filter_map(|(index, extension)| {
            let Value::Object(extension_fields) = &extension.value else {
                return None;
            };
            if string(extension_fields, "url") != Some(CONTENT_REFERENCE_PROFILE) {
                return None;
            }
            Some((index, extension, string(extension_fields, "valueUri")?))
        })
        .collect()
}

/// The object items of an `element` list, with their positions.
fn element_fields(node: Option<&Node>) -> Vec<(usize, &Fields)> {
    elements(node)
        .enumerate()
        .filter_map(|(index, element)| match &element.value {
            Value::Object(fields) => Some((index, fields)),
            _ => None,
        })
        .collect()
}

/// `element` contexts that name no element of the base specification (DEFINITION_005).
/// ExtensionContextType `element`: "any element that has an ElementDefinition.id that
/// matches that found in the expression [...] If there is no #, the Element id is one
/// defined in the base specification."
fn extension_contexts(
    resource: &Resource,
    registry: &Registry,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (index, context) in elements(resource.fields.get("context")).enumerate() {
        let Value::Object(fields) = &context.value else {
            continue;
        };
        let (Some("element"), Some(expression)) =
            (string(fields, "type"), string(fields, "expression"))
        else {
            continue;
        };
        // shortcut: `url#id` contexts and slice ids (`Observation.category:VSCat`, from a core
        // profile) are not checked (the Registry indexes elements by path, not id); resolve
        // them once ElementDefinition keeps its id.
        if expression.contains(['#', ':']) {
            continue;
        }
        // Base specification element ids are their paths.
        let type_name = expression.split('.').next().unwrap_or(expression);
        if registry
            .core(type_name)
            .is_some_and(|sd| sd.element(expression).is_some())
        {
            continue;
        }
        let node = fields.get("expression").unwrap_or(context);
        diagnostics.push(
            Diagnostic::error(
                "DEFINITION_005",
                format!(
                    "The extension context '{expression}' of type element names no element \
                     of the base specification"
                ),
                format!("{path}.context[{index}].expression"),
            )
            .with_location(node.span),
        );
    }
}

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
    let base = base_definition(resource, registry);
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
            // shortcut: the first base element of that path is taken, so a nested slicer inside
            // a narrowed slice may get a larger maximum (check silent, never a false error);
            // a slicer or base missing from the Registry is silent too. Resolve by element id
            // and surface an unresolvable base if those gaps matter.
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
    // u128: a sum of u64 minimums cannot overflow it or clamp to the maximum.
    let mut total = 0u128;
    // shortcut: a slice's unstated min or max is not read from the base, and a slicer
    // declared only in the base is not checked (silent, never a false error); resolve them
    // from the base snapshot if differentials that omit them need checking.
    for &(slice_index, slice) in slices {
        total += u128::from(min(slice).unwrap_or(0));
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
    if total > u128::from(max) {
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

/// Canonical types whose `targetProfile` names a resource without a canonical URL
/// (DEFINITION_004). Targets the Registry does not hold stay silent.
fn canonical_targets(
    registry: &Registry,
    differential: &[(usize, &Fields)],
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for &(index, fields) in differential {
        for (type_index, type_node) in elements(fields.get("type")).enumerate() {
            let Value::Object(type_fields) = &type_node.value else {
                continue;
            };
            if string(type_fields, "code") != Some("canonical") {
                continue;
            }
            let targets = elements(type_fields.get("targetProfile")).enumerate();
            for (target_index, target) in targets {
                let Value::Str(url) = &target.value else {
                    continue;
                };
                if !names_non_canonical_resource(registry, url) {
                    continue;
                }
                let element_path = string(fields, "path").unwrap_or("(no path)");
                diagnostics.push(
                    Diagnostic::error(
                        "DEFINITION_004",
                        format!(
                            "The canonical type of {element_path} targets {url}, a resource \
                             without a canonical URL: only canonical resources can be the \
                             target of a canonical"
                        ),
                        format!(
                            "{path}.differential.element[{index}].type[{type_index}]\
                             .targetProfile[{target_index}]"
                        ),
                    )
                    .with_location(target.span),
                );
            }
        }
    }
}

/// True when `url` names a concrete resource definition (or a profile on one) that is not a
/// canonical resource. Abstract definitions such as `Resource` (canonical(Any)) pass.
fn names_non_canonical_resource(registry: &Registry, url: &str) -> bool {
    let url = url.split_once('|').map_or(url, |(url, _)| url);
    registry
        .structure_definition(url)
        .is_some_and(|sd| &*sd.kind == "resource" && !sd.is_abstract && !is_canonical(registry, sd))
}

/// True when the definition, through its `implements` interfaces or its base chain, reaches
/// `CanonicalResource`, or leaves the Registry (undecided, so no finding).
fn is_canonical(registry: &Registry, definition: &StructureDefinition) -> bool {
    let Some(canonical) = registry.core("CanonicalResource") else {
        return true;
    };
    let mut pending = vec![&*definition.url];
    let mut seen = HashSet::new();
    while let Some(url) = pending.pop() {
        if url == &*canonical.url {
            return true;
        }
        if !seen.insert(url) {
            continue;
        }
        // A definition the Registry does not hold leaves the answer undecided: stay silent.
        let Some(sd) = registry.structure_definition(url) else {
            return true;
        };
        pending.extend(sd.implements.iter().map(|url| &**url));
        pending.extend(sd.base_definition.as_deref());
    }
    false
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
