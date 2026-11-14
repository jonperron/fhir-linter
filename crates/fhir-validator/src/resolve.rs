//! Element lookups in a StructureDefinition shared by the validation layers.
use fhir_definitions::{ElementDefinition, Registry, StructureDefinition};
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

pub(crate) fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
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

/// Primitive FHIR type names start with a lowercase letter.
pub(crate) fn is_primitive(ty: &str) -> bool {
    ty.starts_with(|c: char| c.is_ascii_lowercase())
}
