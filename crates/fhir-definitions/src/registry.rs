use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::Result;
use crate::loader;
use crate::model::{CodeSystem, ConceptMap, Include, StructureDefinition, ValueSet};
use crate::type_code::capitalise;

mod navigation;
mod overlay;
mod terminology;
mod terminology_service;

/// Canonical URL prefix of the core definitions; a type code appended gives its URL.
pub const CORE_PREFIX: &str = "http://hl7.org/fhir/StructureDefinition/";

/// Deepest chain of ValueSet imports followed; guards against cyclic imports.
const MAX_IMPORT_DEPTH: u8 = 8;

/// Indexed registry of FHIR R5 definitions, built once and shared via `Arc`.
///
/// Keyed by canonical URL for all entry types. An overlay built by
/// [`Registry::with_extra`] holds supplied definitions and falls back to its base
/// for every lookup except [`Registry::core`], which reads only the base so supplied
/// files never create a type. No registry is ever mutated after it is built.
pub struct Registry {
    structure_definitions: HashMap<Arc<str>, StructureDefinition>,
    value_sets: HashMap<Arc<str>, ValueSet>,
    code_systems: HashMap<Arc<str>, CodeSystem>,
    concept_maps: HashMap<Arc<str>, ConceptMap>,
    /// Choice element base name (`value` for `*.value[x]`) to its capitalised type suffixes.
    choice_suffixes: HashMap<Arc<str>, HashSet<String>>,
    /// StructureDefinitions loaded without a snapshot, kept out of `structure_definitions`.
    needs_snapshot: HashSet<Arc<str>>,
    /// The registry this overlay extends; `None` for a registry loaded from a directory.
    base: Option<Arc<Registry>>,
}

impl Registry {
    /// Load all FHIR R5 definitions from a directory that contains the standard
    /// FHIR definition JSON files:
    ///
    /// - `profiles-resources.json`
    /// - `profiles-types.json`
    /// - `profiles-others.json` (core profiles such as `vitalsigns`)
    /// - `valuesets.json`
    /// - `conceptmaps.json`
    ///
    /// A missing file is an [`Error::Io`](crate::Error::Io).
    pub fn from_definitions_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();

        let mut defs = loader::Definitions::default();

        for file in &[
            "profiles-resources.json",
            "profiles-types.json",
            "profiles-others.json",
            "valuesets.json",
            "conceptmaps.json",
        ] {
            let path = dir.join(file);
            loader::load_bundle(&path, &mut defs)?;
        }

        Ok(Self::from_loaded(defs, None))
    }

    fn from_loaded(defs: loader::Definitions, base: Option<Arc<Registry>>) -> Self {
        Self {
            choice_suffixes: index_choice_suffixes(&defs.structure_definitions),
            structure_definitions: defs.structure_definitions,
            value_sets: defs.value_sets,
            code_systems: defs.code_systems,
            concept_maps: defs.concept_maps,
            needs_snapshot: defs.needs_snapshot,
            base,
        }
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
        self.structure_definitions
            .get(url)
            .or_else(|| self.base.as_ref()?.structure_definition(url))
    }

    /// Whether a StructureDefinition with this canonical URL was loaded without a
    /// snapshot. Such a definition is not returned by any lookup.
    pub fn needs_snapshot(&self, url: &str) -> bool {
        self.needs_snapshot.contains(url)
            || self
                .base
                .as_ref()
                .is_some_and(|base| base.needs_snapshot(url))
    }

    /// Number of indexed StructureDefinitions across every layer.
    fn structure_definition_total(&self) -> usize {
        self.structure_definitions.len()
            + self
                .base
                .as_ref()
                .map_or(0, |base| base.structure_definition_total())
    }

    /// Look up a core `StructureDefinition` by name (`Patient`, `string`): the
    /// definition whose canonical URL is [`CORE_PREFIX`] followed by `name`.
    ///
    /// Only the base definitions are searched, never an overlay: in R5 only a
    /// specialization defines a type, and supplied definitions are constraints, so
    /// a supplied file at a core-namespace URL never creates a type.
    pub fn core(&self, name: &str) -> Option<&StructureDefinition> {
        self.root()
            .structure_definitions
            .get(format!("{CORE_PREFIX}{name}").as_str())
    }

    /// The registry loaded from a directory, under every overlay.
    fn root(&self) -> &Registry {
        self.base.as_deref().map_or(self, Registry::root)
    }

    /// The `regex` extension R5 publishes on the `<code>.value` element of the primitive type
    /// `code` (`uri` gives `\S*`). `None` for complex types, resources, unknown codes and
    /// `xhtml`, which publishes none.
    ///
    /// The text is kept exactly as published; the caller must:
    /// - anchor it: R5 `datatypes.html` says the regexes "should be qualified with start of
    ///   string and end of string anchors", so wrap it as `\A(?:...)\z` (`string` and
    ///   `markdown` already carry `^...$`);
    /// - not trust the `decimal` one: its exponent `[0-9]{1,9}}` has a stray `}` (a typo in
    ///   R5), so an engine either rejects the pattern or makes every exponent end in `}`.
    pub fn primitive_regex(&self, code: &str) -> Option<&str> {
        self.core(code)?.value_regex.as_deref()
    }

    /// The type `code` and its ancestors, nearest first, by following `baseDefinition`:
    /// `code` gives `code`, `string`, `PrimitiveType`, `DataType`, `Element`, `Base`.
    /// Each ancestor is named by its canonical URL minus `http://hl7.org/fhir/StructureDefinition/`
    /// (the inverse of the lookup). Empty for an unknown `code`; the walk stops at a base outside
    /// the core namespace or not loaded, and after as many steps as there are definitions, so a
    /// cyclic chain cannot loop.
    pub fn type_ancestors<'a>(&'a self, code: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        let first = self.core(code).map(|sd| (code, sd));
        std::iter::successors(first, |(_, sd)| {
            let parent = sd.base_definition.as_deref()?.strip_prefix(CORE_PREFIX)?;
            Some((parent, self.core(parent)?))
        })
        .take(self.structure_definition_total())
        .map(|(code, _)| code)
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
            || self
                .base
                .as_ref()
                .is_some_and(|registry| registry.is_choice_variant(base, suffix))
    }

    /// Look up a `ValueSet` by its canonical URL.
    pub fn value_set(&self, url: &str) -> Option<&ValueSet> {
        self.value_sets
            .get(url)
            .or_else(|| self.base.as_ref()?.value_set(url))
    }

    /// Look up a `CodeSystem` by its canonical URL.
    pub(crate) fn code_system(&self, url: &str) -> Option<&CodeSystem> {
        self.code_systems
            .get(url)
            .or_else(|| self.base.as_ref()?.code_system(url))
    }

    /// Whether `code` (optionally qualified by `system`) belongs to the ValueSet.
    ///
    /// Returns `None` when membership cannot be decided from the loaded
    /// definitions (unknown ValueSet/CodeSystem, no `compose.include`, filters,
    /// excludes, fragments).
    /// With `system == None` the code's system is unknown: `Some(false)` still
    /// holds when no system has the code, but `Some(true)` is given only when the
    /// ValueSet draws its codes from a single system (else `None`, as the code
    /// could belong to another system).
    pub fn value_set_contains(&self, url: &str, system: Option<&str>, code: &str) -> Option<bool> {
        let found = self.vs_contains(url, system, code, 0);
        if found == Some(true) && system.is_none() {
            let mut systems = HashSet::new();
            if !self.collect_systems(url, 0, &mut systems) || systems.len() > 1 {
                return None;
            }
        }
        found
    }

    /// Gathers the code systems a ValueSet includes; `false` when they cannot be told
    /// (unknown ValueSet, one with no `compose.include`, or an include with no system and no
    /// ValueSet).
    fn collect_systems<'a>(&'a self, url: &str, depth: u8, systems: &mut HashSet<&'a str>) -> bool {
        let Some(vs) = self.value_set(url).filter(|_| depth <= MAX_IMPORT_DEPTH) else {
            return false;
        };
        if vs.includes.is_empty() {
            return false;
        }
        vs.includes.iter().all(|inc| {
            systems.extend(inc.system.as_deref());
            let nested = inc
                .value_sets
                .iter()
                .all(|nested| self.collect_systems(nested, depth + 1, systems));
            nested && (inc.system.is_some() || !inc.value_sets.is_empty())
        })
    }

    fn vs_contains(&self, url: &str, system: Option<&str>, code: &str, depth: u8) -> Option<bool> {
        // Depth cap guards against cyclic ValueSet imports.
        if depth > MAX_IMPORT_DEPTH {
            return None;
        }
        let vs = self.value_set(url)?;
        // No compose.include (compose is 0..1 in R5): the codes are not known, not empty.
        if vs.includes.is_empty() {
            return None;
        }
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
        let cs = self.include_code_system(inc_system, inc);
        let found = if inc.concepts.is_empty() {
            let cs = cs?;
            if !cs.codes.contains(code) {
                return cs.complete.then_some(false);
            }
            true
        } else {
            inc.concepts.iter().any(|c| c.code.as_ref() == code)
        };
        // R5 forbids selecting notSelectable codes only in expansions; from a compose
        // membership is undecidable.
        if found && cs.is_some_and(|cs| cs.not_selectable.contains(code)) {
            return None;
        }
        Some(found)
    }

    /// The loaded CodeSystem `system` an include draws from. An include pinned to
    /// another CodeSystem version ("*" means any) cannot use the loaded one: treated
    /// as if the CodeSystem were not loaded.
    fn include_code_system(&self, system: &str, inc: &Include) -> Option<&CodeSystem> {
        self.code_system(system).filter(|cs| {
            inc.version
                .as_deref()
                .is_none_or(|v| v == "*" || cs.version.as_deref() == Some(v))
        })
    }

    /// Total number of indexed `StructureDefinition` entries.
    #[cfg(test)]
    pub(crate) fn structure_definition_count(&self) -> usize {
        self.structure_definitions.len()
    }

    /// Every indexed `StructureDefinition`.
    #[cfg(test)]
    pub(crate) fn structure_definitions(&self) -> impl Iterator<Item = &StructureDefinition> {
        self.structure_definitions.values()
    }

    /// Every snapshot element of every indexed `StructureDefinition`.
    #[cfg(test)]
    pub(crate) fn snapshot_elements(
        &self,
    ) -> impl Iterator<Item = &crate::model::ElementDefinition> {
        self.structure_definitions
            .values()
            .flat_map(|sd| sd.snapshot.iter())
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

#[cfg(test)]
mod tests;
