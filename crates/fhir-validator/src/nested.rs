use fhir_definitions::{ElementDefinition, Registry, StructureDefinition};
use fhir_fhirpath::{Collection, Value as FhirPathValue};
use fhir_parser::{Node, Resource, Span, Value};
use indexmap::IndexMap;
use std::sync::Arc;

use crate::diagnostic::Diagnostic;
use crate::type_check::{check_type, is_valid_id};
use crate::{invariants, references, terminology};

/// What every level of the walk shares.
struct Walk<'a> {
    registry: &'a Registry,
    /// JSON input: array shapes are meaningful (`STRUCTURE_002`).
    json: bool,
    /// Ids of the root's contained resources, for `#id` references.
    contained_ids: Vec<&'a str>,
}

/// Validate the resource's elements at every depth, starting at the top level.
///
/// Applies the structural (`STRUCTURE_001`, and `STRUCTURE_002` array shape when `json`),
/// cardinality (`CARDINALITY_001/002/003`), primitive type (`TYPE_*`), terminology
/// (`TERMINOLOGY_*`), reference (`REFERENCE_*`) and invariant (`INVARIANT_*`) checks
/// recursively into backbone elements and complex types. Each node gets the constraints of
/// its element and of its type's root element (`qty-3` on every Quantity), once per key.
/// The root's own invariants and the profile layer run outside the walk.
///
/// shortcut: `contentReference` elements (skipped silently) and contained/embedded resources are
/// not descended into; upgrade when recursive structures need validating.
pub fn validate(
    resource: &Resource,
    sd: &StructureDefinition,
    registry: &Registry,
    json: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let resource_type = resource.resource_type.as_ref();

    // The parser lifts `id` out of `fields` (already rejecting non-strings), without a span.
    if let Some(id) = resource.id.as_deref().filter(|id| !is_valid_id(id)) {
        let path = format!("{resource_type}.id");
        diagnostics.push(Diagnostic::error(
            "TYPE_005",
            format!("Invalid id format at '{path}': '{id}'"),
            path,
        ));
    }
    if let Some(warning) = resource
        .id
        .as_deref()
        .and_then(|id| crate::type_check::control_char_warning(id, &format!("{resource_type}.id")))
    {
        diagnostics.push(warning);
    }

    let context = Walk {
        registry,
        json,
        contained_ids: references::contained_ids(resource, registry),
    };
    // Converted once; each object's FHIRPath value is handed down the walk (see `descend`).
    let root = fhir_fhirpath::resource_to_typed_value(resource, &|parent, name| {
        registry.child_element(parent, name)
    });
    walk(
        &resource.fields,
        typed_fields(std::slice::from_ref(&root)),
        sd,
        resource_type,
        &context,
        resource_type,
        diagnostics,
    );
}

/// The FHIRPath children of a single object value.
fn typed_fields(value: &[FhirPathValue]) -> Option<&IndexMap<Arc<str>, Collection>> {
    match value {
        [FhirPathValue::Object(fields)] => Some(fields),
        _ => None,
    }
}

/// Names with path punctuation are never valid element names.
fn has_punctuation(name: &str) -> bool {
    name.contains(['.', '[', ']'])
}

/// Find the element for `name` under `parent_path`, and the single type it has
/// for this instance (the suffix picks the type of a `[x]` element). The type is
/// `None` for typeless elements such as `contentReference`; `None` overall means not found.
pub(crate) fn resolve<'a>(
    sd: &'a StructureDefinition,
    path: &str,
    name: &str,
) -> Option<(&'a ElementDefinition, Option<Arc<str>>)> {
    if has_punctuation(name) {
        return None;
    }
    if let Some(e) = sd.element(path) {
        return Some((e, e.types.first().cloned()));
    }
    let parent = path.strip_suffix(name)?;
    // A type suffix is capitalised, so a choice can only split before an uppercase letter.
    (1..name.len())
        .filter(|&i| name.as_bytes()[i].is_ascii_uppercase())
        .find_map(|i| {
            let (base, suffix) = name.split_at(i);
            let e = sd.element(&format!("{parent}{base}[x]"))?;
            let ty = e.types.iter().find(|t| capitalise(t) == suffix)?;
            Some((e, Some(ty.clone())))
        })
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// Check the value of `elem` (instance `path`), recursing into objects.
///
/// `typed` is the node's FHIRPath value as converted with its parent, when known: an object
/// converts to exactly one value, so an array of objects aligns item by item. Anything else
/// (nulls, a stray `_name`) is converted here instead, so each subtree is converted once.
#[allow(clippy::too_many_arguments)] // recursion state threaded through walk/descend
fn descend(
    node: &Node,
    typed: Option<&[FhirPathValue]>,
    elem: &ElementDefinition,
    ty: &str,
    sd: &StructureDefinition,
    context: &Walk<'_>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let registry = context.registry;
    match &node.value {
        Value::Array(items) => {
            let aligned = typed.filter(|typed| {
                typed.len() == items.len()
                    && items.iter().all(|n| matches!(n.value, Value::Object(_)))
            });
            for (i, item) in items.iter().enumerate() {
                descend(
                    item,
                    aligned.map(|typed| &typed[i..=i]),
                    elem,
                    ty,
                    sd,
                    context,
                    &format!("{path}[{i}]"),
                    diagnostics,
                );
            }
        }
        Value::Object(fields) => {
            let backbone = ty == "BackboneElement" || ty == "Element";
            let type_sd = complex_type(ty, registry);
            if !backbone && type_sd.is_none() {
                return;
            }
            let converted;
            let value = match typed.filter(|typed| typed.len() == 1) {
                Some(typed) => typed,
                None => {
                    // Same definition path as `Registry::child_element`, so both conversions agree.
                    let definition = if backbone { &elem.path } else { ty };
                    converted = node_value(Some(node), None, definition, ty, registry);
                    &converted
                }
            };
            check_invariants(value, elem, ty, path, node.span, registry, diagnostics);
            if ty == "Reference" {
                references::check(node, path, &context.contained_ids, registry, diagnostics);
            }
            let typed = typed_fields(value);
            match type_sd.filter(|_| !backbone) {
                Some(type_sd) => walk(
                    fields,
                    typed,
                    type_sd,
                    &type_sd.name,
                    context,
                    path,
                    diagnostics,
                ),
                None => walk(fields, typed, sd, &elem.path, context, path, diagnostics),
            }
        }
        _ if ty == "BackboneElement" || ty == "Element" || complex_type(ty, registry).is_some() => {
            diagnostics.push(
                Diagnostic::error(
                    "TYPE_001",
                    format!("Expected {ty} object at '{path}'"),
                    path.to_owned(),
                )
                .with_location(node.span),
            );
        }
        _ => {}
    }
}

/// The StructureDefinition of `ty` when it is a complex datatype.
pub(crate) fn complex_type<'a>(
    ty: &str,
    registry: &'a Registry,
) -> Option<&'a StructureDefinition> {
    registry
        .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{ty}"))
        .filter(|t| t.kind.as_ref() == "complex-type")
}

/// Validate the children of one object against the elements under `def_path`.
fn walk(
    fields: &IndexMap<Arc<str>, Node>,
    typed: Option<&IndexMap<Arc<str>, Collection>>,
    sd: &StructureDefinition,
    def_path: &str,
    context: &Walk<'_>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_children_cardinality(fields, sd, def_path, path, diagnostics);

    let prefix = format!("{def_path}.");

    for (name, node) in fields {
        let name = name.as_ref();
        let child_path = format!("{path}.{name}");
        // `_name` carries the id/extensions of primitive `name`, so it must name a primitive.
        if let Some(bare) = name.strip_prefix('_') {
            let primitive = resolve(sd, &format!("{prefix}{bare}"), bare)
                .and_then(|(elem, ty)| Some((elem, ty.filter(|t| is_primitive(t))?)));
            if let Some((elem, ty)) = primitive {
                // A primitive with a value is checked with its `_name` when the value is reached.
                if !fields.contains_key(bare) {
                    let bare_path = format!("{path}.{bare}");
                    check_primitive(
                        None,
                        Some(node),
                        elem,
                        &ty,
                        &bare_path,
                        context,
                        diagnostics,
                    );
                }
                walk_primitive_extension(node, context, &child_path, diagnostics);
                continue;
            }
        }
        let Some((elem, ty)) = resolve(sd, &format!("{prefix}{name}"), name) else {
            diagnostics.push(
                Diagnostic::error(
                    "STRUCTURE_001",
                    format!("Unknown element '{child_path}'"),
                    child_path,
                )
                .with_location(node.span),
            );
            continue;
        };
        if context.json {
            check_array_shape(node, elem, &child_path, diagnostics);
        }
        // contentReference element: no type to check here.
        let Some(ty) = ty else { continue };

        // Element.id is a `string`; R5 snapshots label many as `id` in the fhir-type extension.
        let check_as = if name == "id" { "string" } else { ty.as_ref() };
        check_type(&node.value, check_as, &child_path, node.span, diagnostics);
        terminology::check(node, elem, &ty, &child_path, context.registry, diagnostics);
        if is_primitive(&ty) {
            let extension = fields.get(format!("_{name}").as_str());
            check_primitive(
                Some(node),
                extension,
                elem,
                &ty,
                &child_path,
                context,
                diagnostics,
            );
        }
        let typed = typed.and_then(|typed| typed.get(name)).map(Vec::as_slice);
        descend(
            node,
            typed,
            elem,
            &ty,
            sd,
            context,
            &child_path,
            diagnostics,
        );
    }
}

/// Evaluate the invariants of primitive element `elem` (type `ty`, instance `path`) on each
/// value, paired by index with its `_name` element (R5 json.html: the two arrays align).
fn check_primitive(
    value: Option<&Node>,
    extension: Option<&Node>,
    elem: &ElementDefinition,
    ty: &str,
    path: &str,
    context: &Walk<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    fn items(node: Option<&Node>) -> Vec<&Node> {
        match node.map(|n| &n.value) {
            Some(Value::Array(items)) => items.iter().collect(),
            _ => node.into_iter().collect(),
        }
    }
    let repeats = [value, extension]
        .into_iter()
        .flatten()
        .any(|n| matches!(n.value, Value::Array(_)));
    let (values, extensions) = (items(value), items(extension));
    for i in 0..values.len().max(extensions.len()) {
        let (value, extension) = (values.get(i).copied(), extensions.get(i).copied());
        let Some(span) = value.or(extension).map(|n| n.span) else {
            continue;
        };
        let typed = node_value(value, extension, ty, ty, context.registry);
        let item_path = match repeats {
            true => format!("{path}[{i}]"),
            false => path.to_owned(),
        };
        check_invariants(
            &typed,
            elem,
            ty,
            &item_path,
            span,
            context.registry,
            diagnostics,
        );
    }
}

/// The FHIRPath value of one node (and its `_name` element) whose element has definition
/// path `definition` and type `ty`.
fn node_value(
    value: Option<&Node>,
    extension: Option<&Node>,
    definition: &str,
    ty: &str,
    registry: &Registry,
) -> Collection {
    fhir_fhirpath::node_to_typed_value(
        value.map(|n| &n.value),
        extension.map(|n| &n.value),
        Some(&(Arc::from(definition), Arc::from(ty))),
        &|parent, name| registry.child_element(parent, name),
    )
}

/// Evaluate on one node (instance `path`) the constraints of its element and of its type's
/// root element, each key once: a snapshot element repeats `ele-1` but not `qty-3`.
fn check_invariants(
    value: &[FhirPathValue],
    elem: &ElementDefinition,
    ty: &str,
    path: &str,
    span: Span,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // A JSON null or a value-less, extension-less item is no node to constrain.
    if value.is_empty() {
        return;
    }
    let type_constraints = registry
        .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{ty}"))
        .and_then(|type_sd| type_sd.element(ty))
        .map(|root| root.constraints.as_slice())
        .unwrap_or_default();
    let inherited = type_constraints
        .iter()
        .filter(|c| !elem.constraints.iter().any(|own| own.key == c.key));
    for constraint in elem.constraints.iter().chain(inherited) {
        invariants::check(
            constraint,
            value,
            &|base, suffix| registry.is_choice_variant(base, suffix),
            path,
            Some(span),
            diagnostics,
        );
    }
}

/// Check a `_name` primitive extension as an `Element` (an array of them, nulls allowed,
/// for a repeating primitive), so its `id` and `extension` are validated.
fn walk_primitive_extension(
    node: &Node,
    context: &Walk<'_>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(element_sd) = complex_type("Element", context.registry) else {
        return;
    };
    match &node.value {
        // Small subtrees (`id`, `extension`): their children convert where they are checked.
        Value::Object(fields) => walk(
            fields,
            None,
            element_sd,
            "Element",
            context,
            path,
            diagnostics,
        ),
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                let item_path = format!("{path}[{i}]");
                match &item.value {
                    Value::Object(fields) => walk(
                        fields,
                        None,
                        element_sd,
                        "Element",
                        context,
                        &item_path,
                        diagnostics,
                    ),
                    Value::Null => {}
                    _ => push_not_element(item, &item_path, diagnostics),
                }
            }
        }
        _ => push_not_element(node, path, diagnostics),
    }
}

fn push_not_element(node: &Node, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(
        Diagnostic::error(
            "TYPE_001",
            format!("Expected Element object at '{path}'"),
            path.to_owned(),
        )
        .with_location(node.span),
    );
}

/// Emit `STRUCTURE_002` when a JSON value's shape contradicts the base definition:
/// a repeating element (`max != "1"`) must be an array, a single one must not be.
///
/// shortcut: `_name` companion arrays are not shape-checked; upgrade with the `_name` rules.
/// shortcut: uses `elem.max`, which equals the base max in every core SD walked here; switch
/// to the base max if `walk` ever runs on constrained (profile) snapshots.
fn check_array_shape(
    node: &Node,
    elem: &ElementDefinition,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let repeating = elem.max.as_ref() != "1";
    let is_array = matches!(node.value, Value::Array(_));
    if is_array == repeating || matches!(node.value, Value::Null) {
        return;
    }
    let message = if repeating {
        format!(
            "Element '{path}' repeats (max={}) and must be a JSON array",
            elem.max
        )
    } else {
        format!("Element '{path}' has max=1 and must not be a JSON array")
    };
    diagnostics.push(
        Diagnostic::error("STRUCTURE_002", message, path.to_owned()).with_location(node.span),
    );
}

/// Check the cardinality of every child of `def_path` in one object (instance `path`).
pub(crate) fn check_children_cardinality(
    fields: &IndexMap<Arc<str>, Node>,
    sd: &StructureDefinition,
    def_path: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let prefix = format!("{def_path}.");
    for elem in sd.descendants(def_path) {
        let Some(rest) = elem.path.strip_prefix(&prefix).filter(|r| !r.contains('.')) else {
            continue;
        };
        // shortcut: slices (choice slices too) are not matched to instances, so a slice's
        // own cardinality is not enforced; revisit when slicing discriminators are evaluated.
        if elem.slice_name.is_some() {
            continue;
        }
        if rest.ends_with("[x]") {
            check_choice(fields, sd, elem, &prefix, path, diagnostics);
            continue;
        }
        let node = present(fields, elem, rest);
        // An absent optional element cannot fail; skip building its path.
        if node.is_none() && elem.min == 0 {
            if let Some(empty) = fields.get(rest) {
                push_empty_array(empty, &format!("{path}.{rest}"), diagnostics);
            }
            continue;
        }
        check_cardinality(elem, node, &format!("{path}.{rest}"), diagnostics);
    }
}

/// Check a `[x]` choice element as a whole: its variants are the fields `resolve`
/// maps to it (a `_variant` counts only for a primitive type), and it holds at most one.
///
/// shortcut: profile type restrictions are not handled (a disallowed variant reads as
/// missing) and choice slices are skipped by the caller; revisit when profile snapshots
/// are walked.
fn check_choice(
    fields: &IndexMap<Arc<str>, Node>,
    sd: &StructureDefinition,
    elem: &ElementDefinition,
    prefix: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let variants: Vec<(&str, &Node)> = fields
        .iter()
        .filter_map(|(key, node)| {
            let name = key.strip_prefix('_');
            let bare = name.unwrap_or(key);
            let (e, ty) = resolve(sd, &format!("{prefix}{bare}"), bare)?;
            // A `_variant` beside its value is the same variant, counted once via the value.
            let extension_only = name.is_none_or(|n| {
                !fields.contains_key(n)
                    && ty.as_deref().is_some_and(is_primitive)
                    && is_extension_shape(node)
            });
            (e.path == elem.path && extension_only).then_some((bare, node))
        })
        .collect();
    match variants.as_slice() {
        [] => {
            let choice_path = format!("{path}.{}", elem.path.strip_prefix(prefix).unwrap_or(""));
            check_cardinality(elem, None, &choice_path, diagnostics);
        }
        [(name, node)] => {
            check_cardinality(elem, Some(node), &format!("{path}.{name}"), diagnostics)
        }
        [(first, _), (second, node), ..] => {
            let second_path = format!("{path}.{second}");
            diagnostics.push(
                Diagnostic::error(
                    "CARDINALITY_002",
                    format!("Choice element '{second_path}' conflicts with '{first}'"),
                    second_path,
                )
                .with_location(node.span),
            );
        }
    }
}

/// Primitive FHIR type names start with a lowercase letter.
pub(crate) fn is_primitive(ty: &str) -> bool {
    ty.starts_with(|c: char| c.is_ascii_lowercase())
}

/// An array holding no value: empty or only nulls (R5 json.html: "Objects are never empty").
fn is_empty_array(node: &Node) -> bool {
    let Value::Array(items) = &node.value else {
        return false;
    };
    items.iter().all(|item| matches!(item.value, Value::Null))
}

/// Warn (`CARDINALITY_003`) that an optional element holds an empty or all-null array.
fn push_empty_array(node: &Node, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(
        Diagnostic::warning(
            "CARDINALITY_003",
            format!("Element '{path}' is an array with no values and should be omitted"),
            path.to_owned(),
        )
        .with_location(node.span),
    );
}

/// The node that makes `elem` present: its value, or the `_name` primitive extension
/// (which stands in for an absent value only on primitive types, named in lowercase).
/// An empty or all-null value array is no value.
fn present<'a>(
    fields: &'a IndexMap<Arc<str>, Node>,
    elem: &ElementDefinition,
    name: &str,
) -> Option<&'a Node> {
    fields
        .get(name)
        .filter(|node| !is_empty_array(node))
        .or_else(|| {
            // Only primitives take a `_name`; skip building the key for the rest.
            let primitive = elem.types.first().is_some_and(|t| is_primitive(t));
            primitive
                .then(|| fields.get(format!("_{name}").as_str()))
                .flatten()
                .filter(|node| is_extension_shape(node))
        })
}

/// A `_name` primitive extension is an object, or an array of objects and nulls
/// holding at least one object.
fn is_extension_shape(node: &Node) -> bool {
    match &node.value {
        Value::Object(_) => true,
        Value::Array(items) => {
            items
                .iter()
                .all(|item| matches!(item.value, Value::Object(_) | Value::Null))
                && items
                    .iter()
                    .any(|item| matches!(item.value, Value::Object(_)))
        }
        _ => false,
    }
}

/// Emit `CARDINALITY_001` when a required element is absent and `CARDINALITY_002`
/// when it holds more values than a finite maximum.
fn check_cardinality(
    elem: &ElementDefinition,
    node: Option<&Node>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(node) = node else {
        if elem.min > 0 {
            diagnostics.push(Diagnostic::error(
                "CARDINALITY_001",
                format!("Required element '{path}' is missing (min={})", elem.min),
                path.to_owned(),
            ));
        }
        return;
    };
    let Ok(max) = elem.max.parse::<usize>() else {
        return;
    };
    let count = match &node.value {
        Value::Array(items) => items.len(),
        Value::Null => 0,
        _ => 1,
    };
    if count > max {
        diagnostics.push(
            Diagnostic::error(
                "CARDINALITY_002",
                format!("Element '{path}' has {count} value(s) but maximum is {max}"),
                path.to_owned(),
            )
            .with_location(node.span),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elem(path: &str, min: u32) -> ElementDefinition {
        ElementDefinition {
            path: Arc::from(path),
            min,
            max: Arc::from("1"),
            types: vec![],
            constraints: vec![],
            binding: None,
            slice_name: None,
            xml_attr: false,
        }
    }

    #[test]
    fn children_cardinality_checks_direct_children_only() {
        let sd = StructureDefinition::new(
            Arc::from("http://example.org/StructureDefinition/obs-profile"),
            Arc::from("ObsProfile"),
            Arc::from("resource"),
            false,
            vec![
                elem("Observation", 0),
                elem("Observation.subject", 1),
                elem("Observation.value[x]", 1),
                elem("Observation.component.code", 1),
            ],
        );
        let mut found = vec![];
        check_children_cardinality(
            &IndexMap::new(),
            &sd,
            "Observation",
            "Observation",
            &mut found,
        );
        let found: Vec<_> = found
            .iter()
            .map(|d| (d.code.as_ref(), d.path.as_str()))
            .collect();
        // A missing required choice is reported at its placeholder path.
        assert_eq!(
            found,
            [
                ("CARDINALITY_001", "Observation.subject"),
                ("CARDINALITY_001", "Observation.value[x]"),
            ],
        );
    }

    #[test]
    fn children_cardinality_skips_choice_slices() {
        let slice = ElementDefinition {
            slice_name: Some(Arc::from("valueQuantity")),
            ..elem("Observation.value[x]", 1)
        };
        let sd = StructureDefinition::new(
            Arc::from("http://example.org/StructureDefinition/obs-profile"),
            Arc::from("ObsProfile"),
            Arc::from("resource"),
            false,
            vec![
                elem("Observation", 0),
                elem("Observation.value[x]", 0),
                slice,
            ],
        );
        let mut found = vec![];
        check_children_cardinality(
            &IndexMap::new(),
            &sd,
            "Observation",
            "Observation",
            &mut found,
        );
        assert!(found.is_empty(), "{found:?}");
    }
}
