//! Element lookups in a StructureDefinition, and instance node accessors, shared by the
//! validation layers.
use fhir_definitions::{ElementDefinition, Registry, StructureDefinition, capitalise};
use fhir_parser::{Node, Value};
use indexmap::IndexMap;
use std::sync::Arc;

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

/// The StructureDefinition of `ty` when it is a complex datatype.
pub(crate) fn complex_type<'a>(
    ty: &str,
    registry: &'a Registry,
) -> Option<&'a StructureDefinition> {
    registry
        .core(ty)
        .filter(|t| t.kind.as_ref() == "complex-type")
}

/// Split a canonical reference into the URL the Registry indexes and its `|version`
/// (references.html, Canonical URLs). A `#fragment` names a resource contained in the
/// target, which the Registry does not index: the reference is then returned whole, so the
/// lookup misses.
pub(crate) fn canonical_parts(reference: &str) -> (&str, Option<&str>) {
    match reference.split_once('|') {
        Some((url, version)) if !reference.contains('#') => (url, Some(version)),
        _ => (reference, None),
    }
}

/// The items of `node` with their instance paths: each element of an array by index, or a
/// single value (XML grouping, or a non-repeating element) at `path` itself.
pub(crate) fn items<'n>(node: &'n Node, path: &str) -> Vec<(&'n Node, String)> {
    indexed(node, path, true)
}

/// The non-null occurrences of `node`: its [`items`] without the nulls.
pub(crate) fn occurrences<'n>(node: &'n Node, path: &str) -> Vec<(&'n Node, String)> {
    indexed(node, path, false)
}

/// The items of `node`, nulls dropped before their paths are built unless `keep_nulls`.
fn indexed<'n>(node: &'n Node, path: &str, keep_nulls: bool) -> Vec<(&'n Node, String)> {
    let keep = |item: &Node| keep_nulls || !matches!(item.value, Value::Null);
    match &node.value {
        Value::Array(all) => all
            .iter()
            .enumerate()
            .filter(|(_, item)| keep(item))
            .map(|(i, item)| (item, format!("{path}[{i}]")))
            .collect(),
        _ if keep(node) => vec![(node, path.to_owned())],
        _ => Vec::new(),
    }
}

/// The string value of child `name`, if it has one.
pub(crate) fn string<'a>(fields: &'a IndexMap<Arc<str>, Node>, name: &str) -> Option<&'a str> {
    match &fields.get(name)?.value {
        Value::Str(s) => Some(s),
        _ => None,
    }
}

/// The string value of child `name` of an object `node`, if it has one.
pub(crate) fn node_string<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    match &node.value {
        Value::Object(fields) => string(fields, name),
        _ => None,
    }
}

#[cfg(test)]
mod canonical_tests {
    use super::canonical_parts;

    #[test]
    fn canonical_parts_split_version_and_keep_fragments_whole() {
        let url = "http://x.org/sd";
        assert_eq!(canonical_parts(url), (url, None));
        assert_eq!(canonical_parts("http://x.org/sd|1.0"), (url, Some("1.0")));
        let contained = "http://x.org/sd|1.0#inner";
        assert_eq!(canonical_parts(contained), (contained, None));
        assert_eq!(canonical_parts("#inner"), ("#inner", None));
    }
}

#[cfg(test)]
mod node_tests {
    use super::{items, occurrences};
    use fhir_parser::{Node, Span, Value};
    use std::sync::Arc;

    fn node(value: Value) -> Node {
        Node {
            value,
            span: Span::default(),
        }
    }

    fn paths(found: Vec<(&Node, String)>) -> Vec<String> {
        found.into_iter().map(|(_, path)| path).collect()
    }

    #[test]
    fn occurrences_drop_nulls_and_keep_array_indices() {
        let array = node(Value::Array(vec![
            node(Value::Str(Arc::from("a"))),
            node(Value::Null),
            node(Value::Str(Arc::from("b"))),
        ]));
        assert_eq!(paths(items(&array, "x")), ["x[0]", "x[1]", "x[2]"]);
        assert_eq!(paths(occurrences(&array, "x")), ["x[0]", "x[2]"]);
        assert_eq!(paths(items(&node(Value::Null), "x")), ["x"]);
        assert!(occurrences(&node(Value::Null), "x").is_empty());
    }
}
