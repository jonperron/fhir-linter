//! Element lookups by definition or instance path, for typing FHIRPath values
//! and for strict navigation.
use std::sync::Arc;

use super::Registry;
use crate::model::{ElementDefinition, StructureDefinition};
use crate::type_code::capitalise;

impl Registry {
    /// Whether the FHIR type `type_name` declares an element `name`, as FHIRPath
    /// names it: a choice element without its `[x]` (`value`), never with a type
    /// suffix (`valueQuantity`). `None` when the type is unknown, or its children
    /// are defined at the owning element's path (`BackboneElement`, `Element`;
    /// see [`Registry::child_element`]), so the type alone cannot tell.
    ///
    /// `type_name` may instead be the instance path of a backbone element from
    /// its resource (`Patient.contact`, `Questionnaire.item.item`): each step is
    /// resolved as [`Registry::child_element`] does, so through complex types and
    /// `contentReference`. `None` when the path names no element.
    pub fn has_element(&self, type_name: &str, name: &str) -> Option<bool> {
        let resolved = match type_name.split_once('.') {
            Some((resource, steps)) => Some(
                steps
                    .split('.')
                    .try_fold(Arc::from(resource), |parent, step| {
                        Some(self.child_element(&parent, step)?.0)
                    })?,
            ),
            None => None,
        };
        let parent = resolved.as_deref().unwrap_or(type_name);
        let (sd, root) = match parent.split_once('.') {
            // A backbone's children are listed under its definition path.
            Some((owner, _)) => (self.core(owner)?, parent),
            None if matches!(parent, "BackboneElement" | "Element") => return None,
            None => {
                let sd = self.core(parent)?;
                // A constraint (`SimpleQuantity`) lists its elements under the base type's path.
                (sd, &*sd.snapshot.first()?.path)
            }
        };
        // `value[x]` or `a.b` is a definition path, not an element name.
        let declared = !name.contains(['.', '[', ']'])
            && (sd.element(&format!("{root}.{name}")).is_some()
                || sd.element(&format!("{root}.{name}[x]")).is_some());
        Some(declared)
    }

    /// The child `name` of the element at definition path `parent` (a type name
    /// such as `Patient` or `HumanName`, or a backbone path such as
    /// `Patient.contact`), as its definition path and its type code for this
    /// instance (a choice suffix picks the type: `valueDate` gives `date`).
    ///
    /// The definition path is the element's own path for `BackboneElement` and
    /// `Element`, otherwise the type code. A `contentReference` element takes
    /// its target's (`Questionnaire.item.item` gives `Questionnaire.item`).
    /// `None` when the element, or the target of its reference, is unknown or
    /// has no type.
    pub fn child_element(&self, parent: &str, name: &str) -> Option<(Arc<str>, Arc<str>)> {
        let type_name = parent.split('.').next()?;
        let sd = self.core(type_name)?;
        let (element, ty) = match sd.element(&format!("{parent}.{name}")) {
            Some(element) => {
                let element = match element.content_reference.as_deref() {
                    Some(reference) => self.content_target(sd, reference)?,
                    None => element,
                };
                (element, element.types.first()?)
            }
            // A type suffix is capitalised, so a choice can only split before an uppercase letter.
            None => (1..name.len())
                .filter(|&i| name.as_bytes()[i].is_ascii_uppercase())
                .find_map(|i| {
                    let (base, suffix) = name.split_at(i);
                    let element = sd.element(&format!("{parent}.{base}[x]"))?;
                    let ty = element.types.iter().find(|t| capitalise(t) == suffix)?;
                    Some((element, ty))
                })?,
        };
        let definition = match ty.as_ref() {
            "BackboneElement" | "Element" => Arc::clone(&element.path),
            _ => Arc::clone(ty),
        };
        Some((definition, Arc::clone(ty)))
    }

    /// The element a `contentReference` of `sd` names: a local `Questionnaire.item`
    /// (the loader drops the `#`), or `http://.../StructureDefinition/X#X.path`.
    fn content_target<'a>(
        &'a self,
        sd: &'a StructureDefinition,
        reference: &str,
    ) -> Option<&'a ElementDefinition> {
        let (target, path) = match reference.split_once('#') {
            Some((url, path)) => (self.structure_definition(url)?, path),
            None => (sd, reference),
        };
        target.element(path)
    }
}
