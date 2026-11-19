//! Conformance to a StructureDefinition, for FHIRPath `conformsTo()`.

use crate::Registry;

impl Registry {
    /// Whether an instance of the FHIR type `fhir_type` conforms to the
    /// StructureDefinition at `url`, as far as its type tells:
    /// - `None` when no loaded definition has that URL (fhirpath.html: the
    ///   structure "cannot be resolved to a valid profile");
    /// - `Some(None)` when the definition is a constraint (a profile such as
    ///   `vitalsigns` or `SimpleQuantity`), whose rules the type alone cannot check;
    /// - `Some(Some(conforms))` for a base definition: whether it defines `fhir_type`
    ///   itself. A Patient conforms to `Patient`, not to `DomainResource`, whose
    ///   definition declares none of Patient's elements.
    ///
    /// A base definition is the core definition of the type its snapshot root names;
    /// a constraint's root names the type it constrains (`vitalsigns` has `Observation`).
    ///
    /// shortcut: the instance's content is not validated against the definition, only
    /// its type is compared. Upgrade when `conformsTo()` runs a nested validation.
    pub fn conforms_to(&self, fhir_type: &str, url: &str) -> Option<Option<bool>> {
        let sd = self.structure_definition(url)?;
        let defined_type = sd.snapshot.first().map(|root| &*root.path);
        let is_base = defined_type
            .and_then(|defined_type| self.core(defined_type))
            .is_some_and(|core| core.url == sd.url);
        Some(is_base.then(|| defined_type == Some(fhir_type)))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::LazyLock;

    use crate::{CORE_PREFIX, Registry};

    static REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).unwrap()
    });

    fn conforms_to(fhir_type: &str, name: &str) -> Option<Option<bool>> {
        REGISTRY.conforms_to(fhir_type, &format!("{CORE_PREFIX}{name}"))
    }

    #[test]
    fn conforms_to_its_own_base_definition() {
        assert_eq!(conforms_to("Patient", "Patient"), Some(Some(true)));
        assert_eq!(conforms_to("HumanName", "HumanName"), Some(Some(true)));
        assert_eq!(conforms_to("code", "code"), Some(Some(true)));
    }

    #[test]
    fn does_not_conform_to_another_base_definition() {
        assert_eq!(conforms_to("Patient", "Person"), Some(Some(false)));
        assert_eq!(conforms_to("Patient", "DomainResource"), Some(Some(false)));
        assert_eq!(conforms_to("code", "string"), Some(Some(false)));
    }

    #[test]
    fn a_constraint_cannot_be_checked_by_type() {
        assert_eq!(conforms_to("Quantity", "SimpleQuantity"), Some(None));
        assert_eq!(conforms_to("Observation", "vitalsigns"), Some(None));
    }

    #[test]
    fn an_unknown_url_is_unresolved() {
        assert_eq!(REGISTRY.conforms_to("Patient", "http://trash"), None);
    }
}
