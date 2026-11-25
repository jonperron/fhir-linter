//! Element lookups in a StructureDefinition shared by the validation layers.
use fhir_definitions::{ElementDefinition, Registry, StructureDefinition, capitalise};
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
