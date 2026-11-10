use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::Result;
use crate::loader;
use crate::model::{CodeSystem, Include, StructureDefinition, ValueSet};

/// Indexed registry of FHIR R5 definitions, built once and shared via `Arc`.
///
/// Keyed by canonical URL for all entry types.
pub struct Registry {
    structure_definitions: HashMap<Arc<str>, StructureDefinition>,
    value_sets: HashMap<Arc<str>, ValueSet>,
    code_systems: HashMap<Arc<str>, CodeSystem>,
    /// Choice element base name (`value` for `*.value[x]`) to its capitalised type suffixes.
    choice_suffixes: HashMap<Arc<str>, HashSet<String>>,
}

impl Registry {
    /// Load all FHIR R5 definitions from a directory that contains the standard
    /// FHIR definition JSON files:
    ///
    /// - `profiles-resources.json`
    /// - `profiles-types.json`
    /// - `profiles-others.json` (core profiles such as `vitalsigns`)
    /// - `valuesets.json`
    pub fn from_definitions_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();

        let mut structure_definitions = HashMap::new();
        let mut value_sets = HashMap::new();
        let mut code_systems = HashMap::new();

        for file in &[
            "profiles-resources.json",
            "profiles-types.json",
            "profiles-others.json",
        ] {
            let path = dir.join(file);
            loader::load_bundle(
                &path,
                &mut structure_definitions,
                &mut value_sets,
                &mut code_systems,
            )?;
        }

        let valuesets_path = dir.join("valuesets.json");
        loader::load_bundle(
            &valuesets_path,
            &mut structure_definitions,
            &mut value_sets,
            &mut code_systems,
        )?;

        let choice_suffixes = index_choice_suffixes(&structure_definitions);
        Ok(Self {
            structure_definitions,
            value_sets,
            code_systems,
            choice_suffixes,
        })
    }

    /// Load the registry for `dir` once and share it.
    ///
    /// Registries are expensive to build (~60 MB of JSON), so calls with the same
    /// directory, however spelled (`defs`, `./defs`, a symlink), return the same
    /// `Arc`. Failed loads are not cached. Entries are never evicted. The cache
    /// belongs to each linked copy of this crate: two separately built native
    /// modules (e.g. the Python and Node bindings) in one process keep their own.
    pub fn shared(dir: impl AsRef<Path>) -> Result<Arc<Self>> {
        static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<Registry>>>> = OnceLock::new();

        let dir = fs::canonicalize(dir)?;
        // The lock is held while loading so concurrent callers never build twice.
        let mut cache = CACHE
            .get_or_init(Mutex::default)
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(registry) = cache.get(&dir) {
            return Ok(Arc::clone(registry));
        }
        let registry = Arc::new(Self::from_definitions_dir(&dir)?);
        cache.insert(dir, Arc::clone(&registry));
        Ok(registry)
    }

    /// Look up a `StructureDefinition` by its canonical URL.
    pub fn structure_definition(&self, url: &str) -> Option<&StructureDefinition> {
        self.structure_definitions.get(url)
    }

    /// Whether `base` + `suffix` (`value` + `Quantity`) names a variant of a choice
    /// element: some loaded definition declares `base[x]` with a type whose
    /// capitalised code is `suffix`.
    ///
    /// shortcut: matched by element name across every definition, not by path, so
    /// a plain element such as `Immunization.doseQuantity` is found via `dose`
    /// (`dose[x]` is declared by Dosage); no bundled root invariant hits such a
    /// name today. Make the FHIRPath evaluator path-aware when nested invariants run.
    pub fn is_choice_variant(&self, base: &str, suffix: &str) -> bool {
        self.choice_suffixes
            .get(base)
            .is_some_and(|suffixes| suffixes.contains(suffix))
    }

    /// Look up a `ValueSet` by its canonical URL.
    pub fn value_set(&self, url: &str) -> Option<&ValueSet> {
        self.value_sets.get(url)
    }

    /// Look up a `CodeSystem` by its canonical URL.
    #[cfg(test)]
    pub(crate) fn code_system(&self, url: &str) -> Option<&CodeSystem> {
        self.code_systems.get(url)
    }

    /// Whether `code` (optionally qualified by `system`) belongs to the ValueSet.
    ///
    /// Returns `None` when membership cannot be decided from the loaded
    /// definitions (unknown ValueSet/CodeSystem, filters, excludes, fragments).
    /// With `system == None` any system of the ValueSet may match.
    pub fn value_set_contains(&self, url: &str, system: Option<&str>, code: &str) -> Option<bool> {
        self.vs_contains(url, system, code, 0)
    }

    fn vs_contains(&self, url: &str, system: Option<&str>, code: &str, depth: u8) -> Option<bool> {
        // Depth cap guards against cyclic ValueSet imports.
        if depth > 8 {
            return None;
        }
        let vs = self.value_sets.get(url)?;
        let mut undecided = false;
        for inc in &vs.includes {
            match self.include_contains(inc, system, code, depth) {
                Some(true) => return if vs.has_exclude { None } else { Some(true) },
                Some(false) => {}
                None => undecided = true,
            }
        }
        if undecided { None } else { Some(false) }
    }

    fn include_contains(
        &self,
        inc: &Include,
        system: Option<&str>,
        code: &str,
        depth: u8,
    ) -> Option<bool> {
        if inc.has_filter {
            return None;
        }
        if !inc.value_sets.is_empty() {
            let mut result = Some(true);
            for vs in &inc.value_sets {
                match self.vs_contains(vs, system, code, depth + 1) {
                    Some(false) => return Some(false),
                    None => result = None,
                    Some(true) => {}
                }
            }
            if inc.system.is_none() {
                return result;
            }
            // Both system and valueSet: the system check below can only narrow a
            // positive answer, so an undecided intersection stays undecided.
            result?;
        }
        let inc_system = inc.system.as_deref()?;
        if system.is_some_and(|s| s != inc_system) {
            return Some(false);
        }
        if !inc.codes.is_empty() {
            return Some(inc.codes.iter().any(|c| c.as_ref() == code));
        }
        let cs = self.code_systems.get(inc_system)?;
        if cs.codes.contains(code) {
            Some(true)
        } else if cs.complete {
            Some(false)
        } else {
            None
        }
    }

    /// Total number of indexed `StructureDefinition` entries.
    #[cfg(test)]
    pub(crate) fn structure_definition_count(&self) -> usize {
        self.structure_definitions.len()
    }

    /// Total number of indexed `ValueSet` entries.
    #[cfg(test)]
    pub(crate) fn value_set_count(&self) -> usize {
        self.value_sets.len()
    }

    /// Total number of indexed `CodeSystem` entries.
    #[cfg(test)]
    pub(crate) fn code_system_count(&self) -> usize {
        self.code_systems.len()
    }
}

/// Map each choice element's base name to the capitalised codes of its declared types.
fn index_choice_suffixes(
    structure_definitions: &HashMap<Arc<str>, StructureDefinition>,
) -> HashMap<Arc<str>, HashSet<String>> {
    let mut index: HashMap<Arc<str>, HashSet<String>> = HashMap::new();
    for element in structure_definitions.values().flat_map(|sd| &sd.snapshot) {
        let name = element.path.rsplit('.').next().unwrap_or(&element.path);
        let Some(base) = name.strip_suffix("[x]") else {
            continue;
        };
        let suffixes = index.entry(Arc::from(base)).or_default();
        suffixes.extend(element.types.iter().map(|code| capitalise(code)));
    }
    index
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;

    fn definitions_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5")
    }

    // Shared with the `shared_*` tests so the test binary loads the definitions once.
    static REGISTRY: LazyLock<Arc<Registry>> =
        LazyLock::new(|| Registry::shared(definitions_dir()).unwrap());

    #[test]
    fn registry_structure_definition_lookup_is_case_sensitive() {
        let registry = &*REGISTRY;
        assert!(
            registry
                .structure_definition("http://hl7.org/fhir/StructureDefinition/patient")
                .is_none()
        );
    }

    #[test]
    fn choice_variants_come_from_declared_choice_types() {
        assert!(REGISTRY.is_choice_variant("value", "Quantity"));
        assert!(REGISTRY.is_choice_variant("value", "DateTime"));
        assert!(REGISTRY.is_choice_variant("age", "Age"));
        // `modifierExtension` is a plain element, not a variant of `modifier[x]`.
        assert!(!REGISTRY.is_choice_variant("modifier", "Extension"));
        assert!(!REGISTRY.is_choice_variant("value", "Element"));
    }

    #[test]
    fn registry_loads_core_profiles_from_profiles_others() {
        assert!(
            REGISTRY
                .structure_definition("http://hl7.org/fhir/StructureDefinition/vitalsigns")
                .is_some()
        );
    }

    #[test]
    fn registry_loads_patient_structure_definition() {
        let registry = &*REGISTRY;
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
            .expect("Patient StructureDefinition should be present");
        assert_eq!(sd.name.as_ref(), "Patient");
        assert_eq!(sd.kind.as_ref(), "resource");
        assert!(!sd.snapshot.is_empty(), "snapshot should have elements");
    }

    #[test]
    fn registry_resolves_valueset_by_url() {
        let registry = &*REGISTRY;
        let vs = registry
            .value_set("http://hl7.org/fhir/ValueSet/administrative-gender")
            .expect("administrative-gender ValueSet should be present");
        assert_eq!(
            vs.url.as_ref(),
            "http://hl7.org/fhir/ValueSet/administrative-gender"
        );
    }

    #[test]
    fn registry_loads_multiple_structure_definitions() {
        let registry = &*REGISTRY;
        assert!(
            registry.structure_definition_count() > 100,
            "should load more than 100 StructureDefinitions, got {}",
            registry.structure_definition_count()
        );
    }

    #[test]
    fn registry_loads_code_systems() {
        let registry = &*REGISTRY;
        assert!(
            registry.code_system_count() > 0,
            "should load at least one CodeSystem"
        );
    }

    #[test]
    fn patient_snapshot_contains_identifier_element() {
        let registry = &*REGISTRY;
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
            .unwrap();
        let identifier = sd
            .snapshot
            .iter()
            .find(|e| e.path.as_ref() == "Patient.identifier");
        assert!(
            identifier.is_some(),
            "snapshot should contain Patient.identifier"
        );
        let identifier = identifier.unwrap();
        assert_eq!(identifier.min, 0);
        assert_eq!(identifier.max.as_ref(), "*");
    }

    #[test]
    fn registry_returns_none_for_unknown_url() {
        let registry = &*REGISTRY;
        assert!(
            registry
                .structure_definition("http://example.com/unknown")
                .is_none()
        );
        assert!(registry.value_set("http://example.com/unknown").is_none());
        assert!(registry.code_system("http://example.com/unknown").is_none());
    }

    #[test]
    fn structure_definition_has_base_definition() {
        let registry = &*REGISTRY;
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
            .unwrap();
        assert_eq!(
            sd.base_definition.as_deref(),
            Some("http://hl7.org/fhir/StructureDefinition/DomainResource")
        );
    }

    #[test]
    fn registry_value_set_count_is_positive() {
        let registry = &*REGISTRY;
        assert!(
            registry.value_set_count() > 0,
            "should load at least one ValueSet, got {}",
            registry.value_set_count()
        );
    }

    #[test]
    fn registry_code_system_has_url_and_name() {
        let registry = &*REGISTRY;
        let cs = registry
            .code_system("http://hl7.org/fhir/administrative-gender")
            .expect("administrative-gender CodeSystem should be present");
        assert_eq!(cs.url.as_ref(), "http://hl7.org/fhir/administrative-gender");
        assert!(!cs.name.is_empty(), "CodeSystem name should not be empty");
    }

    #[test]
    fn registry_value_set_has_name() {
        let registry = &*REGISTRY;
        let vs = registry
            .value_set("http://hl7.org/fhir/ValueSet/administrative-gender")
            .expect("administrative-gender ValueSet should be present");
        assert!(!vs.name.is_empty(), "ValueSet name should not be empty");
    }

    #[test]
    fn element_definition_types_are_populated() {
        let registry = &*REGISTRY;
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
            .unwrap();
        let id_elem = sd
            .snapshot
            .iter()
            .find(|e| e.path.as_ref() == "Patient.identifier")
            .expect("Patient.identifier element should exist");
        assert!(
            !id_elem.types.is_empty(),
            "Patient.identifier should have at least one type"
        );
        assert!(
            id_elem.types.iter().any(|t| t.as_ref() == "Identifier"),
            "Patient.identifier should have type 'Identifier', got {:?}",
            id_elem.types
        );
    }

    #[test]
    fn structure_definition_is_abstract_for_resource_type() {
        let registry = &*REGISTRY;
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Resource")
            .expect("Resource StructureDefinition should be present");
        assert!(
            sd.is_abstract,
            "Resource StructureDefinition should be abstract"
        );
    }

    #[test]
    fn structure_definition_is_not_abstract_for_patient() {
        let registry = &*REGISTRY;
        let sd = registry
            .structure_definition("http://hl7.org/fhir/StructureDefinition/Patient")
            .unwrap();
        assert!(
            !sd.is_abstract,
            "Patient StructureDefinition should not be abstract"
        );
    }

    #[test]
    fn registry_from_nonexistent_dir_returns_error() {
        let result = Registry::from_definitions_dir("/nonexistent/path/that/does/not/exist");
        assert!(
            result.is_err(),
            "loading from a nonexistent dir should fail"
        );
    }

    #[test]
    fn shared_returns_same_registry_for_equivalent_paths() {
        let dir = definitions_dir();
        let first = Registry::shared(&dir).unwrap();
        let second = Registry::shared(dir.join(".")).unwrap();
        let third = Registry::shared(fs::canonicalize(&dir).unwrap()).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(Arc::ptr_eq(&first, &third));
    }

    #[test]
    fn shared_from_nonexistent_dir_returns_error() {
        assert!(Registry::shared("/nonexistent/path/that/does/not/exist").is_err());
    }
}
